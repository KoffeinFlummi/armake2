//! Needs DayZ Tools and Proton: run with `ARMAKE2_PROTON_TESTS=1 cargo test --test binarize_proton -- --ignored`.
use std::path::PathBuf;
use std::process::Command;

fn run(args: &[&str], cwd: &std::path::Path) -> (i32, Vec<u8>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_armake2")).args(args).current_dir(cwd).output().unwrap();
    (out.status.code().unwrap_or(-1), out.stdout, String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
#[ignore]
fn proton_binarize_sets_alpha_flags() {
    if std::env::var("ARMAKE2_PROTON_TESTS").as_deref() != Ok("1") {
        eprintln!("skipped: set ARMAKE2_PROTON_TESTS=1");
        return;
    }
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bi/binarize");
    let tmp = tempfile::tempdir().unwrap();
    let src = fixtures.join("t21");
    let (code, _, err) = run(&["build", "--proton-binarize", src.to_str().unwrap(), "out.pbo"], tmp.path());
    assert_eq!(code, 0, "{}", err);
    let (code, model, err) = run(&["cat", "out.pbo", "model.p3d"], tmp.path());
    assert_eq!(code, 0, "{}", err);
    let expected = std::fs::read(fixtures.join("model.expected.p3d")).unwrap();
    assert!(model == expected, "binarized model differs from BI's reference (alpha flags at offset 52: {:02x?} vs {:02x?})",
        &model[52..56.min(model.len())], &expected[52..56]);
}
