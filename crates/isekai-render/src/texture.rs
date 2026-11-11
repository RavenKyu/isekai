use crate::Gpu;
use isekai_core::raster::RgbaImage;
pub struct SheetTexture {
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
}
impl SheetTexture {
    pub fn upload(
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        image: &RgbaImage,
    ) -> Result<Self, String> {
        let (w, h) = (image.width, image.height);
        let limit = gpu.device.limits().max_texture_dimension_2d;
        if w > limit || h > limit {
            return Err(format!(
                "sheet {w}x{h} exceeds max texture dimension {limit}"
            ));
        }
        if w == 0 || h == 0 || u64::from(w) * u64::from(h) * 4 != image.pixels.len() as u64 {
            return Err("invalid RGBA image dimensions or pixel length".into());
        }
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("isekai sheet"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::FRAME_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            texture.as_image_copy(),
            &image.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            texture.size(),
        );
        let view = texture.create_view(&Default::default());
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("isekai nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("isekai sheet"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Ok(Self {
            texture,
            bind_group,
        })
    }
}
