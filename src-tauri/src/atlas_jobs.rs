//! Bounded atlas preview/export jobs, temp artifacts, and host save.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use daena_atlas::request::AtlasRenderRequest;
use daena_atlas::{
    render_from_source, AtlasPhase, AtlasProgress, CancelFlag, CODE_RENDER_CANCELLED,
};
use daena_core::maps::atlas::{capabilities_for_map, capture_snapshot, AtlasRenderSnapshot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use super::{current_info, with_read_project, SharedAtlasJobs, SharedCore, ATLAS_PROGRESS_EVENT};

const ATLAS_JOB_TTL: Duration = Duration::from_secs(30 * 60);
const PREVIEW_MAX_WIDTH: u32 = daena_atlas::request::PREVIEW_MAX_WIDTH;
const PREVIEW_MAX_HEIGHT: u32 = daena_atlas::request::PREVIEW_MAX_HEIGHT;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasJobStatus {
    pub job_id: String,
    pub request_id: String,
    pub map_entity_id: String,
    pub kind: String,
    pub state: String,
    pub stage: String,
    pub completed: u32,
    pub total: u32,
    pub sequence: u64,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub width_px: u32,
    pub height_px: u32,
    pub preview_token: Option<String>,
    pub captured_content_generation: Option<i64>,
    pub current_content_generation: Option<i64>,
    pub provenance: Option<serde_json::Value>,
    pub estimate: Option<daena_atlas::request::ResourceEstimate>,
}

pub struct AtlasJob {
    #[allow(dead_code)]
    project_id: String,
    map_entity_id: String,
    kind: String,
    expires_at: Instant,
    cancel: Arc<CancelFlag>,
    status: AtlasJobStatus,
    artifact: Option<PathBuf>,
}

#[derive(Default)]
pub struct AtlasJobManager {
    jobs: std::collections::BTreeMap<String, AtlasJob>,
    export_job_id: Option<String>,
}

impl AtlasJobManager {
    fn reap(&mut self) {
        let now = Instant::now();
        self.jobs.retain(|id, job| {
            if job.expires_at <= now {
                job.cancel.cancel();
                if let Some(path) = &job.artifact {
                    let _ = remove_atlas_path(path);
                }
                if self.export_job_id.as_deref() == Some(id) {
                    self.export_job_id = None;
                }
                false
            } else {
                true
            }
        });
    }

    pub fn cancel_all(&mut self) {
        for job in self.jobs.values() {
            job.cancel.cancel();
            if let Some(path) = &job.artifact {
                let _ = remove_atlas_path(path);
            }
        }
        self.jobs.clear();
        self.export_job_id = None;
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasBeginInput {
    pub map_entity_id: String,
    pub request: AtlasRenderRequest,
    pub request_id: Option<String>,
}

fn install_png_bytes(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    if dest
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("atlas.save.failed: refused to follow a symlink".into());
    }
    let partial = dest.with_extension("png.partial");
    if partial
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("atlas.save.failed: refused to follow a symlink".into());
    }
    fs::write(&partial, bytes).map_err(|error| format!("atlas.save.failed: {error}"))?;
    fs::rename(&partial, dest).map_err(|error| {
        let _ = fs::remove_file(&partial);
        format!("atlas.save.failed: {error}")
    })?;
    Ok(())
}

fn atlas_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?
        .join("daena-atlas");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn remove_atlas_path(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("atlas.save.failed: refused to follow a symlink".into());
    }
    if meta.is_file() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

struct JobProgress {
    jobs: SharedAtlasJobs,
    job_id: String,
    app: AppHandle,
    sequence: u64,
}

impl AtlasProgress for JobProgress {
    fn report(
        &mut self,
        phase: AtlasPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), daena_atlas::AtlasError> {
        self.check_cancelled()?;
        self.sequence += 1;
        if let Ok(mut manager) = self.jobs.lock() {
            if let Some(job) = manager.jobs.get_mut(&self.job_id) {
                job.status.stage = phase.label().into();
                job.status.completed = completed;
                job.status.total = total.max(1);
                job.status.sequence = self.sequence;
                job.status.state = match phase {
                    AtlasPhase::Validating => "validating",
                    AtlasPhase::DerivingEpoch => "deriving-epoch",
                    AtlasPhase::RefiningDetail => "refining-detail",
                    AtlasPhase::Rendering => "rendering",
                    AtlasPhase::Encoding => "encoding",
                }
                .into();
                let _ = self.app.emit(ATLAS_PROGRESS_EVENT, job.status.clone());
            }
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), daena_atlas::AtlasError> {
        if let Ok(manager) = self.jobs.lock() {
            if let Some(job) = manager.jobs.get(&self.job_id) {
                if job.cancel.is_cancelled() {
                    return Err(daena_atlas::AtlasError::cancelled());
                }
            }
        }
        Ok(())
    }
}

fn fail_status(status: &mut AtlasJobStatus, error: daena_atlas::AtlasError) {
    status.state = if error.code == CODE_RENDER_CANCELLED {
        "cancelled"
    } else {
        "failed"
    }
    .into();
    status.error = Some(error.message);
    status.error_code = Some(error.code.to_string());
}

fn spawn_render(
    app: AppHandle,
    jobs: SharedAtlasJobs,
    job_id: String,
    snapshot: AtlasRenderSnapshot,
    kind: &'static str,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let cancel = {
            let manager = match jobs.lock() {
                Ok(manager) => manager,
                Err(_) => return,
            };
            match manager.jobs.get(&job_id) {
                Some(job) => job.cancel.clone(),
                None => return,
            }
        };
        let mut progress = JobProgress {
            jobs: jobs.clone(),
            job_id: job_id.clone(),
            app: app.clone(),
            sequence: 0,
        };
        let rendered = render_from_source(
            &snapshot.source_bytes,
            snapshot.identity.as_bytes(),
            &snapshot.request,
            None,
            Some(snapshot.forcing),
            &mut progress,
        );
        let dir = match atlas_dir(&app) {
            Ok(dir) => dir,
            Err(error) => {
                if let Ok(mut manager) = jobs.lock() {
                    if let Some(job) = manager.jobs.get_mut(&job_id) {
                        fail_status(
                            &mut job.status,
                            daena_atlas::AtlasError::new(daena_atlas::CODE_RENDER_FAILED, error),
                        );
                    }
                }
                return;
            }
        };
        let rendered = match rendered {
            Ok(rendered) => rendered,
            Err(error) => {
                if let Ok(mut manager) = jobs.lock() {
                    if let Some(job) = manager.jobs.get_mut(&job_id) {
                        fail_status(&mut job.status, error);
                        let _ = app.emit(ATLAS_PROGRESS_EVENT, job.status.clone());
                    }
                }
                return;
            }
        };
        let path = dir.join(format!("{job_id}.png"));
        if cancel.is_cancelled() {
            if let Ok(mut manager) = jobs.lock() {
                if let Some(job) = manager.jobs.get_mut(&job_id) {
                    fail_status(&mut job.status, daena_atlas::AtlasError::cancelled());
                    let _ = app.emit(ATLAS_PROGRESS_EVENT, job.status.clone());
                }
            }
            return;
        }
        if let Err(error) = fs::write(&path, &rendered.png) {
            if let Ok(mut manager) = jobs.lock() {
                if let Some(job) = manager.jobs.get_mut(&job_id) {
                    fail_status(
                        &mut job.status,
                        daena_atlas::AtlasError::new(
                            daena_atlas::CODE_ENCODER_FAILED,
                            error.to_string(),
                        ),
                    );
                    let _ = app.emit(ATLAS_PROGRESS_EVENT, job.status.clone());
                }
            }
            return;
        }
        if cancel.is_cancelled() {
            let _ = remove_atlas_path(&path);
            if let Ok(mut manager) = jobs.lock() {
                if let Some(job) = manager.jobs.get_mut(&job_id) {
                    fail_status(&mut job.status, daena_atlas::AtlasError::cancelled());
                    let _ = app.emit(ATLAS_PROGRESS_EVENT, job.status.clone());
                }
            }
            return;
        }
        let mut manager = match jobs.lock() {
            Ok(manager) => manager,
            Err(_) => {
                let _ = remove_atlas_path(&path);
                return;
            }
        };
        let Some(job) = manager.jobs.get_mut(&job_id) else {
            let _ = remove_atlas_path(&path);
            return;
        };
        job.artifact = Some(path.clone());
        job.status.state = "ready-to-save".into();
        job.status.stage = "ready-to-save".into();
        job.status.preview_token = Some(path.to_string_lossy().into_owned());
        job.status.width_px = rendered.request.width_px;
        job.status.height_px = rendered.request.height_px;
        job.status.captured_content_generation = Some(snapshot.content_generation);
        job.status.provenance = serde_json::to_value(&rendered.provenance).ok();
        let _ = app.emit(ATLAS_PROGRESS_EVENT, job.status.clone());
        let _ = kind;
    });
}

#[tauri::command]
pub async fn project_atlas_capabilities(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<daena_core::maps::atlas::AtlasRenderCapabilities, String> {
    with_read_project(state, move |project| {
        capabilities_for_map(project, &map_entity_id)
    })
    .await
}

#[tauri::command]
pub async fn project_atlas_preview_begin(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedAtlasJobs>,
    app: AppHandle,
    input: AtlasBeginInput,
) -> Result<AtlasJobStatus, String> {
    begin_job(state, jobs, app, input, "preview").await
}

#[tauri::command]
pub async fn project_atlas_render_begin(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedAtlasJobs>,
    app: AppHandle,
    input: AtlasBeginInput,
) -> Result<AtlasJobStatus, String> {
    begin_job(state, jobs, app, input, "export").await
}

async fn begin_job(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedAtlasJobs>,
    app: AppHandle,
    mut input: AtlasBeginInput,
    kind: &'static str,
) -> Result<AtlasJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before rendering an atlas map".to_string())?
        .root;
    if kind == "preview" {
        input.request.width_px = input.request.width_px.min(PREVIEW_MAX_WIDTH);
        input.request.height_px = input.request.height_px.min(PREVIEW_MAX_HEIGHT);
        if input.request.width_px != input.request.height_px.saturating_mul(2) {
            input.request.height_px = input.request.width_px / 2;
        }
    }
    let request_id = input
        .request_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    Uuid::parse_str(&request_id).map_err(|_| "atlas request ID must be a UUID".to_string())?;
    let map_entity_id = input.map_entity_id.clone();
    let request = input.request.clone();
    let snapshot = with_read_project(state, move |project| {
        capture_snapshot(project, &map_entity_id, request)
    })
    .await?;
    let job_id = Uuid::new_v4().to_string();
    let cancel = Arc::new(CancelFlag::default());
    let status = AtlasJobStatus {
        job_id: job_id.clone(),
        request_id,
        map_entity_id: snapshot.map_entity_id.clone(),
        kind: kind.into(),
        state: "snapshotting".into(),
        stage: "snapshotting".into(),
        completed: 0,
        total: 1,
        sequence: 0,
        error: None,
        error_code: None,
        width_px: snapshot.request.width_px,
        height_px: snapshot.request.height_px,
        preview_token: None,
        captured_content_generation: Some(snapshot.content_generation),
        current_content_generation: Some(snapshot.content_generation),
        provenance: None,
        estimate: Some(snapshot.estimate.clone()),
    };
    {
        let mut manager = jobs
            .lock()
            .map_err(|_| "atlas job state is unavailable".to_string())?;
        manager.reap();
        if kind == "export" {
            if let Some(existing) = manager.export_job_id.clone() {
                if let Some(job) = manager.jobs.get(&existing) {
                    if job.status.state != "failed"
                        && job.status.state != "cancelled"
                        && job.status.state != "saved"
                    {
                        return Err("atlas.resource-limit: one export job may run at a time".into());
                    }
                }
            }
            manager.export_job_id = Some(job_id.clone());
        } else {
            let superseded = manager
                .jobs
                .iter()
                .filter(|(_, job)| {
                    job.kind == "preview" && job.map_entity_id == snapshot.map_entity_id
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in superseded {
                if let Some(job) = manager.jobs.get_mut(&id) {
                    job.cancel.cancel();
                    job.status.state = "cancelled".into();
                    job.status.error_code = Some(CODE_RENDER_CANCELLED.into());
                    if let Some(path) = job.artifact.take() {
                        let _ = remove_atlas_path(&path);
                    }
                }
            }
        }
        manager.jobs.insert(
            job_id.clone(),
            AtlasJob {
                project_id,
                map_entity_id: snapshot.map_entity_id.clone(),
                kind: kind.into(),
                expires_at: Instant::now() + ATLAS_JOB_TTL,
                cancel,
                status: status.clone(),
                artifact: None,
            },
        );
    }
    spawn_render(app, jobs.inner().clone(), job_id, snapshot, kind);
    Ok(status)
}

#[tauri::command]
pub async fn project_atlas_job_status(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedAtlasJobs>,
    job_id: String,
) -> Result<AtlasJobStatus, String> {
    let current_generation = with_read_project(state, |project| project.content_generation())
        .await
        .ok();
    let mut manager = jobs
        .lock()
        .map_err(|_| "atlas job state is unavailable".to_string())?;
    manager.reap();
    let mut status = manager
        .jobs
        .get(&job_id)
        .ok_or_else(|| "atlas job was not found".to_string())?
        .status
        .clone();
    status.current_content_generation = current_generation;
    Ok(status)
}

#[tauri::command]
pub async fn project_atlas_job_cancel(
    jobs: tauri::State<'_, SharedAtlasJobs>,
    job_id: String,
) -> Result<AtlasJobStatus, String> {
    let mut manager = jobs
        .lock()
        .map_err(|_| "atlas job state is unavailable".to_string())?;
    let job = manager
        .jobs
        .get_mut(&job_id)
        .ok_or_else(|| "atlas job was not found".to_string())?;
    job.cancel.cancel();
    if job.status.state != "ready-to-save" && job.status.state != "saved" {
        job.status.state = "cancelled".into();
        job.status.error_code = Some(CODE_RENDER_CANCELLED.into());
        if let Some(path) = job.artifact.take() {
            let _ = remove_atlas_path(&path);
        }
    }
    Ok(job.status.clone())
}

#[tauri::command]
pub async fn project_atlas_artifact_save(
    jobs: tauri::State<'_, SharedAtlasJobs>,
    app: AppHandle,
    job_id: String,
) -> Result<AtlasJobStatus, String> {
    let (artifact, mut status) = {
        let manager = jobs
            .lock()
            .map_err(|_| "atlas job state is unavailable".to_string())?;
        let job = manager
            .jobs
            .get(&job_id)
            .ok_or_else(|| "atlas job was not found".to_string())?;
        if job.status.state != "ready-to-save" {
            return Err("atlas.save.failed: render is not ready to save".into());
        }
        (
            job.artifact
                .clone()
                .ok_or_else(|| "atlas.save.failed: artifact is missing".to_string())?,
            job.status.clone(),
        )
    };
    let bytes = fs::read(&artifact).map_err(|error| format!("atlas.save.failed: {error}"))?;
    let destination = app
        .dialog()
        .file()
        .add_filter("PNG image", &["png"])
        .set_file_name("atlas-map.png")
        .blocking_save_file();
    let Some(file) = destination else {
        return Ok(status);
    };
    let dest = file.into_path().map_err(|error| error.to_string())?;
    install_png_bytes(&dest, &bytes)?;
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    status.state = "saved".into();
    status.stage = "saved".into();
    status.error = Some(format!("saved {digest}"));
    if let Ok(mut manager) = jobs.lock() {
        if let Some(job) = manager.jobs.get_mut(&job_id) {
            job.status = status.clone();
        }
    }
    Ok(status)
}

#[tauri::command]
pub async fn project_atlas_artifact_discard(
    jobs: tauri::State<'_, SharedAtlasJobs>,
    job_id: String,
) -> Result<AtlasJobStatus, String> {
    let mut manager = jobs
        .lock()
        .map_err(|_| "atlas job state is unavailable".to_string())?;
    let job = manager
        .jobs
        .get_mut(&job_id)
        .ok_or_else(|| "atlas job was not found".to_string())?;
    if let Some(path) = job.artifact.take() {
        remove_atlas_path(&path)?;
    }
    job.status.state = "cancelled".into();
    job.status.preview_token = None;
    Ok(job.status.clone())
}

pub fn cancel_atlas_jobs(jobs: &SharedAtlasJobs) -> Result<(), String> {
    jobs.lock()
        .map_err(|_| "atlas job state is unavailable".to_string())?
        .cancel_all();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_slot_and_preview_supersession_are_tracked() {
        let mut manager = AtlasJobManager::default();
        assert!(manager.export_job_id.is_none());
        manager.reap();
        assert!(manager.jobs.is_empty());
    }

    #[test]
    fn atomic_png_save_overwrites_and_refuses_symlinks() {
        let dir = std::env::temp_dir().join(format!("daena-atlas-save-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("atlas-map.png");
        fs::write(&dest, b"old").unwrap();
        install_png_bytes(&dest, b"new-png").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new-png");
        assert!(!dest.with_extension("png.partial").exists());

        let linked = dir.join("linked.png");
        std::os::unix::fs::symlink(&dest, &linked).unwrap();
        let error = install_png_bytes(&linked, b"other").unwrap_err();
        assert!(error.contains("symlink"));
        assert_eq!(fs::read(&dest).unwrap(), b"new-png");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn locked_destination_fails_cleanly_and_retry_succeeds() {
        let dir = std::env::temp_dir().join(format!("daena-atlas-lock-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("atlas-map.png");
        fs::create_dir(&dest).unwrap();
        let error = install_png_bytes(&dest, b"new-png").unwrap_err();
        assert!(error.contains("atlas.save.failed"));
        assert!(dest.is_dir());
        assert!(!dest.with_extension("png.partial").exists());
        fs::remove_dir(&dest).unwrap();
        install_png_bytes(&dest, b"new-png").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new-png");

        let parent = dir.join("ro");
        fs::create_dir(&parent).unwrap();
        let blocked = parent.join("atlas-map.png");
        fs::write(&blocked, b"old").unwrap();
        let mut permissions = fs::metadata(&parent).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o555);
        fs::set_permissions(&parent, permissions).unwrap();
        let error = install_png_bytes(&blocked, b"new-png").unwrap_err();
        assert!(error.contains("atlas.save.failed"));
        let mut permissions = fs::metadata(&parent).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&parent, permissions).unwrap();
        assert_eq!(fs::read(&blocked).unwrap(), b"old");
        install_png_bytes(&blocked, b"new-png").unwrap();
        assert_eq!(fs::read(&blocked).unwrap(), b"new-png");
        let _ = fs::remove_dir_all(&dir);
    }
}
