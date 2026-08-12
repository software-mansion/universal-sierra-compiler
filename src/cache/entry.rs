//! A single persistent CASM cache entry.
//!
//! An entry owns both the location of `casm.json` and the fingerprint its contents must match.

use super::SierraKind;
use anyhow::{Context, Result};
use scarb_stable_hash::StableHasher;
use serde_json::Value;
use std::fs;
use std::hash::Hasher as _;
use std::io::{self, BufReader, BufWriter, Read as _, Write as _};
use std::path::{Path, PathBuf};
use tempfile::Builder;

const CASM_CACHE_DIR: &str = "casm";
const CASM_FILE_NAME: &str = "casm.json";
const FINGERPRINT_FILE_NAME: &str = "fingerprint";
const USC_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A CASM cache entry for the current contents of a Sierra file.
#[derive(Debug)]
pub(super) struct CasmCacheEntry {
    casm_path: PathBuf,
    expected_fingerprint: String,
}

impl CasmCacheEntry {
    pub(super) fn new(
        cache_dir: &Path,
        sierra_path: &Path,
        sierra_kind: SierraKind,
    ) -> Result<Self> {
        let expected_fingerprint = hash_file_content(sierra_path).with_context(|| {
            format!(
                "Unable to fingerprint Sierra input file: {}",
                sierra_path.display()
            )
        })?;
        let canonical_sierra_path = fs::canonicalize(sierra_path).with_context(|| {
            format!(
                "Unable to canonicalize Sierra path for CASM cache entry: {}",
                sierra_path.display()
            )
        })?;
        let slot_id = short_hash(canonical_sierra_path.to_string_lossy().as_bytes());
        let casm_path = cache_dir
            .join(CASM_CACHE_DIR)
            .join(sierra_kind.as_str())
            .join(USC_VERSION)
            .join(slot_id)
            .join(CASM_FILE_NAME);

        Ok(Self {
            casm_path,
            expected_fingerprint,
        })
    }

    pub(super) fn casm_path(&self) -> &Path {
        &self.casm_path
    }

    fn fingerprint_path(&self) -> PathBuf {
        self.casm_path.with_file_name(FINGERPRINT_FILE_NAME)
    }

    /// Loads the cached CASM if the entry is valid.
    pub(super) fn load(&self) -> Option<Value> {
        if !self.fingerprint_matches() {
            return None;
        }

        let file = match fs::File::open(&self.casm_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
            Err(error) => {
                tracing::warn!(
                    path = %self.casm_path.display(),
                    %error,
                    "failed to open CASM cache entry"
                );
                return None;
            }
        };

        match serde_json::from_reader(BufReader::new(file)) {
            Ok(output) => Some(output),
            Err(error) => {
                tracing::warn!(
                    path = %self.casm_path.display(),
                    %error,
                    "invalid CASM cache entry"
                );
                None
            }
        }
    }

    /// Stores the CASM first and its fingerprint second, so the fingerprint marks a complete entry.
    pub(super) fn store(&self, output: &Value) -> io::Result<()> {
        // Ensure an interrupted replacement leaves a cache miss, not a stale valid fingerprint.
        remove_file_if_exists(&self.fingerprint_path())?;

        write_json_file_atomically(&self.casm_path, output)?;
        write_text_file_atomically(&self.fingerprint_path(), &self.expected_fingerprint)
    }

    fn fingerprint_matches(&self) -> bool {
        let path = self.fingerprint_path();
        let Some(stored) = read_fingerprint(&path) else {
            return false;
        };

        if stored != self.expected_fingerprint {
            tracing::warn!(
                path = %path.display(),
                "CASM cache fingerprint mismatch"
            );
            return false;
        }

        true
    }
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn read_fingerprint(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(value) => Some(value.trim().to_string()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "failed to read CASM cache fingerprint"
            );
            None
        }
    }
}

fn short_hash(bytes: &[u8]) -> String {
    let mut hasher = StableHasher::new();
    hasher.write(bytes);
    hasher.finish_as_short_hash()
}

fn hash_file_content(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = StableHasher::new();
    let mut buffer = vec![0; 64 * 1024].into_boxed_slice();

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
    }

    Ok(hasher.finish_as_short_hash())
}

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

fn write_text_file_atomically(path: &Path, value: &str) -> io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_file(dir: &Path, name: &str, contents: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn entry(cache_dir: &Path, source_path: &Path) -> CasmCacheEntry {
        CasmCacheEntry::new(cache_dir, source_path, SierraKind::Raw).unwrap()
    }

    #[test]
    fn stores_and_loads_entry() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_file(temp.path(), "program.sierra.json", b"{}");
        let entry = entry(temp.path(), &source_path);
        let output = json!({"compiled": true});

        entry.store(&output).unwrap();

        assert_eq!(entry.load().unwrap(), output);
    }

    #[test]
    fn changed_input_is_cache_miss() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_file(temp.path(), "program.sierra.json", b"first");
        let stale_entry = entry(temp.path(), &source_path);
        stale_entry.store(&json!({"compiled": "stale"})).unwrap();

        fs::write(&source_path, b"second").unwrap();
        let current_entry = entry(temp.path(), &source_path);

        assert_eq!(stale_entry.casm_path(), current_entry.casm_path());
        assert!(current_entry.load().is_none());
    }

    #[test]
    fn incomplete_entry_is_cache_miss() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_file(temp.path(), "program.sierra.json", b"{}");
        let entry = entry(temp.path(), &source_path);
        let output = json!({"compiled": true});

        fs::create_dir_all(entry.casm_path().parent().unwrap()).unwrap();
        fs::write(entry.casm_path(), output.to_string()).unwrap();
        assert!(entry.load().is_none());

        fs::remove_file(entry.casm_path()).unwrap();
        write_text_file_atomically(&entry.fingerprint_path(), &entry.expected_fingerprint).unwrap();
        assert!(entry.load().is_none());
    }

    #[test]
    fn malformed_casm_is_cache_miss() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_file(temp.path(), "program.sierra.json", b"{}");
        let entry = entry(temp.path(), &source_path);

        entry.store(&json!({"compiled": true})).unwrap();
        fs::write(entry.casm_path(), "{not-json").unwrap();

        assert!(entry.load().is_none());
    }

    #[test]
    fn hashes_file_content() {
        let temp = tempfile::tempdir().unwrap();
        let a = write_file(temp.path(), "a.json", b"{\"program\": 1}");
        let a_copy = write_file(temp.path(), "a_copy.json", b"{\"program\": 1}");
        let b = write_file(temp.path(), "b.json", b"{\"program\": 2}");

        assert_eq!(
            hash_file_content(&a).unwrap(),
            hash_file_content(&a_copy).unwrap()
        );
        assert_ne!(
            hash_file_content(&a).unwrap(),
            hash_file_content(&b).unwrap()
        );
    }

    #[test]
    fn hashes_files_larger_than_read_buffer() {
        let temp = tempfile::tempdir().unwrap();
        let big: Vec<u8> = (0..200 * 1024)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let mut mutated = big.clone();
        *mutated.last_mut().unwrap() ^= 0xff;

        let big_path = write_file(temp.path(), "big.bin", &big);
        let big_copy_path = write_file(temp.path(), "big_copy.bin", &big);
        let mutated_path = write_file(temp.path(), "mutated.bin", &mutated);

        assert_eq!(
            hash_file_content(&big_path).unwrap(),
            hash_file_content(&big_copy_path).unwrap()
        );
        assert_ne!(
            hash_file_content(&big_path).unwrap(),
            hash_file_content(&mutated_path).unwrap()
        );
    }

    #[test]
    fn uses_expected_layout() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_file(temp.path(), "program.sierra.json", b"{}");
        let cache_dir = temp.path().join("cache");

        for (kind, kind_dir) in [(SierraKind::Raw, "raw"), (SierraKind::Contract, "contract")] {
            let entry = CasmCacheEntry::new(&cache_dir, &source_path, kind).unwrap();
            let relative_path = entry.casm_path().strip_prefix(&cache_dir).unwrap();
            let components: Vec<_> = relative_path.components().collect();

            assert_eq!(components.len(), 5);
            assert_eq!(components[0].as_os_str(), CASM_CACHE_DIR);
            assert_eq!(components[1].as_os_str(), kind_dir);
            assert_eq!(components[2].as_os_str(), USC_VERSION);
            assert_eq!(components[4].as_os_str(), CASM_FILE_NAME);
            assert_eq!(
                entry.fingerprint_path(),
                entry.casm_path().with_file_name(FINGERPRINT_FILE_NAME)
            );
        }
    }
}
