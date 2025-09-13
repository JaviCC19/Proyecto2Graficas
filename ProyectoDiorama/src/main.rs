use raylib::prelude::*;
use std::f32::consts::PI;

mod framebuffers;
mod ray_intersect;
mod cube;
mod camera;
mod light;
mod material;

use framebuffers::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use cube::Cube;
use camera::Camera;
use light::Light;
use material::{Material, vector3_to_color};

const ORIGIN_BIAS: f32 = 1e-4;

fn procedural_sky(dir: Vector3) -> Vector3 {
    let d = dir.normalized();
    let t = (d.y + 1.0) * 0.5; // map y [-1,1] → [0,1]

    let green = Vector3::new(0.1, 0.6, 0.2); // grass green
    let white = Vector3::new(1.0, 1.0, 1.0); // horizon haze
    let blue = Vector3::new(0.3, 0.5, 1.0);  // sky blue

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
            return 1.0; // full shadow
        }
    }

    0.0
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[&dyn RayIntersect],
    light: &Light,
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
    let diffuse = intersect.material.diffuse * diffuse_intensity;

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

    // Reflections
    let reflectivity = intersect.material.albedo[2];
    let reflect_color = if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        cast_ray(&reflect_origin, &reflect_dir, objects, light, depth + 1)
    } else {
        Vector3::zero()
    };

    // Refractions
    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        if let Some(refract_dir) =
            refract(ray_direction, &intersect.normal, intersect.material.refractive_index)
        {
            let refract_origin = offset_origin(&intersect, &refract_dir);
            cast_ray(&refract_origin, &refract_dir, objects, light, depth + 1)
        } else {
            let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            cast_ray(&reflect_origin, &reflect_dir, objects, light, depth + 1)
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
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            let rotated_direction = camera.basis_change(&ray_direction);

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            framebuffer.set_current_color(pixel_color);
            framebuffer.set_pixel(x, y);
        }
    }
}

fn main() {
    let window_width = 1300;
    let window_height = 900;

    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raytracer Example with Cubes")
        .build();
    raylib::set_trace_log(TraceLogLevel::LOG_WARNING);

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    // Materials
    let rubber = Material::new(
        Vector3::new(0.3, 0.1, 0.1),
        10.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );

    let ivory = Material::new(
        Vector3::new(0.4, 0.4, 0.3),
        50.0,
        [0.6, 0.3, 0.1, 0.0],
        0.0,
    );

    let glass = Material::new(
        Vector3::new(0.6, 0.7, 0.8),
        125.0,
        [0.0, 0.5, 0.1, 0.8],
        1.5,
    );

    // Cubes instead of spheres
    let cube1 = Cube {
        center: Vector3::new(0.0, 0.0, 0.0),
        size: 1.0,
        material: rubber,
    };

    let cube2 = Cube {
        center: Vector3::new(-1.5, -1.0, 2.0),
        size: 0.8,
        material: ivory,
    };

    let cube3 = Cube {
        center: Vector3::new(1.0, 0.5, 2.5),
        size: 0.5,
        material: glass,
    };

    // Collect them as trait objects
    let objects: Vec<&dyn RayIntersect> = vec![&cube1, &cube2, &cube3];

    let mut camera = Camera::new(
        Vector3::new(0.0, 0.0, 5.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    let rotation_speed = PI / 100.0;

    let light = Light::new(
        Vector3::new(1.0, -1.0, 5.0),
        Color::new(255, 255, 255, 255),
        1.5,
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

        framebuffer.clear();
        render(&mut framebuffer, &objects, &camera, &light);
        framebuffer.swap_buffers(&mut window, &thread);
    }
}
