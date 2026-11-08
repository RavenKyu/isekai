use crate::raster::RgbaImage;
use glow::HasContext;
use std::rc::Rc;

/// Application-owned texture. Every upload/drop must run on its current context.
pub struct SheetTexture {
    gl: Rc<glow::Context>,
    texture: glow::Texture,
    pub width: u32,
    pub height: u32,
}
impl SheetTexture {
    pub fn upload(gl: &Rc<glow::Context>, image: &RgbaImage) -> Result<Self, String> {
        let bytes = (image.width as usize)
            .checked_mul(image.height as usize)
            .and_then(|n| n.checked_mul(4));
        if image.width == 0 || image.height == 0 || bytes != Some(image.pixels.len()) {
            return Err("invalid RGBA texture dimensions or pixel length".into());
        }
        // SAFETY: App makes its owned GL context current before frame/shutdown.
        // Callers also keep it current during creation and error cleanup.
        unsafe {
            let limit = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE);
            if limit <= 0 || image.width > limit as u32 || image.height > limit as u32 {
                return Err(format!(
                    "sheet {}x{} exceeds GL_MAX_TEXTURE_SIZE {limit}",
                    image.width, image.height
                ));
            }
            let texture = gl.create_texture()?;
            let owned = Self {
                gl: Rc::clone(gl),
                texture,
                width: image.width,
                height: image.height,
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
            for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::NEAREST as i32);
            }
            for parameter in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::CLAMP_TO_EDGE as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                image.width as i32,
                image.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&image.pixels)),
            );
            let error = gl.get_error();
            gl.bind_texture(glow::TEXTURE_2D, None);
            if error != glow::NO_ERROR {
                return Err(format!("sheet GL upload failed: 0x{error:x}"));
            }
            Ok(owned)
        }
    }
    pub fn raw(&self) -> glow::Texture {
        self.texture
    }
}
impl Drop for SheetTexture {
    fn drop(&mut self) {
        // SAFETY: owners drop only after App::frame restores the current context,
        // or inside App::shutdown. App fields containing textures precede
        // gl_context/window. Dev unregisters before this drop and renderer shutdown.
        unsafe {
            self.gl.delete_texture(self.texture);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a display; run under Xvfb"]
    fn upload_limit_filtering_and_drop() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        video
            .gl_attr()
            .set_context_profile(sdl3::video::GLProfile::Core);
        video.gl_attr().set_context_version(3, 3);
        let window = video
            .window("texture lifetime test", 64, 64)
            .opengl()
            .hidden()
            .build()
            .unwrap();
        let context = window.gl_create_context().unwrap();
        window.gl_make_current(&context).unwrap();
        // SAFETY: this test owns a current context until after texture drop.
        let gl = Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        });
        let texture = SheetTexture::upload(&gl, &crate::sprite::example().0).unwrap();
        let raw = texture.raw();
        // SAFETY: context is current and texture is alive.
        unsafe {
            assert!(gl.is_texture(raw));
            gl.bind_texture(glow::TEXTURE_2D, Some(raw));
            assert_eq!(
                gl.get_tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER),
                glow::NEAREST as i32
            );
        }
        drop(texture);
        // SAFETY: the current context outlives this check.
        unsafe {
            assert!(!gl.is_texture(raw));
        }
        let limit = unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) } as u32;
        let oversized = RgbaImage::new(limit + 1, 1);
        assert!(
            SheetTexture::upload(&gl, &oversized)
                .err()
                .unwrap()
                .contains("GL_MAX_TEXTURE_SIZE")
        );
    }
}
