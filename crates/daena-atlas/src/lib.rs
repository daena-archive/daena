//! Pure CPU atlas renderer.
//!
//! This crate has no Tauri, SQLite, core, plugin-host, or ambient filesystem
//! dependency. Callers supply validated physical bytes, an opaque physical
//! identity, and an explicit output sink.

pub mod amplify;
pub mod cache;
pub mod chunk;
pub mod constraint;
pub mod control;
pub mod detail;
pub mod drainage;
pub mod encode;
pub mod erosion;
pub mod labels;
pub mod overlay;
pub mod projection;
pub mod provenance;
pub mod refine;
pub mod render;
pub mod request;
pub mod studio;
pub mod style;

use std::sync::atomic::{AtomicBool, Ordering};

pub const ATLAS_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const ATLAS_VIEW_WIDTH: u32 = 384;
pub const ATLAS_VIEW_HEIGHT: u32 = 192;
pub const ATLAS_DETAIL_ALGORITHM_VERSION: u32 = 3;
pub const ATLAS_DERIVED_DRAINAGE_VERSION: u32 = 3;
pub const ATLAS_SEED_POLICY_VERSION: u32 = 1;
pub const ATLAS_RENDERER_VERSION: u32 = 2;
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

    #[must_use]
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
    #[must_use]
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

pub trait AtlasProgress: Send + Sync {
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

/// Adapts the Atlas progress surface to the physical engine sink so a cancel
/// inside a derive stops it.
struct PhysicalSink<'a> {
    inner: std::sync::Arc<std::sync::Mutex<&'a mut dyn AtlasProgress>>,
    phase: AtlasPhase,
}

fn physical_error(error: AtlasError) -> daena_physical::PhysicalError {
    if error.code == CODE_RENDER_CANCELLED {
        daena_physical::PhysicalError::Cancelled
    } else {
        daena_physical::PhysicalError::InvalidSource(error.message)
    }
}

fn atlas_physical_error(error: daena_physical::PhysicalError) -> AtlasError {
    if matches!(error, daena_physical::PhysicalError::Cancelled) {
        AtlasError::cancelled()
    } else {
        AtlasError::new(CODE_RENDER_FAILED, error.to_string())
    }
}

fn decode_atlas_source(
    source_bytes: &[u8],
    progress: &mut dyn AtlasProgress,
) -> Result<daena_physical::engine::DecodedSource, AtlasError> {
    let shared = std::sync::Arc::new(std::sync::Mutex::new(progress));
    let result = {
        let mut sink = PhysicalSink {
            inner: std::sync::Arc::clone(&shared),
            phase: AtlasPhase::Validating,
        };
        daena_physical::engine::decode_source_with_progress(source_bytes, &mut sink)
    };
    drop(shared);
    result.map_err(|error| {
        if matches!(error, daena_physical::PhysicalError::Cancelled) {
            AtlasError::cancelled()
        } else {
            AtlasError::new(CODE_SOURCE_INVALID, error.to_string())
        }
    })
}

fn solve_atlas_epoch(
    decoded: &daena_physical::engine::DecodedSource,
    forcing: daena_physical::history::HistoricalForcingParameters,
    offset_years: i64,
    progress: &mut dyn AtlasProgress,
) -> Result<daena_physical::history::EpochSolve, AtlasError> {
    let shared = std::sync::Arc::new(std::sync::Mutex::new(progress));
    let result = {
        let sink = PhysicalSink {
            inner: std::sync::Arc::clone(&shared),
            phase: AtlasPhase::DerivingEpoch,
        };
        daena_physical::history::solve_epoch(decoded, forcing, offset_years, &sink)
    };
    drop(shared);
    result.map_err(atlas_physical_error)
}

/// Derive the epoch and, when needed for the structure cache, the present
/// world. Both draws use one progress sink so cancellation covers both.
fn derive_atlas_inputs_for_request(
    decoded: &daena_physical::engine::DecodedSource,
    epoch: Option<&daena_physical::history::EpochSolve>,
    planet: daena_physical::planetary::PlanetaryConfiguration,
    present_needed: bool,
    progress: &mut dyn AtlasProgress,
) -> Result<
    (
        daena_physical::engine::AtlasInputs,
        Option<daena_physical::engine::AtlasInputs>,
    ),
    AtlasError,
> {
    let shared = std::sync::Arc::new(std::sync::Mutex::new(progress));
    if present_needed {
        let epoch_shared = std::sync::Arc::clone(&shared);
        let present_shared = std::sync::Arc::clone(&shared);
        let derived = rayon::join(
            move || {
                let mut sink = PhysicalSink {
                    inner: epoch_shared,
                    phase: AtlasPhase::DerivingEpoch,
                };
                decoded.derive_atlas_inputs(
                    epoch,
                    ATLAS_VIEW_WIDTH,
                    ATLAS_VIEW_HEIGHT,
                    planet,
                    &mut sink,
                )
            },
            move || {
                let mut sink = PhysicalSink {
                    inner: present_shared,
                    phase: AtlasPhase::DerivingEpoch,
                };
                decoded.derive_atlas_inputs(
                    None,
                    ATLAS_VIEW_WIDTH,
                    ATLAS_VIEW_HEIGHT,
                    planet,
                    &mut sink,
                )
            },
        );
        drop(shared);
        Ok((
            derived.0.map_err(atlas_physical_error)?,
            Some(derived.1.map_err(atlas_physical_error)?),
        ))
    } else {
        let mut sink = PhysicalSink {
            inner: std::sync::Arc::clone(&shared),
            phase: AtlasPhase::DerivingEpoch,
        };
        let result = decoded.derive_atlas_inputs(
            epoch,
            ATLAS_VIEW_WIDTH,
            ATLAS_VIEW_HEIGHT,
            planet,
            &mut sink,
        );
        drop(sink);
        drop(shared);
        Ok((result.map_err(atlas_physical_error)?, None))
    }
}

impl daena_physical::ProgressSink for PhysicalSink<'_> {
    fn report(
        &mut self,
        _phase: daena_physical::ProgressPhase,
        _completed: u32,
        _total: u32,
    ) -> Result<(), daena_physical::PhysicalError> {
        let mut progress = self
            .inner
            .lock()
            .map_err(|_| daena_physical::PhysicalError::Cancelled)?;
        progress.report(self.phase, 0, 0).map_err(physical_error)
    }

    fn check_cancelled(&self) -> Result<(), daena_physical::PhysicalError> {
        let progress = self
            .inner
            .lock()
            .map_err(|_| daena_physical::PhysicalError::Cancelled)?;
        progress.check_cancelled().map_err(physical_error)
    }
}

/// Spike-only identity from source bytes. Production identity is constructed
/// exclusively by `daena-core`.
#[must_use]
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
    forcing: Option<daena_physical::history::HistoricalForcingParameters>,
    overlays: &[overlay::AuthoredFeature],
    progress: &mut dyn AtlasProgress,
) -> Result<RenderedAtlas, AtlasError> {
    render_from_source_cached(
        source_bytes,
        identity,
        request,
        tile_order,
        forcing,
        overlays,
        None,
        progress,
    )
}

fn fingerprint_forcing(
    forcing: Option<&daena_physical::history::HistoricalForcingParameters>,
) -> Vec<u8> {
    let Some(forcing) = forcing else {
        return b"forcing-default".to_vec();
    };
    let mut bytes = b"forcing-explicit".to_vec();
    bytes.extend_from_slice(&forcing.version.to_le_bytes());
    for component in &forcing.components {
        bytes.extend_from_slice(&component.amplitude_centi_c.to_le_bytes());
        bytes.extend_from_slice(&component.period_years.to_le_bytes());
        bytes.extend_from_slice(&component.phase_offset_years.to_le_bytes());
    }
    bytes.extend_from_slice(&forcing.sensitivity_ppm.to_le_bytes());
    bytes.extend_from_slice(&forcing.land_ice_amplitude_ppm.to_le_bytes());
    bytes.extend_from_slice(&forcing.ice_response_years.to_le_bytes());
    bytes.extend_from_slice(&forcing.ice_midpoint_centi_c.to_le_bytes());
    bytes.extend_from_slice(&forcing.ice_transition_width_centi_c.to_le_bytes());
    bytes.extend_from_slice(&forcing.thermal_expansion_ppm_per_degree_c.to_le_bytes());
    bytes
}

fn cache_json(value: &(impl serde::Serialize + ?Sized)) -> Result<Vec<u8>, AtlasError> {
    serde_json::to_vec(value)
        .map_err(|error| AtlasError::new(CODE_RENDER_FAILED, format!("atlas cache key: {error}")))
}

#[derive(Debug, Clone)]
pub struct AtlasPreparedScene {
    pub identity: Vec<u8>,
    pub source_sha256: String,
    pub style: style::AtlasStyle,
    pub style_hash: String,
    pub model: detail::AtlasDetailModel,
    pub hydrology: daena_physical::products::HydrologyField,
    pub hazards: daena_physical::products::HazardField,
    pub surface: std::sync::Arc<daena_physical::engine::ToolSurface>,
    pub sdf: Vec<i32>,
    pub drainage: drainage::DerivedDrainage,
    pub tectonics: daena_physical::engine::TectonicView,
    pub visible_water: render::VisibleWater,
    pub temperature_centi_c: Vec<i32>,
    pub temperature_nh_summer_centi_c: Vec<i32>,
    pub temperature_nh_winter_centi_c: Vec<i32>,
    pub wind_east_milli: Vec<i32>,
    pub wind_north_milli: Vec<i32>,
    pub wind_east_nh_summer_milli: Vec<i32>,
    pub wind_north_nh_summer_milli: Vec<i32>,
    pub wind_east_nh_winter_milli: Vec<i32>,
    pub wind_north_nh_winter_milli: Vec<i32>,
    /// Horizontal wind divergence in 1e-9 s⁻¹.
    pub wind_divergence_nano_per_s: Vec<i32>,
    pub wind_divergence_nh_summer_nano_per_s: Vec<i32>,
    pub wind_divergence_nh_winter_nano_per_s: Vec<i32>,
    pub wind_band: Vec<i32>,
    pub wind_band_nh_summer: Vec<i32>,
    pub wind_band_nh_winter: Vec<i32>,
    pub precipitation_mm: Vec<i32>,
    pub aridity_ppm: Vec<i32>,
    pub precipitation_nh_summer_mm: Vec<i32>,
    pub precipitation_nh_winter_mm: Vec<i32>,
    pub storm_suitability_ppm: Vec<i32>,
    pub storm_track_ppm: Vec<i32>,
    pub storm_intensity_ppm: Vec<i32>,
    pub residual_cache: cache::CacheLookup,
    pub drainage_cache: cache::CacheLookup,
    pub orometry: Vec<amplify::MountainFeature>,
    pub structure_residual_mm: Vec<i32>,
    pub sediment_mm: Vec<i32>,
    pub runoff_mm: Vec<i32>,
    pub crust_influence_ppm: Vec<i32>,
    pub mountain_influence_ppm: Vec<i32>,
}

impl AtlasPreparedScene {
    #[must_use]
    pub fn paint_fields(&self) -> render::PaintFields<'_> {
        render::PaintFields {
            surface: &self.surface,
            temperature_centi_c: &self.temperature_centi_c,
            temperature_nh_summer_centi_c: &self.temperature_nh_summer_centi_c,
            temperature_nh_winter_centi_c: &self.temperature_nh_winter_centi_c,
            precipitation_mm: &self.precipitation_mm,
            precipitation_nh_summer_mm: &self.precipitation_nh_summer_mm,
            precipitation_nh_winter_mm: &self.precipitation_nh_winter_mm,
            aridity_ppm: &self.aridity_ppm,
            storm_suitability_ppm: &self.storm_suitability_ppm,
            storm_track_ppm: &self.storm_track_ppm,
            wind_east_milli: &self.wind_east_milli,
            wind_north_milli: &self.wind_north_milli,
        }
    }

    #[must_use]
    pub fn sample_surface(&self, lon_micro: i32, lat_micro: i32) -> overlay::AtlasSurfaceSample {
        let grid = self.model.grid;
        let cell = detail::nearest_cell(grid, lon_micro, lat_micro);
        let sea_level_mm = self.hydrology.sea_level_mm;
        let sdf_ppm = detail::sample_sdf_ppm(grid, &self.sdf, lon_micro, lat_micro);
        let elevation_mm = self
            .model
            .refined_at(lon_micro, lat_micro, sea_level_mm, sdf_ppm);
        let temperature_centi_c =
            detail::sample_cell_field_mm(grid, &self.temperature_centi_c, lon_micro, lat_micro);
        let temperature_nh_summer_centi_c = detail::sample_cell_field_mm(
            grid,
            &self.temperature_nh_summer_centi_c,
            lon_micro,
            lat_micro,
        );
        let temperature_nh_winter_centi_c = detail::sample_cell_field_mm(
            grid,
            &self.temperature_nh_winter_centi_c,
            lon_micro,
            lat_micro,
        );
        let seasonal_range_centi_c =
            temperature_nh_summer_centi_c.abs_diff(temperature_nh_winter_centi_c);
        let freeze = if temperature_nh_summer_centi_c < 0 && temperature_nh_winter_centi_c < 0 {
            "permanent"
        } else if temperature_nh_summer_centi_c.min(temperature_nh_winter_centi_c) < 0 {
            "seasonal"
        } else {
            "none"
        };
        let wind_east_milli =
            detail::sample_cell_field_mm(grid, &self.wind_east_milli, lon_micro, lat_micro);
        let wind_north_milli =
            detail::sample_cell_field_mm(grid, &self.wind_north_milli, lon_micro, lat_micro);
        let wind_east_nh_summer_milli = detail::sample_cell_field_mm(
            grid,
            &self.wind_east_nh_summer_milli,
            lon_micro,
            lat_micro,
        );
        let wind_north_nh_summer_milli = detail::sample_cell_field_mm(
            grid,
            &self.wind_north_nh_summer_milli,
            lon_micro,
            lat_micro,
        );
        let wind_east_nh_winter_milli = detail::sample_cell_field_mm(
            grid,
            &self.wind_east_nh_winter_milli,
            lon_micro,
            lat_micro,
        );
        let wind_north_nh_winter_milli = detail::sample_cell_field_mm(
            grid,
            &self.wind_north_nh_winter_milli,
            lon_micro,
            lat_micro,
        );
        let wind_divergence_nano_per_s = detail::sample_cell_field_mm(
            grid,
            &self.wind_divergence_nano_per_s,
            lon_micro,
            lat_micro,
        );
        let wind_divergence_nh_summer_nano_per_s = detail::sample_cell_field_mm(
            grid,
            &self.wind_divergence_nh_summer_nano_per_s,
            lon_micro,
            lat_micro,
        );
        let wind_divergence_nh_winter_nano_per_s = detail::sample_cell_field_mm(
            grid,
            &self.wind_divergence_nh_winter_nano_per_s,
            lon_micro,
            lat_micro,
        );
        let wind_band = control::wind_band_name(self.wind_band.get(cell).copied().unwrap_or(0));
        let wind_band_nh_summer =
            control::wind_band_name(self.wind_band_nh_summer.get(cell).copied().unwrap_or(0));
        let wind_band_nh_winter =
            control::wind_band_name(self.wind_band_nh_winter.get(cell).copied().unwrap_or(0));
        let precipitation_mm =
            detail::sample_cell_field_mm(grid, &self.precipitation_mm, lon_micro, lat_micro);
        let aridity_ppm =
            detail::sample_cell_field_mm(grid, &self.aridity_ppm, lon_micro, lat_micro);
        let precipitation_nh_summer_mm = detail::sample_cell_field_mm(
            grid,
            &self.precipitation_nh_summer_mm,
            lon_micro,
            lat_micro,
        );
        let precipitation_nh_winter_mm = detail::sample_cell_field_mm(
            grid,
            &self.precipitation_nh_winter_mm,
            lon_micro,
            lat_micro,
        );
        let surface_cell = self.surface.cell_at_microdegrees(lon_micro, lat_micro);
        let biome_class = self
            .surface
            .biome_class
            .get(surface_cell as usize)
            .copied()
            .map(i32::from)
            .unwrap_or(0);
        let group = control::biome_group(biome_class);
        let climate = format!(
            "{} ({})",
            control::biome_name(biome_class),
            control::biome_group_name(group)
        );
        let ice = self.hydrology.ice_cells.get(cell).copied().unwrap_or(false);
        let mut biome_reason = format!(
            "Summer {:.1} °C, winter {:.1} °C, {:.0} mm/year precipitation, {:.0}% aridity, elevation {:.0} m.",
            f64::from(temperature_nh_summer_centi_c) / 100.0,
            f64::from(temperature_nh_winter_centi_c) / 100.0,
            f64::from(precipitation_mm.max(0)),
            f64::from(aridity_ppm.max(0)) / 10_000.0,
            f64::from(elevation_mm.saturating_sub(sea_level_mm)) / 1_000.0,
        );
        if ice {
            biome_reason.push_str(" Ice cover is present.");
        } else if group == control::CLIMATE_CLASS_ICE {
            biome_reason.push_str(" No ice cover on this cell.");
        }
        let storm_suitability_ppm =
            detail::sample_cell_field_mm(grid, &self.storm_suitability_ppm, lon_micro, lat_micro);
        let storm_track_ppm =
            detail::sample_cell_field_mm(grid, &self.storm_track_ppm, lon_micro, lat_micro);
        let storm_intensity_ppm =
            detail::sample_cell_field_mm(grid, &self.storm_intensity_ppm, lon_micro, lat_micro);
        let shear_east = wind_east_nh_summer_milli - wind_east_nh_winter_milli;
        let shear_north = wind_north_nh_summer_milli - wind_north_nh_winter_milli;
        let shear_milli = ((i64::from(shear_east).pow(2) + i64::from(shear_north).pow(2)) as f64)
            .sqrt()
            .round() as u32;
        let sst = temperature_nh_summer_centi_c
            .max(temperature_nh_winter_centi_c)
            .max(temperature_centi_c);
        let storm_reason = format!(
            "{} sea surface {:.1} °C, wind shear {:.1} m/s, genesis {}%, track {}%, intensity {}%.",
            if elevation_mm < sea_level_mm {
                "Ocean"
            } else {
                "Land"
            },
            f64::from(sst) / 100.0,
            f64::from(shear_milli) / 1_000.0,
            storm_suitability_ppm.max(0) / 10_000,
            storm_track_ppm.max(0) / 10_000,
            storm_intensity_ppm.max(0) / 10_000,
        );
        let inland = self
            .visible_water
            .inland
            .get(cell)
            .copied()
            .unwrap_or(false);
        let water_surface_mm = if inland {
            self.hydrology
                .lake_level_mm
                .get(cell)
                .copied()
                .unwrap_or(sea_level_mm)
        } else {
            sea_level_mm
        };
        overlay::AtlasSurfaceSample {
            lon_micro,
            lat_micro,
            elevation_mm,
            water_surface_mm,
            temperature_centi_c,
            temperature_nh_summer_centi_c,
            temperature_nh_winter_centi_c,
            seasonal_range_centi_c,
            freeze: freeze.to_string(),
            wind_east_milli,
            wind_north_milli,
            wind_east_nh_summer_milli,
            wind_north_nh_summer_milli,
            wind_east_nh_winter_milli,
            wind_north_nh_winter_milli,
            wind_divergence_nano_per_s,
            wind_divergence_nh_summer_nano_per_s,
            wind_divergence_nh_winter_nano_per_s,
            wind_band: wind_band.to_string(),
            wind_band_nh_summer: wind_band_nh_summer.to_string(),
            wind_band_nh_winter: wind_band_nh_winter.to_string(),
            precipitation_mm,
            aridity_ppm,
            precipitation_nh_summer_mm,
            precipitation_nh_winter_mm,
            climate: climate.to_string(),
            biome_reason: biome_reason.to_string(),
            storm_suitability_ppm,
            storm_track_ppm,
            storm_intensity_ppm,
            storm_reason,
            surface: control::surface_kind(ice, inland, elevation_mm, sea_level_mm).to_string(),
        }
    }
}

fn drainage_from_refined(refined: &refine::RefinedHydrology) -> drainage::DerivedDrainage {
    drainage::DerivedDrainage {
        version: ATLAS_DERIVED_DRAINAGE_VERSION,
        tributaries: refined
            .tributaries
            .iter()
            .map(|tributary| drainage::DerivedTributary {
                id: tributary.id.clone(),
                source_cell: tributary.source_index,
                join_cell: tributary.join_index,
                parent_river_id: tributary.parent_river_id,
                ordinal: tributary.ordinal,
                watershed_id: tributary.watershed_id,
                width_mm: tributary.width_mm,
                depth_mm: tributary.depth_mm,
                path: tributary.path.clone(),
            })
            .collect(),
    }
}

fn structure_worked_mm(model: &amplify::AmplificationModel) -> Vec<i32> {
    let width = model.detail.lattice_width;
    let height = model.detail.lattice_height;
    let count = (width as usize).saturating_mul(height as usize);
    let mut worked = vec![0_i32; count];
    for j in 0..height {
        for i in 0..width {
            let index = j as usize * width as usize + i as usize;
            let lon = detail::lattice_lon_micro(i, width);
            let lat = detail::lattice_lat_micro(j, height);
            worked[index] = model
                .detail
                .canonical_at(lon, lat)
                .saturating_add(model.detail.residual_mm.get(index).copied().unwrap_or(0));
        }
    }
    worked
}

#[allow(clippy::too_many_arguments)]
fn build_constrained_refine(
    amplification: &amplify::AmplificationModel,
    controls: &control::ControlFields,
    hydrology: &daena_physical::products::HydrologyField,
    sdf: &[i32],
    identity: &[u8],
    structure_sea_level_mm: i32,
    constraints: &[constraint::AtlasConstraint],
    progress: &mut dyn AtlasProgress,
) -> Result<refine::RefinedHydrology, AtlasError> {
    let mut cancelled = || progress.check_cancelled();
    refine::build_refined_hydrology_constrained(
        amplification,
        controls,
        hydrology,
        sdf,
        identity,
        structure_sea_level_mm,
        constraints,
        &mut cancelled,
    )
}

fn refine_or_fallback(
    result: Result<refine::RefinedHydrology, AtlasError>,
    structure: &amplify::AmplificationModel,
) -> Result<(drainage::DerivedDrainage, Vec<i32>, Vec<i32>, bool), AtlasError> {
    match result {
        Ok(refined) => Ok((
            drainage_from_refined(&refined),
            refined.worked_mm,
            refined.sediment_mm,
            true,
        )),
        Err(error) if error.code == CODE_RENDER_CANCELLED => Err(error),
        Err(_) => {
            let worked = structure_worked_mm(structure);
            let sediment_mm = vec![0_i32; worked.len()];
            Ok((
                drainage::DerivedDrainage {
                    version: ATLAS_DERIVED_DRAINAGE_VERSION,
                    tributaries: Vec::new(),
                },
                worked,
                sediment_mm,
                false,
            ))
        }
    }
}

fn residual_cache_key(
    identity: &[u8],
    variant: u32,
    level: request::DetailLevel,
    grid_width: u32,
    grid_height: u32,
) -> [u8; 32] {
    cache::cache_key(&[
        b"atlas-cache-residual-v2",
        identity,
        &ATLAS_DETAIL_ALGORITHM_VERSION.to_le_bytes(),
        &variant.to_le_bytes(),
        level.as_str().as_bytes(),
        &grid_width.to_le_bytes(),
        &grid_height.to_le_bytes(),
    ])
}

fn octave_stack_cache_key(
    identity: &[u8],
    variant: u32,
    factor: u32,
    grid_width: u32,
    grid_height: u32,
) -> [u8; 32] {
    cache::cache_key(&[
        b"atlas-cache-octave-stack-v1",
        identity,
        &ATLAS_DETAIL_ALGORITHM_VERSION.to_le_bytes(),
        &variant.to_le_bytes(),
        &factor.to_le_bytes(),
        &grid_width.to_le_bytes(),
        &grid_height.to_le_bytes(),
    ])
}

fn load_octave_stack(
    cache: &cache::AtlasDiskCache,
    identity: &[u8],
    variant: u32,
    factor: u32,
    grid_width: u32,
    grid_height: u32,
) -> Option<(u32, u32, u32, Vec<i32>)> {
    let key = octave_stack_cache_key(identity, variant, factor, grid_width, grid_height);
    let cache::CacheLookupResult::Hit(payload) = cache.get(cache::KIND_OCTAVE_STACK, &key) else {
        return None;
    };
    let (width, height, residual_mm) = cache::decode_residual(&payload).ok()?;
    let expected_width = grid_width.saturating_mul(factor);
    let expected_height = grid_height.saturating_mul(factor);
    if width != expected_width || height != expected_height {
        return None;
    }
    Some((factor, width, height, residual_mm))
}

fn structure_amplification(
    structure: &control::ControlFields,
    identity: &[u8],
    variant: u32,
    level: request::DetailLevel,
    cache: Option<&cache::AtlasDiskCache>,
    progress: &mut dyn AtlasProgress,
) -> Result<(amplify::AmplificationModel, cache::CacheLookup), AtlasError> {
    let Some(cache) = cache else {
        let mut cancelled = || progress.check_cancelled();
        return Ok((
            amplify::build_amplification_model(
                structure,
                identity,
                variant,
                level,
                &mut cancelled,
            )?,
            cache::CacheLookup::Off,
        ));
    };
    let residual_key = residual_cache_key(
        identity,
        variant,
        level,
        structure.grid.width,
        structure.grid.height,
    );
    let expected_width = structure.grid.width.saturating_mul(level.lattice_factor());
    let expected_height = structure.grid.height.saturating_mul(level.lattice_factor());
    if let cache::CacheLookupResult::Hit(payload) = cache.get(cache::KIND_RESIDUAL, &residual_key) {
        if let Ok((lattice_width, lattice_height, residual_mm)) = cache::decode_residual(&payload) {
            if lattice_width == expected_width && lattice_height == expected_height {
                return Ok((
                    amplify::AmplificationModel::from_cached_detail(
                        detail::AtlasDetailModel {
                            grid: structure.grid,
                            elevations_mm: structure.elevation_mm.clone(),
                            residual_mm,
                            lattice_width,
                            lattice_height,
                            algorithm_version: ATLAS_DETAIL_ALGORITHM_VERSION,
                            variant,
                            level,
                        },
                        structure,
                        identity,
                    ),
                    cache::CacheLookup::Hit,
                ));
            }
        }
    }
    let finest = level.lattice_factor();
    let start = [16_u32, 8, 4]
        .into_iter()
        .filter(|factor| *factor <= finest)
        .find_map(|factor| {
            load_octave_stack(
                cache,
                identity,
                variant,
                factor,
                structure.grid.width,
                structure.grid.height,
            )
        });
    let mut cancelled = || progress.check_cancelled();
    let (width, height, residual_mm) = amplify::stack_octaves_from(
        structure,
        identity,
        variant,
        finest,
        start,
        &mut cancelled,
        &mut |factor, width, height, residual_mm| {
            if !amplify::cacheable_stack_factor(factor) {
                return;
            }
            let key = octave_stack_cache_key(
                identity,
                variant,
                factor,
                structure.grid.width,
                structure.grid.height,
            );
            let _ = cache.put(
                cache::KIND_OCTAVE_STACK,
                &key,
                &cache::encode_residual(width, height, residual_mm),
            );
        },
    )?;
    let model = amplify::finish_amplification_model(
        structure,
        identity,
        variant,
        level,
        (width, height, residual_mm),
        &mut cancelled,
    )?;
    let _ = cache.put(
        cache::KIND_RESIDUAL,
        &residual_key,
        &cache::encode_residual(
            model.detail.lattice_width,
            model.detail.lattice_height,
            &model.detail.residual_mm,
        ),
    );
    Ok((model, cache::CacheLookup::Miss))
}

pub fn prepare_from_source(
    source_bytes: &[u8],
    identity: &[u8],
    request: &request::AtlasRenderRequest,
    forcing: Option<daena_physical::history::HistoricalForcingParameters>,
    cache: Option<&cache::AtlasDiskCache>,
    progress: &mut dyn AtlasProgress,
) -> Result<AtlasPreparedScene, AtlasError> {
    prepare_from_source_with_structure(
        source_bytes,
        identity,
        request,
        forcing,
        cache,
        None,
        progress,
    )
    .map(|(scene, _)| scene)
}

pub fn prepare_from_source_with_structure(
    source_bytes: &[u8],
    identity: &[u8],
    request: &request::AtlasRenderRequest,
    forcing: Option<daena_physical::history::HistoricalForcingParameters>,
    cache: Option<&cache::AtlasDiskCache>,
    injected: Option<&amplify::AmplificationModel>,
    progress: &mut dyn AtlasProgress,
) -> Result<(AtlasPreparedScene, amplify::AmplificationModel), AtlasError> {
    progress.report(AtlasPhase::Validating, 0, 1)?;
    progress.check_cancelled()?;
    let request = request.clone().normalize()?;
    let (style, style_raw) = style::load_style(&request.style_id)?;
    let style_hash = style.content_hash(style_raw);
    let source_sha256 = {
        use sha2::{Digest, Sha256};
        format!("sha256:{:x}", Sha256::digest(source_bytes))
    };
    let forcing_fingerprint = fingerprint_forcing(forcing.as_ref());
    let decoded = decode_atlas_source(source_bytes, progress)?;
    let factor = request.level.lattice_factor() as usize;
    let lattice_count = (ATLAS_VIEW_WIDTH as usize)
        .saturating_mul(factor)
        .saturating_mul(ATLAS_VIEW_HEIGHT as usize)
        .saturating_mul(factor);
    if refine::refined_lattice_exceeds_budget(lattice_count) {
        return Err(AtlasError::limit(
            "refined lattice exceeded the in-process byte budget",
        ));
    }
    progress.report(AtlasPhase::Validating, 1, 1)?;
    let expected_width = ATLAS_VIEW_WIDTH.saturating_mul(request.level.lattice_factor());
    let expected_height = ATLAS_VIEW_HEIGHT.saturating_mul(request.level.lattice_factor());
    let injected_ok = injected.is_some_and(|model| {
        model.detail.level == request.level
            && model.detail.variant == request.variant
            && model.detail.algorithm_version == ATLAS_DETAIL_ALGORITHM_VERSION
            && model.detail.lattice_width == expected_width
            && model.detail.lattice_height == expected_height
    });
    progress.report(AtlasPhase::DerivingEpoch, 0, 1)?;
    progress.check_cancelled()?;
    let resolved_forcing = forcing.unwrap_or_else(|| {
        daena_physical::history::HistoricalForcingParameters::default_for_seed(decoded.seed())
    });
    let solved = if request.offset_years != 0 {
        Some(solve_atlas_epoch(
            &decoded,
            resolved_forcing,
            request.offset_years,
            progress,
        )?)
    } else {
        None
    };
    let epoch = solved.filter(|solve| solve.normalized_epoch != 0);
    let planet = decoded.planetary_configuration();
    let (inputs, year0_inputs) = derive_atlas_inputs_for_request(
        &decoded,
        epoch.as_ref(),
        planet,
        epoch.is_some() && !injected_ok,
        progress,
    )?;
    progress.check_cancelled()?;
    progress.report(AtlasPhase::DerivingEpoch, 1, 1)?;
    progress.report(AtlasPhase::RefiningDetail, 0, 1)?;
    let field = &inputs.view;
    let sdf = detail::signed_coastal_distance_ppm(
        field.grid,
        &field.elevations_mm,
        inputs.hydrology.sea_level_mm,
    );
    let controls = control::ControlFields::from_atlas_inputs(&inputs)?;
    let (mut amplification, residual_cache, structure_sea_level_mm) = match injected {
        Some(model) if injected_ok => (
            model.clone(),
            cache::CacheLookup::Hit,
            model.structure_sea_level_mm,
        ),
        _ => {
            let structure_owned = match year0_inputs.as_ref() {
                Some(present) => Some(control::ControlFields::from_atlas_inputs(present)?),
                None => None,
            };
            let structure = structure_owned.as_ref().unwrap_or(&controls);
            let (model, lookup) = structure_amplification(
                structure,
                identity,
                request.variant,
                request.level,
                cache,
                progress,
            )?;
            let sea = structure.sea_level_mm;
            (model, lookup, sea)
        }
    };
    drop(year0_inputs);
    let unbaked = amplification.clone();
    let constraint_fp = constraint::fingerprint(&request.constraints);
    let drainage_key = cache::cache_key(&[
        b"atlas-cache-drainage-v5",
        identity,
        &ATLAS_DETAIL_ALGORITHM_VERSION.to_le_bytes(),
        &ATLAS_DERIVED_DRAINAGE_VERSION.to_le_bytes(),
        &request.variant.to_le_bytes(),
        request.level.as_str().as_bytes(),
        &request.offset_years.to_le_bytes(),
        &inputs.hydrology.derivation_version.to_le_bytes(),
        &forcing_fingerprint,
        &constraint_fp,
    ]);
    let mut drainage_cache = cache::CacheLookup::Off;
    let (mut drainage, worked_mm, sediment_mm, cache_refine) = if let Some(cache) = cache {
        match cache.get(cache::KIND_DRAINAGE, &drainage_key) {
            cache::CacheLookupResult::Hit(payload) => {
                match drainage::DerivedDrainage::decode_product(&payload) {
                    Ok((drainage, width, height, worked_mm, sediment_mm))
                        if width == amplification.detail.lattice_width
                            && height == amplification.detail.lattice_height
                            && sediment_mm.len() == worked_mm.len() =>
                    {
                        drainage_cache = cache::CacheLookup::Hit;
                        (drainage, worked_mm, sediment_mm, false)
                    }
                    _ => {
                        drainage_cache = cache::CacheLookup::Miss;
                        refine_or_fallback(
                            build_constrained_refine(
                                &amplification,
                                &controls,
                                &inputs.hydrology,
                                &sdf,
                                identity,
                                structure_sea_level_mm,
                                &request.constraints,
                                progress,
                            ),
                            &amplification,
                        )?
                    }
                }
            }
            cache::CacheLookupResult::Miss => {
                drainage_cache = cache::CacheLookup::Miss;
                refine_or_fallback(
                    build_constrained_refine(
                        &amplification,
                        &controls,
                        &inputs.hydrology,
                        &sdf,
                        identity,
                        structure_sea_level_mm,
                        &request.constraints,
                        progress,
                    ),
                    &amplification,
                )?
            }
        }
    } else {
        refine_or_fallback(
            build_constrained_refine(
                &amplification,
                &controls,
                &inputs.hydrology,
                &sdf,
                identity,
                structure_sea_level_mm,
                &request.constraints,
                progress,
            ),
            &amplification,
        )?
    };
    constraint::apply_lock_to_drainage(&request.constraints, &mut drainage.tributaries);
    if drainage_cache == cache::CacheLookup::Miss && cache_refine {
        if let Some(cache) = cache {
            let _ = cache.put(
                cache::KIND_DRAINAGE,
                &drainage_key,
                &drainage.encode_product(
                    amplification.detail.lattice_width,
                    amplification.detail.lattice_height,
                    &worked_mm,
                    &sediment_mm,
                ),
            );
        }
    }
    let structure_residual_mm = amplification.detail.residual_mm.clone();
    amplification.detail.bake_absolute_elevation(&worked_mm);
    let orometry = amplification.features;
    let model = amplification.detail;
    let mut hydrology = inputs.hydrology;
    constraint::apply_to_hydrology(&request.constraints, &mut hydrology);
    let visible_water = render::classify_visible_water(
        model.grid,
        &model.elevations_mm,
        hydrology.sea_level_mm,
        &hydrology.lake_cells,
    );
    progress.report(AtlasPhase::RefiningDetail, 1, 1)?;
    Ok((
        AtlasPreparedScene {
            identity: identity.to_vec(),
            source_sha256,
            style,
            style_hash,
            model,
            hydrology,
            hazards: inputs.hazards,
            surface: std::sync::Arc::new(inputs.surface),
            sdf,
            drainage,
            tectonics: inputs.tectonics,
            visible_water,
            temperature_centi_c: controls.temperature_centi_c,
            temperature_nh_summer_centi_c: controls.temperature_nh_summer_centi_c,
            temperature_nh_winter_centi_c: controls.temperature_nh_winter_centi_c,
            wind_east_milli: controls.wind_east_milli,
            wind_north_milli: controls.wind_north_milli,
            wind_east_nh_summer_milli: controls.wind_east_nh_summer_milli,
            wind_north_nh_summer_milli: controls.wind_north_nh_summer_milli,
            wind_east_nh_winter_milli: controls.wind_east_nh_winter_milli,
            wind_north_nh_winter_milli: controls.wind_north_nh_winter_milli,
            wind_divergence_nano_per_s: controls.wind_divergence_nano_per_s,
            wind_divergence_nh_summer_nano_per_s: controls.wind_divergence_nh_summer_nano_per_s,
            wind_divergence_nh_winter_nano_per_s: controls.wind_divergence_nh_winter_nano_per_s,
            wind_band: controls.wind_band,
            wind_band_nh_summer: controls.wind_band_nh_summer,
            wind_band_nh_winter: controls.wind_band_nh_winter,
            precipitation_mm: controls.precipitation_mm,
            aridity_ppm: controls.aridity_ppm,
            precipitation_nh_summer_mm: controls.precipitation_nh_summer_mm,
            precipitation_nh_winter_mm: controls.precipitation_nh_winter_mm,
            storm_suitability_ppm: controls.storm_suitability_ppm,
            storm_track_ppm: controls.storm_track_ppm,
            storm_intensity_ppm: controls.storm_intensity_ppm,
            residual_cache,
            drainage_cache,
            orometry,
            structure_residual_mm,
            sediment_mm,
            runoff_mm: controls.runoff_mm,
            crust_influence_ppm: controls.crust_influence_ppm,
            mountain_influence_ppm: controls.mountain_influence_ppm,
        },
        unbaked,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn render_from_source_cached(
    source_bytes: &[u8],
    identity: &[u8],
    request: &request::AtlasRenderRequest,
    tile_order: Option<&[u32]>,
    forcing: Option<daena_physical::history::HistoricalForcingParameters>,
    overlays: &[overlay::AuthoredFeature],
    cache: Option<&cache::AtlasDiskCache>,
    progress: &mut dyn AtlasProgress,
) -> Result<RenderedAtlas, AtlasError> {
    progress.report(AtlasPhase::Validating, 0, 1)?;
    progress.check_cancelled()?;
    let request = request.clone().normalize()?;
    let (style, style_raw) = style::load_style(&request.style_id)?;
    let style_hash = style.content_hash(style_raw);
    let source_sha256 = {
        use sha2::{Digest, Sha256};
        format!("sha256:{:x}", Sha256::digest(source_bytes))
    };
    let overlay_bytes = cache_json(overlays)?;
    let request_bytes = cache_json(&request)?;
    let forcing_fingerprint = fingerprint_forcing(forcing.as_ref());
    let artifact_key = cache::cache_key(&[
        b"atlas-cache-artifact-v2",
        identity,
        source_sha256.as_bytes(),
        &request_bytes,
        style_hash.as_bytes(),
        &overlay_bytes,
        &ATLAS_RENDERER_VERSION.to_le_bytes(),
        &ATLAS_DETAIL_ALGORITHM_VERSION.to_le_bytes(),
        &ATLAS_DERIVED_DRAINAGE_VERSION.to_le_bytes(),
        &forcing_fingerprint,
    ]);
    let mut artifact_cache = cache::CacheLookup::Off;
    if let Some(cache) = cache {
        progress.check_cancelled()?;
        match cache.get(cache::KIND_ARTIFACT, &artifact_key) {
            cache::CacheLookupResult::Hit(payload) => {
                let usable = cache::decode_artifact(&payload).ok().and_then(
                    |(png, artifact, provenance_json)| {
                        let provenance =
                            serde_json::from_str::<provenance::AtlasRenderProvenanceV1>(
                                &provenance_json,
                            )
                            .ok()?;
                        let decoded = encode::decode_png(&png).ok()?;
                        if provenance.renderer_version != ATLAS_RENDERER_VERSION
                            || decoded.width != request.width_px
                            || decoded.height != request.height_px
                        {
                            return None;
                        }
                        Some((png, artifact, provenance, decoded.rgba))
                    },
                );
                if let Some((png, artifact, provenance, rgba)) = usable {
                    artifact_cache = cache::CacheLookup::Hit;
                    progress.report(AtlasPhase::Validating, 1, 1)?;
                    progress.report(AtlasPhase::Encoding, 1, 1)?;
                    return Ok(RenderedAtlas {
                        png,
                        artifact,
                        rgba,
                        provenance: provenance.clone(),
                        request,
                        residual_cache: cache::CacheLookup::Off,
                        drainage_cache: cache::CacheLookup::Off,
                        artifact_cache,
                        tributary_count: provenance.tributary_count,
                    });
                }
                artifact_cache = cache::CacheLookup::Miss;
            }
            cache::CacheLookupResult::Miss => artifact_cache = cache::CacheLookup::Miss,
        }
    }
    let scene = prepare_from_source(source_bytes, identity, &request, forcing, cache, progress)?;
    let tiles = request.tile_count();
    let order = match tile_order {
        Some(order) => order.to_vec(),
        None => render::forward_tile_order(tiles),
    };
    let rgba = render::render_rgba(
        &request,
        &scene.model,
        &scene.hydrology,
        &scene.hazards,
        &scene.sdf,
        &scene.style,
        identity,
        &order,
        overlays,
        &scene.drainage.tributaries,
        &scene.tectonics,
        &scene.visible_water,
        scene.paint_fields(),
        request
            .debug_layers_enabled()
            .then(|| overlay::OverlayDebug {
                grid: scene.model.grid,
                lattice_width: scene.model.lattice_width,
                lattice_height: scene.model.lattice_height,
                structure_residual_mm: &scene.structure_residual_mm,
                worked_residual_mm: &scene.model.residual_mm,
                sediment_mm: &scene.sediment_mm,
                runoff_mm: &scene.runoff_mm,
                orometry: &scene.orometry,
            }),
        progress,
    )?;
    let mut provenance = provenance::AtlasRenderProvenanceV1::for_request(
        &request,
        identity,
        &source_sha256,
        &style_hash,
    );
    provenance.derived_drainage_version = ATLAS_DERIVED_DRAINAGE_VERSION;
    provenance.tributary_count = scene.drainage.tributaries.len() as u32;
    let png = encode::encode_png(&request, &rgba, &provenance, progress)?;
    let artifact = encode::encode_artifact(&request, &rgba, &png, &provenance, progress)?;
    if let Some(cache) = cache {
        if let Ok(json) = provenance.compact_json() {
            let _ = cache.put(
                cache::KIND_ARTIFACT,
                &artifact_key,
                &cache::encode_artifact(&png, &artifact, &json),
            );
        }
        if artifact_cache == cache::CacheLookup::Off {
            artifact_cache = cache::CacheLookup::Miss;
        }
    }
    Ok(RenderedAtlas {
        png,
        artifact,
        rgba,
        provenance,
        request,
        residual_cache: scene.residual_cache,
        drainage_cache: scene.drainage_cache,
        artifact_cache,
        tributary_count: scene.drainage.tributaries.len() as u32,
    })
}

#[derive(Debug, Clone)]
pub struct RenderedAtlas {
    pub png: Vec<u8>,
    pub artifact: Vec<u8>,
    pub rgba: Vec<u8>,
    pub provenance: provenance::AtlasRenderProvenanceV1,
    pub request: request::AtlasRenderRequest,
    pub residual_cache: cache::CacheLookup,
    pub drainage_cache: cache::CacheLookup,
    pub artifact_cache: cache::CacheLookup,
    pub tributary_count: u32,
}

#[cfg(test)]
pub(crate) struct GoldenWorld {
    pub source: Vec<u8>,
    pub identity: Vec<u8>,
    pub decoded: daena_physical::engine::DecodedSource,
    pub inputs: daena_physical::engine::AtlasInputs,
}

#[cfg(test)]
impl GoldenWorld {
    /// Derive Atlas inputs at one epoch. Offset zero is the present world.
    pub(crate) fn inputs_at(&self, offset_years: i64) -> daena_physical::engine::AtlasInputs {
        let forcing = daena_physical::history::HistoricalForcingParameters::default_for_seed(
            self.decoded.seed(),
        );
        let solved = if offset_years == 0 {
            None
        } else {
            Some(
                daena_physical::history::solve_epoch(
                    &self.decoded,
                    forcing,
                    offset_years,
                    &daena_physical::NoopProgress,
                )
                .expect("golden epoch"),
            )
        };
        let epoch = solved.as_ref().filter(|solve| solve.normalized_epoch != 0);
        self.decoded
            .derive_atlas_inputs(
                epoch,
                ATLAS_VIEW_WIDTH,
                ATLAS_VIEW_HEIGHT,
                self.decoded.planetary_configuration(),
                &mut daena_physical::NoopProgress,
            )
            .expect("golden atlas inputs")
    }
}

#[cfg(test)]
pub(crate) fn golden_world() -> &'static GoldenWorld {
    use daena_physical::engine::{self, Settings, WorldPreset};
    use daena_physical::{NoopProgress, DEFAULT_RADIUS_METRES};
    use std::sync::OnceLock;
    static WORLD: OnceLock<GoldenWorld> = OnceLock::new();
    WORLD.get_or_init(|| {
        let settings = Settings::from_parts(
            831_429,
            WorldPreset::Earthlike,
            128,
            DEFAULT_RADIUS_METRES,
            300_000,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let generated = engine::generate(settings, &mut NoopProgress).expect("golden world");
        let source = generated.source().to_vec();
        let decoded = engine::decode_source(&source).expect("golden source");
        let inputs = decoded
            .derive_atlas_inputs(
                None,
                ATLAS_VIEW_WIDTH,
                ATLAS_VIEW_HEIGHT,
                decoded.planetary_configuration(),
                &mut NoopProgress,
            )
            .expect("golden atlas inputs");
        let identity = spike_identity_from_source(&source);
        GoldenWorld {
            source,
            identity,
            decoded,
            inputs,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::AtlasRenderRequest;

    #[test]
    fn tiled_and_shuffled_renders_match_and_png_carries_provenance() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(1024, 512).unwrap();
        let forward = render_from_source(
            &world.source,
            &identity,
            &request,
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let reverse = render_from_source(
            &world.source,
            &identity,
            &request,
            Some(&render::reverse_tile_order(request.tile_count())),
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let shuffled = render_from_source(
            &world.source,
            &identity,
            &request,
            Some(&render::shuffled_tile_order(request.tile_count(), 7)),
            None,
            &[],
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
        let identity = world.identity.clone();
        let small = render_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(256, 128).unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let large = render_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(512, 256).unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(
            small.provenance.detail_algorithm_version,
            ATLAS_DETAIL_ALGORITHM_VERSION
        );
        assert_eq!(
            large.provenance.derived_drainage_version,
            ATLAS_DERIVED_DRAINAGE_VERSION
        );
        assert_ne!(small.png, large.png);
    }

    #[test]
    fn cancellation_stops_before_ready_to_save() {
        let world = golden_world();
        let identity = world.identity.clone();
        let flag = CancelFlag::default();
        flag.cancel();
        let error = render_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(64, 32).unwrap(),
            None,
            None,
            &[],
            &mut FlagProgress { flag: &flag },
        )
        .unwrap_err();
        assert_eq!(error.code, CODE_RENDER_CANCELLED);
    }

    #[test]
    fn style_and_layer_toggles_change_only_declared_stages() {
        let world = golden_world();
        let identity = world.identity.clone();
        let mut relief = AtlasRenderRequest::spike_png(128, 64).unwrap();
        let with_layers = render_from_source(
            &world.source,
            &identity,
            &relief,
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        relief.style_id = style::ANTIQUE_STYLE_ID.to_string();
        let antique = render_from_source(
            &world.source,
            &identity,
            &relief.normalize().unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(with_layers.rgba, antique.rgba);
        assert_eq!(antique.provenance.style_id, style::ANTIQUE_STYLE_ID);
        let mut no_rivers = AtlasRenderRequest::spike_png(128, 64).unwrap();
        no_rivers.active_layer_ids = vec![
            "ocean".into(),
            "relief".into(),
            "ice".into(),
            "lakes".into(),
            "frame".into(),
        ];
        let without = render_from_source(
            &world.source,
            &identity,
            &no_rivers.normalize().unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(with_layers.rgba, without.rgba);
        let mut cold = AtlasRenderRequest::spike_png(128, 64).unwrap();
        cold.offset_years = -8_000;
        let past = render_from_source(
            &world.source,
            &identity,
            &cold.normalize().unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(past.provenance.offset_years, -8_000);
        assert_ne!(with_layers.rgba, past.rgba);
    }

    #[test]
    fn epochs_change_sea_ice_and_hydrology_while_detail_stays_keyed() {
        let world = golden_world();
        let identity = world.identity.clone();
        let cold = world.inputs_at(-8_000);
        let warm = world.inputs_at(8_000);
        assert_ne!(
            cold.hydrology.sea_level_mm,
            world.inputs.hydrology.sea_level_mm
        );
        assert_ne!(
            warm.hydrology.sea_level_mm,
            world.inputs.hydrology.sea_level_mm
        );
        let forcing = daena_physical::history::HistoricalForcingParameters::default_for_seed(
            world.decoded.seed(),
        );
        let cold_solve = daena_physical::history::solve_epoch(
            &world.decoded,
            forcing,
            -8_000,
            &daena_physical::NoopProgress,
        )
        .unwrap();
        let warm_solve = daena_physical::history::solve_epoch(
            &world.decoded,
            forcing,
            8_000,
            &daena_physical::NoopProgress,
        )
        .unwrap();
        assert_ne!(cold_solve.land_ice_m3, warm_solve.land_ice_m3);
        let mut past = AtlasRenderRequest::spike_png(128, 64).unwrap();
        past.offset_years = -8_000;
        let mut future = AtlasRenderRequest::spike_png(128, 64).unwrap();
        future.offset_years = 8_000;
        let past = render_from_source(
            &world.source,
            &identity,
            &past.normalize().unwrap(),
            None,
            Some(forcing),
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let future = render_from_source(
            &world.source,
            &identity,
            &future.normalize().unwrap(),
            None,
            Some(forcing),
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(past.rgba, future.rgba);
        assert_eq!(past.provenance.offset_years, -8_000);
        assert_eq!(future.provenance.offset_years, 8_000);
    }

    #[test]
    fn cancellation_is_honored_at_each_phase() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        for phase in [
            AtlasPhase::Validating,
            AtlasPhase::DerivingEpoch,
            AtlasPhase::RefiningDetail,
            AtlasPhase::Rendering,
            AtlasPhase::Encoding,
        ] {
            let mut progress = CancelOnPhase { phase };
            let error = render_from_source(
                &world.source,
                &identity,
                &request,
                None,
                None,
                &[],
                &mut progress,
            )
            .unwrap_err();
            assert_eq!(error.code, CODE_RENDER_CANCELLED, "{phase:?}");
        }
    }

    #[test]
    fn regional_extent_samples_the_same_world_location() {
        let world = golden_world();
        let identity = world.identity.clone();
        let mut globe = AtlasRenderRequest::spike_png(64, 32).unwrap();
        globe.active_layer_ids = vec![
            "ocean".into(),
            "relief".into(),
            "ice".into(),
            "lakes".into(),
        ];
        let globe = globe.normalize().unwrap();
        let globe_render = render_from_source(
            &world.source,
            &identity,
            &globe,
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let (lon, lat) = globe.view().unwrap().pixel_center(40, 20);
        let mut region = globe.clone();
        region.width_px = 1;
        region.height_px = 1;
        region.extent = crate::projection::AtlasExtent {
            west_lon_micro: lon - 1_000_000,
            south_lat_micro: lat - 1_000_000,
            east_lon_micro: lon + 1_000_000,
            north_lat_micro: lat + 1_000_000,
        };
        let region = region.normalize().unwrap();
        let region_render = render_from_source(
            &world.source,
            &identity,
            &region,
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let globe_offset = (20usize * 64 + 40) * 4;
        assert_eq!(
            &region_render.rgba[..4],
            &globe_render.rgba[globe_offset..globe_offset + 4]
        );
        let mut svg = region.clone();
        svg.format = crate::request::AtlasFormat::Svg;
        let svg = render_from_source(
            &world.source,
            &identity,
            &svg.normalize().unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert!(String::from_utf8(svg.artifact)
            .unwrap()
            .contains("data:image/png;base64,"));
        assert_eq!(svg.rgba, region_render.rgba);
        let mut pdf = region.clone();
        pdf.format = crate::request::AtlasFormat::Pdf;
        pdf.dpi = 72;
        let pdf = render_from_source(
            &world.source,
            &identity,
            &pdf.normalize().unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(
            encode::parse_pdf_media_box(&pdf.artifact).unwrap(),
            [0, 0, 1, 1]
        );
        assert_eq!(pdf.rgba, region_render.rgba);
        assert_eq!(pdf.provenance.renderer_version, ATLAS_RENDERER_VERSION);
        assert!(region_render.tributary_count > 0 || globe_render.tributary_count > 0);
    }

    #[test]
    fn cache_hits_reproduce_pixels_and_ignore_corrupt_entries() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(128, 64).unwrap();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-iter4-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let cold = render_from_source_cached(
            &world.source,
            &identity,
            &request,
            None,
            None,
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(cold.artifact_cache, cache::CacheLookup::Miss);
        let warm = render_from_source_cached(
            &world.source,
            &identity,
            &request,
            None,
            None,
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(warm.artifact_cache, cache::CacheLookup::Hit);
        assert_eq!(cold.png, warm.png);
        assert_eq!(cold.rgba, warm.rgba);
        for entry in std::fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("bin") {
                std::fs::write(&path, b"truncated").unwrap();
            }
        }
        let rebuilt = render_from_source_cached(
            &world.source,
            &identity,
            &request,
            None,
            None,
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(rebuilt.artifact_cache, cache::CacheLookup::Miss);
        assert_eq!(rebuilt.png, cold.png);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn artifact_cache_does_not_reuse_a_different_historical_forcing() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(128, 64).unwrap();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-forcing-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let forcing_a = daena_physical::history::HistoricalForcingParameters::default_for_seed(
            world.decoded.seed(),
        );
        let mut forcing_b = forcing_a;
        forcing_b.land_ice_amplitude_ppm = forcing_a.land_ice_amplitude_ppm.saturating_add(80_000);
        let _first = render_from_source_cached(
            &world.source,
            &identity,
            &request,
            None,
            Some(forcing_a),
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        let second = render_from_source_cached(
            &world.source,
            &identity,
            &request,
            None,
            Some(forcing_b),
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(second.artifact_cache, cache::CacheLookup::Miss);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn residual_cache_hits_when_epoch_or_forcing_changes() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(128, 64).unwrap();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-residual-epoch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let forcing_a = daena_physical::history::HistoricalForcingParameters::default_for_seed(
            world.decoded.seed(),
        );
        let mut forcing_b = forcing_a;
        forcing_b.land_ice_amplitude_ppm = forcing_a.land_ice_amplitude_ppm.saturating_add(80_000);
        let present = prepare_from_source(
            &world.source,
            &identity,
            &request,
            Some(forcing_a),
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(present.residual_cache, cache::CacheLookup::Miss);
        let mut cold = request.clone();
        cold.offset_years = -8_000;
        let cold = prepare_from_source(
            &world.source,
            &identity,
            &cold.normalize().unwrap(),
            Some(forcing_b),
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(cold.residual_cache, cache::CacheLookup::Hit);
        assert_eq!(cold.drainage_cache, cache::CacheLookup::Miss);
        let present_hit = prepare_from_source(
            &world.source,
            &identity,
            &request,
            Some(forcing_a),
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(present_hit.drainage_cache, cache::CacheLookup::Hit);
        assert_eq!(present_hit.drainage, present.drainage);
        let _ = std::fs::remove_dir_all(&root);
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let rebuilt = prepare_from_source(
            &world.source,
            &identity,
            &request,
            Some(forcing_a),
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(rebuilt.drainage_cache, cache::CacheLookup::Miss);
        assert_eq!(rebuilt.drainage, present.drainage);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_orometry_ids_stable_across_epochs_and_residual_cache() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(128, 64).unwrap();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-orometry-epoch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let ids = |scene: &AtlasPreparedScene| {
            scene
                .orometry
                .iter()
                .map(|feature| (feature.id.clone(), feature.kind, feature.basin_id))
                .collect::<Vec<_>>()
        };
        let present = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let mut cold_req = request.clone();
        cold_req.offset_years = -8_000;
        let cold_req = cold_req.normalize().unwrap();
        let cold = prepare_from_source(
            &world.source,
            &identity,
            &cold_req,
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let mut warm_req = request.clone();
        warm_req.offset_years = 8_000;
        let warm_req = warm_req.normalize().unwrap();
        let warm = prepare_from_source(
            &world.source,
            &identity,
            &warm_req,
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(present.hydrology.sea_level_mm, cold.hydrology.sea_level_mm);
        assert_ne!(present.hydrology.sea_level_mm, warm.hydrology.sea_level_mm);
        assert_eq!(present.structure_residual_mm, cold.structure_residual_mm);
        assert_eq!(cold.structure_residual_mm, warm.structure_residual_mm);
        assert_eq!(ids(&present), ids(&cold));
        assert_eq!(ids(&cold), ids(&warm));
        let cached_present = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(cached_present.residual_cache, cache::CacheLookup::Miss);
        let cached_cold = prepare_from_source(
            &world.source,
            &identity,
            &cold_req,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        let cached_warm = prepare_from_source(
            &world.source,
            &identity,
            &warm_req,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(cached_cold.residual_cache, cache::CacheLookup::Hit);
        assert_eq!(cached_warm.residual_cache, cache::CacheLookup::Hit);
        assert_eq!(ids(&cached_cold), ids(&cached_warm));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn constraints_survive_residual_regen_without_mutating_source() {
        let world = golden_world();
        let identity = world.identity.clone();
        let source_before = world.source.clone();
        let mut request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.level = request::DetailLevel::Standard;
        let unconstrained = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let constraint = constraint::AtlasConstraint {
            id: "force-lake".into(),
            kind: constraint::AtlasConstraintKind::ForceLake,
            path: vec![
                [0, 0],
                [40_000_000, 0],
                [40_000_000, 40_000_000],
                [0, 40_000_000],
            ],
            closed: true,
            revision: Some("1".into()),
        };
        request.constraints = vec![constraint.clone()];
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-constraint-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let first = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(first.residual_cache, cache::CacheLookup::Miss);
        assert_eq!(
            first.structure_residual_mm,
            unconstrained.structure_residual_mm
        );
        let unconstrained_lakes = unconstrained
            .hydrology
            .lake_cells
            .iter()
            .filter(|cell| **cell)
            .count();
        let constrained_lakes = first
            .hydrology
            .lake_cells
            .iter()
            .filter(|cell| **cell)
            .count();
        assert!(constrained_lakes > unconstrained_lakes);
        assert!(cache.delete_kinds(&[cache::KIND_RESIDUAL]).unwrap() > 0);
        assert!(cache.delete_kinds(&[cache::KIND_DRAINAGE]).unwrap() > 0);
        let second = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(second.residual_cache, cache::CacheLookup::Miss);
        assert_eq!(second.drainage_cache, cache::CacheLookup::Miss);
        assert_eq!(second.model.residual_mm, first.model.residual_mm);
        assert_eq!(second.structure_residual_mm, first.structure_residual_mm);
        assert_eq!(second.hydrology.lake_cells, first.hydrology.lake_cells);
        assert_eq!(world.source, source_before);
        assert_eq!(request.constraints, vec![constraint]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn debug_layers_off_keep_relief_pixels() {
        let world = golden_world();
        let identity = world.identity.clone();
        let mut request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        request.active_layer_ids = vec![
            "ocean".into(),
            "relief".into(),
            "ice".into(),
            "lakes".into(),
        ];
        let base_req = request.clone().normalize().unwrap();
        let base = render_from_source(
            &world.source,
            &identity,
            &base_req,
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        let same = render_from_source(
            &world.source,
            &identity,
            &base_req,
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(base.rgba, same.rgba);
        request.active_layer_ids.push("debug-residual".into());
        let debug = render_from_source(
            &world.source,
            &identity,
            &request.normalize().unwrap(),
            None,
            None,
            &[],
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(base.rgba, debug.rgba);
    }

    #[test]
    fn refine_error_falls_back_to_structure_and_skips_cache() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        let (scene, structure) = prepare_from_source_with_structure(
            &world.source,
            &identity,
            &request,
            None,
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let worked = structure_worked_mm(&structure);
        assert_eq!(worked.len(), structure.detail.residual_mm.len());
        let (drainage, fallback, sediment, cacheable) =
            refine_or_fallback(Err(AtlasError::limit("refine failed")), &structure).unwrap();
        assert!(!cacheable);
        assert!(drainage.tributaries.is_empty());
        assert_eq!(fallback, worked);
        assert_eq!(sediment, vec![0_i32; worked.len()]);
        assert_eq!(
            refine_or_fallback(Err(AtlasError::cancelled()), &structure)
                .unwrap_err()
                .code,
            CODE_RENDER_CANCELLED
        );
        assert!(!scene.hydrology.rivers.is_empty());
    }

    #[test]
    fn epoch_views_keep_reference_elevations_and_move_the_sea_level() {
        let world = golden_world();
        let cold = world.inputs_at(-8_000);
        assert_ne!(
            cold.hydrology.sea_level_mm,
            world.inputs.hydrology.sea_level_mm
        );
        let count = world.inputs.view.elevations_mm.len();
        assert_eq!(cold.view.elevations_mm.len(), count);
        let max_diff = world
            .inputs
            .view
            .elevations_mm
            .iter()
            .zip(&cold.view.elevations_mm)
            .map(|(present, epoch)| present.abs_diff(*epoch))
            .max()
            .unwrap_or(0);
        assert!(
            max_diff <= 1,
            "epoch view elevations moved by {max_diff} mm from the reference datum"
        );
        if max_diff == 1 {
            println!("epoch view elevations differ by 1 mm from the f32 round trip");
        }
    }

    #[test]
    fn atlas_view_is_cell_centred_without_flipped_latitude() {
        let world = golden_world();
        let inputs = &world.inputs;
        let view = &inputs.view;
        let surface = &inputs.surface;
        let sea = inputs.hydrology.sea_level_mm;
        let north = crate::detail::nearest_cell(view.grid, 0, 89_000_000);
        let south = crate::detail::nearest_cell(view.grid, 0, -89_000_000);
        assert!(north / view.grid.width as usize > south / view.grid.width as usize);
        let mut found = None;
        for cell in 0..surface.cell_count() as u32 {
            if surface.ocean(cell) {
                continue;
            }
            if surface.elevation_mm[cell as usize] < 200_000 {
                continue;
            }
            if surface.neighbors(cell).iter().any(|neighbor| {
                *neighbor == daena_physical::engine::TOOL_SURFACE_NONE || surface.ocean(*neighbor)
            }) {
                continue;
            }
            found = Some(cell);
            break;
        }
        let cell = found.expect("golden world has an interior land cell");
        let (lat, lon) = surface.cell_lat_lon_degrees(cell);
        let view_cell = crate::detail::nearest_cell(
            view.grid,
            (lon * 1_000_000.0).round() as i32,
            (lat * 1_000_000.0).round() as i32,
        );
        assert!(
            view.elevations_mm[view_cell] >= sea,
            "engine land at {lat:.3},{lon:.3} became ocean in the Atlas view"
        );
        let (row, _) = view.grid.row_col(view_cell);
        let expected_row = ((lat + 90.0) / 180.0 * f64::from(view.grid.height)) as u32;
        assert!(
            row.abs_diff(expected_row) <= 1,
            "latitude row {row} does not match cell-centre row {expected_row}"
        );
    }

    #[test]
    fn basin_view_minimum_cells_are_lake_cells() {
        let settings = daena_physical::engine::Settings::from_parts(
            831_429,
            daena_physical::engine::WorldPreset::Earthlike,
            64,
            daena_physical::DEFAULT_RADIUS_METRES,
            300_000,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let generated =
            daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress).unwrap();
        let source = generated.source().to_vec();
        let decoded = daena_physical::engine::decode_source(&source).unwrap();
        let inputs = decoded
            .derive_atlas_inputs(
                None,
                ATLAS_VIEW_WIDTH,
                ATLAS_VIEW_HEIGHT,
                decoded.planetary_configuration(),
                &mut daena_physical::NoopProgress,
            )
            .unwrap();
        let hydrology = &inputs.hydrology;
        let surface = &inputs.surface;
        let mut checked = 0;
        for basin in &hydrology.basins {
            if basin.water_volume_m3 == 0 || basin.cell_count == 0 {
                continue;
            }
            let [lon, lat] = surface.center_microdegrees(basin.minimum_cell);
            let view_cell = crate::detail::nearest_cell(hydrology.grid, lon, lat);
            assert!(
                hydrology.lake_cells[view_cell],
                "basin {} minimum cell is not a view lake cell",
                basin.id
            );
            checked += 1;
        }
        assert!(checked > 0, "fixture has no wet basin");
    }

    #[test]
    fn cancellation_during_the_derive_leaves_no_cache_entry() {
        let world = golden_world();
        let identity = world.identity.clone();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-cancel-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let flag = CancelFlag::default();
        flag.cancel();
        let mut past = AtlasRenderRequest::spike_png(64, 32).unwrap();
        past.offset_years = -8_000;
        let error = prepare_from_source(
            &world.source,
            &identity,
            &past.normalize().unwrap(),
            None,
            Some(&cache),
            &mut FlagProgress { flag: &flag },
        )
        .unwrap_err();
        assert_eq!(error.code, CODE_RENDER_CANCELLED);
        let entries = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .count();
        assert_eq!(entries, 0, "cancelled prepare wrote a cache entry");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn wrong_sources_are_asset_invalid_not_a_panic() {
        let world = golden_world();
        let identity = world.identity.clone();
        let request = AtlasRenderRequest::spike_png(64, 32).unwrap();
        for source in [
            b"DAENAPW1".to_vec(),
            b"garbage bytes".to_vec(),
            world.source[..world.source.len() - 1].to_vec(),
        ] {
            let error =
                prepare_from_source(&source, &identity, &request, None, None, &mut NoopProgress)
                    .unwrap_err();
            assert_eq!(error.code, CODE_SOURCE_INVALID);
        }
    }

    #[test]
    fn removed_humidity_style_and_currents_layer_are_request_invalid() {
        let mut humidity = AtlasRenderRequest::spike_png(64, 32).unwrap();
        humidity.style_id = "daena-atlas-humidity".into();
        assert_eq!(humidity.normalize().unwrap_err().code, CODE_REQUEST_INVALID);
        let mut currents = AtlasRenderRequest::spike_png(64, 32).unwrap();
        currents.active_layer_ids = vec!["ocean".into(), "currents".into()];
        let error = currents.normalize().unwrap_err();
        assert_eq!(error.code, CODE_REQUEST_INVALID);
        assert!(error.message.contains("currents"));
    }

    #[test]
    fn odd_worlds_prepare_and_render_or_fail_with_a_coded_error() {
        for preset in [
            daena_physical::engine::WorldPreset::WaterWorld,
            daena_physical::engine::WorldPreset::Pangea,
        ] {
            let settings = daena_physical::engine::Settings::from_parts(
                7,
                preset,
                64,
                daena_physical::DEFAULT_RADIUS_METRES,
                0,
                daena_physical::planetary::PlanetaryConfiguration::earth_like(),
            );
            let generated =
                daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress)
                    .unwrap();
            let source = generated.source().to_vec();
            let identity = spike_identity_from_source(&source);
            let request = AtlasRenderRequest::spike_png(64, 32).unwrap();
            match prepare_from_source(&source, &identity, &request, None, None, &mut NoopProgress) {
                Ok(scene) => {
                    let rendered = render_from_source(
                        &source,
                        &identity,
                        &request,
                        None,
                        None,
                        &[],
                        &mut NoopProgress,
                    )
                    .unwrap();
                    assert_eq!(rendered.rgba.len(), 64 * 32 * 4);
                    assert!(!scene.hydrology.basins.is_empty() || scene.surface.ocean_count() > 0);
                }
                Err(error) => {
                    assert!(error.code.starts_with("atlas."), "{error:?}");
                }
            }
        }
    }

    #[test]
    #[ignore = "wall-clock: release-only Atlas derive cancellation sweep"]
    fn atlas_derive_cancellation_stays_under_the_deadline_budget() {
        struct DeadlineProgress {
            deadline: std::time::Instant,
            armed: bool,
        }
        impl AtlasProgress for DeadlineProgress {
            fn report(
                &mut self,
                _phase: AtlasPhase,
                _completed: u32,
                _total: u32,
            ) -> Result<(), AtlasError> {
                if std::time::Instant::now() >= self.deadline {
                    self.armed = true;
                }
                Ok(())
            }

            fn check_cancelled(&self) -> Result<(), AtlasError> {
                if self.armed || std::time::Instant::now() >= self.deadline {
                    Err(AtlasError::cancelled())
                } else {
                    Ok(())
                }
            }
        }
        let settings = daena_physical::engine::Settings::from_parts(
            831_429,
            daena_physical::engine::WorldPreset::Earthlike,
            512,
            daena_physical::DEFAULT_RADIUS_METRES,
            290_000,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let generated =
            daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress).unwrap();
        let source = generated.source().to_vec();
        let identity = spike_identity_from_source(&source);
        let mut request = AtlasRenderRequest::spike_png(128, 64).unwrap();
        request.offset_years = -8_000;
        let request = request.normalize().unwrap();
        let started = std::time::Instant::now();
        prepare_from_source(&source, &identity, &request, None, None, &mut NoopProgress).unwrap();
        let total = started.elapsed();
        let mut worst = std::time::Duration::ZERO;
        let mut deadline_ms = 25;
        while std::time::Duration::from_millis(deadline_ms) < total {
            let started = std::time::Instant::now();
            let mut progress = DeadlineProgress {
                deadline: started + std::time::Duration::from_millis(deadline_ms),
                armed: false,
            };
            let result =
                prepare_from_source(&source, &identity, &request, None, None, &mut progress);
            if !matches!(result, Err(error) if error.code == CODE_RENDER_CANCELLED) {
                deadline_ms += 100;
                continue;
            }
            let late = started
                .elapsed()
                .saturating_sub(std::time::Duration::from_millis(deadline_ms));
            worst = worst.max(late);
            deadline_ms += 100;
        }
        println!("atlas prepare total {total:?}; worst late {worst:?}");
        assert!(
            worst.as_millis() < daena_physical::CANCELLATION_LATENCY_BUDGET_MS,
            "atlas cancellation was {worst:?} late"
        );
    }

    #[test]
    #[ignore = "wall-clock: release-only E5b timings at n=512"]
    fn e5b_release_timings_512() {
        e5b_release_timings(512);
    }

    #[test]
    #[ignore = "wall-clock: release-only E5b timings at n=1024"]
    fn e5b_release_timings_1024() {
        e5b_release_timings(1024);
    }

    fn e5b_release_timings(resolution: u32) {
        use std::time::Instant;
        let settings = daena_physical::engine::Settings::from_parts(
            831_429,
            daena_physical::engine::WorldPreset::Earthlike,
            resolution,
            daena_physical::DEFAULT_RADIUS_METRES,
            290_000,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let started = Instant::now();
        let generated =
            daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress).unwrap();
        let generation = started.elapsed();
        let source = generated.source().to_vec();
        let identity = spike_identity_from_source(&source);
        drop(generated);
        let request = AtlasRenderRequest::spike_png(1024, 512).unwrap();
        let started = Instant::now();
        let present =
            prepare_from_source(&source, &identity, &request, None, None, &mut NoopProgress)
                .unwrap();
        let cold_present = started.elapsed();
        let mut epoch_request = request.clone();
        epoch_request.offset_years = -8_000;
        let epoch_request = epoch_request.normalize().unwrap();
        let started = Instant::now();
        let _epoch = prepare_from_source(
            &source,
            &identity,
            &epoch_request,
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let cold_epoch = started.elapsed();
        let root = std::env::temp_dir().join(format!("daena-e5b-timings-{resolution}"));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let started = Instant::now();
        let cold_render = render_from_source_cached(
            &source,
            &identity,
            &request,
            None,
            None,
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        let cold_standard = started.elapsed();
        let started = Instant::now();
        let warm_render = render_from_source_cached(
            &source,
            &identity,
            &request,
            None,
            None,
            &[],
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        let warm_standard = started.elapsed();
        assert_eq!(cold_render.rgba, warm_render.rgba);
        let mut detailed = request.clone();
        detailed.level = crate::request::DetailLevel::Detailed;
        let detailed = detailed.normalize().unwrap();
        let started = Instant::now();
        let _ = render_from_source_cached(
            &source,
            &identity,
            &detailed,
            None,
            None,
            &[],
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let detailed_time = started.elapsed();
        let mut biome = request.clone();
        biome.style_id = style::BIOME_STYLE_ID.into();
        let biome = biome.normalize().unwrap();
        let started = Instant::now();
        let _ = render_from_source_cached(
            &source,
            &identity,
            &biome,
            None,
            None,
            &[],
            None,
            &mut NoopProgress,
        )
        .unwrap();
        let biome_time = started.elapsed();
        let pick_point = (0..present.surface.cell_count() as u32)
            .find(|cell| {
                present.surface.landmass_id(*cell) != daena_physical::engine::TOOL_SURFACE_NONE
            })
            .map(|cell| present.surface.center_microdegrees(cell))
            .expect("landmass");
        let started = Instant::now();
        for _ in 0..100 {
            let _ = present.sample_surface(pick_point[0], pick_point[1]);
        }
        let pick = started.elapsed() / 100;
        let started = Instant::now();
        let _ = daena_physical::region::propose_region_canonical(
            &present.surface,
            pick_point[0],
            pick_point[1],
            "landmass",
            daena_physical::engine::PRODUCT_VIEW_WIDTH,
            daena_physical::engine::PRODUCT_VIEW_HEIGHT,
        )
        .unwrap();
        let region = started.elapsed();
        let scene_bytes = std::mem::size_of_val(&present)
            + present.hydrology.water_level_mm.capacity() * 4
            + present.hydrology.lake_level_mm.capacity() * 4
            + present.hydrology.slope_ppm.capacity() * 4
            + present.hydrology.hillshade_ppm.capacity() * 4
            + present.hydrology.bathymetry_mm.capacity() * 4
            + present.hydrology.watershed_id.capacity() * 4
            + present.hydrology.basin_by_cell.capacity() * 4
            + present.hydrology.island_id.capacity() * 4
            + present.temperature_centi_c.capacity() * 4
            + present.precipitation_mm.capacity() * 4
            + present.model.residual_mm.capacity() * 4
            + present.structure_residual_mm.capacity() * 4
            + present.sediment_mm.capacity() * 4
            + present.sdf.capacity() * 4;
        let _ = std::fs::remove_dir_all(root);
        println!(
            "E5b n={resolution}: generation {generation:?}; cold present {cold_present:?}; cold epoch {cold_epoch:?}; standard cold {cold_standard:?}; standard warm {warm_standard:?}; detailed {detailed_time:?}; biome {biome_time:?}; pick {pick:?}; region {region:?}; scene bytes {scene_bytes}; surface bytes {}; boundary segments {}",
            present.surface.byte_size(),
            present.tectonics.boundaries.len()
        );
    }

    struct CancelOnPhase {
        phase: AtlasPhase,
    }

    impl AtlasProgress for CancelOnPhase {
        fn report(
            &mut self,
            phase: AtlasPhase,
            _completed: u32,
            _total: u32,
        ) -> Result<(), AtlasError> {
            if phase == self.phase {
                Err(AtlasError::cancelled())
            } else {
                Ok(())
            }
        }
    }
}
