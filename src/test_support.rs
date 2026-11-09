use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

pub(crate) struct TempDir(pub(crate) PathBuf);

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rn-{label}-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn temp_dirs_are_unique_and_removed() {
    let first = TempDir::new("test");
    let second = TempDir::new("test");
    assert_ne!(first.0, second.0);
    assert!(first.0.is_dir());
    let path = first.0.clone();
    drop(first);
    assert!(!path.exists());
}

pub(crate) struct HiddenGl {
    pub(crate) gl: std::rc::Rc<glow::Context>,
    _context: sdl3::video::GLContext,
    _window: sdl3::video::Window,
    _video: sdl3::VideoSubsystem,
    _sdl: sdl3::Sdl,
}
impl HiddenGl {
    pub(crate) fn new(title: &str) -> Self {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        video
            .gl_attr()
            .set_context_profile(sdl3::video::GLProfile::Core);
        video.gl_attr().set_context_version(3, 3);
        let window = video
            .window(title, 64, 64)
            .opengl()
            .hidden()
            .build()
            .unwrap();
        let context = window.gl_create_context().unwrap();
        window.gl_make_current(&context).unwrap();
        // SAFETY: the fixture keeps the context current until its GL owners are dropped.
        let gl = std::rc::Rc::new(unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        });
        Self {
            gl,
            _context: context,
            _window: window,
            _video: video,
            _sdl: sdl,
        }
    }
}

pub(crate) fn in_child_process(
    test: &str,
    marker: Option<&str>,
    ignored: bool,
    cwd: Option<&std::path::Path>,
) -> bool {
    if marker.is_some_and(|name| std::env::var_os(name).is_some()) {
        return true;
    }
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", test, "--nocapture"]);
    if ignored {
        command.arg("--ignored");
    }
    if let Some(name) = marker {
        command.env(name, "1");
    }
    if let Some(path) = cwd {
        command.current_dir(path);
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "child {test}: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
    false
}
