use std::collections::HashMap;
use std::time::Duration;

use game_common::components::Transform;
use openfo4_animation::animation::{AnimationClip, AnimationClipFlags, Annotation, BlendMode};
use openfo4_asset::{Asset, AssetLoader, Handle};
use openfo4_hkx::animation::BlendHint;
use openfo4_hkx::{HkObject, HkxFile};

use crate::assets::{Error, FileLoadContext};

#[derive(Clone, Debug)]
pub struct FileAnimationClip(pub AnimationClip);

impl Asset for FileAnimationClip {
    type Key = String;
    type Error = Error;
    type Context = FileLoadContext;

    async fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        ctx: &Self::Context,
    ) -> Result<Self, Self::Error> {
        let mut clip = AnimationClip {
            duration: Duration::ZERO,
            num_bones: 0,
            num_frames: 0,
            keyframes: Vec::new(),
            skeleton_bone_to_animation_bone: HashMap::new(),
            annotations: Vec::new(),
            flags: AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING,
            blend_mode: BlendMode::default(),
        };

        let file = ctx.server.load::<HkxFile>(&key).await?;

        for object in file.iter() {
            match object {
                HkObject::SplineCompressedAnimation(anim) => {
                    let keyframes = anim.decompress();

                    clip.duration = Duration::from_secs_f32(anim.animation.duration);
                    clip.num_bones = anim.animation.num_of_transform_tracks;
                    clip.num_frames = anim.num_frames;
                    clip.keyframes = keyframes
                        .into_iter()
                        .map(|t| Transform {
                            translation: t.translation,
                            rotation: t.rotation,
                            scale: t.scale,
                        })
                        .collect();
                }
                HkObject::AnimationBinding(binding) => {
                    for (index, bone) in binding.transform_track_to_bone_indices.iter().enumerate()
                    {
                        clip.skeleton_bone_to_animation_bone
                            .insert(*bone as usize, index);
                    }

                    clip.blend_mode = match binding.blend_hint {
                        BlendHint::Normal => BlendMode::Normal,
                        BlendHint::Additive | BlendHint::AdditiveDeprecated => BlendMode::Additive,
                    };
                }
                _ => (),
            }
        }

        Ok(Self(clip))
    }
}
