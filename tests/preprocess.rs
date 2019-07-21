use std::io::{Write};
use std::fs::{File, create_dir};
use std::path::{PathBuf};

use tempfile::{tempdir};

use armake2::preprocess::*;

#[test]
fn test_preprocess_macros() {
    let input = String::from("\
#define VERSIONAR {3,5, 0, 0}
#define FOO(x  , y ) #x z x_y x##_##y
#define QUOTE(x) #x
#define DOUBLES(x,y) x##_##y
    #define ADDON DOUBLES(ace, frag)

class CfgPatches {
    class ADDON{
        units[] = { };
        weapons[] = {};
        requiredVersion = 1.56;
        requiredAddons[] = {\"ace_common\"};
        author[] = {\"Nou\"}   ;
        version = QUOTE(3.5.0.0) ;versionStr=\"3.5.0.0\";
        versionAr [] = VERSIONAR;
    };
};");

    let (output, _) = preprocess(input, None, &Vec::new()).unwrap();

    assert_eq!("\
class CfgPatches {
class ace_frag{
units[] = { };
weapons[] = {};
requiredVersion = 1.56;
requiredAddons[] = {\"ace_common\"};
author[] = {\"Nou\"}   ;
version = \"3.5.0.0\" ;versionStr=\"3.5.0.0\";
versionAr [] = {3,5, 0, 0};
};
};", output.trim());
}

#[test]
fn test_preprocess_ifdef() {
    let input = String::from("\
#define foo bar
#define foobar whatever
#undef foobar

#ifdef foo
    #ifdef foobar
        def = 5678;
    #endif
    abc = 1234;
#else
    abc = 4321;
#endif
");

    let (output, _) = preprocess(input, None, &Vec::new()).unwrap();

    assert_eq!("abc = 1234;", output.trim());
}

#[test]
fn test_preprocess_include() {
    let input = String::from("\
#include \"\\x\\cba\\addons\\whatever\\include.h\"
DOUBLES(foo,bar)\n");

    let include = String::from("\
#define DOUBLES(x,y) x##_##y
bar_foo\n");

    let prefix = String::from("\
\\x\\cba\\addons\\whatever\n");

    let includedir = tempdir().unwrap();

    let addondir = includedir.path().join("whatever");
    create_dir(&addondir).unwrap();

    File::create(addondir.join("include.h")).unwrap().write_all(include.as_bytes()).unwrap();
    File::create(addondir.join("$PBOPREFIX$")).unwrap().write_all(prefix.as_bytes()).unwrap();

    let includepath = PathBuf::from(addondir.join("include.h")).canonicalize().unwrap();

    let includefolders = vec![PathBuf::from(includedir.path())];
    let (output, info) = preprocess(input, Some(PathBuf::from("myfile")), &includefolders).unwrap();

    assert_eq!("bar_foo\n\nfoo_bar", output.trim());
    assert_eq!((2, Some(includepath)), info.line_origins[0]);
    assert_eq!((2, Some(PathBuf::from("myfile"))), info.line_origins[2]);
}

#[test]
fn test_proprocess_bom() {
    let input = String::from_utf8(vec![0xef,0xbb,0xbf]).unwrap() + "blub";
    let (output, _) = preprocess(input, None, &Vec::new()).unwrap();

    assert_eq!("blub", output.trim());
}

#[test]
fn test_preprocess_lineorigins() {
    let input = String::from("\
#define TEST \"test\"/* foo

bar */
class test\\
{
    foo[] = {1,2,3, \\
    4};
    bar[] = {1,2,3, \\
    4}jashdlasd;
};\n");

    let (_, info) = preprocess(input, None, &Vec::new()).unwrap();
    assert_eq!(5, info.line_origins.len());
    assert_eq!(8, info.line_origins[2].0);
}
