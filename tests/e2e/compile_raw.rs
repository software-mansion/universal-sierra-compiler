use crate::e2e::{runner, temp_dir_with_sierra_file};
use indoc::indoc;

#[test]
fn wrong_json() {
    let sierra_file_name = "wrong_sierra.json";
    let args = vec!["compile-raw", "--sierra-input-path", &sierra_file_name];

    let temp_dir = temp_dir_with_sierra_file("", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().failure().stdout_eq(indoc! {r"
        [ERROR] Unable to deserialize Sierra. Make sure it is in a correct format
    "});
}

#[test]
fn test_happy_case() {
    let sierra_file_name = "sierra_1_4_0.json";
    let args = vec!["compile-raw", "--sierra-input-path", &sierra_file_name];

    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    let output = String::from_utf8(snapbox.assert().success().get_output().stdout.clone()).unwrap();
    println!("{output}");
}
