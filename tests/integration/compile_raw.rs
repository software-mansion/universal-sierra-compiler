use cairo_lang_sierra::program::Program;
use std::fs::File;
use test_case::test_case;
use universal_sierra_compiler::compile_raw;

#[test_case("1_9_0"; "sierra 1.9.0")]
#[test_case("1_8_0"; "sierra 1.8.0")]
#[test_case("1_7_0_trace_hint"; "sierra 1.7.0 with trace hint")]
#[test_case("1_7_0"; "sierra 1.7.0")]
#[test_case("1_6_0"; "sierra 1.6.0")]
#[test_case("1_5_0"; "sierra 1.5.0")]
#[test_case("1_4_0"; "sierra 1.4.0")]
fn compile_raw_sierra(sierra_version: &str) {
    let file =
        File::open("tests/data/sierra_raw/sierra_".to_string() + sierra_version + ".json").unwrap();
    let artifact: Program = serde_json::from_reader(file).unwrap();
    let compiled = compile_raw(&artifact).unwrap();
    let function_costs = compiled["function_costs"].as_object().unwrap();

    assert_eq!(function_costs.len(), artifact.funcs.len());
    assert!(function_costs
        .values()
        .any(|costs| !costs.as_object().unwrap().is_empty()));
    assert!(artifact.funcs.iter().all(|function| {
        function_costs
            .get(&function.entry_point.0.to_string())
            .is_some_and(serde_json::Value::is_object)
    }));
}
