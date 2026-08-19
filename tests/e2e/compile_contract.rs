use crate::e2e::{
    assert_cache_layout, cached_casm_file, copy_sierra_fixture, runner, temp_dir_with_sierra_file,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use indoc::indoc;
use std::fs::{self, File};
use std::path::PathBuf;
use test_case::test_case;

fn verify_output_file(output_path: PathBuf) {
    let file = File::open(output_path).unwrap();
    let casm_json = serde_json::from_reader(file).unwrap();

    assert!(serde_json::from_value::<CasmContractClass>(casm_json).is_ok());
}

fn compile_contract_to_file(
    temp_dir: &tempfile::TempDir,
    sierra_file_name: &str,
    output_file_name: &str,
    cache_dir_name: Option<&str>,
) -> Vec<u8> {
    let mut args = vec![
        "compile-contract",
        "--sierra-path",
        sierra_file_name,
        "--output-path",
        output_file_name,
    ];
    if let Some(cache_dir_name) = cache_dir_name {
        args.extend(["--cache-dir", cache_dir_name]);
    }

    runner(args, temp_dir).assert().success();
    fs::read(temp_dir.path().join(output_file_name)).unwrap()
}

#[test]
fn write_to_existing_file() {
    let sierra_file_name = "sierra_1_4_0.json";
    let casm_file_name = "casm.json";
    let args = vec![
        "compile-contract",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
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
    let args = vec!["compile-contract", "--sierra-path", &sierra_file_name];

    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    let output = String::from_utf8(snapbox.assert().success().get_output().stdout.clone()).unwrap();
    assert!(output.contains("bytecode"));
}

#[test]
fn write_to_cache_dir() {
    let sierra_file_name = "sierra_1_4_0.json";
    let casm_file_name = "casm.json";
    let cache_dir_name = "cache";
    let args = vec![
        "compile-contract",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        casm_file_name,
        "--cache-dir",
        cache_dir_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();
    verify_output_file(temp_dir.path().join(casm_file_name));
    assert_cache_layout(&temp_dir.path().join(cache_dir_name), "contract");
}

#[test]
fn second_run_is_served_from_cache() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);
    let cache_dir = temp_dir.path().join(cache_dir_name);

    // First run populates the cache.
    compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "first.json",
        Some(cache_dir_name),
    );

    // Replace the cached payload. A recompile on the second run would overwrite it.
    let cached_payload = r#"{"cached":"served-from-cache"}"#;
    fs::write(cached_casm_file(&cache_dir), cached_payload).unwrap();

    // Second run - unchanged Sierra input, so the fingerprint matches and cached payload is returned.
    compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "second.json",
        Some(cache_dir_name),
    );

    let served = fs::read_to_string(temp_dir.path().join("second.json")).unwrap();
    assert_eq!(served, cached_payload);
}

#[test]
fn malformed_json_cache_entry_is_recompiled() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);

    compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "first.json",
        Some(cache_dir_name),
    );
    fs::write(
        cached_casm_file(&temp_dir.path().join(cache_dir_name)),
        "{not-json",
    )
    .unwrap();

    let recovered = compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "recovered.json",
        Some(cache_dir_name),
    );
    let uncached = compile_contract_to_file(&temp_dir, sierra_file_name, "uncached.json", None);

    assert_eq!(recovered, uncached);
    verify_output_file(temp_dir.path().join("recovered.json"));
    verify_output_file(cached_casm_file(&temp_dir.path().join(cache_dir_name)));
}

#[test]
fn cache_output_matches_uncached() {
    let sierra_file_name = "sierra_1_4_0.json";
    let cache_dir_name = "cache";
    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);

    let uncached = compile_contract_to_file(&temp_dir, sierra_file_name, "uncached.json", None);
    let miss = compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "miss.json",
        Some(cache_dir_name),
    ); // cold cache: compiles and stores
    let hit = compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "hit.json",
        Some(cache_dir_name),
    ); // warm cache: served from the stored entry

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
    let temp_dir = temp_dir_with_sierra_file("sierra_contract", sierra_file_name);

    let first = compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "first.json",
        Some(cache_dir_name),
    );

    copy_sierra_fixture(
        "sierra_contract",
        "sierra_1_5_0.json",
        &temp_dir.path().join(sierra_file_name),
    );

    let cached = compile_contract_to_file(
        &temp_dir,
        sierra_file_name,
        "changed-cached.json",
        Some(cache_dir_name),
    );
    let uncached =
        compile_contract_to_file(&temp_dir, sierra_file_name, "changed-uncached.json", None);

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
    let casm_file_name = "casm.json";
    let args = vec![
        "compile-contract",
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        casm_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("", sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().failure().stderr_eq(indoc! {r"
        [ERROR] Unable to read sierra_program. Make sure it is an array of felts
    "});
}

#[test_case("1_9_0"; "sierra 1.9.0")]
#[test_case("1_8_0"; "sierra 1.8.0")]
#[test_case("1_7_0_trace_hint"; "sierra 1.7.0 with trace hint")]
#[test_case("1_7_0"; "sierra 1.7.0")]
#[test_case("1_6_0"; "sierra 1.6.0")]
#[test_case("1_5_0"; "sierra 1.5.0")]
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
        "--sierra-path",
        &sierra_file_name,
        "--output-path",
        casm_file_name,
    ];

    let temp_dir = temp_dir_with_sierra_file("sierra_contract", &sierra_file_name);
    let snapbox = runner(args, &temp_dir);

    snapbox.assert().success();

    verify_output_file(temp_dir.path().join(casm_file_name));
}
