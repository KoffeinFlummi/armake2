use criterion::{Criterion, criterion_group, criterion_main};

use armake2::preprocess::*;

fn bench_preprocess_short(c: &mut Criterion) {
    c.bench_function("preprocess", |b| b.iter(|| {
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

        preprocess(input, None, &Vec::new()).unwrap();
    }));
}

criterion_group!(benches, bench_preprocess_short);
criterion_main!(benches);
