use crate::e2e::{
    assert_cache_layout, cached_casm_file, copy_sierra_fixture, runner, temp_dir_with_sierra_file,
};
use cairo_lang_casm::hints::Hint;
use indoc::indoc;
use num_bigint::BigInt;
use serde_json::Value;
use std::fs::{self, File};
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
fn write_to_cache_dir() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cairo_program_file_name = "cairo_program.json";
    let cache_dir_name = "cache";
    let args = vec![
        "compile-raw",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        cairo_program_file_name,
        "--cache-dir",
        cache_dir_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();
    verify_output_file(temp_dir.path().join(cairo_program_file_name));
    assert_cache_layout(&temp_dir.path().join(cache_dir_name), "raw");
}

#[test]
fn cache_hit_is_served_from_cache() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);
    let cache_dir = temp_dir.path().join(cache_dir_name);

    // Runs a cached compilation (to `output`, or to stdout when `None`) and returns its stdout.
    let run = |output: Option<&str>| -> Vec<u8> {
        let mut args = vec!["compile-raw", "--sierra-path", sierra_file_name];
        if let Some(output) = output {
            args.extend(["--output-path", output]);
        }
        args.extend(["--cache-dir", cache_dir_name]);
        runner(args, &temp_dir)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };

    // First run populates the cache.
    run(Some("first.json"));

    // Replace the cached payload. Any later run that recompiled instead of reading the cache would
    // overwrite it, so the assertions below double as proof of a cache hit.
    let cached_payload = r#"{"cached":"served-from-cache"}"#;
    fs::write(cached_casm_file(&cache_dir), cached_payload).unwrap();

    // A cache hit written to a file returns the cached payload verbatim.
    run(Some("second.json"));
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("second.json")).unwrap(),
        cached_payload
    );

    // A cache hit written to stdout (no --output-path) does too.
    let stdout = run(None);
    assert_eq!(
        String::from_utf8(stdout).unwrap().trim_end(),
        cached_payload
    );
}

#[test]
fn malformed_json_cache_entry_is_recompiled() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);

    let run = |output: &str, cache: bool| {
        let mut args = vec![
            "compile-raw",
            "--sierra-path",
            sierra_file_name,
            "--output-path",
            output,
        ];
        if cache {
            args.extend(["--cache-dir", cache_dir_name]);
        }
        runner(args, &temp_dir).assert().success();
        fs::read(temp_dir.path().join(output)).unwrap()
    };

    run("first.json", true);
    fs::write(
        cached_casm_file(&temp_dir.path().join(cache_dir_name)),
        "{not-json",
    )
    .unwrap();

    let recovered = run("recovered.json", true);
    let uncached = run("uncached.json", false);

    assert_eq!(recovered, uncached);
    verify_output_file(temp_dir.path().join("recovered.json"));
    verify_output_file(cached_casm_file(&temp_dir.path().join(cache_dir_name)));
}

#[test]
fn cache_output_matches_uncached() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);

    let run = |output: &str, cache: bool| {
        let mut args = vec![
            "compile-raw",
            "--sierra-path",
            sierra_file_name,
            "--output-path",
            output,
        ];
        if cache {
            args.extend(["--cache-dir", cache_dir_name]);
        }
        runner(args, &temp_dir).assert().success();
        fs::read(temp_dir.path().join(output)).unwrap()
    };

    let uncached = run("uncached.json", false);
    let miss = run("miss.json", true); // cold cache: compiles and stores
    let hit = run("hit.json", true); // warm cache: served from the stored entry

    assert_eq!(
        uncached, miss,
        "cache miss output must match uncached output"
    );
    assert_eq!(uncached, hit, "cache hit output must match uncached output");
}

#[test]
fn changed_sierra_invalidates_cache() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_raw", sierra_file_name);

    let run = |output: &str, cache: bool| {
        let mut args = vec![
            "compile-raw",
            "--sierra-path",
            sierra_file_name,
            "--output-path",
            output,
        ];
        if cache {
            args.extend(["--cache-dir", cache_dir_name]);
        }
        runner(args, &temp_dir).assert().success();
        fs::read(temp_dir.path().join(output)).unwrap()
    };

    let first = run("first.json", true);

    copy_sierra_fixture(
        "sierra_raw",
        "sierra_1_5_0.json",
        &temp_dir.path().join(sierra_file_name),
    );

    let cached = run("changed-cached.json", true);
    let uncached = run("changed-uncached.json", false);

    assert_ne!(first, cached, "changed Sierra should produce new CASM");
    assert_eq!(
        cached, uncached,
        "changed Sierra should invalidate the cache"
    );
    assert!(cached_casm_file(&temp_dir.path().join(cache_dir_name)).is_file());
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

#[test_case("1_9_0"; "sierra 1.9.0")]
#[test_case("1_8_0"; "sierra 1.8.0")]
#[test_case("1_7_0_trace_hint"; "sierra 1.7.0 with trace hint")]
#[test_case("1_7_0"; "sierra 1.7.0")]
#[test_case("1_6_0"; "sierra 1.6.0")]
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
