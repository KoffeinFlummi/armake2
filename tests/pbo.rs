use std::fs::{self, File};
use std::path::{Path, PathBuf};

use armake2::pbo::PBO;
use armake2::sign::{cmd_sign, cmd_verify, BISignVersion};

fn run_armake2(args: &[&str], cwd: &Path) -> (i32, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_armake2")).args(args).current_dir(cwd).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn sign_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bi/sign")
}

/// Signs `<pbo>` with the fixture key; returns the produced bisign bytes.
fn sign_fixture(pbo: &str) -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    let sig = tmp.path().join("out.bisign");
    cmd_sign(sign_dir().join("armk.biprivatekey"), sign_dir().join(pbo), Some(sig.clone()), BISignVersion::V3, true).unwrap();
    fs::read(sig).unwrap()
}

#[test]
fn sign_matches_bi_utf8() {
    assert_eq!(sign_fixture("testsig.pbo"), fs::read(sign_dir().join("testsig.pbo.armk.bisign")).unwrap());
}

#[test]
fn sign_matches_bi_non_utf8() {
    assert_eq!(sign_fixture("nonutf8.pbo"), fs::read(sign_dir().join("nonutf8.pbo.armk.bisign")).unwrap());
}

#[test]
fn verify_bi_signature_non_utf8() {
    cmd_verify(sign_dir().join("armk.bikey"), sign_dir().join("nonutf8.pbo"), Some(sign_dir().join("nonutf8.pbo.armk.bisign"))).unwrap();
}

#[test]
fn read_non_utf8_name() {
    let pbo = PBO::read(&mut File::open(sign_dir().join("nonutf8.pbo")).unwrap()).unwrap();
    assert!(pbo.files.contains_key("data\\a\u{2014}b.txt"), "{:?}", pbo.files.keys().collect::<Vec<_>>());
}

#[test]
fn roundtrip_non_utf8_name() {
    let original = fs::read(sign_dir().join("nonutf8.pbo")).unwrap();
    let pbo = PBO::read(&mut &original[..]).unwrap();
    let mut out = Vec::new();
    pbo.write(&mut out).unwrap();
    assert_eq!(out, original);
}

#[test]
fn cli_unpack_non_utf8_name() {
    let tmp = tempfile::tempdir().unwrap();
    let pbo = sign_dir().join("nonutf8.pbo");
    let (code, err) = run_armake2(&["unpack", pbo.to_str().unwrap(), "out"], tmp.path());
    assert_eq!(code, 0, "{}", err);
    assert!(err.contains("non-utf8-filename"), "{}", err);
    assert!(Path::new(&tmp.path().join("out/data/a\u{2014}b.txt")).exists());
}
