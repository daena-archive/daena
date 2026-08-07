use crate::error::CoreError;
use crate::storage::normalized_project_path;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const TRANSACTION_ROOT: &str = ".daena/transactions";
const COMMITTED_ROOT: &str = ".daena/transactions/committed";
const LOCK_PATH: &str = ".daena/project.lock";
const JOURNAL_FILENAME: &str = "journal.json";
const JOURNAL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Journal {
    format_version: u32,
    request_id: String,
    state: JournalState,
    #[serde(default)]
    result: Option<serde_json::Value>,
    replacements: Vec<Replacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum JournalState {
    Pending,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Replacement {
    target: String,
    staged: String,
    expected_old_hash: Option<String>,
    new_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    format_version: u32,
    request_id: String,
    #[serde(default)]
    result: Option<serde_json::Value>,
    new_hashes: BTreeMap<String, Option<String>>,
}

#[derive(Debug)]
struct WriterLock {
    path: PathBuf,
    token: String,
}

impl WriterLock {
    fn acquire(root: &Path) -> Result<Self, CoreError> {
        let path = root.join(LOCK_PATH);
        let token = Uuid::new_v4().to_string();
        let owner = format!("{}\n{}\n", std::process::id(), token);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(owner.as_bytes())
                    .map_err(|source| CoreError::Io {
                        operation: "write project writer lock",
                        source,
                    })?;
                file.sync_all().map_err(|source| CoreError::Io {
                    operation: "sync project writer lock",
                    source,
                })?;
                sync_directory(path.parent().expect("lock path has a parent"))?;
                Ok(Self { path, token })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if lock_owner_is_dead(&path) {
                    fs::remove_file(&path).map_err(|source| CoreError::Io {
                        operation: "remove stale project writer lock",
                        source,
                    })?;
                    return Self::acquire(root);
                }
                Err(CoreError::Conflict(
                    "project writer lock is already held".into(),
                ))
            }
            Err(source) => Err(CoreError::Io {
                operation: "acquire project writer lock",
                source,
            }),
        }
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let Ok(contents) = fs::read_to_string(&self.path) else {
            return;
        };
        if contents.lines().nth(1) == Some(self.token.as_str()) {
            let _ = fs::remove_file(&self.path);
            if let Some(parent) = self.path.parent() {
                let _ = sync_directory(parent);
            }
        }
    }
}

pub(crate) enum TransactionStart {
    Ready(FileTransaction),
    AlreadyCommitted,
}

pub(crate) struct FileTransaction {
    root: PathBuf,
    directory: PathBuf,
    lock: Option<WriterLock>,
    replacements: Vec<Replacement>,
}

impl FileTransaction {
    pub(crate) fn begin(
        root: impl AsRef<Path>,
        request_id: &str,
    ) -> Result<TransactionStart, CoreError> {
        let root = root.as_ref();
        validate_request_id(request_id)?;
        ensure_transaction_directories(root)?;
        recover_transactions(root)?;
        let receipt_path = root.join(COMMITTED_ROOT).join(format!("{request_id}.json"));
        if receipt_path.is_file() {
            let receipt: Receipt = read_json(&receipt_path)?;
            if receipt.request_id != request_id || receipt.format_version != JOURNAL_VERSION {
                return Err(CoreError::Conflict(
                    "request ID has an incompatible committed receipt".into(),
                ));
            }
            return Ok(TransactionStart::AlreadyCommitted);
        }
        let lock = WriterLock::acquire(root)?;
        let directory = root.join(TRANSACTION_ROOT).join(request_id);
        if directory.exists() {
            return Err(CoreError::Conflict(
                "request ID already has an unfinished transaction".into(),
            ));
        }
        fs::create_dir_all(directory.join("new")).map_err(|source| CoreError::Io {
            operation: "create transaction staging directory",
            source,
        })?;
        Ok(TransactionStart::Ready(Self {
            root: root.to_path_buf(),
            directory,
            lock: Some(lock),
            replacements: Vec::new(),
        }))
    }

    pub(crate) fn stage_bytes(&mut self, target: &str, bytes: &[u8]) -> Result<(), CoreError> {
        let target_path = transaction_target(&self.root, target)?;
        let staged = self.directory.join("new").join(target);
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                operation: "create transaction staging parent",
                source,
            })?;
        }
        let mut file = File::create(&staged).map_err(|source| CoreError::Io {
            operation: "write staged transaction file",
            source,
        })?;
        file.write_all(bytes).map_err(|source| CoreError::Io {
            operation: "write staged transaction file",
            source,
        })?;
        file.sync_all().map_err(|source| CoreError::Io {
            operation: "sync staged transaction file",
            source,
        })?;
        if let Some(parent) = staged.parent() {
            sync_directory(parent)?;
        }

        let expected_old_hash = hash_file_if_present(&target_path)?;
        let replacement = Replacement {
            target: target.into(),
            staged: format!("new/{target}"),
            expected_old_hash,
            new_hash: Some(hash_bytes(bytes)),
        };
        if let Some(existing) = self
            .replacements
            .iter_mut()
            .find(|existing| existing.target == target)
        {
            if existing.expected_old_hash != replacement.expected_old_hash {
                return Err(CoreError::Conflict(
                    "staged target changed during transaction".into(),
                ));
            }
            *existing = replacement;
        } else {
            self.replacements.push(replacement);
        }
        Ok(())
    }

    pub(crate) fn staging_root(&self) -> PathBuf {
        self.directory.join("project")
    }

    pub(crate) fn stage_remove(&mut self, target: &str) -> Result<(), CoreError> {
        let target_path = transaction_target(&self.root, target)?;
        let replacement = Replacement {
            target: target.into(),
            staged: String::new(),
            expected_old_hash: hash_file_if_present(&target_path)?,
            new_hash: None,
        };
        if let Some(existing) = self
            .replacements
            .iter_mut()
            .find(|existing| existing.target == target)
        {
            if existing.expected_old_hash != replacement.expected_old_hash {
                return Err(CoreError::Conflict(
                    "staged target changed during transaction".into(),
                ));
            }
            *existing = replacement;
        } else {
            self.replacements.push(replacement);
        }
        Ok(())
    }

    pub(crate) fn commit(mut self) -> Result<(), CoreError> {
        self.commit_internal(None, None)
    }

    pub(crate) fn commit_with_result<T: Serialize>(mut self, result: &T) -> Result<(), CoreError> {
        let result = serde_json::to_value(result)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.commit_internal(None, Some(result))
    }

    #[cfg(test)]
    fn commit_with_failure(&mut self, failure: FailurePoint) -> Result<(), CoreError> {
        self.commit_internal(Some(failure), None)
    }

    fn commit_internal(
        &mut self,
        failure: Option<FailurePoint>,
        result: Option<serde_json::Value>,
    ) -> Result<(), CoreError> {
        if self.replacements.is_empty() {
            return Err(CoreError::Validation(
                "transaction must contain at least one staged replacement".into(),
            ));
        }
        let request_id = self
            .directory
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CoreError::Validation("transaction request ID is invalid".into()))?
            .to_string();
        let journal = Journal {
            format_version: JOURNAL_VERSION,
            request_id: request_id.clone(),
            state: JournalState::Pending,
            result: result.clone(),
            replacements: self.replacements.clone(),
        };
        persist_json(&self.directory.join(JOURNAL_FILENAME), &journal)?;
        if failure == Some(FailurePoint::AfterJournal) {
            return Err(injected_failure());
        }

        apply_replacements(&self.root, &self.directory, &journal, failure)?;
        let receipt = Receipt {
            format_version: JOURNAL_VERSION,
            request_id,
            result,
            new_hashes: journal
                .replacements
                .iter()
                .map(|replacement| (replacement.target.clone(), replacement.new_hash.clone()))
                .collect(),
        };
        persist_json(
            &self
                .root
                .join(COMMITTED_ROOT)
                .join(format!("{}.json", receipt.request_id)),
            &receipt,
        )?;
        if failure == Some(FailurePoint::AfterReceipt) {
            return Err(injected_failure());
        }

        let complete = Journal {
            state: JournalState::Complete,
            ..journal
        };
        persist_json(&self.directory.join(JOURNAL_FILENAME), &complete)?;
        cleanup_transaction(&self.directory)?;
        self.lock.take();
        Ok(())
    }
}

#[allow(dead_code, clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailurePoint {
    AfterJournal,
    AfterReplacement(usize),
    AfterReceipt,
}

fn apply_replacements(
    root: &Path,
    directory: &Path,
    journal: &Journal,
    failure: Option<FailurePoint>,
) -> Result<(), CoreError> {
    for (index, replacement) in journal.replacements.iter().enumerate() {
        let target = transaction_target(root, &replacement.target)?;
        let current_hash = hash_file_if_present(&target)?;
        if current_hash == replacement.new_hash {
            continue;
        }
        if current_hash != replacement.expected_old_hash {
            return Err(CoreError::Conflict(format!(
                "transaction source revision changed for {}",
                replacement.target
            )));
        }
        if replacement.new_hash.is_none() {
            if target.exists() {
                fs::remove_file(&target).map_err(|source| CoreError::Io {
                    operation: "remove canonical transaction target",
                    source,
                })?;
            }
            if let Some(parent) = target.parent() {
                sync_directory(parent)?;
            }
            if failure == Some(FailurePoint::AfterReplacement(index)) {
                return Err(injected_failure());
            }
            continue;
        }
        let staged = directory.join(&replacement.staged);
        if !staged.is_file() {
            return Err(CoreError::Validation(format!(
                "staged transaction file is missing: {}",
                replacement.staged
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                operation: "create transaction target parent",
                source,
            })?;
        }
        if let Err(error) = fs::rename(&staged, &target) {
            if error.kind() != io::ErrorKind::AlreadyExists {
                return Err(CoreError::Io {
                    operation: "replace canonical transaction target",
                    source: error,
                });
            }
            fs::remove_file(&target).map_err(|source| CoreError::Io {
                operation: "replace existing transaction target",
                source,
            })?;
            fs::rename(&staged, &target).map_err(|source| CoreError::Io {
                operation: "replace canonical transaction target",
                source,
            })?;
        }
        if let Some(parent) = target.parent() {
            sync_directory(parent)?;
        }
        if failure == Some(FailurePoint::AfterReplacement(index)) {
            return Err(injected_failure());
        }
    }
    Ok(())
}

pub(crate) fn recover_transactions(root: &Path) -> Result<(), CoreError> {
    let directory = root.join(TRANSACTION_ROOT);
    if !directory.is_dir() {
        return Ok(());
    }
    let mut transactions = fs::read_dir(&directory)
        .map_err(|source| CoreError::Io {
            operation: "read transaction directory",
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| CoreError::Io {
            operation: "read transaction directory",
            source,
        })?;
    transactions.sort_by_key(|entry| entry.file_name());
    let pending = transactions
        .iter()
        .any(|entry| entry.file_name().to_str() != Some("committed") && entry.path().is_dir());
    if !pending {
        return Ok(());
    }
    let _lock = WriterLock::acquire(root)?;
    for entry in transactions {
        if entry.file_name().to_str() == Some("committed") || !entry.path().is_dir() {
            continue;
        }
        recover_transaction(root, &entry.path())?;
    }
    Ok(())
}

fn recover_transaction(root: &Path, directory: &Path) -> Result<(), CoreError> {
    let journal_path = directory.join(JOURNAL_FILENAME);
    if !journal_path.is_file() {
        cleanup_transaction(directory)?;
        return Ok(());
    }
    let journal: Journal = read_json(&journal_path)?;
    validate_request_id(&journal.request_id)?;
    if journal.format_version != JOURNAL_VERSION {
        return Err(CoreError::Validation(
            "unsupported transaction journal version".into(),
        ));
    }
    if journal.state == JournalState::Pending {
        apply_replacements(root, directory, &journal, None)?;
        let receipt = Receipt {
            format_version: JOURNAL_VERSION,
            request_id: journal.request_id.clone(),
            result: journal.result.clone(),
            new_hashes: journal
                .replacements
                .iter()
                .map(|replacement| (replacement.target.clone(), replacement.new_hash.clone()))
                .collect(),
        };
        persist_json(
            &root
                .join(COMMITTED_ROOT)
                .join(format!("{}.json", receipt.request_id)),
            &receipt,
        )?;
        persist_json(
            &journal_path,
            &Journal {
                state: JournalState::Complete,
                ..journal
            },
        )?;
    }
    cleanup_transaction(directory)
}

pub(crate) fn committed_result(
    root: &Path,
    request_id: &str,
) -> Result<Option<serde_json::Value>, CoreError> {
    validate_request_id(request_id)?;
    let path = root.join(COMMITTED_ROOT).join(format!("{request_id}.json"));
    if !path.is_file() {
        return Ok(None);
    }
    let receipt: Receipt = read_json(&path)?;
    if receipt.request_id != request_id || receipt.format_version != JOURNAL_VERSION {
        return Err(CoreError::Conflict(
            "request ID has an incompatible committed receipt".into(),
        ));
    }
    Ok(receipt.result)
}

fn ensure_transaction_directories(root: &Path) -> Result<(), CoreError> {
    fs::create_dir_all(root.join(COMMITTED_ROOT)).map_err(|source| CoreError::Io {
        operation: "create transaction directory",
        source,
    })
}

fn cleanup_transaction(directory: &Path) -> Result<(), CoreError> {
    if directory.exists() {
        fs::remove_dir_all(directory).map_err(|source| CoreError::Io {
            operation: "remove completed transaction",
            source,
        })?;
    }
    Ok(())
}

fn transaction_target(root: &Path, target: &str) -> Result<PathBuf, CoreError> {
    let allowed_private_target = target.starts_with(".daena/backups/plugins/");
    if target.is_empty()
        || target == ".daena"
        || (target.starts_with(".daena/") && !allowed_private_target)
    {
        return Err(CoreError::Validation(
            "transaction target must be canonical project data".into(),
        ));
    }
    normalized_project_path(root, target)
}

fn validate_request_id(request_id: &str) -> Result<(), CoreError> {
    Uuid::parse_str(request_id)
        .map_err(|_| CoreError::Validation("transaction request ID must be a UUID".into()))?;
    Ok(())
}

fn hash_file_if_present(path: &Path) -> Result<Option<String>, CoreError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CoreError::Io {
                operation: "read transaction source metadata",
                source,
            })
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CoreError::Validation(format!(
            "transaction source must be a regular file: {}",
            path.display()
        )));
    }
    let mut file = File::open(path).map_err(|source| CoreError::Io {
        operation: "read transaction source",
        source,
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|source| CoreError::Io {
            operation: "read transaction source",
            source,
        })?;
    Ok(Some(hash_bytes(&bytes)))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, CoreError> {
    let bytes = fs::read(path).map_err(|source| CoreError::Io {
        operation: "read transaction journal",
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|error| CoreError::Serialization(error.to_string()))
}

fn persist_json<T: Serialize>(path: &Path, value: &T) -> Result<(), CoreError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    bytes.push(b'\n');
    let temporary = path.with_extension("json.tmp");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| CoreError::Io {
            operation: "create transaction metadata directory",
            source,
        })?;
    }
    let mut file = File::create(&temporary).map_err(|source| CoreError::Io {
        operation: "write transaction metadata",
        source,
    })?;
    file.write_all(&bytes).map_err(|source| CoreError::Io {
        operation: "write transaction metadata",
        source,
    })?;
    file.sync_all().map_err(|source| CoreError::Io {
        operation: "sync transaction metadata",
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| CoreError::Io {
        operation: "commit transaction metadata",
        source,
    })?;
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), CoreError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| CoreError::Io {
            operation: "sync transaction directory",
            source,
        })
}

fn lock_owner_is_dead(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let Some(pid) = contents.lines().next().and_then(|value| value.parse().ok()) else {
        return false;
    };
    process_is_dead(pid)
}

#[cfg(unix)]
fn process_is_dead(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    result != 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EPERM)
}

#[cfg(not(unix))]
fn process_is_dead(_pid: u32) -> bool {
    false
}

fn injected_failure() -> CoreError {
    CoreError::Io {
        operation: "injected transaction failure",
        source: io::Error::new(io::ErrorKind::Interrupted, "transaction failure injection"),
    }
}

#[cfg(test)]
mod tests;
