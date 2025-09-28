use std::collections::HashMap;
use crate::textures::Texture;
use raylib::prelude::Color;

/// Gestor de texturas
#[derive(Default)]
pub struct TextureManager {
    /// Map de texturas accesibles por clave
    pub images: HashMap<char, Texture>,
}

impl TextureManager {
    /// Añade una textura al manager con una clave
    pub fn add_texture(&mut self, key: char, tex: Texture) {
        self.images.insert(key, tex);
    }

    /// Obtiene el color de la textura en coordenadas de píxel
    pub fn get_pixel_color(&self, key: char, x: u32, y: u32) -> Color {
        if let Some(tex) = self.images.get(&key) {
            // Clamp para evitar overflow
            let x = x.min(tex.width - 1);
            let y = y.min(tex.height - 1);
            let idx = ((y * tex.width + x) * 4) as usize;
            let data = &tex.data;
            Color::new(data[idx], data[idx + 1], data[idx + 2], data[idx + 3])
        } else {
            // Magenta para debug si no existe la textura
            Color::new(255, 0, 255, 255)
        }
    }
}
