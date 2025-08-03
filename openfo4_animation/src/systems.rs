use game_common::components::Transform;
use game_core::time::Time;
use glam::{Quat, Vec3};
use hecs::{CommandBuffer, World};

use crate::animation::{AnimationClip, AnimationClipFlags};
use crate::driver::{AnimationDriver, BoneWeights};
use crate::skeleton::SkeletonInstance;
use crate::{AnimationSet, NodeInner};

pub fn drive_animations(world: &mut World, time: &Time, event: Option<&'static str>) {
    let mut commands = CommandBuffer::new();

    let mut rng = rand::rng();

    for (entity, (skeleton, driver, set)) in
        world.query_mut::<(&SkeletonInstance, &mut AnimationDriver, &AnimationSet)>()
    {
        let mut events = Vec::new();
        driver.update(time.delta(), set, &mut rng, &mut events);

        for event in events {
            match driver.graph.event_name(event.id) {
                Some("reevaluategraphstate") => {
                    driver.reset();
                }
                _ => (),
            }
        }

        if event == Some("dump") {
            crate::debug::update(&driver);
        }

        let active_anims = || {
            driver
                .animations()
                .filter_map(|(node_id, state, blend_factor)| {
                    let node = &driver.graph.nodes[node_id];
                    let clip = match &node.inner {
                        NodeInner::Clip(clip) => clip,
                        _ => unreachable!(),
                    };

                    let Some(animation) = set.0.get(&clip.path) else {
                        return None;
                    };

                    let mut num_frames = animation.num_frames;
                    let mut anim_duration = animation.duration;
                    if animation
                        .flags
                        .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
                        && clip.looping
                    {
                        num_frames = num_frames.saturating_sub(1);
                        anim_duration -= animation.frame_duration();
                    }

                    if animation.duration.is_zero() || num_frames == 0 {
                        return None;
                    }

                    let progress = state.time.as_secs_f32() / anim_duration.as_secs_f32();

                    // For each bone we linearly interpolate between the closest two frames a and b
                    // based on how far the time has progressed between the two frames.
                    let v = progress * num_frames as f32;
                    let v_a = v.floor();
                    let v_b = v.ceil();

                    let s = f32::clamp(v - v_a, 0.0, 1.0);
                    let mut a = u32::min(v_a as u32, num_frames - 1);
                    let mut b = u32::min(v_b as u32, num_frames - 1);

                    if clip.looping && v_a as u32 >= num_frames {
                        a = v_a as u32 % num_frames;
                    }

                    if clip.looping && v_b as u32 >= num_frames {
                        b = v_b as u32 % num_frames;
                    }

                    Some(Contribution {
                        animation,
                        a,
                        b,
                        s,
                        blend_factor,
                    })
                })
        };

        for (index, entity) in &skeleton.bones {
            let mut translation = Vec3::ZERO;
            let mut scale = Vec3::ZERO;

            let mut rotation = Quat::from_array([0.0; 4]);
            let mut first_rotation = None;

            let mut anim_applied = false;

            let mut weight_sum = 0.0;
            for anim in active_anims() {
                weight_sum += anim.blend_factor.get(*index as u32);
            }

            if weight_sum == 0.0 {
                continue;
            }

            for anim in active_anims() {
                let Some(track) = anim.animation.skeleton_bone_to_animation_bone.get(index) else {
                    continue;
                };

                if anim.blend_factor.get(*index as u32).abs() == 0.0 {
                    continue;
                }

                anim_applied = true;

                let transform_a = anim.animation.get(anim.a, *track as u32).unwrap();
                let transform_b = anim.animation.get(anim.b, *track as u32).unwrap();
                let transform = Transform::lerp(transform_a, transform_b, anim.s);

                let weight = anim.blend_factor.get(*index as u32) / weight_sum;
                debug_assert!(weight >= 0.0 && weight <= 1.0);

                translation += transform.translation * weight;
                scale += transform.scale * weight;

                // Fast approximate quaternion average.
                // Since q = -q for rotations hold the first quaternion will dictate
                // the sign and all other quaternion will be inverted to that sign.
                // See https://math.stackexchange.com/questions/61146/averaging-quaternions
                // FIXME: When updating to glam 30.0 replace rotation = rotation + ...
                // with rotation += ... .
                if let Some(first_rotation) = first_rotation {
                    if transform.rotation.dot(first_rotation) < 0.0 {
                        rotation = rotation - transform.rotation * weight;
                    } else {
                        rotation = rotation + transform.rotation * weight;
                    }
                } else {
                    first_rotation = Some(transform.rotation);
                    rotation = rotation + transform.rotation * weight;
                }
            }

            // It is possible that bones exist that are not driven from animations
            // and are only used for skinning. We must not overwrite those bones.
            if anim_applied {
                debug_assert_ne!(rotation.length(), 0.0);
                rotation = rotation.normalize();

                commands.insert_one(
                    *entity,
                    Transform {
                        translation,
                        rotation,
                        scale,
                    },
                );
            }
        }
    }

    commands.run_on(world);
}

#[derive(Copy, Clone, Debug)]
struct Contribution<'a> {
    animation: &'a AnimationClip,
    a: u32,
    b: u32,
    s: f32,
    blend_factor: &'a BoneWeights,
}

trait TransformLerp {
    fn lerp(self, other: Self, s: f32) -> Self;
}

impl TransformLerp for Transform {
    fn lerp(self, other: Self, s: f32) -> Self {
        Self {
            translation: Vec3::lerp(self.translation, other.translation, s),
            rotation: Quat::slerp(self.rotation, other.rotation, s).normalize(),
            scale: Vec3::lerp(self.scale, other.scale, s),
        }
    }
}
