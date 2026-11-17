use glam::{Quat, U8Vec4, Vec2, Vec3};
use openfo4_asset::HandleOrAsset;
use openfo4_common::components::Transform;
use openfo4_nif::mesh::BSTriShape;
use openfo4_nif::{Block, BlockRef};
use openfo4_render::mesh::{Indices, Mesh};

use crate::assets::nif::{LoadNifFlags, Node, Skin, Visit, Visitor, is_marker, load_material};
use crate::render::RenderObject;

impl Visit for BSTriShape {
    async fn visit(&self, visitor: &mut Visitor<'_>, id: BlockRef) -> Option<Node> {
        if is_marker(&self.name) {
            return None;
        }

        if let Some(collision_object) = self.collision_object {
            visitor.enqueue(collision_object);
        }

        let transform = Transform {
            translation: self.translation,
            rotation: Quat::from_mat3(&self.rotation).normalize().conjugate(),
            scale: Vec3::splat(self.scale),
        };

        if !visitor.key.flags.contains(LoadNifFlags::RENDER_OBJECTS) {
            return Some(Node {
                name: self.name.clone(),
                transform,
                ..Default::default()
            });
        }

        let mut material_attached = None;
        let mut overwrite_mesh_alpha = false;

        let alpha_property =
            self.alpha_property
                .and_then(|alpha_property| match visitor.nif.get(alpha_property) {
                    Some(Block::NiAlphaProperty(block)) => Some(block),
                    _ => None,
                });

        let material = match self.shader_property {
            Some(shader_property) => {
                if let Some(alpha_property) = alpha_property {
                    visitor
                        .state
                        .material_alpha_properties
                        .insert(shader_property, alpha_property);
                }

                if visitor.materials.get(&shader_property).is_none() {
                    let Ok(material) = load_material(
                        visitor.loader,
                        &visitor.ctx.entities,
                        &visitor.nif,
                        shader_property,
                        alpha_property,
                        &mut visitor.controllers_to_load,
                    )
                    .await
                    .inspect_err(|err| {
                        tracing::error!(
                            "invalid material in nif file {}: {}",
                            visitor.key.path,
                            err
                        );
                    }) else {
                        return None;
                    };

                    match &material {
                        HandleOrAsset::Asset(material) => {
                            visitor.texture_handles.extend(material.textures.clone());
                            overwrite_mesh_alpha = material.tree;
                        }
                        HandleOrAsset::Handle(handle) => {
                            visitor.material_handles.push(handle.clone());
                            overwrite_mesh_alpha = material.tree;
                        }
                    }

                    visitor
                        .materials
                        .insert(shader_property, material.material.clone());
                }

                material_attached = Some(shader_property);
                visitor.materials.get(&shader_property).unwrap()
            }
            None => &visitor.ctx.default_material,
        };

        let mut mesh = build_mesh(self, overwrite_mesh_alpha);

        let skin = match self.skin {
            Some(skin) => match visitor.nif.get(skin) {
                Some(Block::BSSkinInstance(skin)) => {
                    let bone_data = match visitor.nif.get(skin.data) {
                        Some(Block::BSSkinBoneData(data)) => data
                            .bones
                            .iter()
                            .map(|bone| Transform {
                                translation: bone.translation,
                                rotation: Quat::from_mat3(&bone.rotation).conjugate(),
                                scale: Vec3::splat(bone.scale),
                            })
                            .collect::<Vec<_>>(),
                        _ => todo!(),
                    };

                    let mut bones = Vec::new();
                    let mut bone_transforms = Vec::new();

                    for bone in &skin.bones {
                        let transform = visitor.global_transform(*bone).unwrap_or_default();

                        match visitor.nif.get(*bone) {
                            Some(Block::NiNode(node)) => {
                                bones.push(node.name.clone());
                                bone_transforms.push(transform);
                            }
                            _ => todo!(),
                        }
                    }

                    let Some(skinning) = &self.mesh.skinning else {
                        todo!();
                    };

                    let weights: Vec<_> =
                        skinning.bone_weights.iter().map(|v| v.as_vec4()).collect();
                    let indices: Vec<_> = skinning.bone_indices.iter().map(|v| *v).collect();

                    let mut positions = vec![Vec3::ZERO; mesh.positions().len()];

                    for i in 0..weights.len() {
                        let weights = weights[i];
                        let indices = indices[i];
                        let pos = mesh.positions()[i];

                        let i0 = indices[0] as usize;
                        let i1 = indices[1] as usize;
                        let i2 = indices[2] as usize;
                        let i3 = indices[3] as usize;

                        let x = (bone_transforms[i0] * bone_data[i0]).transform_point(pos);
                        let y = (bone_transforms[i1] * bone_data[i1]).transform_point(pos);
                        let z = (bone_transforms[i2] * bone_data[i2]).transform_point(pos);
                        let w = (bone_transforms[i3] * bone_data[i3]).transform_point(pos);

                        positions[i] =
                            x * weights.x + y * weights.y + z * weights.z + w * weights.w;
                    }

                    mesh.set_positions(positions);

                    Some(Skin { bones })
                }
                _ => todo!(),
            },
            None => None,
        };

        let mut attaches = self.name.clone();
        for block in &self.extra_data {
            match visitor.nif.get(*block) {
                Some(Block::NiStringExtraData(data)) if data.name.eq_ignore_ascii_case("Prn") => {
                    attaches = data.data.clone();
                }
                _ => {}
            }
        }

        let mesh = visitor.ctx.entities.create_mesh_async(mesh).await;

        Some(Node {
            name: self.name.clone(),
            transform,
            renderable: Some(RenderObject {
                mesh,
                material: material.clone(),
            }),
            skin,
            attaches,
            ..Default::default()
        })
    }

    fn transform(&self) -> Option<Transform> {
        Some(Transform {
            translation: self.translation,
            rotation: Quat::from_mat3(&self.rotation).normalize().conjugate(),
            scale: Vec3::splat(self.scale),
        })
    }
}

fn build_mesh(shape: &BSTriShape, overwrite_alpha: bool) -> Mesh {
    let mut mesh = Mesh::new();

    if let Some(positions) = &shape.mesh.positions {
        mesh.set_positions(positions.clone());
    }

    if let Some(normals) = &shape.mesh.normals {
        mesh.set_normals(normals.clone());
    }

    if let Some(tex_coords) = &shape.mesh.tex_coords {
        let tex_coords = tex_coords.iter().map(|v| v.as_vec2()).collect();
        mesh.set_uvs(tex_coords);
    } else {
        // We need tex coords to be able to render.
        let tex_coords = std::iter::repeat_n(Vec2::ZERO, mesh.positions().len()).collect();
        mesh.set_uvs(tex_coords);
    }

    if let Some(colors) = &shape.mesh.colors {
        let colors = colors
            .iter()
            .map(|v| {
                let mut color = U8Vec4::new(v.r, v.g, v.b, v.a).as_vec4() / 255.0;
                if overwrite_alpha {
                    color.w = 1.0;
                }

                color
            })
            .collect();
        mesh.set_colors(colors);
    }

    let indices = shape
        .mesh
        .indices
        .iter()
        .flatten()
        .map(|v| *v as u32)
        .collect();
    mesh.set_indices(Indices::U32(indices));

    if let Some(tangents) = &shape.mesh.tangents {
        mesh.set_tangents(tangents.clone());
    } else {
        mesh.compute_tangents();
    }

    if let Some(skin) = &shape.mesh.skinning {
        mesh.set_skinning_joint_indices(skin.bone_indices.clone());
        mesh.set_skinning_joint_weights(skin.bone_weights.iter().map(|v| v.as_vec4()).collect());
    }

    mesh
}
