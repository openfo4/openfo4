use std::ops::RangeInclusive;
use std::sync::LazyLock;

use game_tracing::span::EnteredSpan;
use hashbrown::HashMap;
use hecs::World;
use openfo4_animation::Value;
use openfo4_animation::driver::AnimationDriver;
use openfo4_animation::systems::{GameAnimationEvent, GameEvent};
use openfo4_core::events::EventBus;

use crate::state::anim_starter::ActiveBehaviorGraphs;
use crate::systems::GunFireEvent;

#[derive(Clone, Debug)]
pub struct AnimationParameters {
    pub speed: f32,
    pub direction: f32,
    pub is_moving: bool,
    pub is_sprinting: bool,
    pub is_jumping: bool,
    pub is_player: bool,
    pub is_female: bool,
}

pub fn update_animation_parameters(world: &mut World) {
    for (_, (params, driver, graphs)) in world.query_mut::<(
        &AnimationParameters,
        &mut AnimationDriver,
        &ActiveBehaviorGraphs,
    )>() {
        // Hardcoded variables that are always set.
        // These are detected by the game engine and always set based on name.

        if let Some(var) = driver.get_variable_mut("Speed") {
            *var = Value::F32(params.speed);
        }

        if let Some(var) = driver.get_variable_mut("Direction") {
            *var = Value::F32(params.direction);
        }

        if let Some(var) = driver.get_variable_mut("iSyncIdleLocomotion") {
            *var = Value::I32(params.is_moving as i32);
        }

        if let Some(var) = driver.get_variable_mut("iSyncSprintState") {
            *var = Value::I32(params.is_sprinting as i32);
        }

        if let Some(var) = driver.get_variable_mut("bInJumpState") {
            *var = Value::Bool(params.is_jumping);
        }

        if let Some(var) = driver.get_variable_mut("iSyncJumpState") {
            *var = Value::I32(params.is_jumping as i32);
        }

        if let Some(var) = driver.get_variable_mut("IsPlayer") {
            *var = Value::Bool(params.is_player);
        }

        if let Some(var) = driver.get_variable_mut("iIsPlayer") {
            *var = Value::I32(params.is_player as i32);
        }

        if let Some(var) = driver.get_variable_mut("bIsFemale") {
            *var = Value::Bool(params.is_female);
        }

        // Hardcoded variables that come from "animtextdata/animspeedinfo".
        // The exact format is still unknown, but some values are extracted
        // from those files.
        // The CK can automatically generate thse files, so there should not
        // be a reason why we can't generate the same parameters at runtime,
        // however how they are generated still remains a mystery for now.

        for path in &graphs.0 {
            if let Some(vars) = HARDCODED_GRAPH_VARIABLES.get(path.as_str()) {
                for movement in vars.movement {
                    let Some(mode) = movement.modes.iter().min_by(|a, b| {
                        f32::total_cmp(
                            &(a.speed - params.speed).abs(),
                            &(b.speed - params.speed).abs(),
                        )
                    }) else {
                        continue;
                    };

                    if let Some(var) = driver.get_variable_mut(mode.speed_mult) {
                        let speed_mult = params.speed / mode.speed;
                        *var = Value::F32(speed_mult);
                    }

                    if let Some(var) = driver.get_variable_mut(movement.state) {
                        *var = Value::I32(mode.state_value);
                    }

                    driver.send_event(mode.enter_event);
                }

                for direction in vars.directions {
                    if direction.range.contains(&params.direction) {
                        driver.send_event(direction.enter_event);
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct GraphVariables {
    movement: &'static [Movement],
    /// Only the first direction that matches is taken.
    directions: &'static [Direction],
}

static HARDCODED_GRAPH_VARIABLES: LazyLock<HashMap<&'static str, GraphVariables>> =
    LazyLock::new(|| {
        [
            (
                "actors/character/behaviors/mtbehavior.hkx",
                GraphVariables {
                    movement: &[Movement {
                        modes: &[
                            MovementMode {
                                speed: 92.55,
                                speed_mult: "WalkSpeedMult",
                                enter_event: "Walk",
                                state_value: 0,
                            },
                            MovementMode {
                                speed: 225.48,
                                speed_mult: "JogSpeedMult",
                                enter_event: "Jog",
                                state_value: 1,
                            },
                            MovementMode {
                                speed: 372.98,
                                speed_mult: "RunSpeedMult",
                                enter_event: "Run",
                                state_value: 2,
                            },
                        ],
                        state: "iSyncLocomotionSpeed",
                    }],
                    directions: &[],
                },
            ),
            (
                "actors/character/behaviors/weaponbehavior.hkx",
                GraphVariables {
                    movement: &[Movement {
                        modes: &[
                            MovementMode {
                                speed: 92.55,
                                speed_mult: "fLocomotionWalkPlaybackSpeed",
                                enter_event: "walkStart",
                                state_value: 0,
                            },
                            MovementMode {
                                speed: 372.98,
                                speed_mult: "fLocomotionRunPlaybackSpeed",
                                enter_event: "runStart",
                                state_value: 1,
                            },
                        ],
                        state: "",
                    }],
                    directions: &[
                        Direction {
                            range: 0.0..=0.375,
                            enter_event: "moveForward",
                        },
                        Direction {
                            range: 0.875..=1.0,
                            enter_event: "moveForward",
                        },
                        Direction {
                            range: 0.375..=0.875,
                            enter_event: "moveBackward",
                        },
                    ],
                },
            ),
        ]
        .into_iter()
        .collect()
    });

#[derive(Copy, Clone, Debug)]
struct Movement {
    pub modes: &'static [MovementMode],
    pub state: &'static str,
}

#[derive(Copy, Clone, Debug)]
struct MovementMode {
    speed: f32,
    speed_mult: &'static str,
    enter_event: &'static str,
    state_value: i32,
}

#[derive(Clone, Debug)]
struct Direction {
    range: RangeInclusive<f32>,
    enter_event: &'static str,
}

pub fn bounce_animation_events(events: &mut EventBus) {
    for event in events
        .read::<GameAnimationEvent>()
        .cloned()
        .collect::<Vec<_>>()
    {
        match event.event {
            GameEvent::WeaponFire => {
                events.send(GunFireEvent {
                    actor: event.entity,
                });
            }
            _ => (),
        }
    }
}
