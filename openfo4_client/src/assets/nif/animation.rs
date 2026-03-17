use std::{
    ops::{Add, Mul},
    time::Duration,
};

use hashbrown::HashMap;
use openfo4_nif::{
    Block, BlockRef, Nif,
    controller::{BSLightingShaderPropertyFloatVariable, NiDataKey, TimeControllerLoopMode},
};

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
