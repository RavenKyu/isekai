use glow::HasContext;
use std::rc::Rc;

#[cfg(not(target_os = "emscripten"))]
const HEADER: &str = "#version 330 core\n";
#[cfg(target_os = "emscripten")]
const HEADER: &str = "#version 300 es\nprecision mediump float;\n";
const VERT_BODY: &str = r#"
layout(location = 0) in vec2 a_pos;
uniform vec4 u_dest;
uniform vec2 u_uv[4];
out vec2 v_uv;
void main() {
    gl_Position = vec4(u_dest.xy + a_pos * u_dest.zw, 0.0, 1.0);
    v_uv = u_uv[gl_VertexID];
}
"#;
const FRAG_BODY: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 color;
void main() { color = texture(u_tex, v_uv); }
"#;

pub struct SpriteQuad {
    gl: Rc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    u_dest: Option<glow::UniformLocation>,
    u_uv: Option<glow::UniformLocation>,
    u_tex: Option<glow::UniformLocation>,
}

impl SpriteQuad {
    pub fn new(gl: &Rc<glow::Context>) -> Result<Self, String> {
        // SAFETY: App creates all playback objects while its live context is current.
        unsafe {
            let program = gl.create_program()?;
            let link = (|| {
                for (kind, body) in [
                    (glow::VERTEX_SHADER, VERT_BODY),
                    (glow::FRAGMENT_SHADER, FRAG_BODY),
                ] {
                    let source = format!("{HEADER}{body}");
                    if !source.starts_with("#version") {
                        return Err(
                            "sprite shader must start with #version on its first line".into()
                        );
                    }
                    let shader = gl.create_shader(kind)?;
                    gl.shader_source(shader, &source);
                    gl.compile_shader(shader);
                    if !gl.get_shader_compile_status(shader) {
                        let log = gl.get_shader_info_log(shader);
                        gl.delete_shader(shader);
                        return Err(format!("sprite shader compile: {log}"));
                    }
                    gl.attach_shader(program, shader);
                    // Deletion is deferred until program deletion or detachment.
                    gl.delete_shader(shader);
                }
                gl.link_program(program);
                if !gl.get_program_link_status(program) {
                    return Err(format!(
                        "sprite program link: {}",
                        gl.get_program_info_log(program)
                    ));
                }
                Ok(())
            })();
            if let Err(error) = link {
                gl.delete_program(program);
                return Err(error);
            }
            let vao = match gl.create_vertex_array() {
                Ok(vao) => vao,
                Err(error) => {
                    gl.delete_program(program);
                    return Err(error);
                }
            };
            let vbo = match gl.create_buffer() {
                Ok(vbo) => vbo,
                Err(error) => {
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return Err(error);
                }
            };
            let owned = Self {
                gl: Rc::clone(gl),
                program,
                vao,
                vbo,
                u_dest: gl.get_uniform_location(program, "u_dest"),
                u_uv: gl.get_uniform_location(program, "u_uv[0]"),
                u_tex: gl.get_uniform_location(program, "u_tex"),
            };
            let vertices: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
            let bytes: Vec<u8> = vertices.into_iter().flat_map(f32::to_ne_bytes).collect();
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                return Err(format!("sprite quad setup: GL 0x{error:x}"));
            }
            Ok(owned)
        }
    }

    pub fn draw(
        &self,
        texture: glow::Texture,
        dest: [f32; 4],
        corners: [[f32; 2]; 4],
    ) -> Result<(), String> {
        let gl = &self.gl;
        // SAFETY: frame restored this owner's context; texture and buffers are live.
        unsafe {
            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.color_mask(true, true, true, true);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_4_f32(self.u_dest.as_ref(), dest[0], dest[1], dest[2], dest[3]);
            gl.uniform_2_f32_slice(self.u_uv.as_ref(), corners.as_flattened());
            gl.uniform_1_i32(self.u_tex.as_ref(), 0);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                return Err(format!("sprite draw: GL 0x{error:x}"));
            }
        }
        Ok(())
    }
}
impl Drop for SpriteQuad {
    fn drop(&mut self) {
        // SAFETY: like SheetTexture, App field order and shutdown keep this context
        // current and alive through every playback drop, including creation failures.
        unsafe {
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_program(self.program);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        raster::RgbaImage,
        sprite::{Transform, texture::SheetTexture},
    };

    #[test]
    #[ignore = "requires a display; run under Xvfb"]
    fn transformed_pixels_match_gpu_readback() {
        // SDL initialization belongs to one OS thread per process. The texture
        // test initializes SDL in this harness, so only our child may create it.
        if std::env::var_os("RN_SPRITE_QUAD_GL_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "sprite::draw::tests::transformed_pixels_match_gpu_readback",
                    "--nocapture",
                ])
                .env("RN_SPRITE_QUAD_GL_CHILD", "1")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "GPU readback child: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                output.status
            );
            return;
        }

        const N: u32 = 4;
        let mut image = RgbaImage::new(N, N);
        // Red labels are 1..=16 in screen row order; every texel is distinct,
        // nonblack and opaque so SpriteQuad's blending preserves exact bytes.
        for y in 0..N {
            for x in 0..N {
                image.set_pixel(
                    x,
                    y,
                    [
                        (1 + y * N + x) as u8,
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

        // Declaration order keeps SDL/window/context alive through every GL
        // object drop, including assertion unwinding. No other thread uses GL.
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        video
            .gl_attr()
            .set_context_profile(sdl3::video::GLProfile::Core);
        video.gl_attr().set_context_version(3, 3);
        let window = video
            .window("sprite quad readback test", 64, 64)
            .opengl()
            .hidden()
            .build()
            .unwrap();
        let context = window.gl_create_context().unwrap();
        window.gl_make_current(&context).unwrap();
        // SAFETY: the test owns this current context until all GL owners drop.
        let gl = Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        });
        let sheet = SheetTexture::upload(&gl, &image).unwrap();
        let quad = SpriteQuad::new(&gl).unwrap();

        // SAFETY: all calls use the current context on this thread. The FBO's
        // separate renderbuffer cannot feed back into the sampled sheet texture.
        unsafe {
            let target = gl.create_renderbuffer().unwrap();
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(target));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, N as i32, N as i32);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            let fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(target),
            );
            assert_eq!(
                gl.check_framebuffer_status(glow::FRAMEBUFFER),
                glow::FRAMEBUFFER_COMPLETE
            );
            gl.viewport(0, 0, N as i32, N as i32);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            assert_eq!(gl.get_error(), glow::NO_ERROR, "FBO setup");

            for transform in Transform::ALL {
                gl.clear(glow::COLOR_BUFFER_BIT);
                // Same sign convention as Playback::draw: a_pos.y=0 is the
                // screen top, hence negative dest height. Pixel centers align
                // with texel centers at this 1:1 size (NEAREST sheet filtering).
                quad.draw(
                    sheet.raw(),
                    [-1.0, 1.0, 2.0, -2.0],
                    transform.uv_corners([0.0, 0.0], [1.0, 1.0]),
                )
                .unwrap();
                let mut pixels = vec![0; (N * N * 4) as usize];
                gl.read_pixels(
                    0,
                    0,
                    N as i32,
                    N as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(Some(&mut pixels)),
                );
                assert_eq!(gl.get_error(), glow::NO_ERROR, "readback {transform:?}");
                let expected = image.transformed(transform);
                for row in 0..N {
                    for x in 0..N {
                        let offset = ((row * N + x) * 4) as usize;
                        // GL row zero is the screen bottom, while transformed
                        // images start at the top: explicitly reverse y.
                        assert_eq!(
                            pixels[offset..offset + 4],
                            expected.pixel(x, N - 1 - row),
                            "{transform:?}, readback ({x}, {row})"
                        );
                    }
                }

                // Independent handwritten oracles, not derived from source()
                // or uv_corners(). Input screen rows are [1 2 3 4] through
                // [13 14 15 16]. Clockwise 90 has top row [13 9 5 1]; flipping
                // first then rotating has top row [16 12 8 4]. These literals
                // list GL readback rows, bottom first, and catch a shared CPU/UV
                // mistake such as swapping source()'s 90 and 270 branches.
                let handwritten = match (transform.flip_x(), transform.rotation()) {
                    (false, 90) => Some([16, 12, 8, 4, 15, 11, 7, 3, 14, 10, 6, 2, 13, 9, 5, 1]),
                    (true, 90) => Some([13, 9, 5, 1, 14, 10, 6, 2, 15, 11, 7, 3, 16, 12, 8, 4]),
                    _ => None,
                };
                if let Some(labels) = handwritten {
                    let actual: Vec<_> = pixels.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
                    assert_eq!(actual, labels, "handwritten {transform:?}");
                }
            }

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(fbo);
            gl.delete_renderbuffer(target);
            drop(quad);
            drop(sheet);
            assert_eq!(gl.get_error(), glow::NO_ERROR, "GL cleanup");
        }
    }
}
