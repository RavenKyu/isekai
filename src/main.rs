mod app;
mod smoke;
mod ui;

use app::{App, AppResult, Flow};

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
