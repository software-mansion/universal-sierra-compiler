//! Optional persistent cache for CASM compiled from Sierra.
//!
//! A slot picks the cache file for a Sierra artifact path. A fingerprint tells whether that file
//! still matches the current input.

use anyhow::Result;
use serde_json::Value;
use std::fs::File;
use std::path::Path;

mod layout;
mod store;

pub use layout::{CasmCompilationFingerprint, SierraKind};

use layout::{cache_entry_path, CasmCacheSlot};
use store::{read_cache_entry, write_cache_entry};

/// The result of a (possibly cached) CASM compilation, ready to be emitted.
#[derive(Debug)]
pub enum CasmCompilationOutput {
    /// An open handle to a cache file that matched the current input. We keep it open so we can
    /// read it once, without reopening it later (which could race with another process).
    CachedFile(File),
    /// Freshly compiled CASM held in memory - used when the cache is disabled, on a cache miss, or
    /// when writing the entry failed.
    Json(Value),
}

/// Returns the CASM for `sierra_path`, serving it from `cache_dir` when a valid entry exists.
///
/// With no `cache_dir` the cache is skipped and `compile` always runs. Otherwise a hit (matching
/// `fingerprint`) is returned without compiling; on a miss `compile` runs and its output is saved.
/// Saving is best-effort - a failed write is only logged, not returned as an error.
pub fn compile_with_cache(
    cache_dir: Option<&Path>,
    sierra_path: &Path,
    fingerprint: &CasmCompilationFingerprint,
    compile: impl FnOnce() -> Result<Value>,
) -> Result<CasmCompilationOutput> {
    let Some(cache_dir) = cache_dir else {
        return compile().map(CasmCompilationOutput::Json);
    };

    let slot = CasmCacheSlot::new(fingerprint.sierra_kind(), sierra_path)?;
    let cache_entry_path = cache_entry_path(cache_dir, &slot);
    if let Some(output) = read_cache_entry(&cache_entry_path, fingerprint) {
        return Ok(output);
    }

    let output = compile()?;

    if let Err(error) = write_cache_entry(&cache_entry_path, fingerprint, &output) {
        tracing::debug!(
            path = %cache_entry_path.display(),
            %error,
            "failed to write CASM cache entry"
        );
    }

    Ok(CasmCompilationOutput::Json(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::layout::{cache_entry_path, CasmCacheSlot};
    use crate::cache::store::{
        fingerprint_path, read_cache_entry, write_cache_entry, write_text_file_atomically,
    };
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn output_json(output: CasmCompilationOutput) -> Value {
        match output {
            CasmCompilationOutput::Json(value) => value,
            CasmCompilationOutput::CachedFile(file) => serde_json::from_reader(file).unwrap(),
        }
    }

    fn source_path(cache_root: &Path, file_name: &str) -> PathBuf {
        let path = cache_root.join(file_name);
        fs::write(&path, "{}").unwrap();
        path
    }

    #[test]
    fn reuses_cached_output() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "same"})).unwrap();

        let first = compile_with_cache(Some(temp.path()), &source_path, &fingerprint, || {
            Ok(json!({"compiled": 1}))
        })
        .unwrap();

        let second = compile_with_cache(Some(temp.path()), &source_path, &fingerprint, || {
            panic!("matching cache entry should avoid recompilation")
        })
        .unwrap();

        assert_eq!(output_json(first), json!({"compiled": 1}));
        assert_eq!(output_json(second), json!({"compiled": 1}));
    }

    #[test]
    fn separates_cache_entries_by_sierra_kind() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let input = json!({"program": "same"});
        let raw = CasmCompilationFingerprint::new(SierraKind::Raw, &input).unwrap();
        let contract = CasmCompilationFingerprint::new(SierraKind::Contract, &input).unwrap();

        let raw_slot = CasmCacheSlot::new(raw.sierra_kind(), &source_path).unwrap();
        let contract_slot = CasmCacheSlot::new(contract.sierra_kind(), &source_path).unwrap();

        assert_ne!(
            cache_entry_path(temp.path(), &raw_slot),
            cache_entry_path(temp.path(), &contract_slot)
        );
    }

    #[test]
    fn uses_single_cache_entry_for_same_artifact_path() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");

        // Two different inputs for the same artifact path produce different fingerprints but share
        // the same cache slot, so the later compilation overwrites the earlier one.
        let first =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "first"})).unwrap();
        let second =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "second"}))
                .unwrap();
        let slot = CasmCacheSlot::new(SierraKind::Raw, &source_path).unwrap();
        let path = cache_entry_path(temp.path(), &slot);

        let first_output = compile_with_cache(Some(temp.path()), &source_path, &first, || {
            Ok(json!({"compiled": "first"}))
        })
        .unwrap();
        let first_output_json = output_json(first_output);
        let second_output = compile_with_cache(Some(temp.path()), &source_path, &second, || {
            Ok(json!({"compiled": "second"}))
        })
        .unwrap();
        let second_output_json = output_json(second_output);

        assert_eq!(first_output_json, json!({"compiled": "first"}));
        assert_eq!(second_output_json, json!({"compiled": "second"}));
        assert_eq!(cache_entry_path(temp.path(), &slot), path);
        assert!(read_cache_entry(&path, &first).is_none());
        assert_eq!(
            output_json(read_cache_entry(&path, &second).unwrap()),
            second_output_json
        );
    }

    #[test]
    fn recompiles_and_replaces_cache_entry_with_missing_casm_file() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "same"})).unwrap();
        let slot = CasmCacheSlot::new(SierraKind::Raw, &source_path).unwrap();
        let path = cache_entry_path(temp.path(), &slot);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        write_text_file_atomically(&fingerprint_path(&path), fingerprint.digest()).unwrap();

        let output = compile_with_cache(Some(temp.path()), &source_path, &fingerprint, || {
            Ok(json!({"compiled": 2}))
        })
        .unwrap();
        let cached = read_cache_entry(&path, &fingerprint).unwrap();

        assert_eq!(output_json(output), json!({"compiled": 2}));
        assert_eq!(output_json(cached), json!({"compiled": 2}));
    }

    #[test]
    fn ignores_cache_entry_with_mismatched_fingerprint_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "same"})).unwrap();
        let stale_fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "other"})).unwrap();
        let slot = CasmCacheSlot::new(SierraKind::Raw, &source_path).unwrap();
        let path = cache_entry_path(temp.path(), &slot);

        write_cache_entry(&path, &stale_fingerprint, &json!({"compiled": "stale"})).unwrap();

        let output = compile_with_cache(Some(temp.path()), &source_path, &fingerprint, || {
            Ok(json!({"compiled": "fresh"}))
        })
        .unwrap();

        let compiled_output_json = output_json(output);

        assert_eq!(compiled_output_json, json!({"compiled": "fresh"}));
        assert_eq!(
            output_json(read_cache_entry(&path, &fingerprint).unwrap()),
            compiled_output_json
        );
    }

    #[test]
    fn no_cache_dir_compiles_without_writing() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "same"})).unwrap();
        let output = compile_with_cache(None, &source_path, &fingerprint, || {
            Ok(json!({"compiled": 3}))
        })
        .unwrap();

        assert_eq!(output_json(output), json!({"compiled": 3}));
    }

    #[test]
    fn entry_without_fingerprint_is_a_miss() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "same"})).unwrap();
        let slot = CasmCacheSlot::new(SierraKind::Raw, &source_path).unwrap();
        let path = cache_entry_path(temp.path(), &slot);

        // Write casm.json but not the fingerprint file (the commit marker) - the entry is incomplete.
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, json!({"compiled": "orphan"}).to_string()).unwrap();

        assert!(read_cache_entry(&path, &fingerprint).is_none());
    }

    #[test]
    fn compiles_and_returns_output_when_cache_write_fails() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = source_path(temp.path(), "program.sierra.json");
        let fingerprint =
            CasmCompilationFingerprint::new(SierraKind::Raw, &json!({"program": "same"})).unwrap();

        // Point the cache at a regular file, so creating any directory beneath it fails.
        let cache_dir = temp.path().join("not-a-dir");
        fs::write(&cache_dir, "x").unwrap();

        let output = compile_with_cache(Some(&cache_dir), &source_path, &fingerprint, || {
            Ok(json!({"compiled": 4}))
        })
        .unwrap();

        assert_eq!(output_json(output), json!({"compiled": 4}));
    }
}
