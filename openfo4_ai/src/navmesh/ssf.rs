use glam::{Mat3, Vec3};
use openfo4_common::math::LineSegment;

use crate::navmesh::triangle::Triangle;

/// Compute the waypoints for passing through all given [`Triangle`]s using the Simple Stupid
/// Funnel Algorithm.
///
/// The starting point is included in the waypoint list, but the target is not.
pub fn ssf(triangles: &[Triangle], mut start: Vec3) -> Vec<Vec3> {
    let portals = portals(triangles);

    let mut waypoints = Vec::new();
    waypoints.push(start);

    let Some(first) = portals.first() else {
        return waypoints;
    };

    let mut cur_left = first[0];
    let mut cur_right = first[1];
    let mut apex = Vec3::angle_between(cur_left - start, cur_right - start);

    let mut left_locked = false;
    let mut right_locked = false;

    for [left, right] in portals.into_iter().skip(1) {
        let a = Vec3::angle_between(left - start, cur_right - start);
        if a <= apex {
            cur_left = left;
            apex = a;
        } else {
            left_locked = true;
        }

        let a = Vec3::angle_between(right - start, cur_left - start);
        if a <= apex {
            cur_right = right;
            apex = a;
        } else {
            right_locked = true;
        }

        if left_locked && right_locked {
            left_locked = false;
            right_locked = false;

            let point = if start.distance_squared(cur_left) < start.distance_squared(cur_right) {
                cur_left
            } else {
                cur_right
            };

            waypoints.push(point);
            start = point;

            cur_left = left;
            cur_right = right;
            apex = Vec3::angle_between(cur_left - start, cur_right - start);
        }
    }

    waypoints
}

fn portals(triangles: &[Triangle]) -> Vec<[Vec3; 2]> {
    let mut portals = Vec::new();

    for [src, dst] in triangles.array_windows::<2>() {
        let source = src.centroid();
        let target = dst.centroid();

        let mut min = [Vec3::ZERO; 2];
        let mut min_dist = f32::MAX;
        for [a, b] in src.edges() {
            let p = LineSegment::new(a, b).nearest_point(target);
            let dist = p.distance(target);

            if dist <= min_dist {
                // The left and right vertices must be consistent across
                // every portal.
                // We can find the side by spanning a plane between the source
                // and target triangles that passes through the edge and checks
                // on which side the point a is.
                let m = Mat3::from_cols(target - source, src.normal(), a - source);
                if m.determinant() > 0.0 {
                    min = [a, b];
                } else {
                    min = [b, a];
                }

                min_dist = dist;
            }
        }

        portals.push(min);
    }

    portals
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use crate::navmesh::triangle::Triangle;

    use super::ssf;

    const UP: Vec3 = Vec3::Y;

    fn grid_rect(x: i32, z: i32) -> [Triangle; 2] {
        let x_min = x as f32;
        let z_min = z as f32;
        let x_max = (x + 1) as f32;
        let z_max = (z + 1) as f32;

        [
            Triangle::from_vertices(
                [
                    Vec3::new(x_min, 0.0, z_min),
                    Vec3::new(x_min, 0.0, z_max),
                    Vec3::new(x_max, 0.0, z_min),
                ],
                UP,
            ),
            Triangle::from_vertices(
                [
                    Vec3::new(x_min, 0.0, z_max),
                    Vec3::new(x_max, 0.0, z_min),
                    Vec3::new(x_max, 0.0, z_max),
                ],
                UP,
            ),
        ]
    }

    #[test]
    fn ssf_linear_path_rect() {
        let tris = [
            Triangle::from_vertices(
                [
                    Vec3::new(-1.0, 0.0, -1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(-1.0, 0.0, 1.0),
                ],
                UP,
            ),
            Triangle::from_vertices(
                [
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(-1.0, 0.0, 1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                ],
                UP,
            ),
        ];

        let waypoints = ssf(&tris, Vec3::new(-1.0, 0.0, -1.0));
        assert_eq!(waypoints, [Vec3::new(-1.0, 0.0, -1.0)]);
    }

    #[test]
    fn ssf_complex_path() {
        // Path: -Z down, +X right
        // x x x x
        // - - - x
        // x x x x
        let mut tris = Vec::new();
        tris.extend(grid_rect(0, 0));
        tris.extend(grid_rect(1, 0));
        tris.extend(grid_rect(2, 0));
        tris.extend(grid_rect(3, 0));
        tris.extend(grid_rect(3, -1));
        tris.extend(grid_rect(3, -2));
        tris.extend(grid_rect(2, -2));
        tris.extend(grid_rect(1, -2));
        tris.extend(grid_rect(0, -2));

        let waypoints = ssf(&tris, Vec3::ZERO);
        assert_eq!(
            waypoints,
            [
                Vec3::ZERO,
                Vec3::new(3.0, 0.0, 0.0),
                // We're moving up to this point instead of (3, -1)
                // because our triangle mesh is suboptimal.
                Vec3::new(4.0, 0.0, -1.0),
            ]
        );
    }
}
