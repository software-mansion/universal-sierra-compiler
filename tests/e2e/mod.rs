use snapbox::cmd::{cargo_bin, Command};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

mod compile_contract;
mod compile_raw;

#[must_use]
fn runner(args: Vec<&str>, temp_dir: &TempDir) -> Command {
    Command::new(cargo_bin!("universal-sierra-compiler"))
        .current_dir(temp_dir.path())
        .args(args)
}

#[must_use]
fn temp_dir_with_sierra_file(dir_name: &str, file_name: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Unable to create a temporary directory");

    let src_dir = PathBuf::from("tests/data");

    fs_extra::file::copy(
        src_dir.join(dir_name).join(file_name),
        temp_dir.path().join(file_name),
        &fs_extra::file::CopyOptions::new().overwrite(true),
    )
    .unwrap_or_else(|_| panic!("Unable to copy {dir_name}/{file_name}"));

    temp_dir
}

#[must_use]
fn cache_files(path: &Path) -> Vec<PathBuf> {
    let mut files = vec![];
    collect_cache_files(path, &mut files);
    files.sort();
    files
}

fn assert_cache_files_written(path: &Path) {
    let files = cache_files(path);

    assert!(
        !files.is_empty(),
        "expected cache files to be written under {}",
        path.display()
    );
    assert!(
        files.iter().any(|path| path
            .file_name()
            .is_some_and(|file_name| file_name == "casm.json")),
        "expected cached CASM to be written under {}",
        path.display()
    );
    assert!(
        files.iter().any(|path| path
            .file_name()
            .is_some_and(|file_name| file_name == "fingerprint")),
        "expected fingerprint to be written under {}",
        path.display()
    );
}

/// Returns the single cached `casm.json` written under `cache_dir`, failing if there is not exactly
/// one. Used by cache-hit tests to tamper with the cached payload and prove it is served verbatim.
#[must_use]
fn cached_casm_file(cache_dir: &Path) -> PathBuf {
    let casm_files: Vec<PathBuf> = cache_files(cache_dir)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|file_name| file_name == "casm.json")
        })
        .collect();

    assert_eq!(
        casm_files.len(),
        1,
        "expected exactly one cached casm.json under {}, found {casm_files:?}",
        cache_dir.display()
    );

    casm_files.into_iter().next().unwrap()
}

fn collect_cache_files(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries {
        let entry = entry
            .unwrap_or_else(|error| panic!("failed to read entry in {}: {error}", path.display()));
        let path = entry.path();

        if path.is_dir() {
            collect_cache_files(&path, files);
        } else {
            files.push(path);
        }
    }
}
