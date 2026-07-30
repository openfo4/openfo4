use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_esp::records::{NavMesh, NavMeshTriangleFlags2};

use crate::assets::nif::NIF_TO_METERS;
use crate::components::NiTransform;
use crate::state::level::LoadContext;
use crate::state::level::record::SpawnEntity;
use crate::systems::pathing;

pub async fn load_navmesh(ctx: &LoadContext<'_>, data: &NavMesh) -> SpawnEntity {
    let vertices: Vec<_> = data
        .geometry
        .vertices
        .iter()
        .map(|v| v.to_opengl() * NIF_TO_METERS)
        .collect();
    let mut indices = Vec::new();

    for tri in &data.geometry.triangles {
        if tri.flags2.contains(NavMeshTriangleFlags2::FLOAT) {
            continue;
        }

        for index in tri.vertices {
            assert!((index as usize) < vertices.len());
            indices.push(index.into());
        }
    }

    SpawnEntity {
        transform: NiTransform::from_translation(ctx.grid_offset().extend(0.0)),
        navmesh: Some(pathing::NavMesh { vertices, indices }),
        // FIXME: Don't do this.
        persistent: true,
        ..Default::default()
    }
}
