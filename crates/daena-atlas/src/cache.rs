//! Explicit-directory atlas disk cache. Callers supply the root; this module
//! does not discover project paths.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::AtlasError;

pub const CACHE_FORMAT_VERSION: u32 = 1;
pub const KIND_RESIDUAL: u16 = 1;
pub const KIND_DRAINAGE: u16 = 2;
pub const KIND_ARTIFACT: u16 = 3;
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_ENTRY_BYTES: u64 = 160 * 1024 * 1024;
pub const MAX_ENTRIES: u32 = 64;
const MAGIC: &[u8; 8] = b"DAENAATL";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLookup {
    Off,
    Hit,
    Miss,
}

impl CacheLookup {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Hit => "hit",
            Self::Miss => "miss",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CacheIndex {
    schema_version: u32,
    #[serde(default)]
    next_seq: u64,
    entries: Vec<CacheIndexEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheIndexEntry {
    key_hex: String,
    kind: u16,
    bytes: u64,
    accessed_unix: u64,
    #[serde(default)]
    seq: u64,
}

pub struct AtlasDiskCache {
    root: PathBuf,
    max_total_bytes: u64,
    max_entry_bytes: u64,
    max_entries: u32,
    index: Mutex<CacheIndex>,
    io_lock: Arc<Mutex<()>>,
}

fn root_lock(root: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let locks = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = locks
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    map.entry(root.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

impl AtlasDiskCache {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, AtlasError> {
        let root = root.into();
        let io_lock = root_lock(&root);
        let _guard = io_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        fs::create_dir_all(&root).map_err(|error| {
            AtlasError::new(crate::CODE_RENDER_FAILED, format!("atlas cache: {error}"))
        })?;
        if root
            .symlink_metadata()
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(AtlasError::invalid(
                "atlas cache root must not be a symlink",
            ));
        }
        let index = read_index(&root).unwrap_or_default();
        let cache = Self {
            root,
            max_total_bytes: MAX_TOTAL_BYTES,
            max_entry_bytes: MAX_ENTRY_BYTES,
            max_entries: MAX_ENTRIES,
            index: Mutex::new(CacheIndex {
                schema_version: 1,
                next_seq: index.next_seq.max(
                    index
                        .entries
                        .iter()
                        .map(|entry| entry.seq)
                        .max()
                        .unwrap_or(0)
                        + 1,
                ),
                entries: index.entries,
            }),
            io_lock: io_lock.clone(),
        };
        cache.evict()?;
        Ok(cache)
    }

    pub fn with_limits(
        mut self,
        max_total_bytes: u64,
        max_entry_bytes: u64,
        max_entries: u32,
    ) -> Self {
        self.max_total_bytes = max_total_bytes;
        self.max_entry_bytes = max_entry_bytes;
        self.max_entries = max_entries;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn get(&self, kind: u16, key: &[u8; 32]) -> CacheLookupResult {
        let _guard = self
            .io_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let hex = hex_key(key);
        let path = self.root.join(format!("{hex}.bin"));
        match read_entry(&path, kind, key) {
            Ok(payload) => {
                self.touch(&hex, kind, payload.len() as u64);
                let _ = self.persist_index();
                CacheLookupResult::Hit(payload)
            }
            Err(_) => {
                if path.exists() {
                    let _ = fs::remove_file(&path);
                    self.drop_entry(&hex);
                    let _ = self.persist_index();
                }
                CacheLookupResult::Miss
            }
        }
    }

    pub fn put(&self, kind: u16, key: &[u8; 32], payload: &[u8]) -> Result<(), AtlasError> {
        let _guard = self
            .io_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if payload.len() as u64 > self.max_entry_bytes {
            return Ok(());
        }
        let hex = hex_key(key);
        if hex.bytes().any(|byte| !byte.is_ascii_hexdigit()) {
            return Err(AtlasError::invalid("atlas cache key must be hex"));
        }
        let dest = self.root.join(format!("{hex}.bin"));
        if dest
            .symlink_metadata()
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(AtlasError::invalid("atlas cache refused a symlink"));
        }
        let mut file = MAGIC.to_vec();
        file.extend_from_slice(&CACHE_FORMAT_VERSION.to_le_bytes());
        file.extend_from_slice(&kind.to_le_bytes());
        file.extend_from_slice(key);
        file.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        file.extend_from_slice(&Sha256::digest(payload));
        file.extend_from_slice(payload);
        let partial = dest.with_extension("bin.part");
        fs::write(&partial, &file).map_err(io_error)?;
        fs::rename(&partial, &dest).map_err(io_error)?;
        self.touch(&hex, kind, payload.len() as u64);
        self.evict()?;
        self.persist_index()
    }

    fn touch(&self, hex: &str, kind: u16, bytes: u64) {
        let Ok(mut index) = self.index.lock() else {
            return;
        };
        let now = unix_now();
        let seq = {
            index.next_seq = index.next_seq.saturating_add(1);
            index.next_seq
        };
        if let Some(entry) = index.entries.iter_mut().find(|entry| entry.key_hex == hex) {
            entry.accessed_unix = now;
            entry.bytes = bytes;
            entry.kind = kind;
            entry.seq = seq;
        } else {
            index.entries.push(CacheIndexEntry {
                key_hex: hex.to_string(),
                kind,
                bytes,
                accessed_unix: now,
                seq,
            });
        }
    }

    fn drop_entry(&self, hex: &str) {
        if let Ok(mut index) = self.index.lock() {
            index.entries.retain(|entry| entry.key_hex != hex);
        }
    }

    fn evict(&self) -> Result<(), AtlasError> {
        let mut doomed = Vec::new();
        {
            let Ok(mut index) = self.index.lock() else {
                return Ok(());
            };
            index.entries.sort_by(|left, right| {
                left.seq
                    .cmp(&right.seq)
                    .then(left.key_hex.cmp(&right.key_hex))
            });
            let mut total: u64 = index.entries.iter().map(|entry| entry.bytes).sum();
            while index.entries.len() as u32 > self.max_entries || total > self.max_total_bytes {
                if let Some(entry) = index.entries.first().cloned() {
                    total = total.saturating_sub(entry.bytes);
                    doomed.push(entry.key_hex.clone());
                    index.entries.remove(0);
                } else {
                    break;
                }
            }
        }
        for hex in doomed {
            let _ = fs::remove_file(self.root.join(format!("{hex}.bin")));
        }
        Ok(())
    }

    fn persist_index(&self) -> Result<(), AtlasError> {
        let Ok(index) = self.index.lock() else {
            return Ok(());
        };
        let bytes = serde_json::to_vec_pretty(&*index).map_err(|error| {
            AtlasError::new(
                crate::CODE_RENDER_FAILED,
                format!("atlas cache index: {error}"),
            )
        })?;
        let dest = self.root.join("index.json");
        let partial = dest.with_extension("json.part");
        fs::write(&partial, bytes).map_err(io_error)?;
        fs::rename(&partial, dest).map_err(io_error)
    }
}

pub enum CacheLookupResult {
    Hit(Vec<u8>),
    Miss,
}

pub fn cache_key(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(&(part.len() as u32).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn hex_key(key: &[u8; 32]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn io_error(error: std::io::Error) -> AtlasError {
    AtlasError::new(crate::CODE_RENDER_FAILED, format!("atlas cache: {error}"))
}

fn read_index(root: &Path) -> Option<CacheIndex> {
    let bytes = fs::read(root.join("index.json")).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn read_entry(path: &Path, kind: u16, key: &[u8; 32]) -> Result<Vec<u8>, AtlasError> {
    if path
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(AtlasError::invalid("atlas cache refused a symlink"));
    }
    let bytes = fs::read(path).map_err(io_error)?;
    if bytes.len() < 8 + 4 + 2 + 32 + 8 + 32 {
        return Err(AtlasError::invalid("atlas cache entry is truncated"));
    }
    if &bytes[0..8] != MAGIC {
        return Err(AtlasError::invalid("atlas cache magic mismatch"));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().expect("u32"));
    if version != CACHE_FORMAT_VERSION {
        return Err(AtlasError::invalid("atlas cache version mismatch"));
    }
    let stored_kind = u16::from_le_bytes(bytes[12..14].try_into().expect("u16"));
    if stored_kind != kind {
        return Err(AtlasError::invalid("atlas cache kind mismatch"));
    }
    let stored_key: [u8; 32] = bytes[14..46].try_into().expect("key");
    if &stored_key != key {
        return Err(AtlasError::invalid("atlas cache key mismatch"));
    }
    let payload_len = u64::from_le_bytes(bytes[46..54].try_into().expect("u64")) as usize;
    let checksum: [u8; 32] = bytes[54..86].try_into().expect("sha");
    let payload = bytes
        .get(86..86 + payload_len)
        .ok_or_else(|| AtlasError::invalid("atlas cache entry is truncated"))?;
    if bytes.len() != 86 + payload_len {
        return Err(AtlasError::invalid("atlas cache entry has trailing bytes"));
    }
    if checksum != Sha256::digest(payload).as_slice() {
        return Err(AtlasError::invalid("atlas cache checksum mismatch"));
    }
    Ok(payload.to_vec())
}

pub fn encode_residual(lattice_width: u32, lattice_height: u32, residual_mm: &[i32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12 + residual_mm.len() * 4);
    bytes.extend_from_slice(&lattice_width.to_le_bytes());
    bytes.extend_from_slice(&lattice_height.to_le_bytes());
    bytes.extend_from_slice(&(residual_mm.len() as u32).to_le_bytes());
    for value in residual_mm {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn decode_residual(bytes: &[u8]) -> Result<(u32, u32, Vec<i32>), AtlasError> {
    if bytes.len() < 12 {
        return Err(AtlasError::invalid("residual cache is truncated"));
    }
    let lattice_width = u32::from_le_bytes(bytes[0..4].try_into().expect("u32"));
    let lattice_height = u32::from_le_bytes(bytes[4..8].try_into().expect("u32"));
    let count = u32::from_le_bytes(bytes[8..12].try_into().expect("u32")) as usize;
    let expected = (lattice_width as usize)
        .checked_mul(lattice_height as usize)
        .ok_or_else(|| AtlasError::limit("residual lattice overflowed"))?;
    if count != expected || bytes.len() != 12 + count * 4 {
        return Err(AtlasError::invalid("residual cache length mismatch"));
    }
    let mut residual = Vec::with_capacity(count);
    for index in 0..count {
        let start = 12 + index * 4;
        residual.push(i32::from_le_bytes(
            bytes[start..start + 4].try_into().expect("i32"),
        ));
    }
    Ok((lattice_width, lattice_height, residual))
}

pub fn encode_artifact(png: &[u8], artifact: &[u8], provenance_json: &str) -> Vec<u8> {
    let provenance = provenance_json.as_bytes();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(provenance.len() as u32).to_le_bytes());
    bytes.extend_from_slice(provenance);
    bytes.extend_from_slice(&(png.len() as u32).to_le_bytes());
    bytes.extend_from_slice(png);
    bytes.extend_from_slice(&(artifact.len() as u32).to_le_bytes());
    bytes.extend_from_slice(artifact);
    bytes
}

pub fn decode_artifact(bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>, String), AtlasError> {
    let mut offset = 0;
    let provenance_len = read_u32(bytes, &mut offset)? as usize;
    let provenance = bytes
        .get(offset..offset + provenance_len)
        .ok_or_else(|| AtlasError::invalid("artifact cache is truncated"))?;
    offset += provenance_len;
    let png_len = read_u32(bytes, &mut offset)? as usize;
    let png = bytes
        .get(offset..offset + png_len)
        .ok_or_else(|| AtlasError::invalid("artifact cache is truncated"))?
        .to_vec();
    offset += png_len;
    let artifact_len = read_u32(bytes, &mut offset)? as usize;
    let artifact = bytes
        .get(offset..offset + artifact_len)
        .ok_or_else(|| AtlasError::invalid("artifact cache is truncated"))?
        .to_vec();
    offset += artifact_len;
    if offset != bytes.len() {
        return Err(AtlasError::invalid("artifact cache has trailing bytes"));
    }
    let provenance = String::from_utf8(provenance.to_vec())
        .map_err(|_| AtlasError::invalid("artifact cache provenance is not UTF-8"))?;
    Ok((png, artifact, provenance))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, AtlasError> {
    let slice = bytes
        .get(*offset..*offset + 4)
        .ok_or_else(|| AtlasError::invalid("artifact cache is truncated"))?;
    *offset += 4;
    Ok(u32::from_le_bytes(slice.try_into().expect("u32")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache() -> (AtlasDiskCache, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-cache-{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = AtlasDiskCache::open(&root)
            .unwrap()
            .with_limits(2_000, 1_000, 2);
        (cache, root)
    }

    #[test]
    fn round_trip_and_corrupt_entry_is_a_miss() {
        let (cache, root) = temp_cache();
        let key = cache_key(&[b"fixture"]);
        cache.put(KIND_DRAINAGE, &key, b"hello").unwrap();
        match cache.get(KIND_DRAINAGE, &key) {
            CacheLookupResult::Hit(payload) => assert_eq!(payload, b"hello"),
            CacheLookupResult::Miss => panic!("expected hit"),
        }
        fs::write(root.join(format!("{}.bin", hex_key(&key))), b"truncated").unwrap();
        match cache.get(KIND_DRAINAGE, &key) {
            CacheLookupResult::Miss => {}
            CacheLookupResult::Hit(_) => panic!("corrupt cache must miss"),
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lru_evicts_oldest_when_over_quota() {
        let (cache, root) = temp_cache();
        let first = cache_key(&[b"a"]);
        let second = cache_key(&[b"b"]);
        let third = cache_key(&[b"c"]);
        cache.put(KIND_RESIDUAL, &first, &[1; 100]).unwrap();
        cache.put(KIND_RESIDUAL, &second, &[2; 100]).unwrap();
        cache.put(KIND_RESIDUAL, &third, &[3; 100]).unwrap();
        match cache.get(KIND_RESIDUAL, &first) {
            CacheLookupResult::Miss => {}
            CacheLookupResult::Hit(_) => panic!("oldest entry should be evicted"),
        }
        let _ = fs::remove_dir_all(root);
    }
}
