mod ssf;
pub mod triangle;

use glam::Vec3;
use openfo4_common::math::Ray;
use openfo4_tracing::trace_span;
use petgraph::algo::astar;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Graph, Undirected};
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{RTree, AABB};

use crate::navmesh::triangle::{closest_points_between_triangles, is_inside_triangle, Triangle};

const HEIGHT: f32 = 2.0;
const UP: Vec3 = Vec3::Y;

const NEIGHBOR_THRESHOLD: f32 = 0.001;

#[derive(Clone, Debug, Default)]
pub struct NavMesh {
    triangles: Vec<Triangle>,
    centers: Vec<Vec3>,
    graph: Graph<usize, f32, Undirected>,
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
            let triangle = Triangle::from_vertices(
                [
                    mesh.triangles[i0 as usize],
                    mesh.triangles[i1 as usize],
                    mesh.triangles[i2 as usize],
                ],
                UP,
            );

            let new_index = self.triangles.len();

            let center = triangle.centroid();

            let i = self.graph.add_node(new_index);
            debug_assert_eq!(i.index(), new_index);

            let aabb = AABB::from_points([
                &triangle.a.to_array(),
                &triangle.b.to_array(),
                &triangle.c.to_array(),
            ]);

            for rect in self.tree.locate_in_envelope_intersecting(&aabb) {
                let index = rect.data;
                let tri = self.triangles[index];

                let (_, d) = closest_points_between_triangles(tri.vertices(), triangle.vertices());
                if d > NEIGHBOR_THRESHOLD {
                    continue;
                }

                let len = triangle.shared_edge_length(&tri, NEIGHBOR_THRESHOLD);

                self.graph
                    .update_edge(NodeIndex::new(new_index), NodeIndex::new(index), len);
            }

            self.triangles.push(triangle);
            self.centers.push(center);
            self.tree
                .insert(GeomWithData::new(Rectangle::from_aabb(aabb), new_index));
        }
    }

    /// Performs a raycast on the navmesh triangles.
    pub fn cast_ray(&self, ray: Ray, max_toi: f32) -> Option<f32> {
        let _span = trace_span!("NavMesh::cast_ray").entered();

        let mut dist_sq = f32::MAX;
        let max_toi_sq = max_toi * max_toi;

        for tri in &self.triangles {
            if let Some(pt) = tri.ray_intersection(ray) {
                let d_sq = ray.origin.distance_squared(pt);
                if d_sq < 0.0 || d_sq > max_toi_sq {
                    continue;
                }

                dist_sq = f32::min(dist_sq, d_sq);
            }
        }

        (dist_sq != f32::MAX).then(|| dist_sq.sqrt())
    }

    pub fn search(&self, agent: &Agent, target: Vec3) -> Option<Path> {
        // TODO: We can also use the RTree here, but we need to consider triangle height.

        let start = self
            .triangles
            .iter()
            .position(|tri| is_inside_triangle(tri.vertices(), HEIGHT, agent.position))?;
        let end = self
            .triangles
            .iter()
            .position(|tri| is_inside_triangle(tri.vertices(), HEIGHT, target))?;

        let res = astar(
            &self.graph,
            NodeIndex::new(start),
            |index| index.index() == end,
            |edge| {
                let width = edge.weight();
                if *width < agent.width {
                    f32::MAX
                } else {
                    let a = self.centers[edge.source().index()];
                    let b = self.centers[edge.target().index()];
                    a.distance_squared(b)
                }
            },
            |index| {
                let a = self.centers[index.index()];
                let b = self.centers[end];
                a.distance_squared(b)
            },
        )?;

        let triangles: Vec<_> = res.1.iter().map(|v| v.index()).collect();

        let tris: Vec<_> = triangles
            .iter()
            .map(|index| self.triangles[*index])
            .collect();

        // Currently width is included in our A* path cost heuristic, which
        // means A* will never choose it if another valid exists, however
        // when only one path exists, A* will choose paths that are not wide
        // enough.
        // We never want to return a path that is not wide enough, so we filter
        // them out here.
        for [a, b] in triangles.array_windows::<2>() {
            for edge in self
                .graph
                .edges_connecting((*a as u32).into(), (*b as u32).into())
            {
                let width = edge.weight();
                if *width < agent.width {
                    return None;
                }
            }
        }

        let mut waypoints = ssf::ssf(&tris, agent.position);
        waypoints.push(target);

        Some(Path {
            triangles,
            waypoints,
        })
    }

    /// Returns an iterator over all triangles stored in the `NavMesh`.
    ///
    /// Primarily meant for debugging purposes.
    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub triangles: Vec<usize>,
    /// Points the actor should pass through to reach the target.
    pub waypoints: Vec<Vec3>,
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
    pub width: f32,
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
                    width: 0.0,
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
                    width: 0.0,
                },
                Vec3::new(1.0, 0.0, 1.0),
            )
            .unwrap();

        assert_eq!(path.triangles, [0, 1]);
    }
}
