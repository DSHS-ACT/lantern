use bytemuck::cast_slice;
use nalgebra::{Vector2, Vector3};
use rand::RngCore;
use wgpu::{Device, Queue};
use winit::dpi::PhysicalSize;

use crate::lantern::texture::Image;

mod texture;

pub struct Lantern {
    pub final_image: Image,
    pub final_image_data: Vec<u32>
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

    pub fn update(&mut self, queue: &Queue) {
        let size = self.final_image.size();
        let ratio = {
            let size = self.final_image.size();
            size.width as f32 / size.height as f32
        };

        // 메모리 구조상 y 먼저가 더 효율적!
        for y in 0..size.height {
            for x in 0..size.width {
                let index = (y * size.width) + x;
                let mut coord = Vector2::new((x as f32) / (size.width as f32), (y as f32) / (size.height as f32));
                coord *= 2.0;
                coord -= Vector2::new(1.0, 1.0);
                coord.x *= ratio;

                self.final_image_data[index as usize] = self.process_pixel(coord);
            }
        }

        self.final_image.load_image(queue, cast_slice(&self.final_image_data))
    }

    pub fn process_pixel(&mut self, coord: Vector2<f32>) -> u32 {
        let origin = Vector3::new(0.0f32, 0.0, -2.0);
        let ray_direction = Vector3::new(coord.x, coord.y, 1.0).normalize();
        let radius = 0.5f32;

        // a = 빔 시작
        // b = 빔 방향
        // r = 구 반지름
        // t = 빔이 구와 만날 때, 그 빔 길이. 만나지 않으면 t는 정의되지 않음.
        // (bx^2 + by^2 + bz^2) * t^2 + 2 * (ax * bx + ay * by + az * bz) * t + (ax^2 + ay^2 + az^2 - r^2) = 0

        let first = ray_direction.magnitude_squared();
        let second = 2.0 * origin.dot(&ray_direction);
        let third = origin.magnitude_squared() - radius.powi(2);

        // 판별식
        let discriminant = second.powi(2) - 4.0 * first * third;

        let distance = if discriminant >= 0.0 {
            let t = (-second - discriminant.sqrt()) / (2.0 * first);
            t
        } else {
            f32::INFINITY
        };



        let alpha = ((2.0 / distance) * 255.0) as u32;

        alpha << 24 | 0x00FFFF00
    }
}
