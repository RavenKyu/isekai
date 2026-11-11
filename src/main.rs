mod app;
#[cfg(feature = "dev")]
mod dev;
mod raster;
mod smoke;
mod sprite;
#[cfg(test)]
mod test_support;
#[cfg(feature = "dev")]
mod ui;

use app::{App, AppResult, Flow};

#[derive(Debug, PartialEq, Eq)]
struct PlayRequest {
    source: String,
    animation: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct Cli {
    play: Option<PlayRequest>,
    rest: Vec<String>,
}

// Separate playback first; Smoke::parse retains ownership of its original grammar.
fn split_play_args(args: Vec<String>) -> Result<Cli, String> {
    let mut args = args.into_iter().peekable();
    let play = if args.peek().is_some_and(|arg| arg == "--play") {
        args.next();
        let source = args
            .next()
            .filter(|arg| !arg.is_empty() && !arg.starts_with("--"))
            .ok_or("--play requires a source")?;
        let animation = if args.peek().is_some_and(|arg| !arg.starts_with("--")) {
            args.next()
        } else {
            None
        };
        Some(PlayRequest { source, animation })
    } else {
        None
    };
    let rest: Vec<_> = args.collect();
    // Reject a misplaced or repeated play flag before handing smoke its arguments.
    if rest.iter().any(|arg| arg == "--play") {
        return Err("--play must precede --smoke-frames and occur once".into());
    }
    Ok(Cli { play, rest })
}

fn smoke_expect(play: bool) -> smoke::Expect {
    if cfg!(feature = "dev") {
        smoke::Expect::Drawn
    } else if play {
        smoke::Expect::Sprite
    } else {
        smoke::Expect::Blank
    }
}

#[cfg(not(target_os = "emscripten"))]
fn run(smoke: Option<smoke::Smoke>, play: Option<PlayRequest>) -> AppResult<Option<smoke::Report>> {
    let mut app = App::new(smoke, play)?;
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
    let smoking = args.iter().any(|arg| arg == "--smoke-frames");
    let result: AppResult<Option<smoke::Report>> = (|| {
        let cli = split_play_args(args)?;
        let smoke = smoke::Smoke::parse(cli.rest, smoke_expect(cli.play.is_some()))?;
        run(smoke, cli.play)
    })();
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
        let app: AppResult<Box<App>> = (|| {
            let cli = split_play_args(std::env::args().skip(1).collect())?;
            let smoke = smoke::Smoke::parse(cli.rest, smoke_expect(cli.play.is_some()))?;
            App::new(smoke, cli.play)
        })();
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

#[cfg(test)]
mod cli_tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Cli, String> {
        let cli = split_play_args(args.iter().map(|s| s.to_string()).collect())?;
        smoke::Smoke::parse(cli.rest.clone(), smoke_expect(cli.play.is_some()))?;
        Ok(cli)
    }
    #[test]
    fn playback_precedes_smoke_and_has_optional_animation() {
        for args in [
            vec!["--play"],
            vec!["--play", "--smoke-frames", "3"],
            vec!["--play", "a.json", "b", "c"],
            vec!["--smoke-frames", "3", "--play", "x"],
            vec!["--play", "x", "--play", "y"],
            vec!["--unknown"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        for args in [
            vec!["--play", "a.json"],
            vec!["--play", "a.json", "--smoke-frames", "3"],
            vec!["--play", "a.json", "cycle", "--smoke-frames", "3"],
        ] {
            let cli = parse(&args).unwrap();
            let play = cli.play.unwrap();
            assert_eq!(play.source, "a.json");
            assert_eq!(
                play.animation.as_deref(),
                args.contains(&"cycle").then_some("cycle")
            );
        }
        assert!(parse(&[]).unwrap().play.is_none());
        assert!(parse(&["--smoke-frames", "3"]).unwrap().play.is_none());
    }
}
