//! Reading and writing cache entries on disk.
//!
//! Each entry is two files in the same directory: `casm.json` (the compiled output) and
//! `fingerprint` (the digest of the input it was built from). Both are written atomically, and
//! `casm.json` is always written first. So an entry counts as valid only when its `fingerprint` is
//! present, matches, and its `casm.json` can be read - writing the fingerprint last is what marks
//! the entry as complete.

use super::layout::CasmCompilationFingerprint;
use super::CasmCompilationOutput;
use serde_json::Value;
use std::fs;
use std::io::{self, BufWriter, Write as _};
use std::path::{Path, PathBuf};
use tempfile::Builder;

/// The `fingerprint` file that sits next to a `casm.json` entry `output_path`.
pub(super) fn fingerprint_path(output_path: &Path) -> PathBuf {
    output_path
        .parent()
        .expect("CASM cache entry path should always have a parent")
        .join("fingerprint")
}

/// Returns the cached entry at `path` if its stored fingerprint matches `fingerprint` and the
/// `casm.json` is readable; otherwise `None` (a miss).
pub(super) fn read_cache_entry(
    path: &Path,
    fingerprint: &CasmCompilationFingerprint,
) -> Option<CasmCompilationOutput> {
    if !cache_fingerprint_matches(&fingerprint_path(path), fingerprint) {
        return None;
    }

    // Opening the file here is also the existence check: if the fingerprint is present but
    // `casm.json` is missing (e.g. an interrupted write), we treat it as a miss. We pass the open
    // handle on so the file is read only once and never reopened.
    match fs::File::open(path) {
        Ok(file) => Some(CasmCompilationOutput::CachedFile(file)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            tracing::debug!(
                path = %path.display(),
                %error,
                "failed to open CASM cache entry"
            );
            None
        }
    }
}

/// Whether the `fingerprint` file records the same digest as `fingerprint`. A missing or
/// unreadable file counts as no match.
fn cache_fingerprint_matches(
    fingerprint_path: &Path,
    fingerprint: &CasmCompilationFingerprint,
) -> bool {
    let Some(stored) = read_fingerprint(fingerprint_path) else {
        return false;
    };

    if stored != fingerprint.digest() {
        tracing::debug!(
            path = %fingerprint_path.display(),
            "CASM cache fingerprint mismatch"
        );
        return false;
    }

    true
}

/// Reads the digest stored in a `fingerprint` file, or `None` if it is absent or unreadable.
fn read_fingerprint(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(value) => Some(value.trim().to_string()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            tracing::debug!(
                path = %path.display(),
                %error,
                "failed to read CASM cache fingerprint"
            );
            None
        }
    }
}

/// Writes an entry: the CASM `output` to `path`, then its fingerprint. The CASM is written first so
/// that the fingerprint only ever appears once its `casm.json` is fully in place.
pub(super) fn write_cache_entry(
    path: &Path,
    fingerprint: &CasmCompilationFingerprint,
    output: &Value,
) -> io::Result<()> {
    write_json_file_atomically(path, output)?;
    write_text_file_atomically(&fingerprint_path(path), fingerprint.digest())
}

/// Serializes `value` to `path` atomically, via a temp file in the same directory that is renamed
/// into place, so readers never observe a partially written entry.
fn write_json_file_atomically(path: &Path, value: &Value) -> io::Result<()> {
    let parent = path
        .parent()
        .expect("CASM cache entry path should always have a parent");
    fs::create_dir_all(parent)?;

    let mut temp_file = Builder::new()
        .prefix(".casm-cache-")
        .suffix(".json")
        .tempfile_in(parent)?;

    {
        let mut writer = BufWriter::new(&mut temp_file);
        serde_json::to_writer(&mut writer, value).map_err(io::Error::other)?;
        writer.flush()?;
    }

    temp_file.flush()?;
    temp_file.persist(path).map_err(|error| error.error)?;

    Ok(())
}

/// Writes `value` to `path` atomically (temp file + rename), like `write_json_file_atomically` but
/// for the plain-text `fingerprint` file.
pub(super) fn write_text_file_atomically(path: &Path, value: &str) -> io::Result<()> {
    let parent = path
        .parent()
        .expect("CASM cache fingerprint path should always have a parent");
    fs::create_dir_all(parent)?;

    let mut temp_file = Builder::new().prefix(".casm-cache-").tempfile_in(parent)?;

    temp_file.write_all(value.as_bytes())?;
    temp_file.flush()?;
    temp_file.persist(path).map_err(|error| error.error)?;

    Ok(())
}
