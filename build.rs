extern crate peg;

fn main() {
    peg::cargo_build("src/armake/config_grammar.rustpeg");
    peg::cargo_build("src/armake/preprocess_grammar.rustpeg");
}
