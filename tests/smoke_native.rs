use std::{
    io::Write,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Reap(Option<Child>);

impl Drop for Reap {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run(args: &[&str], no_display: bool, limit: Duration) -> (Output, bool) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rusty-nails"));
    command
        .args(args)
        .env_remove("WAYLAND_DISPLAY")
        .env("SDL_VIDEODRIVER", "x11")
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if no_display {
        command.env_remove("DISPLAY");
    }
    let mut child = Reap(Some(command.spawn().expect("spawn rusty-nails")));
    let deadline = Instant::now() + limit;
    let timed_out = loop {
        if child.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            break false;
        }
        if Instant::now() >= deadline {
            child.0.as_mut().unwrap().kill().unwrap();
            break true;
        }
        thread::sleep(Duration::from_millis(10));
    };
    (
        child.0.take().unwrap().wait_with_output().unwrap(),
        timed_out,
    )
}

fn assert_success_line(output: &Output, frames: u32) {
    assert!(output.status.success(), "{output:?}");
    // Use a whole-output regex, including exactly one newline. Keep dependencies
    // unchanged; the script checks that Python is available for this validator.
    let mut validator = Command::new("python3").args(["-c",
        "import re,sys; s=sys.stdin.read(); assert re.fullmatch(r'rn-smoke: ok frames='+sys.argv[1]+r' vtx=[1-9][0-9]* lit=[1-9][0-9]*\\n',s),repr(s)",
        &frames.to_string()]).stdin(Stdio::piped()).spawn().unwrap();
    validator
        .stdin
        .take()
        .unwrap()
        .write_all(&output.stdout)
        .unwrap();
    assert!(validator.wait().unwrap().success(), "{output:?}");
}

#[test]
#[ignore = "requires X11/GL; use scripts/smoke-native.sh"]
fn smoke_native() {
    for frames in [1, 3, 5] {
        let (output, timeout) = run(
            &["--smoke-frames", &frames.to_string()],
            false,
            Duration::from_secs(10),
        );
        assert!(!timeout, "smoke exceeded deadline: {output:?}");
        assert_success_line(&output, frames);
    }
    // A context/window failure must fail, and never silently skip the smoke.
    let (output, timeout) = run(&["--smoke-frames", "3"], true, Duration::from_secs(5));
    assert!(!timeout && !output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rn-smoke: fail "));
    for args in [
        &["--smoke-frames"][..],
        &["--smoke-frames", "0"],
        &["--smoke-frames", "bad"],
    ] {
        let (output, timeout) = run(args, false, Duration::from_secs(2));
        assert!(!timeout && !output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).starts_with("rn-smoke: fail "));
    }
    // No flag means an interactive loop, not an implicit three-frame smoke.
    // This also exercises the watchdog's kill-and-reap path for a running app.
    let (output, timeout) = run(&[], false, Duration::from_millis(500));
    assert!(timeout, "interactive app unexpectedly exited: {output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
}
