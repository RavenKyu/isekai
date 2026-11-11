use isekai_core::{raster::RgbaImage, sprite::Transform};
use isekai_render::*;
fn read(gpu: &Gpu, target: &FrameTarget) -> Vec<u8> {
    let read = Readback::begin(gpu, target);
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    read.try_finish().expect("map callback").unwrap()
}
fn draw(
    gpu: &Gpu,
    pipeline: &SpritePipeline,
    batch: &mut SpriteBatch,
    target: &FrameTarget,
    textures: &[&SheetTexture],
) {
    batch.upload(gpu);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pixel oracle"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        batch.draw(&mut pass, pipeline, textures);
    }
    gpu.queue.submit([encoder.finish()]);
}
#[test]
#[ignore = "requires a GPU adapter; run scripts/smoke-native-isekai.sh"]
fn pixel_contracts() {
    let gpu = pollster::block_on(Gpu::new(Gpu::instance(), None)).unwrap();
    assert_eq!(gpu.backend(), "vulkan");
    let pipeline = SpritePipeline::new(&gpu);
    let mut image = RgbaImage::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            image.set_pixel(
                x,
                y,
                [
                    (1 + y * 4 + x) as u8,
                    40 + 60 * x as u8,
                    40 + 60 * y as u8,
                    255,
                ],
            );
        }
    }
    let variants: std::collections::HashSet<_> = Transform::ALL
        .into_iter()
        .map(|t| image.transformed(t).pixels)
        .collect();
    assert_eq!(variants.len(), 8, "fixture must distinguish all transforms");
    let sheet = SheetTexture::upload(&gpu, &pipeline.layout, &image).unwrap();
    let target = FrameTarget::new(&gpu, 4, 4);
    let mut batch = SpriteBatch::new(&gpu);
    for t in Transform::ALL {
        batch.clear();
        batch.push(
            0,
            SpriteInstance {
                dest: [-1., 1., 2., -2.],
                uv: t.uv_corners([0., 0.], [1., 1.]),
            },
        );
        draw(&gpu, &pipeline, &mut batch, &target, &[&sheet]);
        let pixels = read(&gpu, &target);
        assert_eq!(pixels, image.transformed(t).pixels, "{t:?}");
        let expected = match (t.flip_x(), t.rotation()) {
            (false, 90) => Some([13, 9, 5, 1, 14, 10, 6, 2, 15, 11, 7, 3, 16, 12, 8, 4]),
            (true, 90) => Some([16, 12, 8, 4, 15, 11, 7, 3, 14, 10, 6, 2, 13, 9, 5, 1]),
            _ => None,
        };
        if let Some(expected) = expected {
            assert_eq!(
                pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| p[0])
                    .collect::<Vec<_>>(),
                expected
            );
        }
        assert_eq!(batch.draw_calls, 1);
    }
    let copied = FrameTarget::new(&gpu, 4, 4);
    let blit = Blit::new(&gpu, FRAME_FORMAT);
    let bind = blit.bind(&gpu, &target);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    blit.encode(&mut encoder, &bind, &copied.view);
    gpu.queue.submit([encoder.finish()]);
    assert_eq!(
        read(&gpu, &copied),
        read(&gpu, &target),
        "presentation preserves bytes"
    );
    // A surface offering only sRGB formats gets fs_srgb: decoding before the
    // hardware re-encodes keeps the stored bytes within rounding.
    let srgb = Blit::new(&gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sRGB surface stand-in"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let encoded = FrameTarget {
        texture,
        view,
        width: 4,
        height: 4,
    };
    let bind = srgb.bind(&gpu, &target);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    srgb.encode(&mut encoder, &bind, &encoded.view);
    gpu.queue.submit([encoder.finish()]);
    let (expected, actual) = (read(&gpu, &target), read(&gpu, &encoded));
    assert!(
        expected
            .iter()
            .zip(&actual)
            .all(|(e, a)| e.abs_diff(*a) <= 1),
        "sRGB presentation preserves bytes: {expected:?} vs {actual:?}"
    );
    let image = RgbaImage {
        width: 2,
        height: 1,
        pixels: vec![30, 70, 110, 255, 190, 130, 90, 255],
    };
    let sheet = SheetTexture::upload(&gpu, &pipeline.layout, &image).unwrap();
    let target = FrameTarget::new(&gpu, 4, 1);
    batch.clear();
    batch.push(
        0,
        SpriteInstance {
            dest: [-1., 1., 2., -2.],
            uv: Transform::IDENTITY.uv_corners([0., 0.], [1., 1.]),
        },
    );
    draw(&gpu, &pipeline, &mut batch, &target, &[&sheet]);
    assert_eq!(
        read(&gpu, &target),
        [
            &image.pixels[..4],
            &image.pixels[..4],
            &image.pixels[4..],
            &image.pixels[4..]
        ]
        .concat()
    );
    let target = FrameTarget::new(&gpu, 5, 3);
    batch.clear();
    draw(&gpu, &pipeline, &mut batch, &target, &[]);
    assert_eq!(read(&gpu, &target), [0, 0, 0, 255].repeat(15));
    assert!(SheetTexture::upload(&gpu, &pipeline.layout, &RgbaImage::new(0, 1)).is_err());
    assert!(
        SheetTexture::upload(
            &gpu,
            &pipeline.layout,
            &RgbaImage {
                width: 1,
                height: 1,
                pixels: vec![0]
            }
        )
        .is_err()
    );
    let limit = gpu.device.limits().max_texture_dimension_2d;
    let error = SheetTexture::upload(
        &gpu,
        &pipeline.layout,
        &RgbaImage {
            width: limit + 1,
            height: 1,
            pixels: vec![],
        },
    )
    .err()
    .unwrap();
    assert_eq!(
        error,
        format!(
            "sheet {}x1 exceeds max texture dimension {limit}",
            limit + 1
        )
    );
    for (ids, calls) in [(vec![0, 0, 0], 1), (vec![0, 1, 0], 3)] {
        batch.clear();
        for id in ids {
            batch.push(
                id,
                SpriteInstance {
                    dest: [-1., 1., 2., -2.],
                    uv: Transform::IDENTITY.uv_corners([0., 0.], [1., 1.]),
                },
            );
        }
        draw(&gpu, &pipeline, &mut batch, &target, &[&sheet, &sheet]);
        assert_eq!(batch.draw_calls, calls);
    }
}

#[test]
#[ignore = "requires a GPU adapter; run scripts/smoke-native-isekai.sh"]
fn missing_adapter_is_an_error() {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::empty();
    assert!(pollster::block_on(Gpu::new(wgpu::Instance::new(descriptor), None)).is_err());
}
