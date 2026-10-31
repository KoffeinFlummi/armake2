//! Tests that drive the built `armake2` binary.
#![allow(dead_code)]
use std::path::Path;
use std::process::Command;

/// Runs armake2 in `cwd`; returns (exit code, stdout, stderr). A panic shows up as 101.
pub fn armake2(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let (code, out, err) = armake2_raw(args, cwd);
    (code, String::from_utf8_lossy(&out).into_owned(), err)
}

/// Like `armake2`, but stdout as raw bytes.
pub fn armake2_raw(args: &[&str], cwd: &Path) -> (i32, Vec<u8>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_armake2"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run armake2");
    (
        out.status.code().unwrap_or(-1),
        out.stdout,
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

fn rapify_str(src: &str) -> (i32, String) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "config.cpp", src);
    let (code, _, err) = armake2(&["rapify", "config.cpp", "c.bin"], dir.path());
    (code, err)
}

#[test]
fn warning_on_line_one_no_panic() {
    let (code, err) = rapify_str("class A { x = foo; };\n");
    assert_eq!(code, 0, "{}", err);
    assert!(err.contains(":1:") && err.contains("unquoted-string"), "{}", err);
}

#[test]
fn warning_on_line_two_location() {
    let (code, err) = rapify_str("\nclass A { x = foo; };\n");
    assert_eq!(code, 0, "{}", err);
    assert!(err.contains(":2:"), "{}", err);
}

#[test]
fn warning_printed_once() {
    let (_, err) = rapify_str("class A { x = foo; };\n");
    assert_eq!(err.matches("unquoted-string").count(), 1, "{}", err);
}

fn prerapified_mod() -> (tempfile::TempDir, Vec<u8>) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "src.cpp", "class A { x = 1; };\n");
    let (code, _, err) = armake2(&["rapify", "src.cpp", "src.bin"], dir.path());
    assert_eq!(code, 0, "{}", err);
    let bin = std::fs::read(dir.path().join("src.bin")).unwrap();
    (dir, bin)
}

#[test]
fn build_keeps_prerapified_rvmat() {
    let (dir, bin) = prerapified_mod();
    write(dir.path(), "m/config.cpp", "class B { y = 2; };\n");
    std::fs::create_dir_all(dir.path().join("m/data")).unwrap();
    std::fs::write(dir.path().join("m/data/x.rvmat"), &bin).unwrap();
    let (code, _, err) = armake2(&["build", "m", "out.pbo"], dir.path());
    assert_eq!(code, 0, "{}", err);
    let (code, out, err) = armake2_raw(&["cat", "out.pbo", "data\\x.rvmat"], dir.path());
    assert_eq!(code, 0, "{}", err);
    assert_eq!(out, bin);
}

#[test]
fn build_keeps_prerapified_config_cpp() {
    let (dir, bin) = prerapified_mod();
    std::fs::create_dir_all(dir.path().join("m")).unwrap();
    std::fs::write(dir.path().join("m/config.cpp"), &bin).unwrap();
    let (code, _, err) = armake2(&["build", "m", "out.pbo"], dir.path());
    assert_eq!(code, 0, "{}", err);
    let (code, out, err) = armake2_raw(&["cat", "out.pbo", "config.bin"], dir.path());
    assert_eq!(code, 0, "{}", err);
    assert_eq!(out, bin);
}

#[test]
fn macro_arity_warns() {
    let (code, err) = rapify_str("#define QUOTE(x) #x\nclass A { t = QUOTE(a,b); };\n");
    assert_eq!(code, 0, "{}", err);
    assert!(err.contains("macro-arg-count") && err.contains("QUOTE"), "{}", err);
}

#[test]
fn object_like_macro_no_arity_warning() {
    let (code, err) = rapify_str("#define FOO bar\nclass A { t = FOO(1); };\n");
    assert_eq!(code, 0, "{}", err);
    assert!(!err.contains("macro-arg-count"), "{}", err);
}

#[test]
fn cli_include_case_warns() {
    let dir = tempdir_with(&[("config.cpp", "#include \"INC.hpp\"\n"), ("inc.hpp", "x = 1;\n")]);
    let (code, _, err) = armake2(&["rapify", "config.cpp", "c.bin"], dir.path());
    assert_eq!(code, 0, "{}", err);
    assert!(err.contains("include-case"), "{}", err);
}

#[test]
fn cli_missing_include_folder() {
    let dir = tempdir_with(&[("config.cpp", "x = 1;\n")]);
    let (code, _, err) = armake2(&["rapify", "-i", "nonexist", "config.cpp", "c.bin"], dir.path());
    assert!(code != 0 && code != 101, "code {} {}", code, err);
    assert!(err.contains("nonexist"), "{}", err);
}

#[test]
fn include_not_found_has_line() {
    let dir = tempdir_with(&[("config.cpp", "x = 1;\n#include \"missing.hpp\"\n")]);
    let (code, _, err) = armake2(&["rapify", "config.cpp", "c.bin"], dir.path());
    assert!(code != 0, "{}", err);
    assert!(err.contains("config.cpp:2"), "{}", err);
}

fn tempdir_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (n, c) in files { write(dir.path(), n, c); }
    dir
}
