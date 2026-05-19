use std::{
    ops::{Add, Mul},
    time::Duration,
};

use game_common::components::Transform;
use game_core::hierarchy::Key;
use glam::{EulerRot, Quat, Vec3, Vec4};
use hashbrown::HashMap;
use openfo4_nif::controller::{
    BSLightingShaderPropertyFloatVariable, NiDataConstKey, NiDataKey, NiDataKeyRotation,
    NiDataLinearKey, NiDataQuadraticKey, NiDataTbcKey, SequenceCycleMode, TimeControllerLoopMode,
};
use openfo4_nif::{Block, BlockRef, Nif};

use crate::assets::nif::animation_controller::Sequence;

/// Load a non-manager controller.
pub fn load_controller(nif: &Nif, controller: BlockRef) -> Controller {
    let mut values = HashMap::<_, Vec<_>>::new();

    let mut queue = vec![controller];
    while let Some(block) = queue.pop() {
        match nif.get(block) {
            Some(Block::BSLightingShaderPropertyFloatController(controller)) => {
                queue.extend(controller.next_controller);

                let mode = match controller.flags.mode {
                    TimeControllerLoopMode::Clamp => InterpolationMode::Clamp,
                    TimeControllerLoopMode::Cycle => InterpolationMode::Repeat,
                    TimeControllerLoopMode::Reverse => InterpolationMode::MirroredRepeat,
                };

                let duration = Duration::from_secs_f32(controller.stop_time);

                let interpolator = controller.interpolator.unwrap();
                let target = controller.target.unwrap();

                let Some(mut interpolator) =
                    load_float_interpolator(nif, interpolator, mode, duration)
                else {
                    continue;
                };

                interpolator.offset = Duration::from_secs_f32(controller.phase);
                interpolator.speed = controller.frequency;

                let value = match controller.variable {
                    BSLightingShaderPropertyFloatVariable::UOffset => {
                        ControlledValue::UOffset(interpolator)
                    }
                    BSLightingShaderPropertyFloatVariable::VOffset => {
                        ControlledValue::VOffset(interpolator)
                    }
                    BSLightingShaderPropertyFloatVariable::UScale => {
                        ControlledValue::UScale(interpolator)
                    }
                    BSLightingShaderPropertyFloatVariable::VScale => {
                        ControlledValue::VScale(interpolator)
                    }
                    BSLightingShaderPropertyFloatVariable::Alpha => {
                        ControlledValue::Alpha(interpolator)
                    }
                    BSLightingShaderPropertyFloatVariable::EmissiveStrenth => {
                        // FIXME: We don't support emission on our material yet.
                        continue;
                    }
                    BSLightingShaderPropertyFloatVariable::RefractionStrength => {
                        // FIXME: We don't support refraction on our material yet.
                        continue;
                    }
                };

                values.entry(target).or_default().push(value);
            }
            _ => {}
        }
    }

    Controller { values }
}

pub fn loader_controller_sequences(
    nif: &Nif,
    block_to_node: &HashMap<BlockRef, Key>,
) -> HashMap<String, Sequence> {
    let mut sequences = HashMap::new();

    for block in nif.iter() {
        match block {
            Block::NiControllerManager(manager) => {
                for seq in &manager.sequences {
                    let Some(Block::NiControllerSequence(seq)) = nif.get(*seq) else {
                        todo!();
                    };

                    let mut transforms = HashMap::new();
                    let mut max_duration = Some(Duration::ZERO);

                    for block in &seq.blocks {
                        let Some(Block::NiMultiTargetTransformController(controller)) =
                            nif.get(block.controller)
                        else {
                            // TODO
                            continue;
                        };

                        for target in &controller.targets {
                            let Some(Block::NiNode(node)) = nif.get(*target) else {
                                todo!();
                            };

                            if node.name != block.node_name {
                                continue;
                            }

                            let mode = match seq.cycle_mode {
                                SequenceCycleMode::Loop => InterpolationMode::Repeat,
                                SequenceCycleMode::Reverse => InterpolationMode::MirroredRepeat,
                                SequenceCycleMode::Clamp => InterpolationMode::Clamp,
                            };

                            let duration = Duration::from_secs_f32(seq.stop_time);

                            let interpolator = load_transform_interpolator(
                                nif,
                                block.interpolator,
                                mode,
                                duration,
                            );

                            // TODO: start_time and frequency

                            // If this does not exist we have culled the block and don't
                            // need to animation it.
                            let Some(key) = block_to_node.get(target).copied() else {
                                continue;
                            };

                            if mode == InterpolationMode::Clamp {
                                if let Some(max_duration) = &mut max_duration {
                                    *max_duration = (*max_duration).max(duration);
                                }
                            } else {
                                // The looping modes never stop animating.
                                max_duration = None;
                            }

                            transforms.insert(key, interpolator);
                        }
                    }

                    sequences.insert(
                        seq.name.clone(),
                        Sequence {
                            transforms,
                            duration: max_duration,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    sequences
}

fn load_float_interpolator(
    nif: &Nif,
    block: BlockRef,
    mode: InterpolationMode,
    duration: Duration,
) -> Option<Interpolator<f32>> {
    match nif.get(block) {
        Some(Block::NiFloatInterpolator(interpolator)) => match nif.get(interpolator.data) {
            Some(Block::NiFloatData(data)) => Some(Interpolator::new(&data.data, mode, duration)),
            _ => todo!(),
        },
        // FIXME: Instructions unclear
        Some(Block::NiBlendFloatInterpolator(_)) => None,
        Some(Block::Skipped) => None,
        x => todo!("{:?}", x),
    }
}

fn load_transform_interpolator(
    nif: &Nif,
    block: BlockRef,
    mode: InterpolationMode,
    duration: Duration,
) -> TransformInterpolator {
    match nif.get(block) {
        Some(Block::NiTransformInterpolator(interpolator)) => match nif.get(interpolator.data) {
            Some(Block::NiTransformData(data)) => {
                if let Some(translations) = &data.translations {
                    Interpolator::new(translations, mode, duration);
                }

                let translation = data
                    .translations
                    .as_ref()
                    .map(|keys| Interpolator::new(keys, mode, duration));

                let rotation = match &data.rotations {
                    Some(NiDataKeyRotation::XyzRotation(keys)) => {
                        let mut keys = keys.iter();

                        let x = keys
                            .next()
                            .map(|key| Interpolator::new(&key.keys, mode, duration));
                        let y = keys
                            .next()
                            .map(|key| Interpolator::new(&key.keys, mode, duration));
                        let z = keys
                            .next()
                            .map(|key| Interpolator::new(&key.keys, mode, duration));

                        Some(RotationInterpolator::Euler { x, y, z })
                    }
                    Some(NiDataKeyRotation::Linear(keys)) => {
                        let keys = NiDataKey::Linear(
                            keys.iter()
                                .map(|v| NiDataLinearKey {
                                    time: v.time,
                                    value: Vec4::from(v.value),
                                })
                                .collect(),
                        );

                        Some(RotationInterpolator::Quat(Interpolator::new(
                            &keys, mode, duration,
                        )))
                    }
                    Some(NiDataKeyRotation::Quadratic(keys)) => {
                        let keys = NiDataKey::Quadratic(
                            keys.iter()
                                .map(|v| NiDataQuadraticKey {
                                    time: v.time,
                                    value: Vec4::from(v.value),
                                    forward: Vec4::from(v.forward),
                                    backward: Vec4::from(v.backward),
                                })
                                .collect(),
                        );

                        Some(RotationInterpolator::Quat(Interpolator::new(
                            &keys, mode, duration,
                        )))
                    }
                    Some(NiDataKeyRotation::Const(keys)) => {
                        let keys = NiDataKey::Const(
                            keys.iter()
                                .map(|v| NiDataConstKey {
                                    time: v.time,
                                    value: Vec4::from(v.value),
                                })
                                .collect(),
                        );

                        Some(RotationInterpolator::Quat(Interpolator::new(
                            &keys, mode, duration,
                        )))
                    }
                    Some(NiDataKeyRotation::Tbc(keys)) => {
                        let keys = NiDataKey::Tbc(
                            keys.iter()
                                .map(|v| NiDataTbcKey {
                                    time: v.time,
                                    value: Vec4::from(v.value),
                                    tbc: v.tbc,
                                })
                                .collect(),
                        );

                        Some(RotationInterpolator::Quat(Interpolator::new(
                            &keys, mode, duration,
                        )))
                    }
                    None => None,
                };

                let scale = data
                    .scales
                    .as_ref()
                    .map(|keys| Interpolator::new(keys, mode, duration));

                TransformInterpolator {
                    translation,
                    rotation,
                    scale,
                }
            }
            _ => todo!(),
        },
        _ => todo!(),
    }
}

pub struct Controller {
    pub values: HashMap<BlockRef, Vec<ControlledValue>>,
}

#[derive(Clone, Debug)]
pub enum ControlledValue {
    Alpha(Interpolator<f32>),
    UOffset(Interpolator<f32>),
    VOffset(Interpolator<f32>),
    UScale(Interpolator<f32>),
    VScale(Interpolator<f32>),
}

#[derive(Clone, Debug)]
pub struct Interpolator<T> {
    mode: InterpolationMode,
    duration: Duration,
    keys: InterpolatorKeys<T>,
    speed: f32,
    offset: Duration,
}

impl<T> Interpolator<T>
where
    T: Copy + Mul<f32, Output = T> + Add<Output = T> + Add<f32, Output = T>,
{
    fn new(data: &NiDataKey<T>, mode: InterpolationMode, duration: Duration) -> Self {
        let keys = match data {
            NiDataKey::Const(keys) => {
                let keys = keys
                    .iter()
                    .map(|v| NearestKey {
                        time: Duration::from_secs_f32(v.time),
                        value: v.value,
                    })
                    .collect();

                InterpolatorKeys::Nearest(keys)
            }
            NiDataKey::Linear(keys) => {
                let keys = keys
                    .iter()
                    .map(|v| LinearKey {
                        time: Duration::from_secs_f32(v.time),
                        value: v.value,
                    })
                    .collect();

                InterpolatorKeys::Linear(keys)
            }
            NiDataKey::Quadratic(keys) => {
                let keys = keys
                    .iter()
                    .map(|v| CubicKey {
                        time: Duration::from_secs_f32(v.time),
                        value: v.value,
                        backward: v.backward,
                        forward: v.forward,
                    })
                    .collect();

                InterpolatorKeys::Cubic(keys)
            }
            NiDataKey::Tbc(_) => todo!(),
        };

        Self {
            keys,
            mode,
            duration,
            speed: 1.0,
            offset: Duration::ZERO,
        }
    }

    pub fn sample(&self, mut time: Duration) -> Option<T> {
        time = time.mul_f32(self.speed) + self.offset;

        match self.mode {
            InterpolationMode::Repeat => {
                // FIXME: No Rem on Duration, but we can do better than this.
                time = Duration::from_secs_f32(time.as_secs_f32() % self.duration.as_secs_f32());
            }
            InterpolationMode::MirroredRepeat => {
                todo!()
            }
            InterpolationMode::Clamp => {
                if time >= self.duration {
                    return match &self.keys {
                        InterpolatorKeys::Nearest(keys) => keys.last().map(|v| v.value),
                        InterpolatorKeys::Linear(keys) => keys.last().map(|v| v.value),
                        InterpolatorKeys::Cubic(keys) => keys.last().map(|v| v.value),
                    };
                }
            }
        }

        match &self.keys {
            InterpolatorKeys::Nearest(keys) => {
                let key = keys.iter().min_by(|a, b| {
                    let a = a.time.abs_diff(time);
                    let b = b.time.abs_diff(time);
                    a.cmp(&b)
                })?;

                Some(key.value)
            }
            InterpolatorKeys::Linear(keys) => {
                if keys.len() <= 1 {
                    return keys.first().map(|v| v.value);
                }

                let [a, b] = keys.array_windows::<2>().find(|[_, b]| b.time >= time)?;

                debug_assert!(time >= a.time);
                debug_assert!(time <= b.time);

                let s = (time.as_secs_f32() - a.time.as_secs_f32())
                    / (b.time.as_secs_f32() - a.time.as_secs_f32());

                Some(a.value * (1.0 - s) + b.value * s)
            }
            InterpolatorKeys::Cubic(keys) => {
                if keys.len() <= 1 {
                    return keys.first().map(|v| v.value);
                }

                let [a, b] = keys.array_windows::<2>().find(|[_, b]| b.time >= time)?;

                debug_assert!(time >= a.time);
                debug_assert!(time <= b.time);

                let k0 = a;
                let k1 = b;

                let t = (time.as_secs_f32() - a.time.as_secs_f32())
                    / (b.time.as_secs_f32() - a.time.as_secs_f32());

                // https://en.wikipedia.org/wiki/Cubic_Hermite_spline#Unit_interval_[0,_1]
                let t2 = t * t;
                let t3 = t2 * t;

                let m0 = k0.backward;
                let m1 = k1.forward;
                let p0 = k0.value;
                let p1 = k1.value;

                let s0 = p0 * (2.0 * t3 - 3.0 * t2 + 1.0);
                let s1 = m0 * (t3 - 2.0 * t2 + t);
                let s2 = p1 * (-2.0 * t3 + 3.0 * t2);
                let s3 = m1 * (t3 - t2);
                Some(s0 + s1 + s2 + s3)
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum InterpolationMode {
    Repeat,
    MirroredRepeat,
    Clamp,
}

#[derive(Clone, Debug)]
enum InterpolatorKeys<T> {
    Nearest(Vec<NearestKey<T>>),
    Linear(Vec<LinearKey<T>>),
    Cubic(Vec<CubicKey<T>>),
}

#[derive(Copy, Clone, Debug)]
struct NearestKey<T> {
    time: Duration,
    value: T,
}

#[derive(Copy, Clone, Debug)]
struct LinearKey<T> {
    time: Duration,
    value: T,
}

#[derive(Copy, Clone, Debug)]
struct CubicKey<T> {
    time: Duration,
    value: T,
    backward: T,
    forward: T,
}

#[derive(Clone, Debug)]
pub struct TransformInterpolator {
    translation: Option<Interpolator<Vec3>>,
    rotation: Option<RotationInterpolator>,
    scale: Option<Interpolator<f32>>,
}

impl TransformInterpolator {
    pub fn sample(&self, time: Duration, default: Transform) -> Transform {
        let mut t = default;

        if let Some(translation) = &self.translation {
            t.translation = translation.sample(time).unwrap_or(default.translation);
        }

        match &self.rotation {
            Some(RotationInterpolator::Quat(interpolator)) => {
                if let Some(value) = interpolator.sample(time) {
                    t.rotation = Quat::from_vec4(value.normalize());
                }
            }
            Some(RotationInterpolator::Euler { x, y, z }) => {
                let (d_x, d_y, d_z) = default.rotation.to_euler(EulerRot::XYX);

                let x = x.as_ref().map(|i| i.sample(time)).flatten().unwrap_or(d_x);
                let y = y.as_ref().map(|i| i.sample(time)).flatten().unwrap_or(d_y);
                let z = z.as_ref().map(|i| i.sample(time)).flatten().unwrap_or(d_z);

                t.rotation = Quat::from_euler(EulerRot::XYZ, x, y, z);
            }
            None => {}
        }

        if let Some(scale) = &self.scale {
            t.scale = scale.sample(time).map(Vec3::splat).unwrap_or(default.scale);
        }

        t
    }
}

#[derive(Clone, Debug)]
pub enum RotationInterpolator {
    Quat(Interpolator<Vec4>),
    Euler {
        x: Option<Interpolator<f32>>,
        y: Option<Interpolator<f32>>,
        z: Option<Interpolator<f32>>,
    },
}
