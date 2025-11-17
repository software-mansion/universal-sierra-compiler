use cairo_lang_sierra::program::Program;
use std::fs::File;
use test_case::test_case;
use universal_sierra_compiler::compile_raw;

#[test_case("1_7_0_trace_hint"; "sierra 1.7.0 with trace hint")]
#[test_case("1_7_0"; "sierra 1.7.0")]
#[test_case("1_6_0"; "sierra 1.6.0")]
#[test_case("1_5_0"; "sierra 1.5.0")]
#[test_case("1_4_0"; "sierra 1.4.0")]
fn compile_raw_sierra(sierra_version: &str) {
    let file =
        File::open("tests/data/sierra_raw/sierra_".to_string() + sierra_version + ".json").unwrap();
    let artifact: Program = serde_json::from_reader(file).unwrap();
    let compiled = compile_raw(&artifact);

    assert!(compiled.is_ok());
}
