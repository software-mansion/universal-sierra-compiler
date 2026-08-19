//! A single persistent CASM cache entry.
//!
//! An entry owns both the location of `casm.json` and the fingerprint its contents must match.

use super::SierraKind;
use anyhow::{Context, Result};
use scarb_stable_hash::StableHasher;
use serde_json::Value;
use std::fs;
use std::hash::Hasher as _;
use std::io::{self, BufReader, BufWriter, Write as _};
use std::path::{Path, PathBuf};
use tempfile::Builder;

const CASM_CACHE_DIR: &str = "casm";
const CASM_FILE_NAME: &str = "casm.json";
const FINGERPRINT_FILE_NAME: &str = "fingerprint";
const USC_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A CASM cache entry for the current contents of a Sierra file.
#[derive(Debug)]
pub struct CasmCacheEntry {
    path: PathBuf,
    fingerprint: String,
}

impl CasmCacheEntry {
    pub fn new(
        cache_dir: &Path,
        sierra_path: &Path,
        sierra_content: &[u8],
        sierra_kind: SierraKind,
    ) -> Result<Self> {
        let fingerprint = short_hash(sierra_content);
        let canonical_sierra_path = fs::canonicalize(sierra_path).with_context(|| {
            format!(
                "Unable to canonicalize Sierra path for CASM cache entry: {}",
                sierra_path.display()
            )
        })?;
        let slot_id = short_hash(canonical_sierra_path.as_os_str().as_encoded_bytes());
        let path = cache_dir
            .join(CASM_CACHE_DIR)
            .join(USC_VERSION)
            .join(sierra_kind.as_str())
            .join(slot_id)
            .join(CASM_FILE_NAME);

        Ok(Self { path, fingerprint })
    }

    pub fn casm_path(&self) -> &Path {
        &self.path
    }

    fn fingerprint_path(&self) -> PathBuf {
        self.path.with_file_name(FINGERPRINT_FILE_NAME)
    }

    /// Loads the cached CASM if the entry is valid.
    pub fn load(&self) -> Option<Value> {
        if !self.fingerprint_matches() {
            return None;
        }

        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
            Err(error) => {
                tracing::debug!(
                    path = %self.path.display(),
                    %error,
                    "failed to open CASM cache entry"
                );
                return None;
            }
        };

        match serde_json::from_reader(BufReader::new(file)) {
            Ok(output) => Some(output),
            Err(error) => {
                tracing::debug!(
                    path = %self.path.display(),
                    %error,
                    "invalid CASM cache entry"
                );
                None
            }
        }
    }

    /// Stores the CASM first and its fingerprint second, so the fingerprint marks a complete entry.
    pub fn store(&self, output: &Value) -> io::Result<()> {
        // Ensure an interrupted replacement leaves a cache miss, not a stale valid fingerprint.
        remove_file_if_exists(&self.fingerprint_path())?;

        write_json_file_atomically(&self.path, output)?;
        write_text_file_atomically(&self.fingerprint_path(), &self.fingerprint)
    }

    fn fingerprint_matches(&self) -> bool {
        let path = self.fingerprint_path();
        let Some(stored) = read_fingerprint(&path) else {
            return false;
        };

        if stored != self.fingerprint {
            tracing::debug!(
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
            tracing::debug!(
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

fn write_json_file_atomically(path: &Path, value: &Value) -> io::Result<()> {
    write_file_atomically(path, |writer| {
        serde_json::to_writer(writer, value).map_err(io::Error::other)
    })
}

fn write_text_file_atomically(path: &Path, value: &str) -> io::Result<()> {
    write_file_atomically(path, |writer| writer.write_all(value.as_bytes()))
}

fn write_file_atomically(
    path: &Path,
    write: impl FnOnce(&mut dyn io::Write) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .expect("CASM cache file path should always have a parent");
    fs::create_dir_all(parent)?;

    let mut temp_file = Builder::new().prefix(".casm-cache-").tempfile_in(parent)?;

    {
        let mut writer = BufWriter::new(&mut temp_file);
        write(&mut writer)?;
        writer.flush()?;
    }

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
        let source_content = fs::read(source_path).unwrap();
        CasmCacheEntry::new(cache_dir, source_path, &source_content, SierraKind::Raw).unwrap()
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

        entry.store(&output).unwrap();
        fs::remove_file(entry.fingerprint_path()).unwrap();
        assert!(entry.load().is_none());

        entry.store(&output).unwrap();
        fs::remove_file(entry.casm_path()).unwrap();
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
    fn hashes_content() {
        let a = b"{\"program\": 1}";
        let a_copy = b"{\"program\": 1}";
        let b = b"{\"program\": 2}";

        assert_eq!(short_hash(a), short_hash(a_copy));
        assert_ne!(short_hash(a), short_hash(b));
    }

    #[test]
    fn hashes_large_content() {
        let big: Vec<u8> = (0..200 * 1024)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let big_copy = big.clone();
        let mut mutated = big.clone();
        *mutated.last_mut().unwrap() ^= 0xff;

        assert_eq!(short_hash(&big), short_hash(&big_copy));
        assert_ne!(short_hash(&big), short_hash(&mutated));
    }

    #[test]
    fn uses_expected_layout() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = write_file(temp.path(), "program.sierra.json", b"{}");
        let cache_dir = temp.path().join("cache");

        for (kind, kind_dir) in [(SierraKind::Raw, "raw"), (SierraKind::Contract, "contract")] {
            let source_content = fs::read(&source_path).unwrap();
            let entry =
                CasmCacheEntry::new(&cache_dir, &source_path, &source_content, kind).unwrap();
            let relative_path = entry.casm_path().strip_prefix(&cache_dir).unwrap();
            let components: Vec<_> = relative_path.components().collect();

            assert_eq!(components.len(), 5);
            assert_eq!(components[0].as_os_str(), CASM_CACHE_DIR);
            assert_eq!(components[1].as_os_str(), USC_VERSION);
            assert_eq!(components[2].as_os_str(), kind_dir);
            assert_eq!(components[4].as_os_str(), CASM_FILE_NAME);
            assert_eq!(
                entry.fingerprint_path(),
                entry.casm_path().with_file_name(FINGERPRINT_FILE_NAME)
            );
        }
    }
}
