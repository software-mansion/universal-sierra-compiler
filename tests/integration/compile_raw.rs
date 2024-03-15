use std::fs::File;
use test_case::test_case;
use universal_sierra_compiler::compile_raw;

#[test]
fn wrong_json() {
    let sierra_json = serde_json::json!({
        "wrong": "data"
    });

    let cairo_program = compile_raw(sierra_json);
    assert!(cairo_program.is_err());
}

#[test_case("1_5_0"; "sierra 1.5.0")]
#[test_case("1_4_0"; "sierra 1.4.0")]
fn compile_raw_sierra(sierra_version: &str) {
    let file =
        File::open("tests/data/sierra_raw/sierra_".to_string() + sierra_version + ".json").unwrap();
    let artifact = serde_json::from_reader(file).unwrap();
    let compiled = compile_raw(artifact);

    assert!(compiled.is_ok());
}
