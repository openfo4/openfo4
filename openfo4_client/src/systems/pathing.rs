use glam::Vec3;

#[derive(Clone, Debug)]
pub struct NavMesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}
