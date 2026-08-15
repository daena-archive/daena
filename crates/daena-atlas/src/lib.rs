//! Pure CPU atlas renderer.
//!
//! This crate has no Tauri, SQLite, core, plugin-host, or ambient filesystem
//! dependency. Callers supply validated physical bytes, an opaque physical
//! identity, and an explicit output sink.

pub mod detail;
pub mod encode;
pub mod projection;
pub mod provenance;
pub mod render;
pub mod request;
pub mod style;

use std::sync::atomic::{AtomicBool, Ordering};

pub const ATLAS_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const ATLAS_DETAIL_ALGORITHM_VERSION: u32 = 1;
pub const ATLAS_SEED_POLICY_VERSION: u32 = 1;
pub const ATLAS_RENDERER_VERSION: u32 = 1;
pub const ATLAS_PROVENANCE_SCHEMA_VERSION: u32 = 1;
pub const SPIKE_STYLE_ID: &str = "daena-atlas-relief-spike";

pub const CODE_REQUEST_INVALID: &str = "atlas.request.invalid";
pub const CODE_PROVIDER_UNSUPPORTED: &str = "atlas.provider.unsupported";
pub const CODE_RESOURCE_LIMIT: &str = "atlas.resource-limit";
pub const CODE_RENDER_CANCELLED: &str = "atlas.render.cancelled";
pub const CODE_RENDER_FAILED: &str = "atlas.render.failed";
pub const CODE_ENCODER_FAILED: &str = "atlas.encoder.failed";
pub const CODE_SOURCE_INVALID: &str = "atlas.asset.invalid";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasError {
    pub code: &'static str,
    pub message: String,
}

impl AtlasError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(CODE_REQUEST_INVALID, message)
    }

    pub fn cancelled() -> Self {
        Self::new(CODE_RENDER_CANCELLED, "atlas render was cancelled")
    }

    pub fn limit(message: impl Into<String>) -> Self {
        Self::new(CODE_RESOURCE_LIMIT, message)
    }
}

impl std::fmt::Display for AtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AtlasError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasPhase {
    Validating,
    DerivingEpoch,
    RefiningDetail,
    Rendering,
    Encoding,
}

impl AtlasPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Validating => "validating",
            Self::DerivingEpoch => "deriving-epoch",
            Self::RefiningDetail => "refining-detail",
            Self::Rendering => "rendering",
            Self::Encoding => "encoding",
        }
    }
}

pub trait AtlasProgress {
    fn report(&mut self, phase: AtlasPhase, completed: u32, total: u32) -> Result<(), AtlasError>;

    fn check_cancelled(&self) -> Result<(), AtlasError> {
        Ok(())
    }
}

pub struct NoopProgress;

impl AtlasProgress for NoopProgress {
    fn report(
        &mut self,
        _phase: AtlasPhase,
        _completed: u32,
        _total: u32,
    ) -> Result<(), AtlasError> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CancelFlag {
    cancelled: AtomicBool,
}

impl CancelFlag {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

pub struct FlagProgress<'a> {
    pub flag: &'a CancelFlag,
}

impl AtlasProgress for FlagProgress<'_> {
    fn report(
        &mut self,
        _phase: AtlasPhase,
        _completed: u32,
        _total: u32,
    ) -> Result<(), AtlasError> {
        self.check_cancelled()
    }

    fn check_cancelled(&self) -> Result<(), AtlasError> {
        if self.flag.is_cancelled() {
            Err(AtlasError::cancelled())
        } else {
            Ok(())
        }
    }
}

/// Spike-only identity from source bytes. Production identity is constructed
/// exclusively by `daena-core`.
pub fn spike_identity_from_source(source_bytes: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(source_bytes);
    format!("sha256:{digest:x}").into_bytes()
}

pub fn render_from_source(
    source_bytes: &[u8],
    identity: &[u8],
    request: &request::AtlasRenderRequest,
    tile_order: Option<&[u32]>,
    progress: &mut dyn AtlasProgress,
) -> Result<RenderedAtlas, AtlasError> {
    progress.report(AtlasPhase::Validating, 0, 1)?;
    progress.check_cancelled()?;
    let request = request.clone().normalize()?;
    let world = daena_physical::decode_source(source_bytes)
        .map_err(|error| AtlasError::new(CODE_SOURCE_INVALID, error))?;
    let field = world.physical_field();
    let report = daena_physical::validate_field_report(&field)
        .map_err(|error| AtlasError::new(CODE_SOURCE_INVALID, error.to_string()))?;
    progress.report(AtlasPhase::Validating, 1, 1)?;
    progress.report(AtlasPhase::DerivingEpoch, 0, 1)?;
    progress.check_cancelled()?;
    let historical = daena_physical::history::derive_historical_world(
        &field,
        report.reference_water_inventory_m3,
        Some(&world.crust_by_cell),
        daena_physical::history::HistoricalForcingParameters::default_for(
            field.seed,
            field.retry_index,
        ),
        request.offset_years,
        &mut daena_physical::NoopProgress,
    )
    .map_err(|error| AtlasError::new(CODE_RENDER_FAILED, error.to_string()))?;
    progress.check_cancelled()?;
    progress.report(AtlasPhase::DerivingEpoch, 1, 1)?;
    progress.report(AtlasPhase::RefiningDetail, 0, 1)?;
    let model = {
        let mut cancelled = || progress.check_cancelled();
        detail::build_detail_model(
            field.grid,
            &field.elevations_mm,
            identity,
            request.variant,
            request.level,
            &mut cancelled,
        )?
    };
    let sdf = detail::signed_coastal_distance_ppm(
        field.grid,
        &field.elevations_mm,
        historical.metrics.sea_level_mm,
    );
    progress.report(AtlasPhase::RefiningDetail, 1, 1)?;
    let tiles = request.tile_count();
    let order = match tile_order {
        Some(order) => order.to_vec(),
        None => render::forward_tile_order(tiles),
    };
    let rgba = render::render_rgba(
        &request,
        &model,
        &historical.hydrology,
        &sdf,
        &order,
        progress,
    )?;
    let source_sha256 = {
        use sha2::{Digest, Sha256};
        format!("sha256:{:x}", Sha256::digest(source_bytes))
    };
    let provenance = provenance::AtlasRenderProvenanceV1::spike(&request, identity, &source_sha256);
    let png = encode::encode_png(&request, &rgba, &provenance, progress)?;
    Ok(RenderedAtlas {
        png,
        rgba,
        provenance,
        request,
    })
}

#[derive(Debug, Clone)]
pub struct RenderedAtlas {
    pub png: Vec<u8>,
    pub rgba: Vec<u8>,
    pub provenance: provenance::AtlasRenderProvenanceV1,
    pub request: request::AtlasRenderRequest,
}

#[cfg(test)]
pub(crate) fn golden_world() -> &'static daena_physical::GeneratedWorld {
    use daena_physical::{generate_world, GenerationSettings, NoopProgress, DEFAULT_RADIUS_METRES};
    use std::sync::OnceLock;
    static WORLD: OnceLock<daena_physical::GeneratedWorld> = OnceLock::new();
    WORLD.get_or_init(|| {
        let settings = GenerationSettings {
            width: daena_physical::DEFAULT_WIDTH,
            height: daena_physical::DEFAULT_HEIGHT,
            radius_metres: DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        generate_world(settings, 831_429, 0, &mut NoopProgress).expect("golden world")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::AtlasRenderRequest;

    #[test]
    fn tiled_and_shuffled_renders_match_and_png_carries_provenance() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let request = AtlasRenderRequest::spike_png(1024, 512).unwrap();
        let forward =
            render_from_source(&world.source, &identity, &request, None, &mut NoopProgress)
                .unwrap();
        let reverse = render_from_source(
            &world.source,
            &identity,
            &request,
            Some(&render::reverse_tile_order(request.tile_count())),
            &mut NoopProgress,
        )
        .unwrap();
        let shuffled = render_from_source(
            &world.source,
            &identity,
            &request,
            Some(&render::shuffled_tile_order(request.tile_count(), 7)),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(forward.rgba, reverse.rgba);
        assert_eq!(forward.rgba, shuffled.rgba);
        assert_eq!(forward.png, reverse.png);
        let decoded = encode::decode_png(&forward.png).unwrap();
        assert_eq!(decoded.width, 1024);
        assert_eq!(decoded.height, 512);
        assert_eq!(decoded.rgba, forward.rgba);
        assert!(forward.provenance.physical_identity.starts_with("sha256:"));
        assert!(!String::from_utf8_lossy(&forward.png).contains("http://"));
    }

    #[test]
    fn geographic_samples_match_across_output_sizes() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let small = render_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(256, 128).unwrap(),
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let large = render_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(512, 256).unwrap(),
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let mut cancel = || Ok(());
        let model = detail::build_detail_model(
            world.field.grid,
            &world.field.elevations_mm,
            &identity,
            0,
            request::DetailLevel::Detailed,
            &mut cancel,
        )
        .unwrap();
        assert_eq!(
            model.residual_at(12_345_678, -8_000_000),
            model.residual_at(12_345_678, -8_000_000)
        );
        assert_eq!(
            small.provenance.detail_algorithm_version,
            large.provenance.detail_algorithm_version
        );
        assert_ne!(small.png, large.png);
    }

    #[test]
    fn cancellation_stops_before_ready_to_save() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let flag = CancelFlag::default();
        flag.cancel();
        let error = render_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(64, 32).unwrap(),
            None,
            &mut FlagProgress { flag: &flag },
        )
        .unwrap_err();
        assert_eq!(error.code, CODE_RENDER_CANCELLED);
    }
}
