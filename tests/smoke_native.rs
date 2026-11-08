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

fn assert_success_line(output: &Output, frames: u32, play: bool) {
    assert!(output.status.success(), "{output:?}");
    // Use a whole-output regex, including exactly one newline. Keep dependencies
    // unchanged; the script checks that Python is available for this validator.
    let counts = if cfg!(feature = "dev") {
        "vtx=[1-9][0-9]* lit=[1-9][0-9]*"
    } else if play {
        "vtx=0 lit=[1-9][0-9]*"
    } else {
        "vtx=0 lit=0"
    };
    let mut validator = Command::new("python3").args(["-c",
        "import re,sys; s=sys.stdin.read(); assert re.fullmatch(r'rn-smoke: ok frames='+sys.argv[1]+r' '+sys.argv[2]+r'\\n',s),repr(s)",
        &frames.to_string(), counts]).stdin(Stdio::piped()).spawn().unwrap();
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
        assert_success_line(&output, frames, false);
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

// The sheet sits in <root>/sheet. <root>/example.png is a valid PNG as well, so an
// escaping `../example.png` can only fail on the path rules, not on a missing file.
struct PlayFixture {
    root: std::path::PathBuf,
    sheet: std::path::PathBuf,
}
impl PlayFixture {
    fn new() -> Self {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("smoke-play-{}", std::process::id()));
        let sheet = root.join("sheet");
        std::fs::create_dir_all(&sheet).unwrap();
        let owned = Self { root, sheet };
        for name in ["example.png", "example.sprites.json"] {
            std::fs::copy(fixtures.join(name), owned.sheet.join(name)).unwrap();
        }
        std::fs::copy(fixtures.join("example.png"), owned.root.join("example.png")).unwrap();
        owned
    }
}
impl Drop for PlayFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
#[ignore = "requires X11/GL; use scripts/smoke-native.sh"]
fn smoke_play() {
    let fixture = PlayFixture::new();
    let sidecar = fixture.sheet.join("example.sprites.json");
    let source = sidecar.to_str().unwrap();
    let (output, timeout) = run(
        &["--play", source, "--smoke-frames", "3"],
        false,
        Duration::from_secs(10),
    );
    assert!(!timeout, "{output:?}");
    assert_success_line(&output, 3, true);
    let missing = fixture.sheet.join("missing.sprites.json");
    for args in [
        vec!["--play", missing.to_str().unwrap(), "--smoke-frames", "3"],
        vec!["--play", source, "missing", "--smoke-frames", "3"],
    ] {
        let (output, timeout) = run(&args, false, Duration::from_secs(5));
        assert!(!timeout && !output.status.success(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stdout).starts_with("rn-smoke: fail "),
            "{output:?}"
        );
    }
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
    json["image"] = "../example.png".into();
    std::fs::write(&sidecar, serde_json::to_vec(&json).unwrap()).unwrap();
    let (output, timeout) = run(
        &["--play", source, "--smoke-frames", "3"],
        false,
        Duration::from_secs(5),
    );
    assert!(!timeout && !output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("rn-smoke: fail "),
        "{output:?}"
    );
}
