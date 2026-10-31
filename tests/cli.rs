//! Tests that drive the built `armake2` binary.
#![allow(dead_code)]
use std::path::Path;
use std::process::Command;

/// Runs armake2 in `cwd`; returns (exit code, stdout, stderr). A panic shows up as 101.
pub fn armake2(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_armake2"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run armake2");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn cli_version_runs() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, _) = armake2(&["--version"], dir.path());
    assert_eq!(code, 0);
    assert!(out.starts_with("v"), "{}", out);
}
