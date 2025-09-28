use raylib::prelude::Vector3;
use crate::material::Material;

#[derive(Debug, Clone)]
pub struct Intersect {
    pub point: Vector3,
    pub normal: Vector3,
    pub distance: f32,
    pub is_intersecting: bool,
    pub material: Material,
    pub u: f32,
    pub v: f32,
    pub face: CubeFace,   // which face of the cube was hit
}

#[derive(Debug, Clone, Copy)]
pub enum CubeFace {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

impl Intersect {
    /// Create a filled intersection record
    pub fn new(
        point: Vector3,
        normal: Vector3,
        distance: f32,
        material: Material,
        u: f32,
        v: f32,
        face: CubeFace,          // <- add parameter
    ) -> Self {
        Intersect {
            point,
            normal,
            distance,
            is_intersecting: true,
            material,
            u,
            v,
            face,
        }
    }

    /// Empty intersection (no hit)
    pub fn empty() -> Self {
        Intersect {
            point: Vector3::zero(),
            normal: Vector3::zero(),
            distance: 0.0,
            is_intersecting: false,
            material: Material::with_texture(
                Vector3::zero(), // diffuse color (black)
                0.0,             // specular
                [0.0, 0.0, 0.0, 0.0], // albedo
                1.0,             // refractive index
                'b',             // key or identifier
            ),
            u: 0.0,
            v: 0.0,
            face: CubeFace::Front, // default placeholder
        }
    }
}

pub trait RayIntersect: Sync {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect;
}
