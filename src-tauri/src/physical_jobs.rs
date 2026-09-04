// Physical-world job management.
use super::*;

pub(super) const PHYSICAL_JOB_TTL: Duration = Duration::from_mins(15);
pub(super) const PHYSICAL_HISTORICAL_PROGRESS_EVENT: &str = "physical-historical-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PhysicalJobStatus {
    pub(super) job_id: String,
    pub(super) request_id: String,
    pub(super) state: String,
    pub(super) stage: String,
    pub(super) completed: u32,
    pub(super) total: u32,
    pub(super) error: Option<String>,
    pub(super) error_code: Option<String>,
    pub(super) physical_identity: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct PhysicalJobResult {
    pub(super) source: Vec<u8>,
    pub(super) generation: serde_json::Value,
    pub(super) physical_identity: String,
    pub(super) derived_geojson: String,
    pub(super) climate: daena_physical::climate::ClimateField,
    pub(super) evolution: daena_physical::evolution::EvolutionField,
    pub(super) hydrology: daena_physical::hydrology::HydrologyField,
}

pub(super) struct PhysicalJob {
    pub(super) project_id: String,
    pub(super) session_id: String,
    pub(super) expires_at: Instant,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) status: PhysicalJobStatus,
    pub(super) result: Option<PhysicalJobResult>,
}

#[derive(Default)]
pub(super) struct PhysicalJobManager {
    pub(super) jobs: BTreeMap<String, PhysicalJob>,
    pub(super) active_session: Option<PhysicalSession>,
}

pub(super) struct PhysicalSession {
    pub(super) id: String,
    pub(super) project_id: String,
}

impl PhysicalJobManager {
    pub(super) fn reap_expired(&mut self) {
        let now = Instant::now();
        self.jobs.retain(|_, job| {
            if job.expires_at <= now {
                job.cancel.store(true, Ordering::Relaxed);
                false
            } else {
                true
            }
        });
    }

    pub(super) fn cancel_all(&mut self) {
        for job in self.jobs.values() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.jobs.clear();
        self.active_session = None;
    }

    pub(super) fn begin_session(&mut self, project_id: String) -> String {
        self.cancel_all();
        let id = uuid::Uuid::new_v4().to_string();
        self.active_session = Some(PhysicalSession {
            id: id.clone(),
            project_id,
        });
        id
    }

    pub(super) fn ensure_session(&mut self, project_id: &str) -> String {
        self.reap_expired();
        if let Some(session) = &self.active_session {
            if session.project_id == project_id {
                return session.id.clone();
            }
        }
        self.begin_session(project_id.to_string())
    }

    pub(super) fn active_session_matches(&self, project_id: &str, session_id: &str) -> bool {
        self.active_session
            .as_ref()
            .is_some_and(|session| session.project_id == project_id && session.id == session_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PhysicalGenerationSettingsInput {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) radius_metres: u64,
    pub(super) target_land_fraction_ppm: u32,
    #[serde(default)]
    pub(super) planetary: daena_physical::planetary::PlanetaryConfiguration,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PhysicalGenerationInput {
    pub(super) seed: u32,
    pub(super) retry_index: u32,
    #[serde(default)]
    pub(super) evolution_preset: Option<String>,
    pub(super) settings: PhysicalGenerationSettingsInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MaterializedPhysicalEvent {
    pub(super) entity_id: String,
    #[serde(flatten)]
    pub(super) event: daena_physical::events::MaterializedEvent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PhysicalEventMaterializationResult {
    pub(super) request_id: String,
    pub(super) map_entity_id: String,
    pub(super) materialization_version: u16,
    pub(super) hazard_derivation_version: u16,
    pub(super) prediction: bool,
    pub(super) events: Vec<MaterializedPhysicalEvent>,
}

pub(super) struct PhysicalProgress {
    pub(super) jobs: SharedPhysicalJobs,
    pub(super) job_id: String,
    pub(super) cancel: Arc<AtomicBool>,
}

pub(super) struct HistoricalProgress {
    pub(super) generation: Arc<AtomicU64>,
    pub(super) expected: u64,
    pub(super) reporter: Option<HistoricalProgressReporter>,
}

#[derive(Clone)]
pub(super) struct HistoricalProgressReporter {
    pub(super) app: tauri::AppHandle,
    pub(super) map_entity_id: String,
    pub(super) request_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HistoricalProgressEvent {
    pub(super) map_entity_id: String,
    pub(super) request_id: String,
    pub(super) phase: String,
    pub(super) completed: u32,
    pub(super) total: u32,
}

impl HistoricalProgress {
    pub(super) fn with_reporter(
        generation: Arc<AtomicU64>,
        expected: u64,
        app: tauri::AppHandle,
        map_entity_id: String,
        request_id: String,
    ) -> Self {
        Self {
            generation,
            expected,
            reporter: Some(HistoricalProgressReporter {
                app,
                map_entity_id,
                request_id,
            }),
        }
    }
}

impl daena_physical::ProgressSink for HistoricalProgress {
    fn report(
        &mut self,
        phase: daena_physical::ProgressPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), daena_physical::PhysicalError> {
        self.check_cancelled()?;
        if let Some(reporter) = &self.reporter {
            let _ = reporter.app.emit(
                PHYSICAL_HISTORICAL_PROGRESS_EVENT,
                HistoricalProgressEvent {
                    map_entity_id: reporter.map_entity_id.clone(),
                    request_id: reporter.request_id.clone(),
                    phase: phase.label().into(),
                    completed,
                    total,
                },
            );
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), daena_physical::PhysicalError> {
        if self.generation.load(Ordering::Acquire) != self.expected {
            return Err(daena_physical::PhysicalError::Cancelled);
        }
        Ok(())
    }
}

impl daena_physical::ProgressSink for PhysicalProgress {
    fn report(
        &mut self,
        phase: daena_physical::ProgressPhase,
        completed: u32,
        total: u32,
    ) -> Result<(), daena_physical::PhysicalError> {
        self.check_cancelled()?;
        let mut manager = self.jobs.lock().map_err(|_| {
            daena_physical::PhysicalError::Validation("job state is unavailable".into())
        })?;
        manager.reap_expired();
        if let Some(job) = manager.jobs.get_mut(&self.job_id) {
            job.status.stage = phase.label().into();
            job.status.completed = completed;
            job.status.total = total;
        }
        Ok(())
    }

    fn check_cancelled(&self) -> Result<(), daena_physical::PhysicalError> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(daena_physical::PhysicalError::Cancelled)
        } else {
            Ok(())
        }
    }
}

pub(super) fn current_session(core: &SharedCore) -> Result<Arc<ProjectSession>, String> {
    core.lock()
        .map_err(|_| "project lifecycle lock poisoned".to_string())
        .map(|session| session.clone())
}

pub(super) fn current_info(core: &SharedCore) -> Result<Option<ProjectInfo>, String> {
    let session = current_session(core)?;
    let core = session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    Ok(core.info())
}

/// Project-level AI opt-in gate. Reads the authoritative runtime database on
/// every decision and fails closed when the project cannot be opened.
pub(crate) fn ensure_project_ai_enabled(project_root: &str) -> Result<(), String> {
    let enabled = ProjectStore::open_read_only(project_root)
        .ok()
        .and_then(|project| project.ai_enabled().ok())
        .unwrap_or(false);
    if !enabled {
        return Err("AI is disabled for this project. Enable AI in Settings first.".into());
    }
    Ok(())
}

pub(super) fn cancel_physical_jobs(jobs: &SharedPhysicalJobs) -> Result<(), String> {
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .cancel_all();
    if let Some(atlas) = ATLAS_JOBS.get() {
        atlas_jobs::cancel_atlas_jobs(atlas)?;
    }
    if let Some(studio) = ATLAS_STUDIO.get() {
        atlas_studio::cancel_atlas_studio(studio)?;
    }
    Ok(())
}

pub(super) fn begin_physical_session(
    jobs: &SharedPhysicalJobs,
    project_id: String,
) -> Result<(), String> {
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .begin_session(project_id);
    Ok(())
}

pub(super) fn begin_current_physical_session(
    jobs: &SharedPhysicalJobs,
    core: &SharedCore,
) -> Result<(), String> {
    if let Some(project_id) = current_info(core)?.map(|info| info.root) {
        begin_physical_session(jobs, project_id)
    } else {
        cancel_physical_jobs(jobs)
    }
}
