#[test]
#[ignore = "requires X11, lavapipe and XTEST; run scripts/smoke-native-isekai.sh"]
fn native_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/smoke-native-isekai.py"))
        .arg(env!("CARGO_BIN_EXE_isekai"))
        .arg(if cfg!(feature = "dev") {
            "dev"
        } else {
            "blank"
        })
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
