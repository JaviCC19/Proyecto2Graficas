use raylib::prelude::Color;
use crate::texture_manager::TextureManager;
use raylib::prelude::Vector3;

#[derive(Debug, Clone)]
pub struct Material {
    pub diffuse: Vector3,
    pub albedo: [f32; 4],
    pub specular: f32,
    pub refractive_index: f32,
    pub texture_key: Option<char>,
}

impl Material {
    pub fn with_texture(
        diffuse: Vector3,
        specular: f32,
        albedo: [f32; 4],
        refractive_index: f32,
        key: char,
    ) -> Self {
        Self {
            diffuse,
            albedo,
            specular,
            refractive_index,
            texture_key: Some(key),
        }
    }

    /// Obtiene el color en coordenadas UV [0,1] usando el TextureManager si hay textura
    pub fn color_at(&self, tm: &TextureManager, u: f32, v: f32) -> Color {
        if let Some(k) = self.texture_key {
            if let Some(tex) = tm.images.get(&k) {
                // Convertimos UV normalizado a coordenadas de píxel
                let tx = (u * (tex.width as f32 - 1.0)).clamp(0.0, tex.width as f32 - 1.0) as u32;
                let ty = ((1.0 - v) * (tex.height as f32 - 1.0))
                    .clamp(0.0, tex.height as f32 - 1.0) as u32;
                return tm.get_pixel_color(k, tx, ty);
            }
        }
        // Fallback: color sólido
        vector3_to_color(self.diffuse)
    }
}

/// Convierte un Vector3 (0..1) a Color RGBA
pub fn vector3_to_color(v: Vector3) -> Color {
    Color::new(
        (v.x * 255.0).clamp(0.0, 255.0) as u8,
        (v.y * 255.0).clamp(0.0, 255.0) as u8,
        (v.z * 255.0).clamp(0.0, 255.0) as u8,
        255,
    )
}
