use crate::e2e::{runner, temp_dir_with_sierra_file};
use cairo_lang_casm::hints::Hint;
use indoc::indoc;
use num_bigint::BigInt;
use serde_json::Value;
use std::fs::File;
use std::path::PathBuf;
use test_case::test_case;

fn verify_output_file(output_path: PathBuf) {
    let file = File::open(output_path).unwrap();
    let cairo_program_json: Value = serde_json::from_reader(file).unwrap();

    let bytecode = serde_json::from_value::<Vec<BigInt>>(
        cairo_program_json["assembled_cairo_program"]["bytecode"].clone(),
    );
    let hints = serde_json::from_value::<Vec<(usize, Vec<Hint>)>>(
        cairo_program_json["assembled_cairo_program"]["hints"].clone(),
    );
    let debug_info =
        serde_json::from_value::<Vec<(usize, usize)>>(cairo_program_json["debug_info"].clone());

    assert!(bytecode.is_ok());
    assert!(hints.is_ok());
    assert!(debug_info.is_ok());
}

#[test]
fn write_to_existing_file() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cairo_program_file_name = "cairo_program.json";
    let args = vec![
        "compile-raw",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        cairo_program_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);
    let _ =
        File::create(temp_dir.path().join(cairo_program_file_name)).expect("Unable to create file");

    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();

    verify_output_file(temp_dir.path().join(cairo_program_file_name));
}

#[test]
fn write_to_stdout() {
    let sierra_file_name = "sierra_1_4_0.json";
    let args = vec!["compile-raw", "--sierra-path", &sierra_file_name];

    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    let output = String::from_utf8(snapbox.assert().success().get_output().stdout.clone()).unwrap();
    assert!(output.contains("assembled_cairo_program"));
    assert!(output.contains("debug_info"));
}

#[test]
fn wrong_json() {
    let sierra_file_name = "wrong_sierra.json";
    let cairo_program_file_name = "casm.json";
    let args = vec![
        "compile-raw",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        cairo_program_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().failure().stderr_eq(indoc! {r"
        [ERROR] Unable to deserialize Sierra program. Make sure it is in a correct format
    "});
}

#[test_case("1_5_0"; "sierra 1.5.0")]
#[test_case("1_4_0"; "sierra 1.4.0")]
fn test_happy_case(sierra_version: &str) {
    let sierra_file_name = "sierra_".to_string() + sierra_version + ".json";
    let cairo_program_file_name = "casm.json";
    let args = vec![
        "compile-raw",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        cairo_program_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_raw", &sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();

    verify_output_file(temp_dir.path().join(cairo_program_file_name));
}
