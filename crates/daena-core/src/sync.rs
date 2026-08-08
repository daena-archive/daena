use crate::error::CoreError;
use crate::storage::normalized_project_path;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static TEST_FAIL_EXPORT_AFTER: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static TEST_FAIL_EXPORT_REQUEST: Mutex<Option<String>> = Mutex::new(None);

#[cfg(test)]
pub(crate) fn set_test_export_failure_after(request_id: Option<&str>, limit: usize) {
    *TEST_FAIL_EXPORT_REQUEST.lock().unwrap() = request_id.map(str::to_owned);
    TEST_FAIL_EXPORT_AFTER.store(limit, Ordering::SeqCst);
}

const SYNC_ROOT: &str = ".daena/sync";
const LOCK_PATH: &str = ".daena/project.lock";
const EXPORT_LOCK_PATH: &str = ".daena/export.lock";

struct SyncLock {
    path: PathBuf,
    token: String,
}

pub(crate) struct ProjectSessionLock {
    _lock: SyncLock,
}

impl ProjectSessionLock {
    pub(crate) fn acquire(root: &Path) -> Result<Self, CoreError> {
        let path = root.join(LOCK_PATH);
        let token = Uuid::new_v4().to_string();
        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && lock_is_stale(&path) => {
                fs::remove_file(&path).map_err(|source| CoreError::Io {
                    operation: "reclaim stale project session lock",
                    source,
                })?;
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|source| CoreError::Io {
                        operation: "acquire project session lock",
                        source,
                    })?
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(CoreError::Conflict(
                    "project is already open for writing".into(),
                ));
            }
            Err(source) => {
                return Err(CoreError::Io {
                    operation: "acquire project session lock",
                    source,
                })
            }
        };
        let mut file = file;
        file.write_all(format!("{}\n{}\n", std::process::id(), token).as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|source| CoreError::Io {
                operation: "write project session lock",
                source,
            })?;
        Ok(Self {
            _lock: SyncLock { path, token },
        })
    }
}

impl Drop for SyncLock {
    fn drop(&mut self) {
        let Ok(contents) = fs::read_to_string(&self.path) else {
            return;
        };
        if contents.lines().nth(1) == Some(self.token.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct Replacement {
    target: String,
    staged: Option<PathBuf>,
    expected_old_hash: Option<String>,
    new_hash: Option<String>,
}

pub(crate) struct SyncExporter {
    root: PathBuf,
    directory: PathBuf,
    #[cfg(test)]
    request_id: String,
    replacements: BTreeMap<String, Replacement>,
    lock: Option<SyncLock>,
}

impl SyncExporter {
    pub(crate) fn begin(root: &Path, request_id: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(request_id)
            .map_err(|_| CoreError::Validation("transaction request ID must be a UUID".into()))?;
        let sync_root = root.join(SYNC_ROOT);
        fs::create_dir_all(&sync_root).map_err(|source| CoreError::Io {
            operation: "create sync root",
            source,
        })?;
        let lock_path = root.join(EXPORT_LOCK_PATH);
        let token = Uuid::new_v4().to_string();
        let owner = format!("{}\n{}\n", std::process::id(), token);
        let mut lock_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if lock_is_stale(&lock_path) {
                    fs::remove_file(&lock_path).map_err(|source| CoreError::Io {
                        operation: "reclaim stale project sync lock",
                        source,
                    })?;
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&lock_path)
                        .map_err(|source| CoreError::Io {
                            operation: "acquire project sync lock",
                            source,
                        })?
                } else {
                    return Err(CoreError::Conflict(
                        "project sync lock is already held".into(),
                    ));
                }
            }
            Err(source) => {
                return Err(CoreError::Io {
                    operation: "acquire project sync lock",
                    source,
                })
            }
        };
        lock_file
            .write_all(owner.as_bytes())
            .and_then(|_| lock_file.sync_all())
            .map_err(|source| CoreError::Io {
                operation: "write project sync lock",
                source,
            })?;
        let directory = sync_root.join(request_id);
        fs::create_dir_all(directory.join("new")).map_err(|source| CoreError::Io {
            operation: "create sync staging directory",
            source,
        })?;
        Ok(Self {
            root: root.to_path_buf(),
            directory,
            #[cfg(test)]
            request_id: request_id.into(),
            replacements: BTreeMap::new(),
            lock: Some(SyncLock {
                path: lock_path,
                token,
            }),
        })
    }

    pub(crate) fn staging_root(&self) -> PathBuf {
        self.directory.join("project")
    }

    pub(crate) fn stage_bytes(&mut self, target: &str, bytes: &[u8]) -> Result<(), CoreError> {
        let expected_old_hash = hash_path(&self.root, target)?;
        self.stage_bytes_with_expected(target, bytes, expected_old_hash)
    }

    pub(crate) fn stage_bytes_with_expected(
        &mut self,
        target: &str,
        bytes: &[u8],
        expected_old_hash: Option<String>,
    ) -> Result<(), CoreError> {
        let staged = self.directory.join("new").join(target);
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                operation: "create sync staging parent",
                source,
            })?;
        }
        let mut file = File::create(&staged).map_err(|source| CoreError::Io {
            operation: "create sync staged file",
            source,
        })?;
        file.write_all(bytes).map_err(|source| CoreError::Io {
            operation: "write sync staged file",
            source,
        })?;
        file.sync_all().map_err(|source| CoreError::Io {
            operation: "sync sync staged file",
            source,
        })?;
        self.replacements.insert(
            target.into(),
            Replacement {
                target: target.into(),
                staged: Some(staged),
                expected_old_hash,
                new_hash: Some(hash_bytes(bytes)),
            },
        );
        Ok(())
    }

    pub(crate) fn stage_file_with_expected(
        &mut self,
        target: &str,
        source: &Path,
        expected_old_hash: Option<String>,
    ) -> Result<(), CoreError> {
        let staged = self.directory.join("new").join(target);
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                operation: "create sync staging parent",
                source,
            })?;
        }
        let mut input = File::open(source).map_err(|source| CoreError::Io {
            operation: "open asset source for sync",
            source,
        })?;
        let mut output = File::create(&staged).map_err(|source| CoreError::Io {
            operation: "create sync staged asset",
            source,
        })?;
        std::io::copy(&mut input, &mut output).map_err(|source| CoreError::Io {
            operation: "stream asset into sync staging",
            source,
        })?;
        output.sync_all().map_err(|source| CoreError::Io {
            operation: "sync staged asset",
            source,
        })?;
        let new_hash = hash_file(&staged)?.ok_or_else(|| {
            CoreError::Validation(format!("staged asset is missing: {}", staged.display()))
        })?;
        self.replacements.insert(
            target.into(),
            Replacement {
                target: target.into(),
                staged: Some(staged),
                expected_old_hash,
                new_hash: Some(new_hash),
            },
        );
        Ok(())
    }

    pub(crate) fn stage_remove(&mut self, target: &str) -> Result<(), CoreError> {
        self.replacements.insert(
            target.into(),
            Replacement {
                target: target.into(),
                staged: None,
                expected_old_hash: hash_path(&self.root, target)?,
                new_hash: None,
            },
        );
        Ok(())
    }

    pub(crate) fn commit<T: Serialize>(
        mut self,
        _result: Option<&T>,
    ) -> Result<Vec<String>, CoreError> {
        let mut applied = Vec::with_capacity(self.replacements.len());
        for replacement in self.replacements.values() {
            let target = normalized_project_path(&self.root, &replacement.target)?;
            if hash_file(&target)? != replacement.expected_old_hash {
                return Err(CoreError::Conflict(format!(
                    "portable baseline changed for {}",
                    replacement.target
                )));
            }
            match (&replacement.staged, replacement.new_hash.as_ref()) {
                (Some(staged), Some(expected_hash)) => {
                    if hash_file(staged)?.as_deref() != Some(expected_hash) {
                        return Err(CoreError::Conflict(format!(
                            "staged sync bytes changed for {}",
                            replacement.target
                        )));
                    }
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                            operation: "create portable target parent",
                            source,
                        })?;
                    }
                    replace_staged_file(staged, &target).map_err(|source| CoreError::Io {
                        operation: "replace portable target",
                        source,
                    })?;
                }
                (None, None) => {
                    if target.exists() {
                        fs::remove_file(&target).map_err(|source| CoreError::Io {
                            operation: "remove portable target",
                            source,
                        })?;
                    }
                }
                _ => unreachable!("sync replacement hash/staging mismatch"),
            }
            if let Some(parent) = target.parent() {
                sync_directory(parent)?;
            }
            if hash_file(&target)? != replacement.new_hash {
                return Err(CoreError::Conflict(format!(
                    "portable target verification failed for {}",
                    replacement.target
                )));
            }
            applied.push(replacement.target.clone());
            #[cfg(test)]
            if TEST_FAIL_EXPORT_REQUEST.lock().unwrap().as_deref() == Some(self.request_id.as_str())
                && TEST_FAIL_EXPORT_AFTER.load(Ordering::SeqCst) > 0
                && TEST_FAIL_EXPORT_AFTER.fetch_sub(1, Ordering::SeqCst) == 1
            {
                return Err(CoreError::Conflict(
                    "injected exporter failure after applied item".into(),
                ));
            }
        }
        fs::remove_dir_all(&self.directory).map_err(|source| CoreError::Io {
            operation: "clean sync staging directory",
            source,
        })?;
        self.lock.take();
        Ok(applied)
    }
}

fn hash_file(path: &Path) -> Result<Option<String>, CoreError> {
    if !path.is_file() {
        return Ok(None);
    }
    let mut file = File::open(path).map_err(|source| CoreError::Io {
        operation: "read sync hash target",
        source,
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let count =
            std::io::Read::read(&mut file, &mut buffer).map_err(|source| CoreError::Io {
                operation: "read sync hash target",
                source,
            })?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(Some(format!("sha256:{:x}", digest.finalize())))
}

#[cfg(not(windows))]
fn replace_staged_file(staged: &Path, target: &Path) -> io::Result<()> {
    fs::rename(staged, target)
}

#[cfg(windows)]
fn replace_staged_file(staged: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    if !target.exists() {
        return fs::rename(staged, target);
    }
    let target_wide = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let staged_wide = staged
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    let replaced = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            staged_wide.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(crate) fn hash_path(root: &Path, relative: &str) -> Result<Option<String>, CoreError> {
    hash_file(&normalized_project_path(root, relative)?)
}

fn sync_directory(path: &Path) -> Result<(), CoreError> {
    let directory = File::open(path).map_err(|source| CoreError::Io {
        operation: "open sync directory",
        source,
    })?;
    directory.sync_all().map_err(|source| CoreError::Io {
        operation: "sync portable parent directory",
        source,
    })
}

fn lock_is_stale(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return true;
    };
    let Some(pid) = contents
        .lines()
        .next()
        .and_then(|value| value.parse::<i32>().ok())
    else {
        return true;
    };
    if pid == std::process::id() as i32 {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(pid, 0) != 0
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
