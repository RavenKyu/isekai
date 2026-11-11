fn run(mode: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/smoke-native-isekai.py"))
        .arg(env!("CARGO_BIN_EXE_isekai"))
        .arg(mode)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}
#[test]
#[ignore = "requires X11, lavapipe and XTEST; run scripts/smoke-native-isekai.sh"]
fn native_contract() {
    run(if cfg!(feature = "dev") {
        "dev"
    } else {
        "blank"
    });
}
#[test]
#[ignore = "requires real X server autorepeat; run scripts/smoke-native-isekai.sh"]
fn native_input_repeat() {
    run(if cfg!(feature = "dev") {
        "input-dev"
    } else {
        "input-blank"
    });
}

#[cfg(feature = "dev")]
#[test]
#[ignore = "requires X11 and real XTEST; run scripts/smoke-native-isekai.sh --dev"]
fn native_dev_smoke_stability() {
    for _ in 0..10 {
        run("dev-smoke");
    }
}

#[test]
#[ignore = "requires X11 focus snapshots; run scripts/smoke-native-isekai.sh"]
fn native_focus_snapshot() {
    run(if cfg!(feature = "dev") {
        "focus-dev"
    } else {
        "focus-blank"
    });
}
