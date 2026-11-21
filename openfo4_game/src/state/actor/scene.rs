use openfo4_asset::AssetLoader;
use openfo4_core::WorldExt;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::records::Race;
use openfo4_esp::{RecordData, RecordKind};
use openfo4_physics::components::{Collider, CollisionLayer, RigidBody};
use openfo4_physics::controller::CharacterController;

use crate::state::actor::character::{ActorCharacter, BodyPartBoneProps, load_character};
use crate::state::esp::EspLoader;
use crate::state::npc::{
    ActorAnimationGraph, ActorAnimationGraphData, ActorBodyPart, ActorBodyParts, RaceSubgraphData,
};
use crate::systems::controller::ControllerShape;

/// The 3D scene of an actor.
#[derive(Clone, Debug)]
pub struct ActorScene {
    character: ActorCharacter,
    body_parts: ActorBodyParts,
    anim_graphs: ActorAnimationGraphData,
    controller: ControllerShape,
}

impl ActorScene {
    pub async fn load(
        esp: &EspLoader,
        loader: &AssetLoader,
        race_id: RecordId,
        race: &Race,
        female: bool,
    ) -> Result<Self, ()> {
        let mut character = load_character(race, female, loader).await.unwrap();

        let mut actor_body_parts = Vec::new();
        if let Some(body_part_form) = race.body_part_data {
            let body_part_form = esp.resolve_form(body_part_form, race_id).unwrap();

            let mut body_parts = Vec::new();

            let body_part_data_record = esp
                .lookup(Some(RecordKind::BodyPartData), body_part_form)
                .await
                .unwrap();

            let RecordData::BodyPartData(body_part_data) = body_part_data_record.local.data else {
                unreachable!();
            };

            for (id, part) in body_part_data.parts.iter().enumerate() {
                body_parts.push(BodyPartBoneProps {
                    id: id as u32,
                    start_bone: std::str::from_utf8(&part.part_node).unwrap().to_owned(),
                });

                let condition = esp
                    .resolve_form(part.data.actor_value, body_part_data_record.id)
                    .unwrap();

                actor_body_parts.push(ActorBodyPart {
                    condition,
                    damage_factor: part.data.damage_mult,
                });
            }

            character.body_parts = body_parts;
        }

        let mut subgraphs = Vec::new();
        for subgraph in load_subgraphs(esp, race_id, race).await {
            let search_paths = subgraph
                .animation_paths
                .iter()
                .map(DataPathBuf::new)
                .collect::<Vec<_>>();

            subgraphs.push(ActorAnimationGraph {
                behavior: DataPathBuf::new(&subgraph.behavior_graph),
                actor_keywords: subgraph.actor_keywords.clone(),
                target_keywords: subgraph.target_keywords.clone(),
                flags: subgraph.flags,
                search_paths,
            });
        }

        Ok(Self {
            anim_graphs: ActorAnimationGraphData {
                character: character.clone(),
                root_path: character.root_path.clone(),
                root_behavior_path: character.behavior_filename.clone(),
                subgraphs,
            },
            controller: ControllerShape {
                offset: character.controller_offset,
                shape: character.controller_shape.clone(),
            },
            character,
            body_parts: ActorBodyParts(actor_body_parts),
        })
    }

    pub fn instantiate(self, world: &mut World, root: Entity) {
        let collider = world.spawn((
            self.controller.offset,
            Collider {
                friction: 1.0,
                restitution: 1.0,
                shape: self.controller.shape.clone(),
                layer: CollisionLayer::CHARACTER_CONTROLLER,
            },
        ));

        self.character.instantiate(root, world);

        world
            .insert(
                root,
                (
                    self.anim_graphs,
                    self.body_parts,
                    RigidBody::FIXED,
                    self.controller,
                    CharacterController::new(),
                ),
            )
            .unwrap();

        world.add_children(root, collider);
    }
}

async fn load_subgraphs(
    esp_loader: &EspLoader,
    race_form: RecordId,
    race_data: &Race,
) -> Vec<RaceSubgraphData> {
    // If we have a template race all subgraph data from the referenced race
    // is added to our race.
    let template_race = race_data
        .subgraph_template_race
        .map(|form| esp_loader.resolve_form(form, race_form).unwrap());

    let mut subgraphs = Vec::new();
    for record in esp_loader.lookup_all(Some(RecordKind::Race)).await.unwrap() {
        let Some(data) = record.local.data.into_variant::<Race>() else {
            continue;
        };

        let is_original_race = record.id == race_form;
        let is_template_race = template_race.is_some_and(|id| id == record.id);
        let is_additive_race = data.subgraph_additive_race.is_some_and(|id| {
            let id = esp_loader.resolve_form(id, record.id).unwrap();
            id == record.id || Some(id) == template_race
        });

        if !is_original_race && !is_template_race && !is_additive_race {
            continue;
        }

        for data in data.subgraph_data.into_iter() {
            let map_form = |form| esp_loader.resolve_form(form, record.id).unwrap();

            subgraphs.push(RaceSubgraphData {
                actor_keywords: data.actor_keywords.into_iter().map(map_form).collect(),
                target_keywords: data.target_keywords.into_iter().map(map_form).collect(),
                behavior_graph: data.behavior_graph,
                animation_paths: data.animation_paths,
                flags: data.flags,
            });
        }
    }

    subgraphs
}
