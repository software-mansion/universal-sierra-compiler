//! On-disk layout of the CASM cache: where each entry lives (`CasmCacheSlot` / `cache_entry_path`)
//! and how we decide whether a cached entry is still valid (`CasmCompilationFingerprint`).

use anyhow::{Context, Result};
use scarb_stable_hash::StableHasher;
#[cfg(test)]
use serde_core::Serialize;
use std::fs;
use std::hash::Hasher as _;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

const CASM_CACHE_DIR: &str = "casm";
// Entries are namespaced by `USC_VERSION`, so releases invalidate old ones automatically. If you
// change the hardcoded codegen config without bumping the version, clear the cache manually.
const USC_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SierraKind {
    /// A `compile-raw` target (a plain Sierra `Program`).
    Raw,
    /// A `compile-contract` target (a Starknet `ContractClass`).
    Contract,
}

impl SierraKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Contract => "contract",
        }
    }
}

/// Identifies which cache file a given Sierra artifact path maps to.
///
/// The slot is keyed on the artifact's canonical path and kind, so a given path has at most one
/// entry per kind - recompiling the same artifact overwrites its slot rather than piling up
/// historical entries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CasmCacheSlot {
    sierra_kind: SierraKind,
    slot_id: String,
}

impl CasmCacheSlot {
    /// Builds the slot for `sierra_path`, canonicalizing it first so that different spellings of
    /// the same file resolve to one slot.
    pub(super) fn new(sierra_kind: SierraKind, sierra_path: &Path) -> Result<Self> {
        let sierra_path = fs::canonicalize(sierra_path).with_context(|| {
            format!(
                "Unable to canonicalize Sierra path for CASM cache slot: {}",
                sierra_path.display()
            )
        })?;
        // The slot id only turns the (arbitrary) artifact path into a stable, filesystem-safe
        // directory name; the kind and versions are separate path segments in `cache_entry_path`.
        Ok(Self {
            sierra_kind,
            slot_id: short_hash(sierra_path.to_string_lossy().as_bytes()),
        })
    }
}

/// Captures everything about the input that must match for a cache entry to be reusable.
/// A cached entry is served only when its stored fingerprint equals the current one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasmCompilationFingerprint {
    sierra_kind: SierraKind,
    digest: String,
}

impl CasmCompilationFingerprint {
    /// Builds a fingerprint from an in-memory input (test-only).
    #[cfg(test)]
    pub fn new(sierra_kind: SierraKind, sierra_input: &impl Serialize) -> Result<Self> {
        let bytes = serde_json::to_vec(sierra_input)
            .context("Unable to serialize Sierra input for cache fingerprint")?;
        Ok(Self {
            sierra_kind,
            digest: short_hash(&bytes),
        })
    }

    /// Builds a fingerprint by hashing the raw bytes of the Sierra file at `sierra_path`.
    pub fn from_file(sierra_kind: SierraKind, sierra_path: &Path) -> Result<Self> {
        // We hash the raw file bytes, not a parsed form. On a cache hit this is the only time we
        // read the file, and we never parse it. On a miss the compile closure reads it again to
        // parse it, but that extra read is tiny next to the cost of compiling.
        let digest = hash_file_content(sierra_path).with_context(|| {
            format!(
                "Unable to fingerprint Sierra input file: {}",
                sierra_path.display()
            )
        })?;
        Ok(Self {
            sierra_kind,
            digest,
        })
    }

    /// The artifact kind this fingerprint was built for.
    pub(super) fn sierra_kind(&self) -> SierraKind {
        self.sierra_kind
    }

    /// The digest compared against a stored entry to decide a hit or miss. It is the hash of the
    /// Sierra input alone; the USC version and kind live in the entry's path.
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }
}

/// Hashes bytes into a short, filesystem-safe hex string.
fn short_hash(bytes: &[u8]) -> String {
    let mut hasher = StableHasher::new();
    hasher.write(bytes);
    hasher.finish_as_short_hash()
}

/// Full path to a slot's `casm.json`, under `<cache_dir>/casm/<kind>/<usc-version>/<slot-id>/`.
pub(super) fn cache_entry_path(cache_dir: &Path, slot: &CasmCacheSlot) -> PathBuf {
    // Every segment here is filesystem-safe by construction: the version/kind are trusted
    // constants, and `slot.slot_id` comes from `finish_as_short_hash`, so it is a hex string.
    cache_dir
        .join(CASM_CACHE_DIR)
        .join(slot.sierra_kind.as_str())
        .join(USC_VERSION)
        .join(&slot.slot_id)
        .join("casm.json")
}

/// Streams a file through the hasher in fixed-size chunks, so hashing does not depend on file size.
fn hash_file_content(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = StableHasher::new();
    let mut buffer = [0; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
    }

    Ok(hasher.finish_as_short_hash())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(dir: &Path, name: &str, contents: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn hash_file_content_is_stable_and_content_sensitive() {
        let temp = tempfile::tempdir().unwrap();
        let a = write_file(temp.path(), "a.json", b"{\"program\": 1}");
        let a_copy = write_file(temp.path(), "a_copy.json", b"{\"program\": 1}");
        let b = write_file(temp.path(), "b.json", b"{\"program\": 2}");

        assert_eq!(
            hash_file_content(&a).unwrap(),
            hash_file_content(&a_copy).unwrap(),
            "identical bytes should hash equally regardless of path"
        );
        assert_ne!(
            hash_file_content(&a).unwrap(),
            hash_file_content(&b).unwrap(),
            "different bytes should hash differently"
        );
    }

    #[test]
    fn hash_file_content_handles_files_larger_than_read_buffer() {
        let temp = tempfile::tempdir().unwrap();
        // Larger than the 64 KiB read buffer, so hashing spans multiple `read` calls.
        let big: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
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
            hash_file_content(&mutated_path).unwrap(),
            "a single flipped byte past the buffer boundary must change the hash"
        );
    }

    #[test]
    fn from_file_digest_depends_on_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = write_file(temp.path(), "program.sierra.json", b"{\"program\": 1}");
        let other = write_file(temp.path(), "other.sierra.json", b"{\"program\": 2}");

        let base = CasmCompilationFingerprint::from_file(SierraKind::Raw, &path).unwrap();

        // Same content - identical digest.
        let same = CasmCompilationFingerprint::from_file(SierraKind::Raw, &path).unwrap();
        assert_eq!(base.digest(), same.digest());

        // Different content - different digest.
        let changed_content =
            CasmCompilationFingerprint::from_file(SierraKind::Raw, &other).unwrap();
        assert_ne!(base.digest(), changed_content.digest());
    }

    #[test]
    fn from_file_fails_for_missing_input() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("does-not-exist.json");

        let error = CasmCompilationFingerprint::from_file(SierraKind::Raw, &missing).unwrap_err();

        assert!(error
            .to_string()
            .contains("Unable to fingerprint Sierra input file"));
    }
}
