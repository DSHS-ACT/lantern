use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use image::ImageFormat;
use wgpu::{
    Device, Extent3d, FilterMode, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, Sampler,
    SamplerDescriptor, Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

pub struct Image {
    pub gpu_texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub name: String,
}

impl Image {
    pub fn new(device: &Device, width: u32, height: u32, label: &str) -> Image {
        let gpu_texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1, // 이미지의 레이어 갯수. 단순한 2차원 이미지니 1개로
            },
            mip_level_count: 1, // 거리에 따라 다른 텍스쳐 쓰기. 우린 그런거 없음.
            sample_count: 1,    // 안티 에일리징을 위한 멀티 샘플링. 우린 그런거 안씀
            dimension: TextureDimension::D2, // 2차원 텍스쳐
            format: TextureFormat::Rgba8UnormSrgb, // 이미지 포맷. 일단 rgba srgb 사용

            // Texture Binding: 쉐이더에서 쓸 예정
            // Copy destination: CPU에서 GPU로 데이터가 복사될 예정
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = gpu_texture.create_view(&TextureViewDescriptor {
            label: Some(&format!("{} view", label)),
            ..Default::default() // label 뺴고 나머진 기본값 그대로
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some(&format!("{} sampler", label)),
            mag_filter: FilterMode::Linear, // 이미지를 확대해야 할 경우 선형으로 색상을 유추함
            min_filter: FilterMode::Nearest, // 이미지를 축소해야 할 경우 가장 가까운 픽셀의 값을 그대로 사용
            ..Default::default()
        });

        Self {
            gpu_texture,
            view,
            sampler,
            name: label.to_string(),
        }
    }

    pub fn load_image(&mut self, queue: &Queue, rgba: &[u8]) {
        let pixel_count = {
            let size = self.gpu_texture.size();
            size.width * size.height
        } as usize;
        assert_eq!(pixel_count, rgba.len() / 4, "");

        queue.write_texture(
            ImageCopyTexture {
                texture: &self.gpu_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.gpu_texture.width()),
                rows_per_image: Some(self.gpu_texture.height()),
            },
            self.gpu_texture.size(),
        )
    }

    pub fn from_path<P: AsRef<Path>>(
        path: P,
        device: &Device,
        queue: &Queue,
        label: Option<&str>,
    ) -> Option<Self> {
        let format = match path.as_ref().extension() {
            None => None,
            Some(extension) => {
                if extension.eq(OsStr::new("jpg")) | extension.eq(OsStr::new("jpeg")) {
                    Some(ImageFormat::Jpeg)
                } else if extension.eq(OsStr::new("png")) {
                    Some(ImageFormat::Png)
                } else {
                    None
                }
            }
        }?;

        let label = label.or(path.as_ref().file_name()?.to_str())?.to_owned();
        let reader = BufReader::new(File::open(path).ok()?);
        let loaded = image::load(reader, format).ok()?;

        let mut to_return = Self::new(device, loaded.width(), loaded.height(), &label);
        to_return.load_image(queue, &loaded.into_rgba8());
        Some(to_return)
    }

    pub fn resize(&mut self, device: &Device, new_size: PhysicalSize<u32>) {
        if self.gpu_texture.width() == new_size.width
            && self.gpu_texture.height() == new_size.height
        {
            return;
        }

        let new = Self::new(device, new_size.width, new_size.height, &self.name);
        self.sampler = new.sampler;
        self.view = new.view;
        self.gpu_texture = new.gpu_texture;
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.gpu_texture.width(), self.gpu_texture.height())
    }
}
