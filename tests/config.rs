use std::io::{Cursor, Seek, SeekFrom};

use armake2::config::*;

#[test]
fn config_read() {
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
    let mut cursor = Cursor::new(input);

    let mut config = Config::read(&mut cursor, None, &Vec::new()).unwrap();

    let mut rapified = config.to_cursor().unwrap();
    rapified.seek(SeekFrom::Start(0)).unwrap();

    config = Config::read_rapified(&mut rapified).unwrap();

    let output = config.to_string().unwrap();

    assert_eq!("class CfgPatches {
    class ace_frag {
        units[] = {};
        weapons[] = {};
        requiredVersion = 1.56;
        requiredAddons[] = {\"ace_common\"};
        author[] = {\"Nou\"};
        version = \"3.5.0.0\";
        versionStr = \"3.5.0.0\";
        versionAr[] = {3, 5, 0, 0};
    };
};", output.trim());
}
