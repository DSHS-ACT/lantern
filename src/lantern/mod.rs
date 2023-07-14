use bytemuck::cast_slice;
use nalgebra::{SimdPartialOrd, Vector3, Vector4};
use wgpu::{Device, Queue};
use winit::dpi::PhysicalSize;

use crate::camera::Camera;
use crate::lantern::ray::Ray;
use crate::lantern::scene::{Scene, Sphere};
use crate::lantern::texture::Image;
use crate::vec4_to_rgba;

mod texture;
mod ray;
pub mod scene;

pub struct Lantern {
    pub final_image: Image,
    pub final_image_data: Vec<u32>,
}

impl Lantern {
    pub fn new(device: &Device, viewport_size: PhysicalSize<u32>) -> Self {
        let final_image = Image::new(device, viewport_size.width, viewport_size.height, "Lantern Output");
        let final_image_data = vec![0; (viewport_size.width * viewport_size.height) as usize];

        Self {
            final_image,
            final_image_data,
        }
    }

    pub fn resize(&mut self, device: &Device, new_size: PhysicalSize<u32>) {
        if self.final_image.size() == new_size {
            return;
        }

        self.final_image.resize(device, new_size);
        self.final_image_data = vec![0; (new_size.width * new_size.height) as usize];
    }

    pub fn update(&mut self, scene: &Scene, queue: &Queue, camera: &Camera) {
        let size = self.final_image.size();

        let origin = camera.position;
        let mut ray = Ray {
            origin,
            direction: Default::default(),
        };

        // 메모리 구조상 y 먼저가 더 효율적!
        for y in 0..size.height {
            for x in 0..size.width {
                let index = ((y * size.width) + x) as usize;
                ray.direction = camera.rays[index];

                self.final_image_data[index] = vec4_to_rgba(
                    &self.trace_ray(scene, &ray)
                        .simd_clamp(Vector4::zeros(), Vector4::new(1.0, 1.0, 1.0, 1.0))
                );
            }
        }

        self.final_image.load_image(queue, cast_slice(&self.final_image_data))
    }

    pub fn trace_ray(&mut self, scene: &Scene, ray: &Ray) -> Vector4<f32> {
        if scene.spheres.is_empty() {
            return Vector4::zeros();
        }

        let mut closest: Option<(&Sphere, f32)> = None;
        for sphere in &scene.spheres {
            // a = 빔 시작
            // b = 빔 방향
            // r = 구 반지름
            // t = 빔이 구와 만날 때, 그 빔 길이. 만나지 않으면 t는 정의되지 않음.
            // (bx^2 + by^2 + bz^2) * t^2 + 2 * (ax * bx + ay * by + az * bz) * t + (ax^2 + ay^2 + az^2 - r^2) = 0
            // 이 식은 구가 원점에 존재할 것을 가정하고 작성한 것. 구가 원점에 존재하지 않을 때는 그만큼 카메라 자체를 이동시켜 해결함.

            let origin = ray.origin - sphere.position;

            let first = ray.direction.magnitude_squared();
            let second = 2.0 * origin.coords.dot(&ray.direction);
            let third = origin.coords.magnitude_squared() - sphere.radius.powi(2);

            // 판별식
            let discriminant = second.powi(2) - 4.0 * first * third;

            if discriminant < 0.0 {
                continue;
            }

            let distance = (-second - discriminant.sqrt()) / (2.0 * first);
            if distance < 0.0 {
                continue;
            }

            if let Some((_, previous_distance)) = closest {
                if previous_distance > distance {
                    closest = Some((sphere, distance))
                }
            } else {
                closest = Some((sphere, distance))
            }
        }

        let Some((sphere, distance)) = closest else { return Vector4::new(0.0, 0.0, 0.0, 1.0); };

        let origin = ray.origin - sphere.position;

        let point = origin + (ray.direction * distance);
        let normalized = point / sphere.radius;
        let mut color = sphere.albedo;

        let light_direction = Vector3::new(-1.0, -1.0, 1.0).normalize();
        let flipped = -light_direction;

        let intensity = flipped.dot(&normalized.coords).max(0.0); // cos(v1, v2) = v1 * v2 IF both normal
        color *= intensity;

        Vector4::new(color.x, color.y, color.z, 1.0)
    }
}

