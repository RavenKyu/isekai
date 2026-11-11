mod app;
mod cli;
mod input;
#[cfg(not(target_arch = "wasm32"))]
mod native_probe;
mod playback;
mod smoke;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().collect();
    let parsed = cli::Cli::try_parse_from(&args);
    let smoking = cli::smoke_requested(&args, parsed.as_ref().ok());
    if let Err(error) = &parsed
        && error.exit_code() == 0
    {
        let _ = error.print();
        return std::process::ExitCode::SUCCESS;
    }
    let result = (|| {
        let cli = parsed.map_err(|e| e.to_string())?;
        let smoke = cli.smoke()?;
        let event_loop = winit::event_loop::EventLoop::<app::UserEvent>::with_user_event()
            .build()
            .map_err(|e| e.to_string())?;
        let mut app = app::App::new(smoke, cli.command);
        event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
        app.result
    })();
    match result {
        Ok(line) => {
            if let Some(line) = line {
                println!("{line}");
            }
            std::process::ExitCode::SUCCESS
        }
        Err(reason) => {
            if smoking {
                println!("{}", smoke::failure(&reason));
            } else {
                eprintln!("isekai: {reason}");
            }
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}
