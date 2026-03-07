mod triangle;

use glam::Vec3;
use petgraph::algo::astar;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Graph, Undirected};
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{RTree, AABB};

use crate::navmesh::triangle::{closest_points_between_triangles, is_inside_triangle};

const HEIGHT: f32 = 2.0;
const UP: Vec3 = Vec3::Y;

const NEIGHBOR_THRESHOLD: f32 = 0.001;

#[derive(Clone, Debug, Default)]
pub struct NavMesh {
    triangles: Vec<[Vec3; 3]>,
    centers: Vec<Vec3>,
    graph: Graph<usize, (), Undirected>,
    tree: RTree<GeomWithData<Rectangle<[f32; 3]>, usize>>,
}

impl NavMesh {
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
            centers: Vec::new(),
            graph: Graph::new_undirected(),
            tree: RTree::new(),
        }
    }

    pub fn append(&mut self, mesh: &Mesh) {
        for &[i0, i1, i2] in mesh.indices.as_chunks().0 {
            let triangle = [
                mesh.triangles[i0 as usize],
                mesh.triangles[i1 as usize],
                mesh.triangles[i2 as usize],
            ];

            let new_index = self.triangles.len();

            let center = Vec3::new(
                (triangle[0].x + triangle[1].x + triangle[2].x) / 3.0,
                (triangle[0].y + triangle[1].y + triangle[2].y) / 3.0,
                (triangle[0].z + triangle[1].z + triangle[2].z) / 3.0,
            );

            self.graph.add_node(new_index);

            let aabb = AABB::from_points([
                &triangle[0].to_array(),
                &triangle[1].to_array(),
                &triangle[2].to_array(),
            ]);

            for rect in self.tree.locate_in_envelope_intersecting(&aabb) {
                let index = rect.data;
                let tri = self.triangles[index];

                let (_, d) = closest_points_between_triangles(tri, triangle);
                if d > NEIGHBOR_THRESHOLD {
                    continue;
                }

                self.graph
                    .add_edge(NodeIndex::new(new_index), NodeIndex::new(index), ());
            }

            self.triangles.push(triangle);
            self.centers.push(center);
            self.tree
                .insert(GeomWithData::new(Rectangle::from_aabb(aabb), new_index));
        }
    }

    pub fn search(&self, agent: &Agent, target: Vec3) -> Option<Path> {
        let start = self
            .triangles
            .iter()
            .position(|tri| is_inside_triangle(*tri, HEIGHT, agent.position))
            .unwrap();
        let end = self
            .triangles
            .iter()
            .position(|tri| is_inside_triangle(*tri, HEIGHT, target))
            .unwrap();

        let res = astar(
            &self.graph,
            NodeIndex::new(start),
            |index| index.index() == end,
            |edge| {
                let a = self.centers[edge.source().index()];
                let b = self.centers[edge.target().index()];
                a.distance_squared(b)
            },
            |index| {
                let a = self.centers[index.index()];
                let b = self.centers[end];
                a.distance_squared(b)
            },
        )?;

        let triangles: Vec<_> = res.1.iter().map(|v| v.index()).collect();

        Some(Path { triangles })
    }

    /// Returns an iterator over all triangles stored in the `NavMesh`.
    ///
    /// Primarily meant for debugging purposes.
    pub fn triangles(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        self.triangles.iter().copied()
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub triangles: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Obstacle {
    pub triangles: Vec<[Vec3; 3]>,
}

pub struct Mesh {
    pub triangles: Vec<Vec3>,
    pub indices: Vec<u32>,
}

pub struct ObstacleId();

pub struct Agent {
    pub position: Vec3,
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use crate::navmesh::{Agent, Mesh, NavMesh};

    #[test]
    fn navmesh_search_single_tri() {
        let mut mesh = NavMesh::new();
        mesh.append(&Mesh {
            triangles: vec![
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            indices: vec![0, 1, 2],
        });

        let path = mesh
            .search(
                &Agent {
                    position: Vec3::new(-1.0, 0.0, -1.0),
                },
                Vec3::Z,
            )
            .unwrap();

        assert_eq!(path.triangles, [0]);
    }

    #[test]
    fn navmesh_search_two_tris() {
        let mut mesh = NavMesh::new();
        mesh.append(&Mesh {
            triangles: vec![
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(-1.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, 1.0),
            ],
            indices: vec![0, 1, 2, 2, 1, 3],
        });

        let path = mesh
            .search(
                &Agent {
                    position: Vec3::new(-1.0, 0.0, -1.0),
                },
                Vec3::new(1.0, 0.0, 1.0),
            )
            .unwrap();

        assert_eq!(path.triangles, [0, 1]);
    }
}
