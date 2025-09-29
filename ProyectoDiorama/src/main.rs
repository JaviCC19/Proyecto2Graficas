use raylib::prelude::*;
use std::f32::consts::PI;
use rayon::prelude::*;

mod framebuffers;
mod ray_intersect;
mod cube;
mod camera;
mod light;
mod material;
mod textures;
mod color_ops;
mod texture_manager;

use framebuffers::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use cube::Cube;
use camera::Camera;
use light::Light;
use material::{Material, vector3_to_color};

const ORIGIN_BIAS: f32 = 1e-4;

fn procedural_sky(dir: Vector3) -> Vector3 {
    let d = dir.normalized();
    let t = (d.y + 1.0) * 0.5;

    let green = Vector3::new(0.1, 0.6, 0.2);
    let white = Vector3::new(1.0, 1.0, 1.0);
    let blue = Vector3::new(0.3, 0.5, 1.0);

    if t < 0.54 {
        let k = t / 0.55;
        green * (1.0 - k) + white * k
    } else if t < 0.55 {
        white
    } else if t < 0.8 {
        let k = (t - 0.55) / 0.25;
        white * (1.0 - k) + blue * k
    } else {
        blue
    }
}

fn offset_origin(intersect: &Intersect, direction: &Vector3) -> Vector3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vector3, normal: &Vector3) -> Vector3 {
    *incident - *normal * 2.0 * incident.dot(*normal)
}

fn refract(incident: &Vector3, normal: &Vector3, refractive_index: f32) -> Option<Vector3> {
    let mut cosi = incident.dot(*normal).max(-1.0).min(1.0);

    let mut etai = 1.0;
    let mut etat = refractive_index;
    let mut n = *normal;

    if cosi > 0.0 {
        std::mem::swap(&mut etai, &mut etat);
        n = -n;
    } else {
        cosi = -cosi;
    }

    let eta = etai / etat;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);

    if k < 0.0 {
        None
    } else {
        Some(*incident * eta + n * (eta * cosi - k.sqrt()))
    }
}

fn cast_shadow(
    intersect: &Intersect,
    light: &Light,
    objects: &[&dyn RayIntersect],
) -> f32 {
    let light_dir = (light.position - intersect.point).normalized();
    let light_distance = (light.position - intersect.point).length();

    let shadow_ray_origin = offset_origin(intersect, &light_dir);

    for object in objects {
        let shadow_intersect = object.ray_intersect(&shadow_ray_origin, &light_dir);
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            return 1.0;
        }
    }

    0.0
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[&dyn RayIntersect],
    light: &Light,
    tm: &texture_manager::TextureManager,   // <-- ahora recibe TextureManager
    depth: u32,
) -> Vector3 {
    if depth > 3 {
        return procedural_sky(*ray_direction);
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = object.ray_intersect(ray_origin, ray_direction);
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return procedural_sky(*ray_direction);
    }

    let light_dir = (light.position - intersect.point).normalized();
    let view_dir = (*ray_origin - intersect.point).normalized();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalized();

    let shadow_intensity = cast_shadow(&intersect, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    let diffuse_intensity = intersect.normal.dot(light_dir).max(0.0) * light_intensity;

    // ---- USAR TEXTURA (si existe) en lugar del color diffuse fijo ----
    let tex_color = intersect
        .material
        .color_at(tm, intersect.u, intersect.v); // devuelve raylib::Color
    let tex_v3 = Vector3::new(
        tex_color.r as f32 / 255.0,
        tex_color.g as f32 / 255.0,
        tex_color.b as f32 / 255.0,
    );
    let diffuse = tex_v3 * diffuse_intensity;
    // ------------------------------------------------------------------

    let specular_intensity =
        view_dir.dot(reflect_dir).max(0.0).powf(intersect.material.specular) * light_intensity;
    let light_color_v3 = Vector3::new(
        light.color.r as f32 / 255.0,
        light.color.g as f32 / 255.0,
        light.color.b as f32 / 255.0,
    );
    let specular = light_color_v3 * specular_intensity;

    let albedo = intersect.material.albedo;
    let phong_color = diffuse * albedo[0] + specular * albedo[1];

    let reflectivity = intersect.material.albedo[2];
    let reflect_color = if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        // <-- pasar `tm` en la llamada recursiva
        cast_ray(&reflect_origin, &reflect_dir, objects, light, tm, depth + 1)
    } else {
        Vector3::zero()
    };

    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        if let Some(refract_dir) =
            refract(ray_direction, &intersect.normal, intersect.material.refractive_index)
        {
            let refract_origin = offset_origin(&intersect, &refract_dir);
            // <-- pasar `tm` en la llamada recursiva
            cast_ray(&refract_origin, &refract_dir, objects, light, tm, depth + 1)
        } else {
            let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            // <-- pasar `tm` en la llamada recursiva
            cast_ray(&reflect_origin, &reflect_dir, objects, light, tm, depth + 1)
        }
    } else {
        Vector3::zero()
    };

    phong_color * (1.0 - reflectivity - transparency)
        + reflect_color * reflectivity
        + refract_color * transparency
}

pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[&dyn RayIntersect],
    camera: &Camera,
    light: &Light,
    tm: &texture_manager::TextureManager,   // <-- recibe TextureManager
) {
    let width_f = framebuffer.width as f32;
    let height_f = framebuffer.height as f32;
    let aspect_ratio = width_f / height_f;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    let width = framebuffer.width as usize;
    let height = framebuffer.height as usize;
    let total = width * height;

    let pixels: Vec<(usize, Color)> = (0..total)
        .into_par_iter()
        .map(|idx| {
            let x = idx % width;
            let y = idx / width;

            let screen_x = (2.0 * x as f32) / width_f - 1.0;
            let screen_y = -(2.0 * y as f32) / height_f + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            let rotated_direction = camera.basis_change(&ray_direction);

            // <-- pasar `tm` al cast_ray
            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, tm, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            (idx, pixel_color)
        })
        .collect();

    for (idx, pixel_color) in pixels {
        let x = (idx % width) as u32;
        let y = (idx / width) as u32;
        framebuffer.set_current_color(pixel_color);
        framebuffer.set_pixel(x, y);
    }
}

fn main() {
    let window_width = 1300;
    let window_height = 900;

    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Pokeball Diorama - Capas")
        .build();
    raylib::set_trace_log(TraceLogLevel::LOG_WARNING);

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    // --- Texturas ---
    let mut texture_manager = texture_manager::TextureManager::default();
    let black_texture = textures::Texture::load("./assets/wool_colored_black.png");
    let white_texture = textures::Texture::load("./assets/wool_colored_white.png");
    let red_texture   = textures::Texture::load("./assets/wool_colored_red.png");
    let yellow_texture= textures::Texture::load("./assets/wool_colored_yellow.png");
    let blackstone_texture = textures::Texture::load("./assets/blackstone_top.png");
    let glowstone_texture = textures::Texture::load("./assets/glowstone.png");
    let quartz_texture = textures::Texture::load("./assets/quartz_block_top.png");
    let redstone_texture = textures::Texture::load("./assets/redstone_block.png");

    texture_manager.add_texture('n', black_texture);
    texture_manager.add_texture('w', white_texture);
    texture_manager.add_texture('r', red_texture);
    texture_manager.add_texture('y', yellow_texture);
    texture_manager.add_texture('B', blackstone_texture);
    texture_manager.add_texture('G', glowstone_texture);
    texture_manager.add_texture('Q', quartz_texture);
    texture_manager.add_texture('S', redstone_texture);


    // --- Materiales ---
    let mat_black = Material::with_texture(Vector3::new(0.5, 0.5, 0.5), 10.0, [0.9, 0.1, 0.0, 0.0], 0.0, 'n');
    let mat_white = Material::with_texture(Vector3::new(0.5, 0.5, 0.5), 10.0, [0.9, 0.1, 0.0, 0.0], 0.0, 'w');
    let mat_red   = Material::with_texture(Vector3::new(0.5, 0.5, 0.5), 10.0, [0.9, 0.1, 0.0, 0.0], 0.0, 'r');
    let mat_yellow= Material::with_texture(Vector3::new(0.5, 0.5, 0.5), 10.0, [0.9, 0.1, 0.0, 0.0], 0.0, 'y');
    // --- Materiales nuevos ---
    let mat_blackstone = Material::with_texture(
        Vector3::new(0.0, 0.0, 0.0), // negro completo
        25.0,                        // rugosidad
        [0.0, 0.0, 0.0, 0.0],        // sin especular, sin emisión
        0.0,                         // reflectividad
        'B'                           // símbolo
    );

    let mat_glowstone = Material::with_texture(
    Vector3::new(1.0, 0.85, 0.4), // tono dorado-amarillo
    5.0,                          // un poco de rugosidad (no espejo)
    [0.8, 0.1, 0.1, 1.5],         // fuerte difusión, poca reflexión, algo especular, emisión fuerte
    0.0,                          // no refracta
    'G'                           // símbolo
);


    let mat_quartz = Material::with_texture(
        Vector3::new(1.0, 1.0, 1.0), // blanco puro
        50.0,                         
        [0.9, 0.9, 0.9, 0.0],        // especular alta para reflejar
        0.3,                          // completamente reflectivo
        'Q'                           // símbolo
    );

    let mat_redstone = Material::with_texture(
        Vector3::new(0.8, 0.0, 0.0), // rojo oscuro
        25.0,                         
        [0.5, 0.0, 0.0, 0.0],        
        0.2,                    
        'S'                             
    );


    fn get_material(
    c: char,
    mat_white: &Material,
    mat_black: &Material,
    mat_red: &Material,
    mat_yellow: &Material,
    mat_blackstone: &Material,
    mat_glowstone: &Material,
    mat_quartz: &Material,
    mat_redstone: &Material
) -> Option<Material> {
    match c {
        'W' | 'w' => Some(mat_white.clone()),
        'N' | 'n' => Some(mat_black.clone()),
        'R' | 'r' => Some(mat_red.clone()),
        'Y' | 'y' => Some(mat_yellow.clone()),
        'B' | 'b' => Some(mat_blackstone.clone()),
        'G' | 'g' => Some(mat_glowstone.clone()),
        'Q' | 'q' => Some(mat_quartz.clone()),
        'S' | 's' => Some(mat_redstone.clone()),
        _ => None,
    }
}

    // --- Definición de capas (ejemplo reducido con tus matrices 1–13) ---
    // Cada capa es un Vec<&str> de 10 columnas
    let layers: Vec<Vec<&str>> = vec![
        // Layer 1
        vec![
            "0000000000",
            "0000000000",
            "0000000000",
            "0000WW0000",
            "0000WW0000",
            "0000000000",
            "0000000000",
            "0000000000",
            "0000000000",
            "0000000000",
        ],
        // Layer 2
        vec![
            "0000000000",
            "0000000000",
            "0000WWWW00",
            "000W00W000",
            "000W00W000",
            "0000WWWW00",
            "0000000000",
            "0000000000",
            "0000000000",
            "0000000000",
        ],
        // Layer 3  (interpreted from tu matrix con w/o)
        vec![
            "0000000000",
            "0000000000",
            "00WWWWWW00",
            "00WOOOOOW0",
            "00WOOOOOW0",
            "00WOOOOOW0",
            "00WOOOOOW0",
            "00WWWWWW00",
            "0000000000",
            "0000000000",
        ],
        // Layer 4 (borde W)
        vec![
            "0000000000",
            "0WWWWWWWW0",
            "0W0000000W",
            "0W0000000W",
            "0W0000000W",
            "0W0000000W",
            "0W0000000W",
            "0W0000000W",
            "0W0000000W",
            "0000000000",
        ],
        // Layer 5 (negra alrededor, N en interior)
        vec![
            "WWWWWWWWWW",
            "WGGGGGGGGW",
            "WGGGGGGGGW",
            "0GGGGGGGGGW",
            "0GGGGGGGGW",
            "0GGGGGGGGW",
            "0GGGGGGGGW",
            "WGGGGGGGGW",
            "WGGGGGGGGW",
            "WWWWWWWWWW",
        ],
        // Pikachu - Layer 6 (piernas delanteras)
    vec![
        "0000000000",
        "0000000000",
        "000000Y0Y0",
        "0000000000",
        "000000Y0Y0",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Pikachu - Layer 7 (piernas traseras)
    vec![
        "0000000000",
        "0000000000",
        "000000Y0Y0",
        "0000000000",
        "000000Y0Y0",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Pikachu - Layer 8 (cuerpo central con base cola)
    vec![
        "0000000000",
        "0000000000",
        "000SYYYYY0",
        "000NYYYYY0",
        "000SYYYYY0",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Pikachu - Layer 9 (cabeza con mejillas rojas)
    vec![
        "0000000000",
        "0000000000",
        "000YYYYYY0",
        "000YYYYYY",
        "000YYYYYY0",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Pikachu - Layer 10 (cabeza + fin del cuerpo + cola negra)
    vec![
        "0000000000",
        "0000000000",
        "000NYY0000",
        "000YYY000Y",
        "000NYY0000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Pikachu - Layer 11 (ojos negros)
    vec![
        "0000000000",
        "0000000000",
        "000YYY0000",
        "000YYY000N",
        "000YYY0000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Pikachu - Layer 12 (antenas en la cabeza)
    vec![
        "0000000000",
        "0000000000",
        "00000Y0000",
        "0000000000",
        "00000Y0000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],
    // Pikachu - Layer 13 (antenas en la cabeza)
    vec![
        "0000000000",
        "0000000000",
        "00000N0000",
        "0000000000",
        "00000N0000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // --- Pokeball superior ---
    // Layer 2 (vacía, para elevar)
    vec![
        "0000000000",
        "0000000000",
        "B000000000",
        "B000000000",
        "B000000000",
        "B000000000",
        "B000000000",
        "B000000000",
        "0000000000",
        "0000000000",
    ],

    vec![
        "0000000000",
        "0000000000",
        "B000000000",
        "Q000000000",
        "Q000000000",
        "Q000000000",
        "Q000000000",
        "B000000000",
        "0000000000",
        "0000000000",
    ],

    // Layer 3 (capa exterior superior)
    vec![
        "BBBBBBBBBB",
        "B00000000B",
        "B00000000B",
        "Q00000000B",
        "Q00000000B",
        "Q00000000B",
        "Q00000000B",
        "B00000000B",
        "B00000000B",
        "BBBBBBBBBB",
    ],

    // Layer 4
    vec![
        "0000000000",
        "0RRRRRRRR0",
        "B00000000R",
        "Q00000000R",
        "Q00000000R",
        "Q00000000R",
        "Q00000000R",
        "B00000000R",
        "0RRRRRRRR0",
        "0000000000",
    ],

    // Layer 5
    vec![
        "0000000000",
        "0000000000",
        "B0RRRRRR00",
        "BR000000R0",
        "BR000000R0",
        "BR000000R0",
        "BR000000R0",
        "B0RRRRRR00",
        "0000000000",
        "0000000000",
    ],

    // Layer 6
    vec![
        "0000000000",
        "0000000000",
        "0000000000",
        "0000RRRR000",
        "000RR00R000",
        "000RR00R000",
        "0000RRRR000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],

    // Layer 7
    vec![
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000RR0000",
        "0000RR0000",
        "0000000000",
        "0000000000",
        "0000000000",
        "0000000000",
    ],



    ];

    // --- Construcción de cubos ---
    let mut cubes: Vec<Cube> = Vec::new();

    for (layer_index, layer) in layers.iter().enumerate() {
        let y = layer_index as f32; // altura según índice
        for (z, row) in layer.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if let Some(mat) = get_material(c, &mat_white, &mat_black, &mat_red, &mat_yellow, &mat_blackstone, &mat_glowstone, &mat_quartz, &mat_redstone) {
                    cubes.push(Cube {
                        center: Vector3::new(x as f32, y, z as f32),
                        size: 1.0,
                        material: mat,
                    });
                }
            }
        }
    }

    let objects: Vec<&dyn RayIntersect> = cubes.iter().map(|c| c as &dyn RayIntersect).collect();

    // --- Cámara ---
    let mut camera = Camera::new(
        Vector3::new(0.0, 15.0, 30.0),
        Vector3::new(5.0, 5.0, 5.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let rotation_speed = PI / 100.0;

    // --- Luz ---

        let light2 = Light::new(
        Vector3::new(-20.0, 20.0, 15.0), // un poco más arriba y adelante
        Color::new(255, 255, 255, 255),
        3.0, // más intensidad
    );


    while !window.window_should_close() {
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            camera.orbit(rotation_speed, 0.0);
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            camera.orbit(-rotation_speed, 0.0);
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            camera.orbit(0.0, -rotation_speed);
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            camera.orbit(0.0, rotation_speed);
        }

        if window.is_key_down(KeyboardKey::KEY_EQUAL) {
            camera.zoom(0.95);
        }
        if window.is_key_down(KeyboardKey::KEY_MINUS) {
            camera.zoom(1.05);
        }

        framebuffer.clear();
        render(&mut framebuffer, &objects, &camera, &light2, &texture_manager);
        framebuffer.swap_buffers(&mut window, &thread);
    }
}
