use std::collections::HashMap;
use std::time::Duration;

use game_common::components::Transform;
use glam::{Quat, Vec3};
use hecs::{CommandBuffer, Entity, World};
use openfo4_core::events::EventBus;

use crate::animation::{AnimationClip, AnimationClipFlags, BlendMode};
use crate::driver::AnimationDriver;
use crate::nodes::Modifier;
use crate::skeleton::SkeletonInstance;
use crate::{AnimationProvider, BoneWeights, Value};

pub fn drive_animations<T>(
    world: &mut World,
    delta: Duration,
    event: Option<&'static str>,
    event_bus: &mut EventBus,
) where
    T: AnimationProvider + Send + Sync + 'static,
{
    let mut commands = CommandBuffer::new();

    let mut rng = rand::rng();

    for (entity, (skeleton, driver, set)) in
        world.query_mut::<(&mut SkeletonInstance, &mut AnimationDriver, &T)>()
    {
        let mut new_transforms = HashMap::new();

        let mut events = Vec::new();
        driver.update(delta, set, &mut events, &skeleton.skeleton);

        let game_events = events
            .into_iter()
            .filter_map(|event| {
                driver
                    .graph()
                    .event_name(event.id)
                    .and_then(|name| GameEvent::from_name_and_payload(name, event.payload))
            })
            .collect::<Vec<_>>();

        for event in game_events {
            match &event {
                GameEvent::ReevaluateGraphState => {
                    driver.reset();

                    for index in skeleton.culled.drain() {
                        let transform = *skeleton.bones_bind_pose.get(&index).unwrap();
                        let entity = skeleton.bones.get(&index).unwrap();
                        new_transforms.insert(*entity, transform);
                    }
                }
                GameEvent::CullBone(name) => {
                    let index = skeleton
                        .skeleton
                        .bones
                        .iter()
                        .position(|b| b.name.to_lowercase() == name.to_lowercase())
                        .unwrap();

                    let entity = skeleton.bones.get(&index).unwrap();

                    // FIXME: Setting this to NAN will cause a NAN for the MVP matrix
                    // effectively hiding the entity, but saying this is hacky is the
                    // understatement of the century.
                    new_transforms.insert(*entity, Transform::from_translation(Vec3::NAN));
                    skeleton.culled.insert(index);
                }
                GameEvent::UncullBone(name) => {
                    let index = skeleton
                        .skeleton
                        .bones
                        .iter()
                        .position(|b| b.name.to_lowercase() == name.to_lowercase())
                        .unwrap();

                    let transform = *skeleton.bones_bind_pose.get(&index).unwrap();

                    let entity = skeleton.bones.get(&index).unwrap();
                    new_transforms.insert(*entity, transform);
                    skeleton.culled.remove(&index);
                }
                GameEvent::WeaponFire => {}
                GameEvent::ReloadComplete => {}
            }

            event_bus.send(GameAnimationEvent { entity, event });
        }

        if event == Some("dump") {
            crate::debug::update(&driver);
        }

        let active_anims = driver
            .animations()
            .filter_map(|state| {
                let Some(animation) = set.get(state.path) else {
                    return None;
                };

                let mut num_frames = animation.num_frames;
                let mut anim_duration = animation.duration;
                if animation
                    .flags
                    .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
                    && state.looping
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

                if state.looping && v_a as u32 >= num_frames {
                    a = v_a as u32 % num_frames;
                }

                if state.looping && v_b as u32 >= num_frames {
                    b = v_b as u32 % num_frames;
                }

                let additive = matches!(animation.blend_mode, BlendMode::Additive);

                Some(Contribution {
                    animation,
                    a,
                    b,
                    s,
                    blend_factor: state.weight,
                    additive,
                })
            })
            .collect::<Vec<_>>();

        for (index, entity) in &skeleton.bones {
            if skeleton.culled.contains(index) {
                continue;
            }

            let mut translation = Vec3::ZERO;
            let mut scale = Vec3::ZERO;

            let mut rotation = Quat::from_array([0.0; 4]);
            let mut first_rotation = None;

            let mut anim_applied = false;

            let mut weight_sum = 0.0;
            for anim in &active_anims {
                // Animations that have no track bound to a bone should
                // not contribute towards the final transform of that bone.
                if !anim
                    .animation
                    .skeleton_bone_to_animation_bone
                    .contains_key(index)
                {
                    continue;
                }

                // Additive animations are blended on top later.
                if anim.additive {
                    continue;
                }

                weight_sum += anim.blend_factor.get(*index as u32);
            }

            if weight_sum == 0.0 {
                continue;
            }

            for anim in &active_anims {
                let Some(track) = anim.animation.skeleton_bone_to_animation_bone.get(index) else {
                    continue;
                };

                if anim.blend_factor.get(*index as u32).abs() == 0.0 {
                    continue;
                }

                if anim.additive {
                    continue;
                }

                anim_applied = true;

                let weight = anim.blend_factor.get(*index as u32) / weight_sum;
                debug_assert!(weight >= 0.0 && weight <= 1.0);

                let transform_a = anim.animation.get(anim.a, *track as u32).unwrap();
                let transform_b = anim.animation.get(anim.b, *track as u32).unwrap();
                let transform = Transform::lerp(transform_a, transform_b, anim.s);

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

            for anim in &active_anims {
                if !anim.additive {
                    continue;
                }

                let Some(track) = anim.animation.skeleton_bone_to_animation_bone.get(index) else {
                    continue;
                };

                let transform_a = anim.animation.get(anim.a, *track as u32).unwrap();
                let transform_b = anim.animation.get(anim.b, *track as u32).unwrap();
                let mut transform = Transform::lerp(transform_a, transform_b, anim.s);

                // The first frame is our "reference frame" that we compute the additive
                // delta to apply from.
                let ref_frame = anim.animation.get(0, *track as u32).unwrap();

                transform.translation -= ref_frame.translation;
                transform.rotation = ref_frame.rotation.conjugate() * transform.rotation;
                transform.scale -= ref_frame.scale;

                // FIXME: This should probably be blended on top of bind pose.
                if !anim_applied {
                    continue;
                }

                debug_assert_ne!(rotation.length(), 0.0);
                rotation = rotation.normalize();

                translation += transform.translation;
                rotation *= transform.rotation;
                scale += transform.scale;
            }

            // It is possible that bones exist that are not driven from animations
            // and are only used for skinning. We must not overwrite those bones.
            if anim_applied {
                debug_assert_ne!(rotation.length(), 0.0);
                rotation = rotation.normalize();

                new_transforms.insert(
                    *entity,
                    Transform {
                        translation,
                        rotation,
                        scale,
                    },
                );
            }
        }

        // Apply post process modifiers.
        for modifier in driver.modifiers() {
            match modifier {
                Modifier::Offset(offset) => {
                    let bone_entity = skeleton.bones[&offset.bone];

                    // FIXME: This only works for bones inserted this frame,
                    // what if a modifier is applied to another bone?
                    let transform = new_transforms.get_mut(&bone_entity).unwrap();
                    *transform *= offset.transform;
                }
            }
        }

        for (entity, transform) in new_transforms {
            commands.insert_one(entity, transform);
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
    additive: bool,
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

#[derive(Clone, Debug)]
pub struct GameAnimationEvent {
    pub entity: Entity,
    pub event: GameEvent,
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    ReevaluateGraphState,
    CullBone(String),
    UncullBone(String),
    WeaponFire,
    ReloadComplete,
}

impl GameEvent {
    fn from_name_and_payload(name: &str, value: Value) -> Option<Self> {
        match name {
            "reevaluategraphstate" => Some(Self::ReevaluateGraphState),
            "cullbone" => value.into_string().map(Self::CullBone),
            "uncullbone" => value.into_string().map(Self::UncullBone),
            "weaponfire" => Some(Self::WeaponFire),
            "reloadcomplete" => Some(Self::ReloadComplete),
            _ => None,
        }
    }
}
