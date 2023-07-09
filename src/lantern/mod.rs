use bytemuck::cast_slice;
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

        // 메모리 구조상 y 먼저가 더 효율적!
        for y in 0..size.height {
            for x in 0..size.width {
                let index = (y * size.width) + x;

                self.final_image_data[index as usize] = self.process_pixel(((x as f32) / (size.height as f32), (y as f32) / (size.width as f32)))
            }
        }

        self.final_image.load_image(queue, cast_slice(&self.final_image_data))
    }

    pub fn process_pixel(&mut self, coord: (f32, f32)) -> u32 {
        let red = (255.0 * coord.0) as u8;
        let green = (255.0 * coord.1) as u8;

        // 왜 반대야 :(((
        0xFF000000u32 | ((green as u32) << 8) | (red as u32)
    }
}
