//! Pure CPU atlas renderer.
//!
//! This crate has no Tauri, SQLite, core, plugin-host, or ambient filesystem
//! dependency. Callers supply validated physical bytes, an opaque physical
//! identity, and an explicit output sink.

pub mod amplify;
pub mod cache;
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
pub const ATLAS_DETAIL_ALGORITHM_VERSION: u32 = 1;
pub const ATLAS_DERIVED_DRAINAGE_VERSION: u32 = 1;
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
    pub hydrology: daena_physical::hydrology::HydrologyField,
    pub sdf: Vec<i32>,
    pub drainage: drainage::DerivedDrainage,
    pub tectonics: daena_physical::tectonics::TectonicWorld,
    pub visible_water: render::VisibleWater,
    pub climate_class: Vec<i32>,
    pub temperature_centi_c: Vec<i32>,
    pub temperature_nh_summer_centi_c: Vec<i32>,
    pub temperature_nh_winter_centi_c: Vec<i32>,
    pub wind_east_milli: Vec<i32>,
    pub wind_north_milli: Vec<i32>,
    pub wind_east_nh_summer_milli: Vec<i32>,
    pub wind_north_nh_summer_milli: Vec<i32>,
    pub wind_east_nh_winter_milli: Vec<i32>,
    pub wind_north_nh_winter_milli: Vec<i32>,
    pub wind_divergence_ppm: Vec<i32>,
    pub wind_divergence_nh_summer_ppm: Vec<i32>,
    pub wind_divergence_nh_winter_ppm: Vec<i32>,
    pub wind_band: Vec<i32>,
    pub wind_band_nh_summer: Vec<i32>,
    pub wind_band_nh_winter: Vec<i32>,
    pub precipitation_mm: Vec<i32>,
    pub residual_cache: cache::CacheLookup,
    pub drainage_cache: cache::CacheLookup,
}

impl AtlasPreparedScene {
    #[must_use]
    pub fn paint_fields(&self) -> render::PaintFields<'_> {
        render::PaintFields {
            climate_class: &self.climate_class,
            temperature_centi_c: &self.temperature_centi_c,
            precipitation_mm: &self.precipitation_mm,
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
            detail::sample_field_mm(grid, &self.temperature_centi_c, lon_micro, lat_micro);
        let temperature_nh_summer_centi_c = detail::sample_field_mm(
            grid,
            &self.temperature_nh_summer_centi_c,
            lon_micro,
            lat_micro,
        );
        let temperature_nh_winter_centi_c = detail::sample_field_mm(
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
            detail::sample_field_mm(grid, &self.wind_east_milli, lon_micro, lat_micro);
        let wind_north_milli =
            detail::sample_field_mm(grid, &self.wind_north_milli, lon_micro, lat_micro);
        let wind_east_nh_summer_milli =
            detail::sample_field_mm(grid, &self.wind_east_nh_summer_milli, lon_micro, lat_micro);
        let wind_north_nh_summer_milli =
            detail::sample_field_mm(grid, &self.wind_north_nh_summer_milli, lon_micro, lat_micro);
        let wind_east_nh_winter_milli =
            detail::sample_field_mm(grid, &self.wind_east_nh_winter_milli, lon_micro, lat_micro);
        let wind_north_nh_winter_milli =
            detail::sample_field_mm(grid, &self.wind_north_nh_winter_milli, lon_micro, lat_micro);
        let wind_divergence_ppm =
            detail::sample_field_mm(grid, &self.wind_divergence_ppm, lon_micro, lat_micro);
        let wind_divergence_nh_summer_ppm = detail::sample_field_mm(
            grid,
            &self.wind_divergence_nh_summer_ppm,
            lon_micro,
            lat_micro,
        );
        let wind_divergence_nh_winter_ppm = detail::sample_field_mm(
            grid,
            &self.wind_divergence_nh_winter_ppm,
            lon_micro,
            lat_micro,
        );
        let wind_band = control::wind_band_name(self.wind_band.get(cell).copied().unwrap_or(0));
        let wind_band_nh_summer =
            control::wind_band_name(self.wind_band_nh_summer.get(cell).copied().unwrap_or(0));
        let wind_band_nh_winter =
            control::wind_band_name(self.wind_band_nh_winter.get(cell).copied().unwrap_or(0));
        let precipitation_mm =
            detail::sample_field_mm(grid, &self.precipitation_mm, lon_micro, lat_micro);
        let climate = control::climate_class_name(
            self.climate_class
                .get(cell)
                .copied()
                .unwrap_or(control::CLIMATE_CLASS_GRASSLAND),
        );
        let ice = self.hydrology.ice_cells.get(cell).copied().unwrap_or(false);
        let inland = self
            .visible_water
            .inland
            .get(cell)
            .copied()
            .unwrap_or(false);
        let ice_thickness_mm = i32::try_from(
            self.hydrology
                .ice_thickness_mm
                .get(cell)
                .copied()
                .unwrap_or(0),
        )
        .unwrap_or(i32::MAX);
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
            wind_divergence_ppm,
            wind_divergence_nh_summer_ppm,
            wind_divergence_nh_winter_ppm,
            wind_band: wind_band.to_string(),
            wind_band_nh_summer: wind_band_nh_summer.to_string(),
            wind_band_nh_winter: wind_band_nh_winter.to_string(),
            precipitation_mm,
            climate: climate.to_string(),
            surface: control::surface_kind(ice, inland, elevation_mm, sea_level_mm).to_string(),
            ice_thickness_mm,
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
                watershed_id: tributary.watershed_id,
                path: tributary.path.clone(),
            })
            .collect(),
    }
}

fn amplification_from_controls(
    field: &daena_physical::PhysicalField,
    world: &daena_physical::tectonics::TectonicWorld,
    historical: &daena_physical::history::HistoricalWorld,
    identity: &[u8],
    variant: u32,
    level: request::DetailLevel,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<amplify::AmplificationModel, AtlasError> {
    let controls = control::ControlFields::from_accepted(
        field,
        world,
        &historical.climate,
        &historical.hydrology,
    )?;
    amplify::build_amplification_model(&controls, identity, variant, level, check_cancelled)
}

pub fn prepare_from_source(
    source_bytes: &[u8],
    identity: &[u8],
    request: &request::AtlasRenderRequest,
    forcing: Option<daena_physical::history::HistoricalForcingParameters>,
    cache: Option<&cache::AtlasDiskCache>,
    progress: &mut dyn AtlasProgress,
) -> Result<AtlasPreparedScene, AtlasError> {
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
    let world = daena_physical::decode_source(source_bytes)
        .map_err(|error| AtlasError::new(CODE_SOURCE_INVALID, error))?;
    let field = world.physical_field();
    let report = daena_physical::validate_field_report(&field)
        .map_err(|error| AtlasError::new(CODE_SOURCE_INVALID, error.to_string()))?;
    progress.report(AtlasPhase::Validating, 1, 1)?;
    progress.report(AtlasPhase::DerivingEpoch, 0, 1)?;
    progress.check_cancelled()?;
    let historical = daena_physical::history::derive_historical_world_with_planet(
        &field,
        report.reference_water_inventory_m3,
        Some(&world.crust_by_cell),
        forcing.unwrap_or_else(|| {
            daena_physical::history::HistoricalForcingParameters::default_for(
                field.seed,
                field.retry_index,
            )
        }),
        request.offset_years,
        daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        &mut daena_physical::NoopProgress,
    )
    .map_err(|error| AtlasError::new(CODE_RENDER_FAILED, error.to_string()))?;
    progress.check_cancelled()?;
    progress.report(AtlasPhase::DerivingEpoch, 1, 1)?;
    progress.report(AtlasPhase::RefiningDetail, 0, 1)?;
    let sdf = detail::signed_coastal_distance_ppm(
        field.grid,
        &field.elevations_mm,
        historical.metrics.sea_level_mm,
    );
    let controls = control::ControlFields::from_accepted(
        &field,
        &world,
        &historical.climate,
        &historical.hydrology,
    )?;
    let residual_key = cache::cache_key(&[
        b"atlas-cache-residual-v1",
        identity,
        &ATLAS_DETAIL_ALGORITHM_VERSION.to_le_bytes(),
        &request.variant.to_le_bytes(),
        request.level.as_str().as_bytes(),
        &field.grid.width.to_le_bytes(),
        &field.grid.height.to_le_bytes(),
        &request.offset_years.to_le_bytes(),
        &forcing_fingerprint,
    ]);
    let mut residual_cache = cache::CacheLookup::Off;
    let expected_width = field
        .grid
        .width
        .saturating_mul(request.level.lattice_factor());
    let expected_height = field
        .grid
        .height
        .saturating_mul(request.level.lattice_factor());
    let mut amplification = if let Some(cache) = cache {
        match cache.get(cache::KIND_RESIDUAL, &residual_key) {
            cache::CacheLookupResult::Hit(payload) => match cache::decode_residual(&payload) {
                Ok((lattice_width, lattice_height, residual_mm))
                    if lattice_width == expected_width && lattice_height == expected_height =>
                {
                    residual_cache = cache::CacheLookup::Hit;
                    amplify::AmplificationModel::from_cached_detail(
                        detail::AtlasDetailModel {
                            grid: field.grid,
                            elevations_mm: field.elevations_mm.clone(),
                            residual_mm,
                            lattice_width,
                            lattice_height,
                            algorithm_version: ATLAS_DETAIL_ALGORITHM_VERSION,
                            variant: request.variant,
                            level: request.level,
                        },
                        &controls,
                        identity,
                    )
                }
                _ => {
                    residual_cache = cache::CacheLookup::Miss;
                    let mut cancelled = || progress.check_cancelled();
                    let model = amplification_from_controls(
                        &field,
                        &world,
                        &historical,
                        identity,
                        request.variant,
                        request.level,
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
                    model
                }
            },
            cache::CacheLookupResult::Miss => {
                residual_cache = cache::CacheLookup::Miss;
                let mut cancelled = || progress.check_cancelled();
                let model = amplification_from_controls(
                    &field,
                    &world,
                    &historical,
                    identity,
                    request.variant,
                    request.level,
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
                model
            }
        }
    } else {
        let mut cancelled = || progress.check_cancelled();
        amplification_from_controls(
            &field,
            &world,
            &historical,
            identity,
            request.variant,
            request.level,
            &mut cancelled,
        )?
    };
    let drainage_key = cache::cache_key(&[
        b"atlas-cache-drainage-v1",
        identity,
        &ATLAS_DERIVED_DRAINAGE_VERSION.to_le_bytes(),
        &request.variant.to_le_bytes(),
        request.level.as_str().as_bytes(),
        &request.offset_years.to_le_bytes(),
        &historical.hydrology.derivation_version.to_le_bytes(),
        &forcing_fingerprint,
    ]);
    let mut drainage_cache = cache::CacheLookup::Off;
    let (drainage, worked_mm) = if let Some(cache) = cache {
        match cache.get(cache::KIND_DRAINAGE, &drainage_key) {
            cache::CacheLookupResult::Hit(payload) => {
                match drainage::DerivedDrainage::decode_product(&payload) {
                    Ok((drainage, width, height, worked_mm))
                        if width == amplification.detail.lattice_width
                            && height == amplification.detail.lattice_height =>
                    {
                        drainage_cache = cache::CacheLookup::Hit;
                        (drainage, worked_mm)
                    }
                    _ => {
                        drainage_cache = cache::CacheLookup::Miss;
                        let mut cancelled = || progress.check_cancelled();
                        let refined = refine::build_refined_hydrology(
                            &amplification,
                            &controls,
                            &historical.hydrology,
                            &sdf,
                            identity,
                            &mut cancelled,
                        )?;
                        let drainage = drainage_from_refined(&refined);
                        let _ = cache.put(
                            cache::KIND_DRAINAGE,
                            &drainage_key,
                            &drainage.encode_product(
                                refined.lattice_width,
                                refined.lattice_height,
                                &refined.worked_mm,
                            ),
                        );
                        (drainage, refined.worked_mm)
                    }
                }
            }
            cache::CacheLookupResult::Miss => {
                drainage_cache = cache::CacheLookup::Miss;
                let mut cancelled = || progress.check_cancelled();
                let refined = refine::build_refined_hydrology(
                    &amplification,
                    &controls,
                    &historical.hydrology,
                    &sdf,
                    identity,
                    &mut cancelled,
                )?;
                let drainage = drainage_from_refined(&refined);
                let _ = cache.put(
                    cache::KIND_DRAINAGE,
                    &drainage_key,
                    &drainage.encode_product(
                        refined.lattice_width,
                        refined.lattice_height,
                        &refined.worked_mm,
                    ),
                );
                (drainage, refined.worked_mm)
            }
        }
    } else {
        let mut cancelled = || progress.check_cancelled();
        let refined = refine::build_refined_hydrology(
            &amplification,
            &controls,
            &historical.hydrology,
            &sdf,
            identity,
            &mut cancelled,
        )?;
        (drainage_from_refined(&refined), refined.worked_mm)
    };
    amplification.detail.bake_absolute_elevation(&worked_mm);
    let model = amplification.detail;
    let visible_water = render::classify_visible_water(
        model.grid,
        &model.elevations_mm,
        historical.hydrology.sea_level_mm,
        &historical.hydrology.lake_cells,
    );
    progress.report(AtlasPhase::RefiningDetail, 1, 1)?;
    Ok(AtlasPreparedScene {
        identity: identity.to_vec(),
        source_sha256,
        style,
        style_hash,
        model,
        hydrology: historical.hydrology,
        sdf,
        drainage,
        tectonics: world,
        visible_water,
        climate_class: controls.climate_class,
        temperature_centi_c: controls.temperature_centi_c,
        temperature_nh_summer_centi_c: controls.temperature_nh_summer_centi_c,
        temperature_nh_winter_centi_c: controls.temperature_nh_winter_centi_c,
        wind_east_milli: controls.wind_east_milli,
        wind_north_milli: controls.wind_north_milli,
        wind_east_nh_summer_milli: controls.wind_east_nh_summer_milli,
        wind_north_nh_summer_milli: controls.wind_north_nh_summer_milli,
        wind_east_nh_winter_milli: controls.wind_east_nh_winter_milli,
        wind_north_nh_winter_milli: controls.wind_north_nh_winter_milli,
        wind_divergence_ppm: controls.wind_divergence_ppm,
        wind_divergence_nh_summer_ppm: controls.wind_divergence_nh_summer_ppm,
        wind_divergence_nh_winter_ppm: controls.wind_divergence_nh_winter_ppm,
        wind_band: controls.wind_band,
        wind_band_nh_summer: controls.wind_band_nh_summer,
        wind_band_nh_winter: controls.wind_band_nh_winter,
        precipitation_mm: controls.precipitation_mm,
        residual_cache,
        drainage_cache,
    })
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
        b"atlas-cache-artifact-v1",
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
        &scene.sdf,
        &scene.style,
        identity,
        &order,
        overlays,
        &scene.drainage.tributaries,
        &scene.tectonics,
        &scene.visible_water,
        scene.paint_fields(),
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
        let identity = spike_identity_from_source(&world.source);
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
        let identity = spike_identity_from_source(&world.source);
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
        let identity = spike_identity_from_source(&world.source);
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
        let identity = spike_identity_from_source(&world.source);
        let report = &world.report;
        let forcing = daena_physical::history::HistoricalForcingParameters::default_for(
            world.field.seed,
            world.field.retry_index,
        );
        let derive = |offset| {
            daena_physical::history::derive_historical_world_with_planet(
                &world.field,
                report.reference_water_inventory_m3,
                Some(&world.tectonics.crust_by_cell),
                forcing,
                offset,
                world.climate.planetary,
                &mut daena_physical::NoopProgress,
            )
            .unwrap()
        };
        let present = derive(0);
        let cold = derive(-8_000);
        let warm = derive(8_000);
        assert_ne!(cold.metrics.sea_level_mm, present.metrics.sea_level_mm);
        assert_ne!(warm.metrics.sea_level_mm, present.metrics.sea_level_mm);
        assert_ne!(cold.metrics.land_ice_m3, warm.metrics.land_ice_m3);
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
        let identity = spike_identity_from_source(&world.source);
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
        let identity = spike_identity_from_source(&world.source);
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
        let identity = spike_identity_from_source(&world.source);
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
        let identity = spike_identity_from_source(&world.source);
        let request = AtlasRenderRequest::spike_png(128, 64).unwrap();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-forcing-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = cache::AtlasDiskCache::open(&root).unwrap();
        let forcing_a = daena_physical::history::HistoricalForcingParameters::default_for(
            world.field.seed,
            world.field.retry_index,
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
