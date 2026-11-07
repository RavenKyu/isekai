mod app;
mod smoke;
mod ui;

use app::{App, AppResult, Flow};

#[cfg(not(target_os = "emscripten"))]
fn run(smoke: Option<smoke::Smoke>) -> AppResult<Option<smoke::Report>> {
    let mut app = App::new(smoke)?;
    let result = (|| {
        loop {
            match app.frame()? {
                Flow::Continue => {}
                Flow::Quit => return Ok(None),
                Flow::SmokeComplete(report) => return Ok(Some(report)),
            }
        }
    })();
    let shutdown = app.shutdown();
    shutdown?;
    result
}

#[cfg(not(target_os = "emscripten"))]
fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let smoking = !args.is_empty();
    let result = smoke::Smoke::parse(args).map_err(Into::into).and_then(run);
    match result {
        Ok(report) => {
            if let Some(report) = report {
                println!("{}", report.line());
            }
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            if smoking {
                println!("{}", smoke::failure(&error.to_string()));
            } else {
                eprintln!("rusty-nails: {error}");
            }
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "emscripten")]
fn main() {
    web::run();
}

#[cfg(target_os = "emscripten")]
mod web {
    use super::*;
    use std::ffi::c_void;

    unsafe extern "C" {
        fn emscripten_set_main_loop_arg(
            callback: extern "C" fn(*mut c_void),
            arg: *mut c_void,
            fps: i32,
            simulate_infinite_loop: bool,
        );
        fn emscripten_cancel_main_loop();
    }

    extern "C" fn tick(arg: *mut c_void) {
        // SAFETY: run transfers exactly one Box<App> into the browser loop.
        // RAF callbacks are serialized on the initializing thread. No stack data
        // is borrowed and the allocation is never reclaimed during page lifetime.
        let app = unsafe { &mut *arg.cast::<App>() };
        let result = app.frame();
        if matches!(result, Ok(Flow::Continue)) {
            return;
        }
        // SAFETY: called on the same browser thread as registration; cancellation
        // ends future callbacks before reporting completion. Keep App/GL alive so
        // the final canvas remains visible until the page itself is destroyed.
        unsafe { emscripten_cancel_main_loop() };
        match result {
            Ok(Flow::SmokeComplete(report)) => println!("{}", report.line()),
            Err(error) => println!("{}", smoke::failure(&error.to_string())),
            Ok(Flow::Quit | Flow::Continue) => {}
        }
    }

    pub fn run() {
        let app: AppResult<Box<App>> = smoke::Smoke::parse(std::env::args().skip(1))
            .map_err(Into::into)
            .and_then(App::new);
        let app = match app {
            Ok(app) => app,
            Err(error) => {
                println!("{}", smoke::failure(&error.to_string()));
                return;
            }
        };
        let app = Box::into_raw(app);
        // SAFETY: the callback owns access to this stable heap allocation. fps=0
        // selects RAF; false lets main return normally without a JS exception
        // crossing Rust stack frames. EXIT_RUNTIME stays disabled (the default).
        unsafe { emscripten_set_main_loop_arg(tick, app.cast(), 0, false) };
    }
}
