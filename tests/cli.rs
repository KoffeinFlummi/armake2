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

fn write(dir: &Path, name: &str, content: &str) {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
}

#[test]
fn unquoted_before_brace() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "c.cpp", "\nclass A { x = foo };\n"); // leading newline: line-1 warnings are fixed in a later task
    let (code, _, err) = armake2(&["rapify", "c.cpp", "c.bin"], dir.path());
    assert_eq!(code, 0, "{}", err);
    let (code, out, _) = armake2(&["derapify", "c.bin"], dir.path());
    assert_eq!(code, 0);
    assert!(out.contains("x = \"foo\";"), "{}", out);
}
