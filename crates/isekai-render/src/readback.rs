use crate::{FrameTarget, Gpu};
use std::sync::{Arc, Mutex};
pub fn lit_count(bytes: &[u8]) -> usize {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[..3].iter().any(|&v| v != 0))
        .count()
}
type MapResult = Arc<Mutex<Option<Result<(), String>>>>;
pub struct Readback {
    buffer: wgpu::Buffer,
    result: MapResult,
    width: u32,
    height: u32,
    stride: u32,
}
impl Readback {
    pub fn begin(gpu: &Gpu, frame: &FrameTarget) -> Self {
        let stride = (frame.width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("isekai readback"),
            size: u64::from(stride) * u64::from(frame.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            frame.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        let result = Arc::new(Mutex::new(None));
        let callback = result.clone();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            *callback.lock().unwrap() = Some(r.map_err(|e| e.to_string()));
        });
        Self {
            buffer,
            result,
            width: frame.width,
            height: frame.height,
            stride,
        }
    }
    /// The size of the frame this readback copied; the target may have been resized since.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn try_finish(&self) -> Option<Result<Vec<u8>, String>> {
        let status = self.result.lock().unwrap().take()?;
        Some(status.and_then(|()| {
            let mapped = self
                .buffer
                .slice(..)
                .get_mapped_range()
                .map_err(|e| e.to_string())?;
            let mut pixels = Vec::with_capacity(self.width as usize * self.height as usize * 4);
            for row in mapped.chunks_exact(self.stride as usize) {
                pixels.extend_from_slice(&row[..self.width as usize * 4]);
            }
            drop(mapped);
            self.buffer.unmap();
            Ok(pixels)
        }))
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn alpha_is_not_light() {
        assert_eq!(
            super::lit_count(&[0, 0, 0, 255, 1, 0, 0, 0, 0, 2, 0, 255, 0, 0, 3, 255]),
            3
        );
    }
}
