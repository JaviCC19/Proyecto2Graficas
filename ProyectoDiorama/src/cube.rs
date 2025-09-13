use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use raylib::prelude::Vector3;

pub struct Cube {
    pub center: Vector3,
    pub size: f32,
    pub material: Material,
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect {
        // Half-size
        let half = self.size * 0.5;
        let min = self.center - Vector3::new(half, half, half);
        let max = self.center + Vector3::new(half, half, half);

        // Avoid divide by zero: use reciprocal
        let inv_dir = Vector3::new(
            1.0 / ray_direction.x,
            1.0 / ray_direction.y,
            1.0 / ray_direction.z,
        );

        // Compute intersections with x, y, z slabs
        let mut tmin = (min.x - ray_origin.x) * inv_dir.x;
        let mut tmax = (max.x - ray_origin.x) * inv_dir.x;
        if tmin > tmax {
            std::mem::swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (min.y - ray_origin.y) * inv_dir.y;
        let mut tymax = (max.y - ray_origin.y) * inv_dir.y;
        if tymin > tymax {
            std::mem::swap(&mut tymin, &mut tymax);
        }

        if (tmin > tymax) || (tymin > tmax) {
            return Intersect::empty();
        }

        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }

        let mut tzmin = (min.z - ray_origin.z) * inv_dir.z;
        let mut tzmax = (max.z - ray_origin.z) * inv_dir.z;
        if tzmin > tzmax {
            std::mem::swap(&mut tzmin, &mut tzmax);
        }

        if (tmin > tzmax) || (tzmin > tmax) {
            return Intersect::empty();
        }

        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }

        // Closest intersection distance
        let t = if tmin > 0.0 { tmin } else { tmax };
        if t < 0.0 {
            return Intersect::empty();
        }

        // Hit point
        let point = *ray_origin + *ray_direction * t;

        // Normal: determine which face we hit
        let mut normal = Vector3::zero();
        let epsilon = 1e-4;
        if (point.x - min.x).abs() < epsilon {
            normal = Vector3::new(-1.0, 0.0, 0.0);
        } else if (point.x - max.x).abs() < epsilon {
            normal = Vector3::new(1.0, 0.0, 0.0);
        } else if (point.y - min.y).abs() < epsilon {
            normal = Vector3::new(0.0, -1.0, 0.0);
        } else if (point.y - max.y).abs() < epsilon {
            normal = Vector3::new(0.0, 1.0, 0.0);
        } else if (point.z - min.z).abs() < epsilon {
            normal = Vector3::new(0.0, 0.0, -1.0);
        } else if (point.z - max.z).abs() < epsilon {
            normal = Vector3::new(0.0, 0.0, 1.0);
        }

        Intersect::new(point, normal, t, self.material)
    }
}
