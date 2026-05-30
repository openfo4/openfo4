//! Utility functions for triangle math

use game_common::math::Ray;
use glam::Vec3;

/// A triangle with counter-clockwise winding.
#[derive(Copy, Clone, Debug)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    pub fn from_vertices(verts: [Vec3; 3], up: Vec3) -> Self {
        debug_assert!(up.is_normalized());

        let tri = Self {
            a: verts[0],
            b: verts[1],
            c: verts[2],
        };

        if tri.normal().dot(up).is_sign_negative() {
            tri.flipped()
        } else {
            tri
        }
    }

    pub const fn centroid(&self) -> Vec3 {
        Vec3 {
            x: (self.a.x + self.b.x + self.c.x) / 3.0,
            y: (self.a.y + self.b.y + self.c.y) / 3.0,
            z: (self.a.z + self.b.z + self.c.z) / 3.0,
        }
    }

    pub fn normal(&self) -> Vec3 {
        -(self.b - self.a).cross(self.c - self.a).normalize_or_zero()
    }

    pub const fn flipped(self) -> Self {
        Self {
            a: self.a,
            b: self.c,
            c: self.b,
        }
    }

    pub const fn vertices(&self) -> [Vec3; 3] {
        [self.a, self.b, self.c]
    }

    pub const fn edges(&self) -> [[Vec3; 2]; 3] {
        [[self.a, self.b], [self.b, self.c], [self.c, self.a]]
    }

    /// Returns the intersection point between the `Triangle` and the `Ray`.
    pub fn ray_intersection(&self, ray: Ray) -> Option<Vec3> {
        // https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm

        let edge1 = self.b - self.a;
        let edge2 = self.c - self.a;

        let re2 = ray.direction.cross(edge2);
        let det = edge1.dot(re2);
        // Triangle and ray parallel.
        if det.abs() <= f32::EPSILON {
            return None;
        }

        let inv_det = 1.0 / det;
        let s = ray.origin - self.a;
        let u = inv_det * s.dot(re2);
        if u < -f32::EPSILON || u - 1.0 > f32::EPSILON {
            return None;
        }

        let se1 = s.cross(edge1);
        let v = inv_det * ray.direction.dot(se1);
        if v < -f32::EPSILON || u + v - 1.0 > f32::EPSILON {
            return None;
        }

        let t = inv_det * edge2.dot(se1);
        Some(ray.point(t))
    }
}

pub fn is_inside_triangle(triangle: [Vec3; 3], height: f32, point: Vec3) -> bool {
    let tri = Triangle::from_vertices(triangle, crate::navmesh::UP);

    let normal = tri.normal();
    let d = normal.dot(point - tri.a);
    if d < 0.0 || d > height {
        return false;
    }

    let proj = point - tri.normal() * d;

    let n = (tri.b - tri.a).cross(tri.c - tri.a);
    let d0 = (tri.b - tri.a).cross(proj - tri.a);
    let d1 = (tri.c - tri.b).cross(proj - tri.b);
    let d2 = (tri.a - tri.c).cross(proj - tri.c);

    n.dot(d0) >= 0.0 && n.dot(d1) >= 0.0 && n.dot(d2) >= 0.0
}

pub fn closest_points_between_triangles(
    lhs: [Vec3; 3],
    rhs: [Vec3; 3],
) -> (Option<[Vec3; 2]>, f32) {
    // https://stackoverflow.com/a/53622315

    let mut min_d = f32::MAX;
    let mut min_p = Vec3::ZERO;
    let mut min_q = Vec3::ZERO;
    let mut disjoint = false;

    for i in 0..3 {
        for j in 0..3 {
            let p0 = lhs[i];
            let p1 = lhs[(i + 1) % 3];
            let q0 = rhs[j];
            let q1 = rhs[(j + 1) % 3];

            let [cp, cq] = closest_points_between_lines([p0, p1], [q0, q1]);
            let v = cq - cp;
            let dd = v.dot(v);

            if dd > min_d {
                continue;
            }

            min_p = cp;
            min_q = cq;
            min_d = dd;

            // Off edges
            let op = lhs[(i + 2) % 3];
            let oq = rhs[(i + 2) % 3];

            let zp = op - cp;
            let zq = oq - cq;
            let mut a = zp.dot(v);
            let mut b = zq.dot(v);

            // Same side
            if a <= 0.0 && b >= 0.0 {
                return (Some([cp, cq]), v.dot(v));
            }

            if a < 0.0 {
                a = 0.0;
            } else if b > 0.0 {
                b = 0.0;
            }

            if min_d - a + b > 0.0 {
                disjoint = true;
            }
        }
    }

    for [tri_a, tri_b] in [[lhs, rhs], [rhs, lhs]] {
        let normal_a = (tri_a[0] - tri_a[1]).cross(tri_a[2] - tri_a[0]);

        if normal_a.length_squared() <= f32::EPSILON {
            continue;
        }

        let p0 = tri_a[0];
        let p1 = tri_a[1];
        let p2 = tri_a[2];
        let q0 = tri_b[0];
        let q1 = tri_b[1];
        let q2 = tri_b[2];

        let proj0 = (p0 - q0).dot(normal_a);
        let proj1 = (p1 - q1).dot(normal_a);
        let proj2 = (p2 - q2).dot(normal_a);
        let proj = Vec3::new(proj0, proj1, proj2);

        // All points have the same sign.
        // Find the point with the shortest distance.
        let index = if proj.cmpgt(Vec3::ZERO).all() {
            Some(proj.min_position())
        } else if proj.cmplt(Vec3::ZERO).all() {
            Some(proj.max_position())
        } else {
            None
        };

        if let Some(index) = index {
            disjoint = true;

            let v = [
                tri_b[index] - tri_b[0],
                tri_b[index] - tri_b[1],
                tri_b[index] - tri_b[2],
            ];
            let z = [
                normal_a.cross(tri_a[1] - tri_a[0]),
                normal_a.cross(tri_a[2] - tri_a[1]),
                normal_a.cross(tri_a[0] - tri_a[2]),
            ];

            if v.iter().zip(z).all(|(v, z)| v.dot(z) > 0.0) {
                let cp = rhs[index] + normal_a * proj[index] / normal_a.length_squared();
                let cq = rhs[index];
                return (Some([cp, cq]), (cp - cq).length_squared());
            }
        }
    }

    if disjoint {
        (Some([min_p, min_q]), min_d)
    } else {
        // Triangles intersect
        (None, 0.0)
    }
}

fn closest_points_between_lines([p0, p1]: [Vec3; 2], [q0, q1]: [Vec3; 2]) -> [Vec3; 2] {
    // https://math.stackexchange.com/a/4289668
    let v1 = p1 - p0;
    let v2 = q1 - q0;
    let v21 = q0 - p0;

    let d22 = v2.dot(v2);
    let d11 = v1.dot(v1);
    let d21 = v2.dot(v1);
    let d21_1 = v21.dot(v1);
    let d21_2 = v21.dot(v2);
    let denom = d21 * d21 - d22 * d11;

    let mut s;
    let mut t;
    if denom.abs() < f32::EPSILON {
        s = 0.0;
        t = d11 / d21;
    } else {
        s = (d21_2 * d21 - d22 * d21_1) / denom;
        t = (-d21_1 * d21 + d11 * d21_2) / denom;
    }

    s = s.clamp(0.0, 1.0);
    t = t.clamp(0.0, 1.0);

    let p_a = p0 + s * v1;
    let p_b = q0 + t * v2;

    [p_a, p_b]
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use game_common::assert_approx_eq;
    use game_common::math::Ray;
    use glam::Vec3;

    use crate::navmesh::triangle::Triangle;

    use super::is_inside_triangle;

    #[test]
    fn test_is_inside_triangle() {
        const HEIGHT: f32 = 2.0;

        assert!(is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            Vec3::ZERO,
        ));
        assert!(is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            Vec3::Z,
        ));
        assert!(!is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            Vec3::X,
        ));
        assert!(is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            Vec3::Y
        ));
        assert!(!is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            -Vec3::Y * f32::EPSILON
        ));
        assert!(is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            Vec3::Y * HEIGHT
        ));
        assert!(!is_inside_triangle(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            HEIGHT,
            Vec3::Y * HEIGHT * 1.1
        ));
        assert!(is_inside_triangle(
            [
                Vec3::new(-742.95374, 64.64856, -773.8204),
                Vec3::new(-739.74963, 64.579506, -774.95404),
                Vec3::new(-742.49286, 64.579506, -779.06885),
            ],
            HEIGHT,
            Vec3::new(-741.2727, 64.590034, -776.333)
        ));
        assert!(is_inside_triangle(
            [
                Vec3::new(-742.95374, 64.64856, -773.8204),
                Vec3::new(-742.49286, 64.579506, -779.06885),
                Vec3::new(-739.74963, 64.579506, -774.95404),
            ],
            HEIGHT,
            Vec3::new(-741.6442, 64.59865, -776.14966)
        ));
        assert!(is_inside_triangle(
            [
                Vec3::new(-742.95374, 64.64856, -773.8204,),
                Vec3::new(-742.49286, 64.579506, -779.06885,),
                Vec3::new(-739.74963, 64.579506, -774.95404,),
            ],
            HEIGHT,
            Vec3::new(-741.38403, 64.59744, -775.8629)
        ));
    }

    #[test]
    fn triangle_normal() {
        let triangle = Triangle::from_vertices(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            Vec3::Y,
        );

        assert_eq!(triangle.normal(), Vec3::Y);

        let triangle = Triangle::from_vertices(
            [
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, -1.0),
            ],
            Vec3::Y,
        );

        assert_eq!(triangle.normal(), Vec3::Y);
    }

    #[test]
    fn triangle_normal_rotated() {
        let triangle = Triangle::from_vertices(
            [
                Vec3::new(-1.0, 0.0, -1.0).rotate_x(PI / 4.0),
                Vec3::new(1.0, 0.0, -1.0).rotate_x(PI / 4.0),
                Vec3::new(0.0, 0.0, 1.0).rotate_x(PI / 4.0),
            ],
            Vec3::Y,
        );

        assert_approx_eq!(triangle.normal(), Vec3::Y.rotate_x(PI / 4.0));

        let triangle = Triangle::from_vertices(
            [
                Vec3::new(-1.0, 0.0, -1.0).rotate_x(PI / 4.0),
                Vec3::new(0.0, 0.0, 1.0).rotate_x(PI / 4.0),
                Vec3::new(1.0, 0.0, -1.0).rotate_x(PI / 4.0),
            ],
            Vec3::Y,
        );

        assert_approx_eq!(triangle.normal(), Vec3::Y.rotate_x(PI / 4.0));
    }

    #[test]
    fn triangle_normal_winding_order_independent() {
        let a = Vec3::new(1.0, 0.0, 1.0);
        let b = Vec3::new(0.0, 0.0, 1.0);
        let c = Vec3::new(1.0, 0.0, 0.0);

        let tri0 = Triangle::from_vertices([a, b, c], Vec3::Y);
        let tri1 = Triangle::from_vertices([a, c, b], Vec3::Y);
        let tri2 = Triangle::from_vertices([b, a, c], Vec3::Y);
        let tri3 = Triangle::from_vertices([b, c, a], Vec3::Y);
        let tri4 = Triangle::from_vertices([c, b, a], Vec3::Y);
        let tri5 = Triangle::from_vertices([c, a, b], Vec3::Y);
        assert_eq!(tri0.normal(), tri1.normal());
        assert_eq!(tri0.normal(), tri2.normal());
        assert_eq!(tri0.normal(), tri3.normal());
        assert_eq!(tri0.normal(), tri4.normal());
        assert_eq!(tri0.normal(), tri5.normal());
    }

    #[test]
    fn triangle_ray_intersection() {
        let a = Vec3::new(1.0, 0.0, 1.0);
        let b = Vec3::new(0.0, 0.0, 1.0);
        let c = Vec3::new(1.0, 0.0, 0.0);
        let tri = Triangle::from_vertices([a, b, c], Vec3::Y);

        let ray = Ray::new(Vec3::new(1.0, -1.0, 1.0), Vec3::Y);
        assert_eq!(tri.ray_intersection(ray), Some(Vec3::new(1.0, 0.0, 1.0)));

        let ray = Ray::new(Vec3::new(1.0, 0.0, 1.0), Vec3::Y);
        assert_eq!(tri.ray_intersection(ray), Some(Vec3::new(1.0, 0.0, 1.0)));

        let ray = Ray::new(Vec3::new(0.0, -1.0, 0.0), Vec3::ONE.normalize());
        assert_eq!(tri.ray_intersection(ray), Some(Vec3::new(1.0, 0.0, 1.0)));
    }
}
