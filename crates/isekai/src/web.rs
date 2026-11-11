use crate::{
    app::{App, UserEvent},
    cli::Cli,
};
use clap::Parser;
use isekai_render::Gpu;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use winit::{event_loop::ActiveEventLoop, window::Window};
fn log() -> web_sys::Element {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("isekai-log")
        .unwrap()
}
pub fn output(line: &str) {
    let log = log();
    log.set_text_content(Some(&format!("{line}\n")));
    web_sys::console::log_1(&line.into());
}
pub fn failure(reason: &str) {
    output(&crate::smoke::failure(reason));
    let _ = log().set_attribute("data-error", "true");
}
pub fn ready() {
    thread_local! { static READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
    READY.with(|ready| {
        if !ready.replace(true) {
            let _ = log().set_attribute("data-ready", "true");
        }
    });
}
fn params() -> web_sys::UrlSearchParams {
    web_sys::UrlSearchParams::new_with_str(&web_sys::window().unwrap().location().search().unwrap())
        .unwrap()
}
pub fn window(event_loop: &ActiveEventLoop) -> Result<Arc<Window>, String> {
    use winit::platform::web::WindowAttributesExtWebSys;
    let browser = web_sys::window().unwrap();
    let canvas = browser
        .document()
        .unwrap()
        .get_element_by_id("isekai-canvas")
        .ok_or("missing canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "invalid canvas")?;
    let scale = browser.device_pixel_ratio();
    let attributes = Window::default_attributes()
        .with_title("isekai")
        .with_canvas(Some(canvas))
        .with_inner_size(winit::dpi::PhysicalSize::new(
            (1280. * scale) as u32,
            (720. * scale) as u32,
        ));
    Ok(Arc::new(
        event_loop
            .create_window(attributes)
            .map_err(|e| e.to_string())?,
    ))
}
pub async fn gpu(window: Arc<Window>) -> Result<(Gpu, wgpu::Surface<'static>), String> {
    let backend = params().get("backend").unwrap_or("auto".into());
    let backends: &[wgpu::Backends] = match backend.as_str() {
        "auto" => &[wgpu::Backends::BROWSER_WEBGPU, wgpu::Backends::GL],
        "webgpu" => &[wgpu::Backends::BROWSER_WEBGPU],
        "webgl2" => &[wgpu::Backends::GL],
        _ => return Err(format!("unknown backend: {backend}")),
    };
    let mut errors = Vec::new();
    for &backend in backends {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backend;
        if backend == wgpu::Backends::BROWSER_WEBGPU
            && !wgpu::util::is_browser_webgpu_supported().await
        {
            errors.push("WebGPU adapter unavailable".into());
            continue;
        }
        let instance = wgpu::Instance::new(descriptor);
        if backend == wgpu::Backends::GL {
            let surface = instance
                .create_surface(window.clone())
                .map_err(|e| e.to_string())?;
            match Gpu::new(instance, Some(&surface)).await {
                Ok(gpu) => return Ok((gpu, surface)),
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            }
        }
        match Gpu::new(instance, None).await {
            Ok(gpu) => {
                let surface = gpu
                    .instance
                    .create_surface(window.clone())
                    .map_err(|e| e.to_string())?;
                return Ok((gpu, surface));
            }
            Err(e) => errors.push(e),
        }
    }
    Err(errors.join("; "))
}
pub fn start() {
    use winit::platform::web::EventLoopExtWebSys;
    console_error_panic_hook::set_once();
    let result = (|| -> Result<(), String> {
        let params = params();
        let mut args = vec!["isekai".to_string()];
        if let Some(play) = params.get("play") {
            args.extend(["play".into(), play]);
            if let Some(animation) = params.get("animation").or_else(|| params.get("clip")) {
                args.push(animation);
            }
        }
        if let Some(smoke) = params.get("smoke") {
            args.extend(["--smoke-frames".into(), smoke]);
        }
        let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
        let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|e| e.to_string())?;
        let mut app = App::new(cli.smoke()?, cli.command);
        app.set_proxy(event_loop.create_proxy());
        event_loop.spawn_app(app);
        Ok(())
    })();
    if let Err(e) = result {
        failure(&e);
    }
}
