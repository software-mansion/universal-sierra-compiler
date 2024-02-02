use std::fs::File;
use universal_sierra_compiler::commands;

#[test]
fn wrong_json() {
    let sierra_json = serde_json::json!({
        "wrong": "data"
    });

    let cairo_program = commands::compile_raw::compile(sierra_json);
    assert!(cairo_program.is_err());
}

#[test]
fn compile_raw_sierra() {
    let file = File::open("tests/data/sierra_raw/sierra_1_4_0.json").unwrap();
    let artifact = serde_json::from_reader(file).unwrap();
    let compiled = commands::compile_raw::compile(artifact);

    assert!(compiled.is_ok());
}
