mod app;
mod cli;
mod input;
mod playback;
mod smoke;
use clap::Parser;
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().collect();
    let smoking = args.iter().any(|s| s == "--smoke-frames");
    let result = (|| {
        let cli = cli::Cli::try_parse_from(&args).map_err(|e| e.to_string())?;
        let smoke = cli.smoke()?;
        let event_loop = winit::event_loop::EventLoop::new().map_err(|e| e.to_string())?;
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
