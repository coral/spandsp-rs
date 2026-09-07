#![cfg(all(feature = "fax", unix))]
use std::{path::PathBuf, process::Command};
#[test]
fn native_recovery_faults() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("spandsp-native-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flags = Command::new("pkg-config")
        .args(["--cflags", "--libs", "libtiff-4", "libjpeg"])
        .output()
        .unwrap();
    assert!(flags.status.success());
    let mut cmd = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
    cmd.arg("-std=c99")
        .arg("-g")
        .arg("-I")
        .arg(env!("OUT_DIR"))
        .arg("-I")
        .arg(manifest.join("vendor/src"))
        .arg(manifest.join("tests/receive_recovery.c"))
        .arg(PathBuf::from(env!("OUT_DIR")).join("libspandsp.a"))
        .args(String::from_utf8(flags.stdout).unwrap().split_whitespace())
        .arg("-lm")
        .arg("-o")
        .arg(dir.join("test"));
    if let Ok(flags) = std::env::var("CFLAGS") {
        cmd.args(flags.split_whitespace());
    }
    let built = cmd.output().unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let result = Command::new(dir.join("test"))
        .arg(dir.join("output.tif"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}
