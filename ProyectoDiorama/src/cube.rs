use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect, CubeFace};
use raylib::prelude::Vector3;

#[derive(Debug, Clone)]
pub struct Cube {
    pub center: Vector3,
    pub size: f32,
    pub material: Material,
}

impl Cube {
    pub fn new(center: Vector3, size: f32, material: Material) -> Self {
        Cube { center, size, material }
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect {
        // Half-size
        let half = self.size * 0.5;
        let min = self.center - Vector3::new(half, half, half);
        let max = self.center + Vector3::new(half, half, half);

        // Reciprocal to avoid divide-by-zero
        let inv_dir = Vector3::new(
            1.0 / ray_direction.x,
            1.0 / ray_direction.y,
            1.0 / ray_direction.z,
        );

        // Intersections with x slabs
        let mut tmin = (min.x - ray_origin.x) * inv_dir.x;
        let mut tmax = (max.x - ray_origin.x) * inv_dir.x;
        if tmin > tmax {
            std::mem::swap(&mut tmin, &mut tmax);
        }

        // y slabs
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

        // z slabs
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

        // Determine which face was hit
        let epsilon = 1e-4;
        let (normal, face, u, v) = if (point.x - min.x).abs() < epsilon {
            // Left face (−X), project to Z/Y
            let u = (point.z - min.z) / (max.z - min.z);
            let v = (point.y - min.y) / (max.y - min.y);
            (Vector3::new(-1.0, 0.0, 0.0), CubeFace::Left, u, v)
        } else if (point.x - max.x).abs() < epsilon {
            // Right face (+X)
            let u = (point.z - min.z) / (max.z - min.z);
            let v = (point.y - min.y) / (max.y - min.y);
            (Vector3::new(1.0, 0.0, 0.0), CubeFace::Right, u, v)
        } else if (point.y - min.y).abs() < epsilon {
            // Bottom face (−Y)
            let u = (point.x - min.x) / (max.x - min.x);
            let v = (point.z - min.z) / (max.z - min.z);
            (Vector3::new(0.0, -1.0, 0.0), CubeFace::Bottom, u, v)
        } else if (point.y - max.y).abs() < epsilon {
            // Top face (+Y)
            let u = (point.x - min.x) / (max.x - min.x);
            let v = (point.z - min.z) / (max.z - min.z);
            (Vector3::new(0.0, 1.0, 0.0), CubeFace::Top, u, v)
        } else if (point.z - min.z).abs() < epsilon {
            // Back face (−Z)
            let u = (point.x - min.x) / (max.x - min.x);
            let v = (point.y - min.y) / (max.y - min.y);
            (Vector3::new(0.0, 0.0, -1.0), CubeFace::Back, u, v)
        } else {
            // Front face (+Z)
            let u = (point.x - min.x) / (max.x - min.x);
            let v = (point.y - min.y) / (max.y - min.y);
            (Vector3::new(0.0, 0.0, 1.0), CubeFace::Front, u, v)
        };

        Intersect::new(point, normal, t, self.material.clone(), u, v, face)
    }
}
