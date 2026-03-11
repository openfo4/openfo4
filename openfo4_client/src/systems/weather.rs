use std::sync::Arc;

use game_common::components::{AmbientLight, Color};
use game_common::utils::vec_ext::VecExt;
use game_core_pipeline::entities::Entities;
use game_core_pipeline::sky::SkyMaterial;
use game_render::backend::TextureFormat;
use game_render::mesh::{Indices, Mesh};
use game_render::texture::Image;
use game_render::texture::image::MipImage;
use game_tasks::TaskPool;
use glam::{FloatExt, Quat, Vec3, Vec4};
use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_dds::Format;
use openfo4_esp::records::{ClimateTime, Rgba, TimeOfDaySetExtended};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_nif::{Block, Nif};

use crate::assets::data_file::DataFile;
use crate::components::{Player, WorldSpace};
use crate::shared::SharedState;
use crate::state::esp::EspLoader;
use crate::state::hardcoded::{HARDCODED_MODULE_ID, PATH_SKY_ATMOSPHERE};
use crate::systems::time::GameTime;

pub struct WeatherSystem {
    climates: HashMap<RecordId, WorldspaceClimate>,
    ambient_light: Option<Entity>,
    sun_light: Option<Entity>,
    entities: Entities,
}

impl WeatherSystem {
    pub fn new(
        esp_loader: Arc<EspLoader>,
        pool: Arc<TaskPool>,
        asset_loader: &AssetLoader,
        shared: &SharedState,
    ) -> Self {
        let mut climate = futures_lite::future::block_on(load_worldspace_weather_props(
            asset_loader,
            &esp_loader,
            WorldSpace {
                record: RecordId::new(HARDCODED_MODULE_ID, FormId(0x3c)),
                is_interior: false,
            },
        ));

        let mesh = futures_lite::future::block_on(load_sky_mesh(asset_loader));
        shared.entities.set_sky_mesh(mesh);

        if let Some(sun) = climate.sun.take() {
            shared.entities.set_sun_texture(sun);
        }

        shared.entities.set_sky_material(SkyMaterial {
            sky_lower: Color::WHITE,
            sky_upper: Color::RED,
            horizon: Color::GREEN,
            sun: Color::BLACK,
            sun_scale: 1.0,
            sun_position: Vec3::Y,
            sky_scale: 1.0,
        });

        let mut climates = HashMap::new();
        climates.insert(RecordId::new(HARDCODED_MODULE_ID, FormId(0x3c)), climate);

        Self {
            climates,
            ambient_light: None,
            sun_light: None,
            entities: shared.entities.clone(),
        }
    }

    pub fn update(&mut self, world: &mut World) {
        let Some(time) = world
            .query::<&GameTime>()
            .iter()
            .map(|v| v.1.clone())
            .next()
        else {
            return;
        };

        let Some((_, (_, &worldspace))) = world
            .query_mut::<(&Player, &WorldSpace)>()
            .into_iter()
            .next()
        else {
            return;
        };

        let Some(climate) = self.climates.get(&worldspace.record) else {
            return;
        };

        let weather = climate.weathers.first().unwrap();

        let Some(params) = weather.get_interpolated(&time) else {
            return;
        };

        if let Some(entity) = self.ambient_light {
            world.despawn(entity).unwrap();
        }

        self.entities.set_sky_material(SkyMaterial {
            sky_lower: params.sky_lower,
            sky_upper: params.sky_upper,
            horizon: params.horizon,
            sun: params.sun,
            sun_position: compute_sun_position(&time),
            sun_scale: params.sky_scale,
            sky_scale: params.sky_scale,
        });

        self.ambient_light = Some(world.spawn((
            AmbientLight {
                color: params.ambient,
                brightness: 1.0,
            },
            worldspace,
        )));
    }
}

struct WorldspaceClimate {
    sun: Option<MipImage>,
    weathers: Vec<Weather>,
    /// Time when the sun spawns in.
    sunrise: ClimateTime,
    /// Time when the sun despawns.
    sunset: ClimateTime,
}

#[derive(Clone, Debug)]
struct Weather {
    params: Vec<WeatherParams>,
    phases: Vec<WeatherPhase>,
}

impl Weather {
    fn get_interpolated(&self, time: &GameTime) -> Option<WeatherParams> {
        let hour = time.hour() as f32 + time.minute() as f32 / 60.0;

        let phase = self
            .phases
            .iter()
            .find(|phase| hour >= phase.min_hour && hour <= phase.max_hour)?;

        let s = (hour - phase.min_hour) / (phase.max_hour - phase.min_hour);

        let a = &self.params[phase.min_param];
        let b = &self.params[phase.max_param];
        Some(a.interpolate(b, s))
    }
}

#[derive(Copy, Clone, Debug)]
struct WeatherPhase {
    min_hour: f32,
    max_hour: f32,
    min_param: usize,
    max_param: usize,
}

#[derive(Clone, Debug)]
struct WeatherParams {
    ambient: Color,
    sky_lower: Color,
    sky_upper: Color,
    horizon: Color,
    sun: Color,
    sky_scale: f32,
}

impl WeatherParams {
    fn interpolate(&self, other: &Self, s: f32) -> Self {
        let ambient = self.ambient.lerp(other.ambient, s);
        let sky_lower = self.sky_lower.lerp(other.sky_lower, s);
        let sky_upper = self.sky_upper.lerp(other.sky_upper, s);
        let horizon = self.horizon.lerp(other.horizon, s);
        let sun = self.sun.lerp(other.sun, s);
        let sky_scale = self.sky_scale.lerp(other.sky_scale, s);

        Self {
            ambient,
            sky_lower,
            sky_upper,
            horizon,
            sun,
            sky_scale,
        }
    }
}

async fn load_sky_mesh(loader: &AssetLoader) -> Mesh {
    let file = loader
        .load::<DataFile<Nif>, _>(PATH_SKY_ATMOSPHERE)
        .await
        .unwrap();

    for block in file.iter() {
        match block {
            Block::BSTriShape(shape) => {
                let mut mesh = Mesh::new();
                mesh.set_positions(
                    shape
                        .mesh
                        .positions
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|v| v.to_opengl())
                        .collect(),
                );
                mesh.set_uvs(
                    shape
                        .mesh
                        .tex_coords
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_vec2())
                        .collect(),
                );
                mesh.set_colors(
                    shape
                        .mesh
                        .colors
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|v| {
                            Vec4::from_array([
                                v.r as f32 / 255.0,
                                v.g as f32 / 255.0,
                                v.b as f32 / 255.0,
                                1.0,
                            ])
                        })
                        .collect(),
                );

                mesh.set_indices(Indices::U32(
                    shape
                        .mesh
                        .indices
                        .iter()
                        .map(|v| v.map(|v| v as u32))
                        .flatten()
                        .collect(),
                ));

                return mesh;
            }
            _ => {}
        }
    }

    todo!("no BSTriShape in atmosphere.nif")
}

async fn load_worldspace_weather_props(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    worldspace: WorldSpace,
) -> WorldspaceClimate {
    let worldspace_record = esp_loader
        .lookup(Some(RecordKind::Worldspace), worldspace.record)
        .await
        .unwrap();
    let RecordData::Worldspace(worldspace) = &worldspace_record.local.data else {
        todo!();
    };

    let Some(climate) = &worldspace.climate else {
        todo!();
    };
    let climate = esp_loader
        .resolve_form(*climate, worldspace_record.id)
        .unwrap();

    let climate_record = esp_loader
        .lookup(Some(RecordKind::Climate), climate)
        .await
        .unwrap();
    let RecordData::Climate(climate) = climate_record.local.data else {
        todo!();
    };

    let mut sun = None;
    if let Some(path) = &climate.sun_texture {
        let mut path = DataPathBuf::new(path.as_bytes());
        path.push_front("textures");
        let img = loader
            .load::<DataFile<openfo4_dds::Image>, _>(&path)
            .await
            .unwrap();

        let mips = img
            .mips
            .iter()
            .map(|mip| {
                let format = match img.format {
                    Format::Rgba8Unorm => TextureFormat::Rgba8Unorm,
                    Format::Rgba8UnormSrgb => TextureFormat::Rgba8UnormSrgb,
                    Format::Bc3Unorm => TextureFormat::Bc3RgbaUnorm,
                    _ => todo!(),
                };

                Image::new(mip.size, format, mip.bytes.clone())
            })
            .collect();

        sun = Some(MipImage::new(mips));
    }

    let mut weathers = Vec::new();
    for weather in &climate.weathers {
        assert!(
            weather.global.is_null(),
            "TODO: Weather globals are not implemented"
        );

        let weather_id = esp_loader
            .resolve_form(weather.weather, climate_record.id)
            .unwrap();

        let RecordData::Weather(weather) = esp_loader
            .lookup(Some(RecordKind::Weather), weather_id)
            .await
            .unwrap()
            .local
            .data
        else {
            todo!();
        };

        fn to_params<F>(weather: &openfo4_esp::records::Weather, f: F) -> WeatherParams
        where
            F: Fn(TimeOfDaySetExtended<Rgba<u8>>) -> Rgba<u8>,
        {
            WeatherParams {
                ambient: rgb_to_color(f(weather.colors.ambient)),
                sky_lower: rgb_to_color(f(weather.colors.sky_lower)),
                sky_upper: rgb_to_color(f(weather.colors.sky_upper)),
                horizon: rgb_to_color(f(weather.colors.horizon)),
                sun: rgb_to_color(f(weather.colors.sun)),
                sky_scale: 1.0,
            }
        }

        let mut params = Vec::new();
        let p_night = params.push_and_index(to_params(&weather, |v| v.night));
        let p_day = params.push_and_index(to_params(&weather, |v| v.day));
        let p_early_sunrise = params.push_and_index(to_params(&weather, |v| v.early_sunrise));
        let p_sunrise = params.push_and_index(to_params(&weather, |v| v.sunrise));
        let p_late_sunrise = params.push_and_index(to_params(&weather, |v| v.late_sunrise));
        let p_early_sunset = params.push_and_index(to_params(&weather, |v| v.early_sunset));
        let p_sunset = params.push_and_index(to_params(&weather, |v| v.sunset));
        let p_late_sunset = params.push_and_index(to_params(&weather, |v| v.late_sunset));

        for (index, id) in [
            (p_night, weather.image_spaces.night),
            (p_day, weather.image_spaces.day),
            (p_early_sunrise, weather.image_spaces.early_sunrise),
            (p_sunrise, weather.image_spaces.sunrise),
            (p_late_sunrise, weather.image_spaces.late_sunrise),
            (p_early_sunset, weather.image_spaces.early_sunset),
            (p_sunset, weather.image_spaces.sunset),
            (p_late_sunset, weather.image_spaces.late_sunset),
        ] {
            let params = &mut params[index];

            let id = esp_loader.resolve_form(id, weather_id).unwrap();
            let image_space = esp_loader
                .lookup(Some(RecordKind::ImageSpace), id)
                .await
                .unwrap();

            let RecordData::ImageSpace(data) = image_space.local.data else {
                unreachable!();
            };

            if let Some(data) = data.data {
                params.sky_scale = data.sky_scale;
            }

            if let Some(data) = data.hdr {
                params.sky_scale = data.sky_scale;
            }
        }

        let sunrise_begin = climate.timing.sunrise_begin.hours();
        let sunrise_end = climate.timing.sunrise_end.hours();
        let sunset_begin = climate.timing.sunset_begin.hours();
        let sunset_end = climate.timing.sunset_end.hours();

        let sunrise_len = sunrise_end - sunrise_begin;
        let sunset_len = sunset_end - sunset_begin;

        let early_sunrise = (sunrise_begin - 1.5 + (0.25 * sunrise_len)).rem_euclid(24.0);
        let sunrise = (sunrise_begin - 1.0 + (0.5 * sunrise_len)).rem_euclid(24.0);
        let late_sunrise = (sunrise_end - 0.5 - (0.25 * sunrise_len)).rem_euclid(24.0);
        let early_sunset = (sunset_begin + 0.5 + (0.25 * sunset_len)).rem_euclid(24.0);
        let sunset = (sunset_begin + 1.0 + (0.5 * sunset_len)).rem_euclid(24.0);
        let late_sunset = (sunset_end + 1.5 - (0.25 * sunset_len)).rem_euclid(24.0);

        let sunrise_timestep = sunrise - early_sunrise;
        let sunset_timestep = sunset - early_sunset;

        // FIXME: Double check what happens if we go negative / over 24 here.
        let full_night_end = early_sunrise - sunrise_timestep;
        let full_day_start = late_sunrise + sunrise_timestep;
        let full_day_end = early_sunset - sunset_timestep;
        let full_night_start = late_sunset + sunset_timestep;

        let mut phases = Vec::new();

        for ([start, end], [p_start, p_end]) in [
            ([0.0, full_night_end], [p_night, p_night]),
            ([full_night_end, early_sunrise], [p_night, p_early_sunrise]),
            ([early_sunrise, sunrise], [p_early_sunrise, p_sunrise]),
            ([sunrise, late_sunrise], [p_sunrise, p_late_sunrise]),
            ([late_sunrise, full_day_start], [p_late_sunrise, p_day]),
            ([full_day_start, full_day_end], [p_day, p_day]),
            ([full_day_end, early_sunset], [p_day, p_early_sunset]),
            ([early_sunset, sunset], [p_early_sunset, p_sunset]),
            ([sunset, late_sunset], [p_sunset, p_late_sunset]),
            ([late_sunset, full_night_start], [p_late_sunset, p_night]),
            ([full_night_start, 24.0], [p_night, p_night]),
        ] {
            if end - start > 0.0 {
                phases.push(WeatherPhase {
                    min_hour: start,
                    max_hour: end,
                    min_param: p_start,
                    max_param: p_end,
                });
            }
        }

        weathers.push(Weather { params, phases });
    }

    WorldspaceClimate {
        sun,
        weathers,
        sunrise: climate.timing.sunrise_begin,
        sunset: climate.timing.sunset_begin,
    }
}

fn rgb_to_color(input: Rgba<u8>) -> Color {
    Color::from_rgb([
        input.r as f32 / 255.0,
        input.g as f32 / 255.0,
        input.b as f32 / 255.0,
    ])
}

// It seems like the sun position is not affected by the climate times
// and is always active from 7:00 AM to 7:00 PM.
fn compute_sun_position(time: &GameTime) -> Vec3 {
    // Somewhere in Boston
    let lat: f64 = 42.35675_f64.to_radians();
    // let long = -71.04111.to_radians();

    let mut hour = time.hour() as f64 + time.minute() as f64 / 60.0;

    // Tuned for sunrise at 7:00 AM and sunset at  7:00 PM.
    // This seems to replicate the values in game relatively closely.
    let day: f64 = 269.0;
    hour = (hour - 1.0).rem_euclid(24.0);

    let delta = 23.45_f64.to_radians() * (((360.0 / 365.0) * (day + 284.0)).to_radians()).sin();
    let h = (15.0 * (hour - 12.0)).to_radians();

    let altitude = (lat.sin() * delta.sin() + lat.cos() * delta.cos() * h.cos()).asin();
    let azimuth =
        ((delta.sin() - altitude.sin() * lat.sin()) / (altitude.cos() * lat.cos())).acos();

    let azimuth = if h.is_sign_negative() {
        azimuth
    } else {
        360f64.to_radians() - azimuth
    };

    // For our game world:
    // +X is east, +Z is south
    let mut dir = Quat::from_axis_angle(Vec3::Y, azimuth as f32)
        * Quat::from_axis_angle(Vec3::X, altitude as f32)
        * -Vec3::Z;
    dir.x *= -1.0;

    dir
}
