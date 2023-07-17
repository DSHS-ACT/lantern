use std::ptr;

use bytemuck::cast_slice;
use nalgebra::{Point3, Reflection3, Unit, Vector3, Vector4};
use rayon::prelude::*;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalSize;

use crate::camera::Camera;
use crate::lantern::ray::Ray;
use crate::lantern::scene::{Scene, Sphere};
use crate::lantern::texture::Image;
use crate::util::random_vec;
use crate::{SharePtr, vec4_to_rgba};

mod texture;
mod ray;
pub mod scene;

pub struct Settings {
    pub should_accumulate: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            should_accumulate: true,
        }
    }
}

pub struct Lantern {
    pub final_image: Image,
    pub final_image_data: Vec<u32>,
    path_acc: Vec<Vector4<f32>>,
    acc_counter: u32,
    pub settings: Settings,
}

impl Lantern {
    pub fn new(device: &Device, viewport_size: PhysicalSize<u32>) -> Self {
        let final_image = Image::new(device, viewport_size.width, viewport_size.height, "Lantern Output");
        let final_image_data = vec![0; (viewport_size.width * viewport_size.height) as usize];
        let path_acc = vec![Vector4::zeros(); (viewport_size.width * viewport_size.height) as usize];

        Self {
            final_image,
            final_image_data,
            path_acc,
            acc_counter: 1,
            settings: Default::default(),
        }
    }

    pub fn resize(&mut self, device: &Device, new_size: PhysicalSize<u32>) {
        if self.final_image.size() == new_size {
            return;
        }

        self.final_image.resize(device, new_size);
        self.final_image_data = vec![0; (new_size.width * new_size.height) as usize];
        self.path_acc = vec![Vector4::zeros(); (new_size.width * new_size.height) as usize];
    }

    pub fn update(&mut self, scene: &Scene, camera: &Camera, queue: &Queue) {
        let size = self.final_image.size();
        if self.acc_counter == 1 {
            unsafe {
                let paths = self.path_acc.as_mut_ptr();
                ptr::write_bytes(paths, 0x00, self.path_acc.capacity());
            }
        }

        unsafe {
            let path_ptr = SharePtr(self.path_acc.as_mut_ptr());
            let image_ptr = SharePtr(self.final_image_data.as_mut_ptr());

            (0..size.height).into_par_iter().for_each(|y| {
                let _ = &path_ptr;
                let _ = &image_ptr;

                for x in 0..size.width {
                    let index = ((y * self.final_image.size().width) + x) as isize;

                    let color = self.per_pixel(scene, camera, x, y);

                    let path_ref = path_ptr.0.offset(index).as_mut().unwrap();
                    *path_ref += color;

                    let accumulated = *path_ref / (self.acc_counter as f32);

                    *image_ptr.0.offset(index).as_mut().unwrap() = vec4_to_rgba(
                        &(accumulated / accumulated.max()) // Alpha가 언제나 1이니까 괜찮지 않을까?
                    );
                }
            });
        }

        self.final_image.load_image(queue, cast_slice(&self.final_image_data));

        if self.settings.should_accumulate {
            self.acc_counter += 1;
        } else {
            self.acc_counter = 1;
        }
    }

    pub fn reset_counter(&mut self) {
        self.acc_counter = 1;
    }

    const BOUNCE_LIMIT: usize = 2;

    // DirectX의 RayGen 쉐이더와 같음
    pub fn per_pixel(&self, scene: &Scene, camera: &Camera, x: u32, y: u32) -> Vector4::<f32> {
        let index = ((y * self.final_image.size().width) + x) as usize;

        let origin = camera.position;
        let mut ray = Ray {
            origin,
            direction: camera.rays[index],
        };

        // BOUNCE_LIMIT, multipler 다 무작위 값
        let mut color = Vector3::zeros();
        let mut multiplier = 1.0;

        for _ in 0..Self::BOUNCE_LIMIT {
            let Some(HitPayload { position, normal, sphere, .. }) = self.trace_ray(&ray, scene) else {
                let sky = Vector3::new(0.6, 0.7, 0.9);
                color += sky * multiplier;
                break;
            };

            let material = &scene.materials[sphere.material_index];
            let mut sphere_color = material.albedo;

            let light_direction = Vector3::new(-1.0, -1.0, 1.0).normalize();

            let intensity = normal.dot(&-light_direction).max(0.0); // cos(v1, v2) = v1 * v2 IF both normal
            sphere_color *= intensity;

            // 아랫줄 주석 처리하면 밝기 효과를 제대로 볼 수 있음
            color += sphere_color * multiplier;
            multiplier *= 0.7;

            // position 자체가 구에 접하기 때문에 position을 다음 레이 트레이싱에 바로 사용하면 제대로 안할 것임.
            // 그래서 조금이라도 옮겨야 함
            ray.origin = position + (normal.as_ref() * 0.0001);
            let reflection_axis = Unit::new_unchecked({
                (normal.as_ref() + (material.roughness * random_vec(-0.5..0.5))).normalize()
            });
            Reflection3::new(reflection_axis, 0.0).reflect(ray.direction.as_mut_unchecked());
        }

        Vector4::new(color.x, color.y, color.z, 1.0)
    }

    pub fn closest_hit<'a>(&self, ray: &Ray, distance: f32, sphere: &'a Sphere) -> HitPayload<'a> {
        let fake_origin = ray.origin - sphere.position;
        let fake_position = fake_origin + (ray.direction.as_ref() * distance);

        let mut normal = Unit::new_unchecked(fake_position.coords / sphere.radius);
        normal.renormalize_fast();
        let position = fake_position + sphere.position;

        HitPayload {
            distance,
            position,
            normal,
            sphere,
        }
    }

    pub fn trace_ray<'a>(&'a self, ray: &Ray, scene: &'a Scene) -> Option<HitPayload<'a>> {
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

        closest.map(move |(sphere, distance)| {
            self.closest_hit(ray, distance, sphere)
        })
    }
}

// Cherno씨와 같은 디자인 선택, HitPayload는 빛의 경로에 대한 정보만 담고
// 이를 이용해 색상을 알아내는건 나중에 함
pub struct HitPayload<'a> {
    distance: f32,
    position: Point3<f32>,
    normal: Unit<Vector3<f32>>,
    sphere: &'a Sphere,
}

