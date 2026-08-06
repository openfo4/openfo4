use bevy_mikktspace::{Geometry, TangentSpace};
use game_tracing::trace_span;
use glam::{Vec2, Vec3, Vec4};

// FIXME: Meshes will be duplicated quite a bit, so
// we don't want to have it attached to every entity.
#[derive(Clone, Debug)]
pub struct Mesh {
    indices: Option<Indices>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    tangents: Vec<Vec4>,
    tangents_set: bool,
    colors: Vec<Vec4>,
    skinning_joint_indices: Vec<[u8; 4]>,
    skinning_join_weights: Vec<Vec4>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            indices: None,
            positions: vec![],
            normals: vec![],
            uvs: vec![],
            tangents: vec![],
            tangents_set: false,
            colors: Vec::new(),
            skinning_joint_indices: Vec::new(),
            skinning_join_weights: Vec::new(),
        }
    }

    pub fn set_indices(&mut self, indices: Indices) {
        self.indices = Some(indices);
    }

    pub fn set_positions(&mut self, positions: Vec<Vec3>) {
        self.positions = positions;
    }

    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    pub fn set_normals(&mut self, normals: Vec<Vec3>) {
        self.normals = normals;
    }

    pub fn normals(&self) -> &[Vec3] {
        &self.normals
    }

    pub fn tangents(&self) -> &[Vec4] {
        &self.tangents
    }

    pub fn colors(&self) -> &[Vec4] {
        &self.colors
    }

    pub fn set_tangents(&mut self, tangents: Vec<Vec4>) {
        self.tangents = tangents;
        self.tangents_set = true;
    }

    pub fn set_colors(&mut self, colors: Vec<Vec4>) {
        self.colors = colors;
    }

    pub fn set_uvs(&mut self, uvs: Vec<Vec2>) {
        self.uvs = uvs;
    }

    pub fn uvs(&self) -> &[Vec2] {
        &self.uvs
    }

    pub fn skinning_joint_weights(&self) -> &[Vec4] {
        &self.skinning_join_weights
    }

    pub fn set_skinning_joint_weights(&mut self, v: Vec<Vec4>) {
        self.skinning_join_weights = v;
    }

    pub fn skinning_joint_indices(&self) -> &[[u8; 4]] {
        &self.skinning_joint_indices
    }

    pub fn set_skinning_joint_indices(&mut self, v: Vec<[u8; 4]>) {
        self.skinning_joint_indices = v;
    }

    pub fn is_skinned(&self) -> bool {
        !self.positions.is_empty() && !self.skinning_join_weights.is_empty()
    }

    pub fn indicies(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    pub fn compute_tangents(&mut self) {
        let _span = trace_span!("Mesh::compute_tangents").entered();

        // TODO: Checks and precomputes should move to gltf crate.
        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.uvs.len());

        let len = self.positions.len();

        let mut mesh = Mikktpsace {
            indices: self.indices.as_ref().unwrap().as_u32(),
            positions: &self.positions,
            normals: &self.normals,
            uvs: &self.uvs,
            tangents: vec![Vec4::ZERO; len],
        };
        bevy_mikktspace::generate_tangents(&mut mesh).unwrap();

        self.tangents = mesh.tangents;
    }

    pub fn tangents_set(&self) -> bool {
        self.tangents_set
    }

    pub fn validate_indices(&self) -> Result<(), InvalidIndices> {
        let Some(indices) = &self.indices else {
            return Ok(());
        };

        let has_normals = !self.normals.is_empty();
        let has_uvs = !self.uvs.is_empty();
        let has_tangents = !self.tangents.is_empty();
        let has_colors = !self.colors.is_empty();
        let has_skinning_indices = !self.skinning_joint_indices.is_empty();
        let has_skinning_weights = !self.skinning_join_weights.is_empty();

        fn assert_in_bounds<T>(arr: &[T], index: usize) -> Result<(), InvalidIndices> {
            if arr.len() <= index {
                Err(InvalidIndices)
            } else {
                Ok(())
            }
        }

        let assert_in_bounds = |index| {
            assert_in_bounds(&self.positions, index)?;
            if has_normals {
                assert_in_bounds(&self.normals, index)?;
            }
            if has_uvs {
                assert_in_bounds(&self.uvs, index)?;
            }
            if has_tangents {
                assert_in_bounds(&self.tangents, index)?;
            }
            if has_colors {
                assert_in_bounds(&self.colors, index)?;
            }
            if has_skinning_indices {
                assert_in_bounds(&self.skinning_joint_indices, index)?;
            }
            if has_skinning_weights {
                assert_in_bounds(&self.skinning_join_weights, index)?;
            }

            Ok(())
        };

        match indices {
            Indices::U16(indices) => {
                for index in indices {
                    assert_in_bounds(*index as usize)?;
                }
            }
            Indices::U32(indices) => {
                for index in indices {
                    assert_in_bounds(*index as usize)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidIndices;

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    pub fn len(&self) -> u32 {
        match self {
            Self::U16(buf) => buf.len() as u32,
            Self::U32(buf) => buf.len() as u32,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_u32(&self) -> &[u32] {
        match self {
            Self::U32(val) => val,
            _ => panic!("`Indicies` is not `U32`"),
        }
    }

    pub fn as_u16(&self) -> &[u16] {
        match self {
            Self::U16(val) => val,
            _ => panic!("`Indices` is not `U16`"),
        }
    }

    pub fn into_u32(self) -> Vec<u32> {
        match self {
            Self::U16(val) => val.into_iter().map(u32::from).collect(),
            Self::U32(val) => val,
        }
    }
}

struct Mikktpsace<'a> {
    indices: &'a [u32],
    positions: &'a [Vec3],
    normals: &'a [Vec3],
    uvs: &'a [Vec2],
    tangents: Vec<Vec4>,
}

impl<'a> Mikktpsace<'a> {
    fn index(&self, face: usize, vert: usize) -> usize {
        self.indices[face * 3 + vert] as usize
    }
}

impl<'a> Geometry for Mikktpsace<'a> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.index(face, vert)].to_array()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.index(face, vert)].to_array()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.uvs[self.index(face, vert)].to_array()
    }

    fn set_tangent(&mut self, tangent_space: Option<TangentSpace>, face: usize, vert: usize) {
        let tangent = tangent_space.unwrap_or_default().tangent_encoded();
        let tangent = Vec4::from_array(tangent);
        let index = self.index(face, vert);
        self.tangents[index] = tangent;
    }
}
