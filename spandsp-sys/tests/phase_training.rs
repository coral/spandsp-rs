#![cfg(all(feature = "fax", unix))]

use std::{path::PathBuf, process::Command};

#[test]
fn fax_modems_train_across_carrier_phase_wrap() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("spandsp-phase-training-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flags = Command::new("pkg-config")
        .args(["--cflags", "--libs", "libtiff-4", "libjpeg"])
        .output()
        .unwrap();
    assert!(
        flags.status.success(),
        "{}",
        String::from_utf8_lossy(&flags.stderr)
    );
    let mut cmd = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
    cmd.args(["-std=c99", "-O2", "-I"])
        .arg(env!("OUT_DIR"))
        .arg("-I")
        .arg(manifest.join("vendor/src"))
        .arg(manifest.join("tests/phase_training.c"))
        .arg(PathBuf::from(env!("OUT_DIR")).join("libspandsp.a"))
        .args(String::from_utf8(flags.stdout).unwrap().split_whitespace())
        .args(["-lm", "-o"])
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
    let result = Command::new(dir.join("test")).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}
