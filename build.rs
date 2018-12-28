fn main() {
    peg::cargo_build("src/config_grammar.rustpeg");
    peg::cargo_build("src/preprocess_grammar.rustpeg");
}
