use std::fs::File;
use test_case::test_case;
use universal_sierra_compiler::compile_contract;

#[test]
fn wrong_json() {
    let sierra_json = serde_json::json!({
        "wrong": "data"
    });

    let casm_class = compile_contract(sierra_json);
    assert!(casm_class.is_err());
}

#[test_case("1_6_0"; "sierra 1.6.0")]
#[test_case("1_5_0"; "sierra 1.5.0")]
#[test_case("1_4_0"; "sierra 1.4.0")]
#[test_case("1_3_0"; "sierra 1.3.0")]
#[test_case("1_2_0"; "sierra 1.2.0")]
#[test_case("1_1_0"; "sierra 1.1.0")]
#[test_case("1_0_0"; "sierra 1.0.0")]
#[test_case("0_1_0"; "sierra 0.1.0")]
fn compile_sierra(sierra_version: &str) {
    let file =
        File::open("tests/data/sierra_contract/sierra_".to_string() + sierra_version + ".json")
            .unwrap();
    let sierra_json = serde_json::from_reader(file).unwrap();

    let casm_class = compile_contract(sierra_json);
    assert!(casm_class.is_ok());
}
