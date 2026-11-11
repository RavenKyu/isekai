pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}
impl Gpu {
    pub async fn new(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: surface,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = adapter.limits().max_texture_dimension_2d;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("isekai"),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
    pub fn instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env())
    }
    pub fn backend(&self) -> &'static str {
        match self.adapter.get_info().backend {
            wgpu::Backend::Vulkan => "vulkan",
            wgpu::Backend::Metal => "metal",
            wgpu::Backend::Dx12 => "dx12",
            wgpu::Backend::Gl => "gl",
            wgpu::Backend::BrowserWebGpu => "webgpu",
            _ => "unknown",
        }
    }
}
