use glow::HasContext;
use std::rc::Rc;

#[cfg(not(target_os = "emscripten"))]
const HEADER: &str = "#version 330 core\n";
#[cfg(target_os = "emscripten")]
const HEADER: &str = "#version 300 es\nprecision mediump float;\n";
const VERT_BODY: &str = r#"
layout(location = 0) in vec2 a_pos;
uniform vec4 u_dest;
uniform vec4 u_uv;
out vec2 v_uv;
void main() {
    gl_Position = vec4(u_dest.xy + a_pos * u_dest.zw, 0.0, 1.0);
    v_uv = mix(u_uv.xy, u_uv.zw, a_pos);
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
                u_uv: gl.get_uniform_location(program, "u_uv"),
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

    pub fn draw(&self, texture: glow::Texture, dest: [f32; 4], uv: [f32; 4]) -> Result<(), String> {
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
            gl.uniform_4_f32(self.u_uv.as_ref(), uv[0], uv[1], uv[2], uv[3]);
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
