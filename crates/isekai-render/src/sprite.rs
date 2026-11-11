use crate::{Gpu, SheetTexture};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub dest: [f32; 4],
    pub uv: [[f32; 2]; 4],
}
pub struct SpritePipeline {
    pub layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}
impl SpritePipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("isekai sheet"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("isekai sprite"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("isekai sprite"),
                source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
            });
        let blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline=gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:Some("isekai sprite"),layout:Some(&pipeline_layout),
            vertex:wgpu::VertexState { module:&shader,entry_point:Some("vs"),compilation_options:Default::default(), buffers:&[Some(wgpu::VertexBufferLayout { array_stride:std::mem::size_of::<SpriteInstance>() as u64, step_mode:wgpu::VertexStepMode::Instance,attributes:&wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x2,2=>Float32x2,3=>Float32x2,4=>Float32x2] })] },
            fragment:Some(wgpu::FragmentState { module:&shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState { format:crate::FRAME_FORMAT,blend:Some(wgpu::BlendState { color:blend,alpha:blend }),write_mask:wgpu::ColorWrites::ALL })] }),
            primitive:wgpu::PrimitiveState { topology:wgpu::PrimitiveTopology::TriangleStrip,..Default::default() },
            depth_stencil:None,multisample:Default::default(),multiview_mask:None,cache:None,
        });
        Self { layout, pipeline }
    }
}
pub struct SpriteBatch {
    instances: Vec<SpriteInstance>,
    runs: Vec<(usize, usize)>,
    buffer: wgpu::Buffer,
    capacity: usize,
    pub draw_calls: usize,
}
impl SpriteBatch {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            instances: Vec::new(),
            runs: Vec::new(),
            buffer: Self::buffer(gpu, 1),
            capacity: 1,
            draw_calls: 0,
        }
    }
    fn buffer(gpu: &Gpu, capacity: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("isekai instances"),
            size: (capacity * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
    pub fn clear(&mut self) {
        self.instances.clear();
        self.runs.clear();
        self.draw_calls = 0;
    }
    pub fn push(&mut self, texture: usize, instance: SpriteInstance) {
        if self.runs.last().is_none_or(|&(id, _)| id != texture) {
            self.runs.push((texture, self.instances.len()));
        }
        self.instances.push(instance);
    }
    pub fn upload(&mut self, gpu: &Gpu) {
        if self.instances.len() > self.capacity {
            self.capacity = self.instances.len().next_power_of_two();
            self.buffer = Self::buffer(gpu, self.capacity);
        }
        if !self.instances.is_empty() {
            gpu.queue
                .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.instances));
        }
    }
    pub fn draw(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        pipeline: &SpritePipeline,
        textures: &[&SheetTexture],
    ) {
        if self.instances.is_empty() {
            return;
        }
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        for (i, &(texture, start)) in self.runs.iter().enumerate() {
            let end = self
                .runs
                .get(i + 1)
                .map_or(self.instances.len(), |&(_, start)| start);
            pass.set_bind_group(0, &textures[texture].bind_group, &[]);
            pass.draw(0..4, start as u32..end as u32);
        }
        self.draw_calls = self.runs.len();
    }
}
