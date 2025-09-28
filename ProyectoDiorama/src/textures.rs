use raylib::color::Color;


/// Textura en memoria (RGBA8)
#[derive(Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA8 plano
}

impl Texture {
    /// Carga la textura desde un archivo de imagen usando la crate `image`
    /// (PNG, JPG, etc. soportados por `image`).
    pub fn load(path: &str) -> Self {
        // Abrimos y convertimos a RGBA8
        let img = image::open(path)
            .unwrap_or_else(|_| panic!("No pude abrir textura: {}", path))
            .to_rgba8();

        let (w, h) = img.dimensions();
        Self {
            width: w,
            height: h,
            data: img.into_raw(),
        }
    }

    /// Muestra el color en coordenadas UV normalizadas [0,1] con wrapping
    /// y nearest-neighbor sampling.
    pub fn sample(&self, uv: (f32, f32)) -> Color {
        let (mut u, mut v) = uv;

        // Wrap para que valores fuera de [0,1] se repitan
        u = u - u.floor();
        v = v - v.floor();

        // Y invertida (v=0 arriba)
        let x = (u * (self.width as f32 - 1.0))
            .round()
            .clamp(0.0, self.width as f32 - 1.0) as u32;
        let y = ((1.0 - v) * (self.height as f32 - 1.0))
            .round()
            .clamp(0.0, self.height as f32 - 1.0) as u32;

        let idx = ((y * self.width + x) * 4) as usize;

        // Convierte RGBA8 a tu tipo Color (ignora alpha si no lo usas)
        Color::new(
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        )
    }

    /// Devuelve una nueva textura rotada 180° (útil si tu sistema de coords
    /// está invertido).
    pub fn rotated_180(self) -> Self {
        use image::{imageops, RgbaImage};

        let img = RgbaImage::from_raw(self.width, self.height, self.data)
            .expect("Buffer de textura inválido");
        let rot = imageops::rotate180(&img);
        let (w, h) = rot.dimensions();

        Self {
            width: w,
            height: h,
            data: rot.into_raw(),
        }
    }
}
