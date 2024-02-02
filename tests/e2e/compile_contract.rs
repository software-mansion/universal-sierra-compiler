use crate::e2e::{runner, temp_dir_with_sierra_file};
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use indoc::indoc;
use std::fs::File;
use std::path::PathBuf;
use test_case::test_case;

fn verify_output_file(output_path: PathBuf) {
    let file = File::open(output_path).unwrap();
    let casm_json = serde_json::from_reader(file).unwrap();

    assert!(serde_json::from_value::<CasmContractClass>(casm_json).is_ok());
}

#[test]
fn write_to_existing_file() {
    let sierra_file_name = "sierra_1_4_0.json";
    let casm_file_name = "casm.json";
    let args = vec![
        "compile-contract",
        "--sierra-input-path",
        &sierra_file_name,
        "--casm-output-path",
        casm_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);
    let _ = File::create(temp_dir.path().join(casm_file_name)).expect("Unable to create file");

    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();

    verify_output_file(temp_dir.path().join(casm_file_name));
}

#[test]
fn write_to_stdout() {
    let sierra_file_name = "sierra_1_4_0.json";
    let args = vec!["compile-contract", "--sierra-input-path", &sierra_file_name];

    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    let output = String::from_utf8(snapbox.assert().success().get_output().stdout.clone()).unwrap();
    assert!(output.contains("bytecode"));
}

#[test]
fn wrong_json() {
    let sierra_file_name = "wrong_sierra.json";
    let casm_file_name = "casm.json";
    let args = vec![
        "compile-contract",
        "--sierra-input-path",
        &sierra_file_name,
        "--casm-output-path",
        casm_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().failure().stdout_eq(indoc! {r"
        [ERROR] Unable to read sierra_program. Make sure it is an array of felts
    "});
}

#[test_case("1_4_0"; "sierra 1.4.0")]
#[test_case("1_3_0"; "sierra 1.3.0")]
#[test_case("1_2_0"; "sierra 1.2.0")]
#[test_case("1_1_0"; "sierra 1.1.0")]
#[test_case("1_0_0"; "sierra 1.0.0")]
#[test_case("0_1_0"; "sierra 0.1.0")]
fn test_happy_case(sierra_version: &str) {
    let sierra_file_name = "sierra_".to_string() + sierra_version + ".json";
    let casm_file_name = "casm.json";
    let args = vec![
        "compile-contract",
        "--sierra-input-path",
        &sierra_file_name,
        "--casm-output-path",
        casm_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_contract", &sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();

    verify_output_file(temp_dir.path().join(casm_file_name));
}
