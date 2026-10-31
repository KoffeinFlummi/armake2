//! Golden tests against output produced by BI's CfgConvert (see tests/fixtures/bi/README.md).
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;

use armake2::config::Config;

fn golden(name: &str) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bi/cfgconvert");
    let src = dir.join(format!("{}.cpp", name));
    let expected = std::fs::read_to_string(dir.join(format!("{}.expected.cpp", name))).unwrap();

    let mut input = File::open(&src).unwrap();
    let config = Config::read(&mut input, Some(src.clone()), &[dir.clone()]).unwrap();
    let rapified = config.to_cursor().unwrap();
    let mut cursor = Cursor::new(rapified.into_inner());
    let decoded = Config::read_rapified(&mut cursor).unwrap();
    assert_eq!(decoded.to_string().unwrap(), expected, "golden {}", name);
}

#[test] fn golden_g() { golden("g"); }
#[test] fn golden_w1() { golden("w1"); }
#[test] fn golden_us() { golden("us"); }
#[test] fn golden_c51() { golden("c51"); }
#[test] fn golden_rap() { golden("rap"); }
#[test] fn golden_ma() { golden("ma"); }
#[test] fn golden_m28() { golden("m28"); }
#[test] #[ignore] fn golden_case() { golden("case"); }
#[test] #[ignore] fn golden_ev() { golden("ev"); }
#[test] #[ignore] fn golden_ex() { golden("ex"); }
#[test] #[ignore] fn golden_e23() { golden("e23"); }
#[test] #[ignore] fn golden_mac() { golden("mac"); }
