mod app;
mod ui;

use app::{App, AppResult, Flow};

fn run() -> AppResult<()> {
    let mut app = App::new()?;
    let result = (|| {
        while app.frame()? == Flow::Continue {}
        Ok(())
    })();
    let shutdown = app.shutdown();
    result.and(shutdown)
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rusty-nails: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
