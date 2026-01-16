use std::sync::Arc;

use game_common::components::{AmbientLight, Color};
use game_core_pipeline::entities::Entities;
use game_core_pipeline::sky::SkyMaterial;
use game_render::mesh::{Indices, Mesh};
use game_tasks::TaskPool;
use glam::Vec4;
use hashbrown::HashMap;
use hecs::{Entity, World};
use openfo4_asset::AssetLoader;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_esp::records::{ClimateTime, Rgba};
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
        let climate = futures_lite::future::block_on(load_worldspace_weather_props(
            &esp_loader,
            WorldSpace {
                record: RecordId::new(HARDCODED_MODULE_ID, FormId(0x3c)),
                is_interior: false,
            },
        ));

        let mesh = futures_lite::future::block_on(load_sky_mesh(asset_loader));
        shared.entities.set_sky_mesh(mesh);

        shared.entities.set_sky_material(SkyMaterial {
            sky_lower: Color::WHITE,
            sky_upper: Color::RED,
            horizon: Color::GREEN,
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
        let Some((_, time)) = world.query::<&GameTime>().iter().next() else {
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
        let span = &weather.spans[0];

        if let Some(entity) = self.ambient_light {
            world.despawn(entity).unwrap();
        }

        self.entities.set_sky_material(SkyMaterial {
            sky_lower: span.sky_lower,
            sky_upper: span.sky_upper,
            horizon: span.horizon,
        });

        self.ambient_light = Some(world.spawn((
            AmbientLight {
                color: span.ambient,
                brightness: 1.0,
            },
            worldspace,
        )));
    }
}

struct WorldspaceClimate {
    weathers: Vec<Weather>,
    /// Time when the sun spawns in.
    sunrise: ClimateTime,
    /// Time when the sun despawns.
    sunset: ClimateTime,
}

#[derive(Clone, Debug)]
struct Weather {
    spans: Vec<WeatherSpan>,
}

impl Weather {
    fn get_interpolated(&self, time: &GameTime) {
        // let time = time.hour() * 60 + time.minute();

        // for (index, span) in self.spans.iter().enumerate() {
        //     let start = u64::from(span.start.hour) * 60 + u64::from(span.start.minute);
        //     let end = u64::from(span.end.hour) * 60 + u64::from(span.end.minute);

        //     if time < start || time > end {
        //         continue;
        //     }

        //     let next = self.spans.get(index + 1).unwrap_or(&self.spans[0]);

        // }
    }
}

#[derive(Clone, Debug)]
struct WeatherSpan {
    start: ClimateTime,
    end: ClimateTime,
    ambient: Color,
    sky_lower: Color,
    sky_upper: Color,
    horizon: Color,
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

        // The interpoltion times between times of day don't seem to be anywhere,
        // but based on observation they seem to follow a pattern close to following:
        // Based on the 5:00 AM - 9:00 AM default climate times:
        // 3:00 AM => 100% Night
        // 4:00 AM => 100% Early Sunrise
        // 5:00 AM => 50% Early Sunrise, 50% Sunrise
        // 6:00 AM => 100% Sunrise
        // 7:00 AM => 50% Sunrise, 50% Late Sunrise
        // 8:00 AM => 50% Late Sunrise, 50% Day
        // 9:00 AM => 100% Day

        weathers.push(Weather {
            spans: vec![
                WeatherSpan {
                    start: climate.timing.sunrise_begin,
                    end: climate.timing.sunrise_end,
                    ambient: rgb_to_color(weather.colors.ambient.sunrise),
                    sky_lower: rgb_to_color(weather.colors.sky_lower.sunrise),
                    sky_upper: rgb_to_color(weather.colors.sky_upper.sunrise),
                    horizon: rgb_to_color(weather.colors.horizon.sunrise),
                },
                WeatherSpan {
                    start: climate.timing.sunrise_end,
                    end: climate.timing.sunset_begin,
                    ambient: rgb_to_color(weather.colors.ambient.day),
                    sky_lower: rgb_to_color(weather.colors.sky_lower.day),
                    sky_upper: rgb_to_color(weather.colors.sky_upper.day),
                    horizon: rgb_to_color(weather.colors.horizon.day),
                },
                WeatherSpan {
                    start: climate.timing.sunset_begin,
                    end: climate.timing.sunset_end,
                    ambient: rgb_to_color(weather.colors.ambient.sunset),
                    sky_lower: rgb_to_color(weather.colors.sky_lower.sunset),
                    sky_upper: rgb_to_color(weather.colors.sky_upper.sunset),
                    horizon: rgb_to_color(weather.colors.horizon.sunset),
                },
                WeatherSpan {
                    start: climate.timing.sunset_end,
                    end: climate.timing.sunrise_begin,
                    ambient: rgb_to_color(weather.colors.ambient.night),
                    sky_lower: rgb_to_color(weather.colors.sky_lower.night),
                    sky_upper: rgb_to_color(weather.colors.sky_upper.night),
                    horizon: rgb_to_color(weather.colors.horizon.night),
                },
            ],
        });

        dbg!(&weather.id);
        dbg!(&weather.colors);

        weather.colors.ambient.sunset;
    }

    dbg!(climate.timing);

    WorldspaceClimate {
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
