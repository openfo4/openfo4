use std::sync::Arc;
use std::time::Duration;

use game_common::components::{AmbientLight, Color, DirectionalLight, Transform};
use game_common::utils::vec_ext::VecExt;
use game_core_pipeline::entities::Entities;
use game_core_pipeline::sky::SkyMaterial;
use game_render::backend::TextureFormat;
use game_render::mesh::{Indices, Mesh};
use game_render::texture::Image;
use game_render::texture::image::MipImage;
use game_tasks::TaskPool;
use glam::{FloatExt, Quat, Vec2, Vec3, Vec4};
use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_dds::Format;
use openfo4_esp::records::{Rgba, TimeOfDaySetExtended};
use openfo4_esp::{FormId, RecordData, RecordKind};
use openfo4_nif::{Block, Nif};

use crate::assets::data_file::DataFile;
use crate::assets::texture::{FileImage, FileImageKey};
use crate::components::{Player, WorldSpace};
use crate::shared::SharedState;
use crate::state::esp::EspLoader;
use crate::state::hardcoded::{HARDCODED_MODULE_ID, PATH_SKY_ATMOSPHERE, PATH_SKY_CLOUDS};
use crate::systems::time::GameTime;

// The clear weather image space uses a sunlight scale of 4.5 for the day sun.
const SUNLIGHT_BASE_ILLUMINANCE: f32 = 40_000.0 / 4.5;

pub struct WeatherSystem {
    climates: HashMap<RecordId, WorldspaceClimate>,
    ambient_light: Option<Entity>,
    sun_light: Option<Entity>,
    entities: Entities,
    cloud_offsets: [Vec2; 32],
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

        let meshes = futures_lite::future::block_on(load_cloud_meshes(asset_loader));
        shared.entities.set_clouds_meshes(meshes);

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
            cloud_offsets: [Vec2::ZERO; 32],
        }
    }

    pub fn update(&mut self, world: &mut World, delta: Duration) {
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

        let Some(climate) = self.climates.get_mut(&worldspace.record) else {
            return;
        };

        let mut weather_changed = false;
        let active_weather = *climate.active_weather.get_or_insert_with(|| {
            weather_changed = true;
            0
        });

        let weather = &climate.weathers[active_weather];

        let Some(params) = weather.get_interpolated(&time) else {
            return;
        };

        let sun_position = compute_sun_position(&time);

        // Update sky
        self.entities.set_sky_material(SkyMaterial {
            sky_lower: params.sky_lower,
            sky_upper: params.sky_upper,
            horizon: params.horizon,
            sun: params.sun,
            sun_position,
            sun_scale: params.sky_scale,
            sky_scale: params.sky_scale,
        });

        // Update ambient lighting
        if let Some(entity) = self.ambient_light {
            world.despawn(entity).unwrap();
        }

        self.ambient_light = Some(world.spawn((
            AmbientLight {
                top: params.ambient_top,
                bottom: params.ambient_bottom,
                left: params.ambient_left,
                right: params.ambient_right,
                front: params.ambient_front,
                back: params.ambient_right,
                brightness: 1.0,
            },
            worldspace,
        )));

        // Update sun light
        if let Some(entity) = self.sun_light.take() {
            world.despawn(entity).unwrap();
        }

        self.sun_light = Some(world.spawn((
            Transform::from_translation(sun_position).looking_at(Vec3::ZERO, Vec3::Y),
            worldspace,
            DirectionalLight {
                color: params.sunlight,
                illuminance: SUNLIGHT_BASE_ILLUMINANCE * params.sunlight_scale,
            },
        )));

        // Update Clouds
        if weather_changed {
            for (layer, image) in weather.clouds.iter().enumerate() {
                self.entities.set_cloud_texture(layer, image.clone());
            }
        }

        for (layer, color) in params.cloud_colors.iter().enumerate() {
            self.entities.set_cloud_color(layer, *color);
        }

        for (layer, speed) in weather.cloud_speed.iter().enumerate() {
            let offset = &mut self.cloud_offsets[layer];
            *offset = (*offset + speed * delta.as_secs_f32()) % 1.0;

            self.entities.set_cloud_uv_offset(layer, *offset);
        }
    }
}

struct WorldspaceClimate {
    sun: Option<MipImage>,
    weathers: Vec<Weather>,
    active_weather: Option<usize>,
}

#[derive(Clone, Debug)]
struct Weather {
    params: Vec<WeatherParams>,
    phases: Vec<WeatherPhase>,
    clouds: [Option<MipImage>; 32],
    cloud_speed: [Vec2; 32],
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
    /// `+Z`
    ambient_top: Color,
    /// `-Z`
    ambient_bottom: Color,
    ambient_left: Color,
    ambient_right: Color,
    ambient_front: Color,
    ambient_back: Color,
    sky_lower: Color,
    sky_upper: Color,
    horizon: Color,
    /// Color of the sun
    sun: Color,
    sky_scale: f32,
    /// Color of the directional light of the sun.
    sunlight: Color,
    sunlight_scale: f32,
    cloud_colors: [Color; 32],
}

impl WeatherParams {
    fn interpolate(&self, other: &Self, s: f32) -> Self {
        let ambient_top = self.ambient_top.lerp(other.ambient_top, s);
        let ambient_bottom = self.ambient_bottom.lerp(other.ambient_bottom, s);
        let ambient_left = self.ambient_left.lerp(other.ambient_left, s);
        let ambient_right = self.ambient_right.lerp(other.ambient_right, s);
        let ambient_front = self.ambient_front.lerp(other.ambient_front, s);
        let ambient_back = self.ambient_back.lerp(other.ambient_back, s);

        let sky_lower = self.sky_lower.lerp(other.sky_lower, s);
        let sky_upper = self.sky_upper.lerp(other.sky_upper, s);
        let horizon = self.horizon.lerp(other.horizon, s);
        let sun = self.sun.lerp(other.sun, s);
        let sky_scale = self.sky_scale.lerp(other.sky_scale, s);
        let sunlight = self.sunlight.lerp(other.sunlight, s);
        let sunlight_scale = self.sunlight_scale.lerp(other.sunlight_scale, s);

        let mut cloud_colors = [Color::TRANSPARENT; 32];
        for i in 0..32 {
            cloud_colors[i] = self.cloud_colors[i].lerp(other.cloud_colors[i], s);
        }

        Self {
            ambient_top,
            ambient_bottom,
            ambient_left,
            ambient_right,
            ambient_front,
            ambient_back,
            sky_lower,
            sky_upper,
            horizon,
            sun,
            sky_scale,
            sunlight,
            sunlight_scale,
            cloud_colors,
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

async fn load_cloud_meshes(loader: &AssetLoader) -> Vec<Mesh> {
    let file = loader
        .load::<DataFile<Nif>, _>(PATH_SKY_CLOUDS)
        .await
        .unwrap();

    let mut meshes = Vec::new();
    for block in file.iter() {
        match block {
            Block::BSTriShape(shape) => {
                let transform = Transform {
                    translation: shape.translation,
                    rotation: Quat::from_mat3(&shape.rotation),
                    scale: Vec3::splat(shape.scale),
                };

                let positions = shape
                    .mesh
                    .positions
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|pos| transform.transform_point(*pos).to_opengl())
                    .collect();
                let colors = shape
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
                            v.a as f32 / 255.0,
                        ])
                    })
                    .collect();
                let uvs = shape
                    .mesh
                    .tex_coords
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_vec2())
                    .collect();

                let indices = Indices::U32(
                    shape
                        .mesh
                        .indices
                        .iter()
                        .map(|v| v.map(|v| v as u32))
                        .flatten()
                        .collect(),
                );

                let mut mesh = Mesh::new();
                mesh.set_positions(positions);
                mesh.set_colors(colors);
                mesh.set_uvs(uvs);
                mesh.set_indices(indices);

                meshes.push(mesh);
            }
            _ => {}
        }
    }

    meshes
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
        let img = loader
            .load::<FileImage, _>(&FileImageKey {
                path: DataPathBuf::new(path.as_bytes()),
                force_srgb: None,
            })
            .await
            .unwrap();

        sun = Some(img.0.clone());
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

        let mut clouds = [const { None }; 32];
        for (index, path) in [
            &weather.cloud_texture_layer00,
            &weather.cloud_texture_layer01,
            &weather.cloud_texture_layer02,
            &weather.cloud_texture_layer03,
            &weather.cloud_texture_layer04,
            &weather.cloud_texture_layer05,
            &weather.cloud_texture_layer06,
            &weather.cloud_texture_layer07,
            &weather.cloud_texture_layer08,
            &weather.cloud_texture_layer09,
            &weather.cloud_texture_layer10,
            &weather.cloud_texture_layer11,
            &weather.cloud_texture_layer12,
            &weather.cloud_texture_layer13,
            &weather.cloud_texture_layer14,
            &weather.cloud_texture_layer15,
            &weather.cloud_texture_layer16,
            &weather.cloud_texture_layer17,
            &weather.cloud_texture_layer18,
            &weather.cloud_texture_layer19,
            &weather.cloud_texture_layer20,
            &weather.cloud_texture_layer21,
            &weather.cloud_texture_layer22,
            &weather.cloud_texture_layer23,
            &weather.cloud_texture_layer24,
            &weather.cloud_texture_layer25,
            &weather.cloud_texture_layer26,
            &weather.cloud_texture_layer27,
            &weather.cloud_texture_layer28,
            &weather.cloud_texture_layer29,
            &weather.cloud_texture_layer30,
            &weather.cloud_texture_layer31,
        ]
        .iter()
        .enumerate()
        {
            let Some(path) = path else {
                continue;
            };

            let mut path = DataPathBuf::new(path.as_bytes());
            path.push_front("textures");

            let Ok(img) = loader
                .load::<FileImage, _>(&FileImageKey {
                    path: path.clone(),
                    force_srgb: Some(true),
                })
                .await
                .inspect_err(|err| {
                    tracing::error!(
                        "failed to load cloud texture for layer {} from {}: {}",
                        index,
                        path,
                        err
                    );
                })
            else {
                continue;
            };

            clouds[index] = Some(img.0.clone());
        }

        let mut cloud_speed = [Vec2::ZERO; 32];
        for (i, (x, y)) in weather
            .cloud_speed_x
            .0
            .into_iter()
            .zip(weather.cloud_speed_y.0)
            .enumerate()
        {
            // The representable values are between -0.1 and 0.1.
            // It seems that with factor 0.1 the textures repeates every 100 seconds,
            // therefore we need the extra factor of 0.1 here.
            cloud_speed[i] = Vec2::new(x, y) * 0.1;
        }

        fn to_params<F>(
            weather: &openfo4_esp::records::Weather,
            dir_light_index: usize,
            f: F,
        ) -> WeatherParams
        where
            F: Fn(TimeOfDaySetExtended<Rgba<u8>>) -> Rgba<u8>,
        {
            let dir_light = weather.lighting_data[dir_light_index];

            WeatherParams {
                ambient_top: rgb_to_color(dir_light.directional_ambient_z_up),
                ambient_bottom: rgb_to_color(dir_light.directional_ambient_z_down),
                ambient_right: rgb_to_color(dir_light.directional_ambient_y_up),
                ambient_left: rgb_to_color(dir_light.directional_ambient_y_down),
                ambient_front: rgb_to_color(dir_light.directional_ambient_x_down),
                ambient_back: rgb_to_color(dir_light.directional_ambient_x_up),
                sky_lower: rgb_to_color(f(weather.colors.sky_lower)),
                sky_upper: rgb_to_color(f(weather.colors.sky_upper)),
                horizon: rgb_to_color(f(weather.colors.horizon)),
                sun: rgb_to_color(f(weather.colors.sun)),
                sky_scale: 1.0,
                sunlight: rgb_to_color(f(weather.colors.sunlight)),
                sunlight_scale: 1.0,
                cloud_colors: [Color::TRANSPARENT; 32],
            }
        }

        // FIXME: The format for directional lights is unforunate here,
        // can we get this into a similar form to the other colors to avoid
        // indexing and potential out of bounds error?
        let mut params = Vec::new();
        let p_night = params.push_and_index(to_params(&weather, 3, |v| v.night));
        let p_day = params.push_and_index(to_params(&weather, 1, |v| v.day));
        let p_early_sunrise = params.push_and_index(to_params(&weather, 4, |v| v.early_sunrise));
        let p_sunrise = params.push_and_index(to_params(&weather, 0, |v| v.sunrise));
        let p_late_sunrise = params.push_and_index(to_params(&weather, 5, |v| v.late_sunrise));
        let p_early_sunset = params.push_and_index(to_params(&weather, 6, |v| v.early_sunset));
        let p_sunset = params.push_and_index(to_params(&weather, 2, |v| v.sunset));
        let p_late_sunset = params.push_and_index(to_params(&weather, 7, |v| v.late_sunset));

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
                params.sunlight_scale = data.sunlight_scale;
            }

            if let Some(data) = data.hdr {
                params.sky_scale = data.sky_scale;
                params.sunlight_scale = data.sunlight_scale;
            }
        }

        for (layer, (color, alpha)) in weather
            .cloud_texture_color
            .iter()
            .zip(weather.cloud_texture_alpha)
            .enumerate()
        {
            params[p_night].cloud_colors[layer] = rgb_to_color(color.night);
            params[p_night].cloud_colors[layer].a = alpha.night;

            params[p_day].cloud_colors[layer] = rgb_to_color(color.day);
            params[p_day].cloud_colors[layer].a = alpha.day;

            params[p_early_sunrise].cloud_colors[layer] = rgb_to_color(color.early_sunrise);
            params[p_early_sunrise].cloud_colors[layer].a = alpha.early_sunrise;

            params[p_sunrise].cloud_colors[layer] = rgb_to_color(color.sunrise);
            params[p_sunrise].cloud_colors[layer].a = alpha.sunrise;

            params[p_late_sunrise].cloud_colors[layer] = rgb_to_color(color.late_sunrise);
            params[p_late_sunrise].cloud_colors[layer].a = alpha.late_sunrise;

            params[p_early_sunset].cloud_colors[layer] = rgb_to_color(color.early_sunset);
            params[p_early_sunset].cloud_colors[layer].a = alpha.early_sunset;

            params[p_sunset].cloud_colors[layer] = rgb_to_color(color.sunset);
            params[p_sunset].cloud_colors[layer].a = alpha.sunset;

            params[p_late_sunset].cloud_colors[layer] = rgb_to_color(color.late_sunset);
            params[p_late_sunset].cloud_colors[layer].a = alpha.late_sunset;
        }

        let mut disabled_cloud_layers = weather.disabled_cloud_layers.unwrap_or_default();
        while disabled_cloud_layers != 0 {
            let index = disabled_cloud_layers.trailing_zeros();
            disabled_cloud_layers ^= 1 << index;

            for param in &mut params {
                param.cloud_colors[index as usize].a = 0.0;
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

        weathers.push(Weather {
            params,
            phases,
            clouds,
            cloud_speed,
        });
    }

    WorldspaceClimate {
        sun,
        weathers,
        active_weather: None,
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

    let mut hour = time.hour() as f64 + time.minute() as f64 / 60.0 + time.second() as f64 / 3600.0;

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
