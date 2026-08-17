//! Optional persistent cache for CASM compiled from Sierra.

use anyhow::Result;
use entry::CasmCacheEntry;
use serde_json::Value;
use std::path::Path;

mod entry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SierraKind {
    Raw,
    Contract,
}

impl SierraKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Contract => "contract",
        }
    }
}

/// Returns the CASM for `sierra_path`, serving it from `cache_dir` when a valid entry exists.
/// With no `cache_dir` provided or a cache miss, the `compile` closure is called.
pub fn compile_with_cache(
    sierra_path: &Path,
    sierra_kind: SierraKind,
    cache_dir: Option<&Path>,
    compile: impl FnOnce() -> Result<Value>,
) -> Result<Value> {
    let Some(cache_dir) = cache_dir else {
        return compile();
    };

    let entry = CasmCacheEntry::new(cache_dir, sierra_path, sierra_kind)?;
    if let Some(output) = entry.load() {
        return Ok(output);
    }

    let output = compile()?;

    if let Err(error) = entry.store(&output) {
        tracing::warn!(
            path = %entry.casm_path().display(),
            %error,
            "failed to write CASM cache entry"
        );
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn write_source(cache_root: &Path, file_name: &str, input: &Value) -> PathBuf {
        let path = cache_root.join(file_name);
        fs::write(&path, serde_json::to_vec(input).unwrap()).unwrap();
        path
    }

    fn count_files_named(path: &Path, file_name: &str) -> usize {
        let Ok(entries) = fs::read_dir(path) else {
            return 0;
        };

        entries
            .map(|entry| entry.unwrap().path())
            .map(|path| {
                if path.is_dir() {
                    count_files_named(&path, file_name)
                } else {
                    usize::from(path.file_name().is_some_and(|name| name == file_name))
                }
            })
            .sum()
    }

    #[test]
    fn reuses_cached_output() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_source(
            temp.path(),
            "program.sierra.json",
            &json!({"program": "same"}),
        );

        let first = compile_with_cache(&source_path, SierraKind::Raw, Some(temp.path()), || {
            Ok(json!({"compiled": 1}))
        })
        .unwrap();

        let second = compile_with_cache(&source_path, SierraKind::Raw, Some(temp.path()), || {
            panic!("matching cache entry should avoid recompilation")
        })
        .unwrap();

        assert_eq!(first, json!({"compiled": 1}));
        assert_eq!(second, first);
    }

    #[test]
    fn separates_entries_by_kind() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_source(
            temp.path(),
            "program.sierra.json",
            &json!({"program": "same"}),
        );

        let raw = compile_with_cache(&source_path, SierraKind::Raw, Some(temp.path()), || {
            Ok(json!({"compiled": "raw"}))
        })
        .unwrap();
        let contract = compile_with_cache(
            &source_path,
            SierraKind::Contract,
            Some(temp.path()),
            || Ok(json!({"compiled": "contract"})),
        )
        .unwrap();

        assert_ne!(raw, contract);
        assert_eq!(count_files_named(temp.path(), "casm.json"), 2);
    }

    #[test]
    fn changed_input_replaces_entry() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_source(
            temp.path(),
            "program.sierra.json",
            &json!({"program": "first"}),
        );

        let first = compile_with_cache(&source_path, SierraKind::Raw, Some(temp.path()), || {
            Ok(json!({"compiled": "first"}))
        })
        .unwrap();

        fs::write(
            &source_path,
            serde_json::to_vec(&json!({"program": "second"})).unwrap(),
        )
        .unwrap();
        let second = compile_with_cache(&source_path, SierraKind::Raw, Some(temp.path()), || {
            Ok(json!({"compiled": "second"}))
        })
        .unwrap();
        let cached = compile_with_cache(&source_path, SierraKind::Raw, Some(temp.path()), || {
            panic!("updated cache entry should avoid recompilation")
        })
        .unwrap();

        assert_ne!(first, second);
        assert_eq!(cached, second);
        assert_eq!(count_files_named(temp.path(), "casm.json"), 1);
    }

    #[test]
    fn without_cache_compiles_directly() {
        let temp = tempfile::tempdir().unwrap();
        let missing_source_path = temp.path().join("missing.sierra.json");

        let output = compile_with_cache(&missing_source_path, SierraKind::Raw, None, || {
            Ok(json!({"compiled": 3}))
        })
        .unwrap();

        assert_eq!(output, json!({"compiled": 3}));
    }

    #[test]
    fn cache_write_failure_does_not_fail_compilation() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_source(
            temp.path(),
            "program.sierra.json",
            &json!({"program": "same"}),
        );

        // Point the cache at a regular file, so creating any directory beneath it fails.
        let cache_dir = temp.path().join("not-a-dir");
        fs::write(&cache_dir, "x").unwrap();

        let output = compile_with_cache(&source_path, SierraKind::Raw, Some(&cache_dir), || {
            Ok(json!({"compiled": 4}))
        })
        .unwrap();

        assert_eq!(output, json!({"compiled": 4}));
    }
}
