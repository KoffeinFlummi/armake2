fn main() {
    peg::cargo_build("src/grammars/config_grammar.rustpeg");
    peg::cargo_build("src/grammars/preprocess_grammar.rustpeg");
}
