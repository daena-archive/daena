// Physical-world commands and derived data.
use std::collections::VecDeque;

use super::*;

pub(super) const HISTORICAL_CACHE_CAPACITY: usize = 16;
/// Byte budget of the in-process tool-surface cache.
///
/// One n=1024 surface measures about 252 MB by `ToolSurface::heap_bytes`, so
/// this holds the present and one epoch surface with headroom. A single
/// surface larger than the budget is not cached.
pub(super) const TOOL_SURFACE_CACHE_BYTES: usize = 640 * 1024 * 1024;
pub(super) type ToolSurfaceCache =
    Mutex<VecDeque<(String, Arc<daena_physical::engine::ToolSurface>)>>;
pub(super) static TOOL_SURFACE_CACHE: OnceLock<ToolSurfaceCache> = OnceLock::new();
pub(super) static HISTORICAL_EPOCH_CACHE: OnceLock<Mutex<BTreeMap<String, serde_json::Value>>> =
    OnceLock::new();
pub(super) static HISTORICAL_EPOCH_REQUESTS: OnceLock<Mutex<BTreeMap<String, Arc<AtomicU64>>>> =
    OnceLock::new();

fn planetary_from_generation(
    generation: &serde_json::Value,
) -> daena_physical::planetary::PlanetaryConfiguration {
    generation
        .get("settings")
        .and_then(|settings| settings.get("planetary"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(daena_physical::planetary::PlanetaryConfiguration::earth_like)
}

pub(super) fn generation_source_grid(generation: &serde_json::Value) -> Result<(u32, f64), String> {
    let settings = generation
        .get("settings")
        .ok_or_else(|| "physical generation settings are missing".to_string())?;
    let resolution = settings
        .get("resolution")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "physical generation is missing its resolution".to_string())?;
    let radius_metres = settings
        .get("radiusMetres")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as f64)
        .ok_or_else(|| "physical generation is missing its radius".to_string())?;
    Ok((resolution, radius_metres))
}

#[tauri::command]
pub(super) async fn project_physical_generate(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    input: PhysicalGenerationInput,
    request_id: Option<String>,
) -> Result<PhysicalJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before generating a physical map".to_string())?
        .root;
    let session_id = {
        let mut manager = jobs
            .lock()
            .map_err(|_| "physical job state is unavailable".to_string())?;
        manager.ensure_session(&project_id)
    };
    let job_id = uuid::Uuid::new_v4().to_string();
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    uuid::Uuid::parse_str(&request_id)
        .map_err(|_| "physical generation request ID must be a UUID".to_string())?;
    let preset = daena_physical::engine::WorldPreset::parse(&input.settings.preset)
        .ok_or_else(|| format!("unknown world preset {}", input.settings.preset))?;
    input
        .settings
        .planetary
        .validate()
        .map_err(|error| error.to_string())?;
    if input.settings.planetary.radius_metres != input.settings.radius_metres {
        return Err("planetary.radiusMetres must match settings.radiusMetres".into());
    }
    let engine_settings = daena_physical::engine::Settings::from_parts(
        input.seed,
        preset,
        input.settings.resolution,
        input.settings.radius_metres,
        input.settings.target_land_fraction_ppm,
        input.settings.planetary,
    );
    engine_settings
        .validate_product()
        .map_err(|error| error.to_string())?;
    let cancel = Arc::new(AtomicBool::new(false));
    let status = PhysicalJobStatus {
        job_id: job_id.clone(),
        request_id: request_id.clone(),
        state: "running".into(),
        stage: daena_physical::ProgressPhase::BuildingTectonicStructure
            .label()
            .into(),
        completed: 0,
        total: 1,
        error: None,
        error_code: None,
        physical_identity: None,
    };
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .jobs
        .insert(
            job_id.clone(),
            PhysicalJob {
                project_id,
                session_id,
                expires_at: Instant::now() + PHYSICAL_JOB_TTL,
                cancel: cancel.clone(),
                status: status.clone(),
                result: None,
            },
        );
    let jobs_for_worker = jobs.inner().clone();
    let worker_job_id = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut progress = PhysicalProgress {
                jobs: jobs_for_worker.clone(),
                job_id: worker_job_id.clone(),
                cancel: cancel.clone(),
            };
            daena_physical::engine::generate(engine_settings, &mut progress)
        }));
        let mut manager = match jobs_for_worker.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(job) = manager.jobs.get_mut(&worker_job_id) else {
            return;
        };
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(_) => {
                job.status.state = "failed".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.error = Some("worker panicked during physical generation".into());
                job.status.error_code = Some("WORKER_PANIC".into());
                return;
            }
        };
        match outcome {
            Ok(products) => {
                let generation = products
                    .generation()
                    .cloned()
                    .expect("engine generation records its generation metadata");
                let source = products.source().to_vec();
                let physical_identity =
                    match daena_core::maps::physical::validate_source(&source, &generation) {
                        Ok(validated) => validated.identity,
                        Err(error) => {
                            job.status.state = "failed".into();
                            job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                                .label()
                                .into();
                            job.status.error = Some(error.to_string());
                            job.status.error_code =
                                Some(daena_core::maps::physical::CODE_INVALID_SOURCE.into());
                            return;
                        }
                    };
                job.status.state = "completed".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.completed = 1;
                job.status.total = 1;
                job.status.error = None;
                job.status.error_code = None;
                job.status.physical_identity = Some(physical_identity.clone());
                job.result = Some(PhysicalJobResult {
                    source,
                    generation,
                    physical_identity,
                    derived_geojson: products.derived_geojson().to_owned(),
                    climate: products.climate().clone(),
                    hydrology: products.hydrology().clone(),
                    hazards: products.hazards().clone(),
                });
            }
            Err(daena_physical::PhysicalError::Cancelled) => {
                job.status.state = "cancelled".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.error = None;
                job.status.error_code = Some(daena_physical::CODE_GENERATOR_CANCELLED.into());
            }
            Err(error) => {
                job.status.state = "failed".into();
                job.status.stage = daena_physical::ProgressPhase::ValidatingWorld
                    .label()
                    .into();
                job.status.error = Some(error.to_string());
                job.status.error_code = Some(error.code().into());
            }
        }
    });
    Ok(status)
}

#[tauri::command]
pub(super) fn project_physical_status(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<PhysicalJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading a physical job".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    Ok(job.status.clone())
}

#[tauri::command]
pub(super) fn project_physical_cancel(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<PhysicalJobStatus, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before cancelling a physical job".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let session_matches = manager
        .jobs
        .get(&job_id)
        .is_some_and(|job| manager.active_session_matches(&project_id, &job.session_id));
    if !session_matches {
        return Err("physical job was not found, expired, or belongs to another session".into());
    }
    let job = manager
        .jobs
        .get_mut(&job_id)
        .ok_or_else(|| "physical job was not found or has expired".to_string())?;
    if job.status.state == "running" {
        job.cancel.store(true, Ordering::Relaxed);
        job.status.state = "cancelling".into();
    }
    Ok(job.status.clone())
}

#[tauri::command]
pub(super) fn project_physical_preview(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<String, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading a physical preview".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    job.result
        .as_ref()
        .map(|result| result.derived_geojson.clone())
        .ok_or_else(|| "physical job preview is not ready".to_string())
}

pub(super) fn engine_climate_products(
    climate: &daena_physical::products::ClimateField,
) -> serde_json::Value {
    let mut products = serde_json::json!({
        "derivationVersion": climate.derivation_version,
        "width": climate.grid.width,
        "height": climate.grid.height,
        "temperatureCentiC": climate.temperature_centi_c,
        "temperatureNhSummerCentiC": climate.temperature_nh_summer_centi_c,
        "temperatureNhWinterCentiC": climate.temperature_nh_winter_centi_c,
        "precipitationMmPerYear": climate.precipitation_mm_per_year,
        "runoffMmPerYear": climate.runoff_mm_per_year,
        "runoffVolumeM3PerYear": climate.runoff_volume_m3_per_year,
        "maritimeFactorPpm": climate.maritime_factor_ppm,
        "windEastMilli": climate.wind_east_milli,
        "windNorthMilli": climate.wind_north_milli,
        "windEastNhSummerMilli": climate.wind_east_nh_summer_milli,
        "windNorthNhSummerMilli": climate.wind_north_nh_summer_milli,
        "windEastNhWinterMilli": climate.wind_east_nh_winter_milli,
        "windNorthNhWinterMilli": climate.wind_north_nh_winter_milli,
        "windDivergenceNanoPerS": climate.wind_divergence_nano_per_s,
        "windDivergenceNhSummerNanoPerS": climate.wind_divergence_nh_summer_nano_per_s,
        "windDivergenceNhWinterNanoPerS": climate.wind_divergence_nh_winter_nano_per_s,
        "windBand": climate.wind_band,
        "windBandNhSummer": climate.wind_band_nh_summer,
        "windBandNhWinter": climate.wind_band_nh_winter,
        "aridityPpm": climate.aridity_ppm,
        "precipitationNhSummerMm": climate.precipitation_nh_summer_mm,
        "precipitationNhWinterMm": climate.precipitation_nh_winter_mm,
        "biomeClass": climate.biome_class,
        "stormSuitabilityPpm": climate.storm_suitability_ppm,
        "stormTrackPpm": climate.storm_track_ppm,
        "stormIntensityPpm": climate.storm_intensity_ppm,
    });
    products["biomeLegend"] = serde_json::json!(daena_physical::products::biome_legend()
        .iter()
        .map(|(id, name, fill)| serde_json::json!({
            "id": id,
            "name": name,
            "fill": fill,
        }))
        .collect::<Vec<_>>());
    products["metrics"] = serde_json::json!({
            "precipitationVolumeM3PerYear": climate.metrics.precipitation_volume_m3_per_year,
            "runoffVolumeM3PerYear": climate.metrics.runoff_volume_m3_per_year,
            "meanTemperatureCentiC": climate.metrics.mean_temperature_centi_c,
            "minimumTemperatureCentiC": climate.metrics.minimum_temperature_centi_c,
            "maximumTemperatureCentiC": climate.metrics.maximum_temperature_centi_c,
            "meanPrecipitationMmPerYear": climate.metrics.mean_precipitation_mm_per_year,
            "meanRunoffMmPerYear": climate.metrics.mean_runoff_mm_per_year,
            "wettestCellPrecipitationMmPerYear": climate.metrics.wettest_cell_precipitation_mm_per_year,
            "driestLandCellPrecipitationMmPerYear": climate.metrics.driest_land_cell_precipitation_mm_per_year,
            "climateSpinupYears": climate.metrics.climate_spinup_years,
            "meanSeasonalRangeCentiC": climate.metrics.mean_seasonal_range_centi_c,
            "minimumSeasonalTemperatureCentiC": climate.metrics.minimum_seasonal_temperature_centi_c,
            "maximumSeasonalTemperatureCentiC": climate.metrics.maximum_seasonal_temperature_centi_c,
            "permanentlyFrozenLandPpm": climate.metrics.permanently_frozen_land_ppm,
            "seasonallyFrozenLandPpm": climate.metrics.seasonally_frozen_land_ppm,
            "meanWindSpeedMilli": climate.metrics.mean_wind_speed_milli,
            "itczLatitudeMilliDeg": climate.metrics.itcz_latitude_milli_deg,
            "easterlyCellPpm": climate.metrics.easterly_cell_ppm,
            "convergingCellPpm": climate.metrics.converging_cell_ppm,
            "meanLandAridityPpm": climate.metrics.mean_land_aridity_ppm,
            "meanSeasonalPrecipitationRangeMm": climate.metrics.mean_seasonal_precipitation_range_mm,
            "dominantLandBiome": climate.metrics.dominant_land_biome,
            "meanOceanStormSuitabilityPpm": climate.metrics.mean_ocean_storm_suitability_ppm,
            "stormProneOceanPpm": climate.metrics.storm_prone_ocean_ppm,
            "meanStormIntensityPpm": climate.metrics.mean_storm_intensity_ppm,
            "meanLandStormTrackPpm": climate.metrics.mean_land_storm_track_ppm,
            "expectedStormsPerYearMilli": climate.metrics.expected_storms_per_year_milli,
            "meanLandDroughtPotentialPpm": climate.metrics.mean_land_drought_potential_ppm,
            "meanHeatWavePotentialPpm": climate.metrics.mean_heat_wave_potential_ppm,
            "meanExtremeRainfallPotentialPpm": climate.metrics.mean_extreme_rainfall_potential_ppm,
            "meanLandGrowingSeasonPpm": climate.metrics.mean_land_growing_season_ppm,
            "meanLandDrySeasonPpm": climate.metrics.mean_land_dry_season_ppm,
            "meanLandWetSeasonPpm": climate.metrics.mean_land_wet_season_ppm,
    });
    products
}

pub(super) fn engine_hydrology_products(
    hydrology: &daena_physical::products::HydrologyField,
) -> serde_json::Value {
    serde_json::json!({
        "derivationVersion": hydrology.derivation_version,
        "width": hydrology.grid.width,
        "height": hydrology.grid.height,
        "seaLevelMm": hydrology.sea_level_mm,
        "waterLevelMm": hydrology.water_level_mm,
        "lakeLevelMm": hydrology.lake_level_mm,
        "slopePpm": hydrology.slope_ppm,
        "hillshadePpm": hydrology.hillshade_ppm,
        "bathymetryMm": hydrology.bathymetry_mm,
        "watershedId": hydrology.watershed_id,
        "basinByCell": hydrology.basin_by_cell,
        "lakeCells": hydrology.lake_cells,
        "iceCells": hydrology.ice_cells,
        "shelfCells": hydrology.shelf_cells,
        "islandId": hydrology.island_id,
        "basins": hydrology.basins.iter().map(|basin| serde_json::json!({
            "id": basin.id,
            "minimumCell": basin.minimum_cell,
            "minimumElevationMm": basin.minimum_elevation_mm,
            "cellCount": basin.cell_count,
            "spillCell": basin.spill_cell,
            "spillElevationMm": basin.spill_elevation_mm,
            "waterLevelMm": basin.water_level_mm,
            "waterVolumeM3": basin.water_volume_m3,
            "inflowM3PerYear": basin.inflow_m3_per_year,
            "evaporationM3PerYear": basin.evaporation_m3_per_year,
            "outflowM3PerYear": basin.outflow_m3_per_year,
            "status": basin.status.label(),
        })).collect::<Vec<_>>(),
        "rivers": hydrology.rivers.iter().map(|river| serde_json::json!({
            "id": river.id,
            "sourceCell": river.source_cell,
            "mouthCell": river.mouth_cell,
            "strahlerOrder": river.strahler_order,
            "destination": river.destination.label(),
            "spillOutlet": river.spill_outlet,
            "coordinateCount": river.coordinate_count,
        })).collect::<Vec<_>>(),
        "metrics": {
            "lakeCount": hydrology.metrics.lake_count,
            "riverCount": hydrology.metrics.river_count,
            "watershedCount": hydrology.metrics.watershed_count,
            "coastlineSegmentCount": hydrology.metrics.coastline_segment_count,
            "landPolygonCount": hydrology.metrics.land_polygon_count,
            "oceanPolygonCount": hydrology.metrics.ocean_polygon_count,
            "shelfCellCount": hydrology.metrics.shelf_cell_count,
            "bathymetryContourCount": hydrology.metrics.bathymetry_contour_count,
            "islandCount": hydrology.metrics.island_count,
        },
    })
}

pub(super) fn engine_hazard_products(
    hazards: &daena_physical::products::HazardField,
    resolution: u32,
    radius_metres: f64,
) -> serde_json::Value {
    let volcanic_sources = hazards
        .volcanic_sources
        .iter()
        .map(|source| {
            // Hazard sources come from a validated engine world, so the cell is
            // always inside the canonical grid. Keep the command total anyway.
            let [longitude_micro, latitude_micro] =
                daena_physical::engine::canonical_cell_center_microdegrees(
                    resolution,
                    radius_metres,
                    source.cell,
                )
                .unwrap_or([0, 0]);
            serde_json::json!({
                "stableId": source.stable_id.to_string(),
                "cell": source.cell,
                "longitudeMicro": longitude_micro,
                "latitudeMicro": latitude_micro,
                "origin": source.origin.label(),
                "activityClass": source.activity_class.label(),
                "eruptionsPerModelYearNano": source.eruptions_per_model_year_nano,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "derivationVersion": hazards.metrics.derivation_version,
        "volcanicSourceDerivationVersion": hazards.metrics.volcanic_source_derivation_version,
        "earthquakeHazardPpm": hazards.earthquake_hazard_ppm,
        "volcanicHazardPpm": hazards.volcanic_hazard_ppm,
        "earthquakeAnnualRateNano": hazards.earthquake_annual_rate_nano,
        "volcanicAnnualRateNano": hazards.volcanic_annual_rate_nano,
        "earthquakeRatePerMillionYearsPpm": hazards.earthquake_rate_per_million_years_ppm,
        "eruptionRatePerMillionYearsPpm": hazards.eruption_rate_per_million_years_ppm,
        "volcanicSources": volcanic_sources,
        "metrics": {
            "volcanicSourceCount": hazards.metrics.volcanic_source_count,
            "nonDormantSourceCount": hazards.metrics.non_dormant_source_count,
            "earthquakeMeanPpm": hazards.metrics.earthquake_mean_ppm,
            "earthquakeMaxPpm": hazards.metrics.earthquake_max_ppm,
            "volcanicMeanPpm": hazards.metrics.volcanic_mean_ppm,
            "volcanicMaxPpm": hazards.metrics.volcanic_max_ppm,
            "earthquakeRateMassNano": hazards.metrics.earthquake_rate_mass_nano,
            "volcanicRateMassNano": hazards.metrics.volcanic_rate_mass_nano,
        },
    })
}

pub(super) fn historical_forcing_from_generation(
    generation: &serde_json::Value,
) -> Result<daena_physical::history::HistoricalForcingParameters, String> {
    let Some(value) = generation
        .get("settings")
        .and_then(|settings| settings.get("historicalForcing"))
    else {
        return Err("historicalForcing is required for physical sources".into());
    };
    let settings: daena_core::maps::HistoricalForcingSettings =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if settings.components.len() != daena_physical::history::FORCING_COMPONENT_COUNT {
        return Err("historicalForcing.components must contain three independent terms".into());
    }
    let parameters = daena_physical::history::HistoricalForcingParameters {
        version: settings.version,
        components: [
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: settings.components[0].amplitude_centi_c,
                period_years: settings.components[0].period_years,
                phase_offset_years: settings.components[0].phase_offset_years,
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: settings.components[1].amplitude_centi_c,
                period_years: settings.components[1].period_years,
                phase_offset_years: settings.components[1].phase_offset_years,
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: settings.components[2].amplitude_centi_c,
                period_years: settings.components[2].period_years,
                phase_offset_years: settings.components[2].phase_offset_years,
            },
        ],
        sensitivity_ppm: settings.sensitivity_ppm,
        land_ice_amplitude_ppm: settings.land_ice_amplitude_ppm,
        ice_response_years: settings.ice_response_years,
        ice_midpoint_centi_c: settings.ice_midpoint_centi_c,
        ice_transition_width_centi_c: settings.ice_transition_width_centi_c,
        thermal_expansion_ppm_per_degree_c: settings.thermal_expansion_ppm_per_degree_c,
    };
    parameters.validate().map_err(|error| error.to_string())?;
    Ok(parameters)
}

pub(super) fn historical_forcing_products(
    parameters: daena_physical::history::HistoricalForcingParameters,
) -> serde_json::Value {
    serde_json::json!({
        "version": parameters.version,
        "components": parameters.components.iter().map(|component| serde_json::json!({
            "amplitudeCentiC": component.amplitude_centi_c,
            "periodYears": component.period_years,
            "phaseOffsetYears": component.phase_offset_years,
        })).collect::<Vec<_>>(),
        "sensitivityPpm": parameters.sensitivity_ppm,
        "landIceAmplitudePpm": parameters.land_ice_amplitude_ppm,
        "iceResponseYears": parameters.ice_response_years,
        "iceMidpointCentiC": parameters.ice_midpoint_centi_c,
        "iceTransitionWidthCentiC": parameters.ice_transition_width_centi_c,
        "thermalExpansionPpmPerDegreeC": parameters.thermal_expansion_ppm_per_degree_c,
    })
}

pub(super) fn historical_epoch_cache() -> &'static Mutex<BTreeMap<String, serde_json::Value>> {
    HISTORICAL_EPOCH_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub(super) fn clear_historical_epoch_cache() -> Result<(), String> {
    if let Some(requests) = HISTORICAL_EPOCH_REQUESTS.get() {
        for generation in requests
            .lock()
            .map_err(|_| "historical request state is unavailable".to_string())?
            .values()
        {
            generation.fetch_add(1, Ordering::AcqRel);
        }
    }
    historical_epoch_cache()
        .lock()
        .map_err(|_| "historical cache is unavailable".to_string())?
        .clear();
    Ok(())
}

pub(super) fn historical_cache_key(
    physical_identity: &str,
    forcing: daena_physical::history::HistoricalForcingParameters,
    normalized_epoch: i64,
    planetary: daena_physical::planetary::PlanetaryConfiguration,
    view_width: u32,
    view_height: u32,
) -> String {
    format!(
        "{physical_identity}|fmt:{}|engine:{}|history-v{}|hazards-v{}|volcanic-v{}|climate-v{}|hydro-v{}|view:{}x{}|planet:{}|epoch:{normalized_epoch}|forcing:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        daena_physical::engine::SOURCE_VERSION,
        daena_physical::engine::WORLD_ENGINE_VERSION,
        daena_physical::history::HISTORICAL_DERIVATION_VERSION,
        daena_physical::products::HAZARD_DERIVATION_VERSION,
        daena_physical::products::VOLCANIC_SOURCE_DERIVATION_VERSION,
        daena_physical::products::CLIMATE_DERIVATION_VERSION,
        daena_physical::products::HYDROLOGY_DERIVATION_VERSION,
        view_width,
        view_height,
        planetary.climate_cache_token(),
        forcing.version,
        forcing.components[0].amplitude_centi_c,
        forcing.components[0].period_years,
        forcing.components[0].phase_offset_years,
        forcing.components[1].amplitude_centi_c,
        forcing.components[1].period_years,
        forcing.components[1].phase_offset_years,
        forcing.components[2].amplitude_centi_c,
        forcing.components[2].period_years,
        forcing.components[2].phase_offset_years,
        forcing.sensitivity_ppm,
        forcing.land_ice_amplitude_ppm,
        forcing.ice_response_years,
        forcing.ice_midpoint_centi_c,
        forcing.ice_transition_width_centi_c,
        forcing.thermal_expansion_ppm_per_degree_c,
    )
}

pub(super) fn hash_json_value(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).expect("JSON values used for hashes are serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn begin_historical_epoch_request(
    map_entity_id: &str,
) -> Result<(Arc<AtomicU64>, u64), String> {
    let requests = HISTORICAL_EPOCH_REQUESTS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut requests = requests
        .lock()
        .map_err(|_| "historical request state is unavailable".to_string())?;
    let generation = requests
        .entry(map_entity_id.to_string())
        .or_insert_with(|| Arc::new(AtomicU64::new(0)))
        .clone();
    let expected = generation.fetch_add(1, Ordering::AcqRel).saturating_add(1);
    Ok((generation, expected))
}

pub(super) fn engine_static_derived(
    decoded: &daena_physical::engine::DecodedSource,
    planetary: daena_physical::planetary::PlanetaryConfiguration,
) -> Result<daena_physical::derived_cache::StaticDerivedPhysics, String> {
    let mut progress = daena_physical::NoopProgress;
    let derived = decoded
        .derive_products(planetary, &mut progress)
        .map_err(|error| format!("maps.physical: {error}"))?;
    Ok(daena_physical::derived_cache::StaticDerivedPhysics::new(
        derived.view,
        derived.climate,
        derived.hydrology,
        derived.hazards,
        derived.derived_geojson,
        derived.hashes,
    ))
}

pub(super) fn load_or_fill_engine_static(
    project_root: &str,
    identity: &str,
    decoded: &daena_physical::engine::DecodedSource,
    planetary: daena_physical::planetary::PlanetaryConfiguration,
) -> Result<daena_physical::derived_cache::StaticDerivedPhysics, String> {
    let cache_dir = daena_core::maps::physical::physical_derived_cache_dir(
        Path::new(project_root),
        identity,
        planetary,
        daena_physical::engine::PRODUCT_VIEW_WIDTH,
        daena_physical::engine::PRODUCT_VIEW_HEIGHT,
    )
    .map_err(|error| error.to_string())?;
    if let Some(hit) = daena_physical::derived_cache::load(&cache_dir)
        .map_err(|error| format!("maps.physical: {error}"))?
    {
        if hit.matches_hashes(&decoded.hashes()) {
            return Ok(hit);
        }
    }
    let physics = engine_static_derived(decoded, planetary)?;
    let _ = daena_physical::derived_cache::save(&cache_dir, &physics);
    Ok(physics)
}

pub(super) fn tool_surface_cache() -> &'static ToolSurfaceCache {
    TOOL_SURFACE_CACHE.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub(super) fn clear_tool_surface_cache() -> Result<(), String> {
    tool_surface_cache()
        .lock()
        .map_err(|_| "tool surface cache is unavailable".to_string())?
        .clear();
    Ok(())
}

fn cached_tool_surface(key: &str) -> Option<Arc<daena_physical::engine::ToolSurface>> {
    let mut cache = tool_surface_cache().lock().ok()?;
    let index = cache.iter().position(|(entry, _)| entry == key)?;
    let entry = cache.remove(index)?;
    cache.push_back(entry.clone());
    Some(entry.1)
}

fn store_tool_surface(key: String, surface: Arc<daena_physical::engine::ToolSurface>) {
    let surface_bytes = surface.heap_bytes();
    if surface_bytes > TOOL_SURFACE_CACHE_BYTES {
        return;
    }
    let Ok(mut cache) = tool_surface_cache().lock() else {
        return;
    };
    if let Some(index) = cache.iter().position(|(entry, _)| entry == &key) {
        cache.remove(index);
    }
    let mut used = cache
        .iter()
        .map(|(_, entry)| entry.heap_bytes())
        .sum::<usize>();
    while let Some((_, oldest)) = cache.front() {
        if used.saturating_add(surface_bytes) <= TOOL_SURFACE_CACHE_BYTES {
            break;
        }
        used = used.saturating_sub(oldest.heap_bytes());
        cache.pop_front();
    }
    cache.push_back((key, surface));
}

/// Return the compact tool surface for one map and epoch. The small in-process
/// LRU holds the present and one epoch; a cold surface re-derives the world and
/// drops it once the surface exists. The cache key reuses the historical key
/// shape, so physical identity, engine and source versions, the planetary
/// token, the normalized epoch and every forcing field separate entries.
pub(super) fn tool_surface_for_source(
    physical_identity: &str,
    decoded: &daena_physical::engine::DecodedSource,
    generation: &serde_json::Value,
    epoch_offset_years: i64,
) -> Result<Arc<daena_physical::engine::ToolSurface>, String> {
    let forcing = historical_forcing_from_generation(generation)?;
    let planetary = planetary_from_generation(generation);
    let normalized_epoch = daena_physical::history::normalize_epoch_offset(epoch_offset_years)
        .map_err(|error| error.to_string())?;
    let cache_key = historical_cache_key(
        physical_identity,
        forcing,
        normalized_epoch,
        planetary,
        daena_physical::engine::PRODUCT_VIEW_WIDTH,
        daena_physical::engine::PRODUCT_VIEW_HEIGHT,
    );
    if let Some(surface) = cached_tool_surface(&cache_key) {
        return Ok(surface);
    }
    let mut progress = daena_physical::NoopProgress;
    let surface = if normalized_epoch == 0 {
        decoded.derive_tool_surface_present(&mut progress)
    } else {
        let solved =
            daena_physical::history::solve_epoch(decoded, forcing, normalized_epoch, &progress)
                .map_err(|error| error.to_string())?;
        decoded.derive_tool_surface_epoch(
            solved.sea_level_mm,
            f64::from(solved.temperature_offset_centi_c) / 100.0,
            &mut progress,
        )
    }
    .map_err(|error| format!("maps.physical: {error}"))?;
    let surface = Arc::new(surface);
    store_tool_surface(cache_key, surface.clone());
    Ok(surface)
}

pub(super) fn physical_find_place_response(
    physical_identity: &str,
    decoded: &daena_physical::engine::DecodedSource,
    generation: &serde_json::Value,
    epoch_offset_years: i64,
    query: &daena_physical::find_place::FindPlaceQuery,
) -> Result<daena_physical::find_place::FindPlaceResult, String> {
    let surface =
        tool_surface_for_source(physical_identity, decoded, generation, epoch_offset_years)?;
    daena_physical::find_place::find_place_canonical(&surface, query)
        .map_err(|error| error.to_string())
}

pub(super) fn physical_routes_response(
    physical_identity: &str,
    decoded: &daena_physical::engine::DecodedSource,
    generation: &serde_json::Value,
    epoch_offset_years: i64,
    query: &daena_physical::routing::RouteSuggestQuery,
) -> Result<daena_physical::routing::RouteSuggestResult, String> {
    let surface =
        tool_surface_for_source(physical_identity, decoded, generation, epoch_offset_years)?;
    daena_physical::routing::suggest_routes_canonical(&surface, query)
        .map_err(|error| error.to_string())
}

pub(super) fn physical_region_response(
    physical_identity: &str,
    decoded: &daena_physical::engine::DecodedSource,
    generation: &serde_json::Value,
    epoch_offset_years: i64,
    lon_micro: i32,
    lat_micro: i32,
    detector: &str,
) -> Result<crate::atlas_studio::AtlasRegionProposal, String> {
    let surface =
        tool_surface_for_source(physical_identity, decoded, generation, epoch_offset_years)?;
    let proposal = daena_physical::region::propose_region_canonical(
        &surface,
        lon_micro,
        lat_micro,
        detector,
        daena_physical::engine::PRODUCT_VIEW_WIDTH,
        daena_physical::engine::PRODUCT_VIEW_HEIGHT,
    )
    .map_err(|error| error.to_string())?;
    crate::atlas_studio::region_proposal_geometry(proposal)
}

pub(super) fn engine_historical_hashes(
    source_hash: &str,
    geojson: &str,
    climate: &serde_json::Value,
    hydrology: &serde_json::Value,
    view: &daena_physical::PhysicalField,
) -> serde_json::Value {
    let elevation = serde_json::to_value(&view.elevations_mm)
        .expect("elevation fields used for hashes are serializable");
    serde_json::json!({
        "canonicalSource": source_hash,
        "finalElevation": hash_json_value(&elevation),
        "tectonics": source_hash,
        "geography": format!("sha256:{:x}", Sha256::digest(geojson.as_bytes())),
        "climate": hash_json_value(climate),
        "hydrology": hash_json_value(hydrology),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn engine_historical_response(
    source_hash: &str,
    physical_identity: &str,
    cache_key: String,
    epoch_offset_years: i64,
    historical: &daena_physical::history::DerivedHistoricalWorld,
    parameters: daena_physical::history::HistoricalForcingParameters,
    resolution: u32,
    radius_metres: f64,
) -> serde_json::Value {
    let products = &historical.products;
    let climate = engine_climate_products(&products.climate);
    let hydrology = engine_hydrology_products(&products.hydrology);
    let derived_hashes = engine_historical_hashes(
        source_hash,
        &products.derived_geojson,
        &climate,
        &hydrology,
        &products.view,
    );
    serde_json::json!({
        "cacheKey": cache_key,
        "sourceHash": source_hash,
        "physicalIdentity": physical_identity,
        "epochOffsetYears": historical.metrics.epoch_offset_years,
        "normalizedEpoch": historical.metrics.normalized_epoch,
        "chronology": {
            "contractVersion": 1,
            "kind": "physical-offset-years",
            "reference": "accepted-source",
            "epochOffsetYears": epoch_offset_years,
        },
        "geojson": products.derived_geojson,
        "climate": climate,
        "hydrology": hydrology,
        "hazards": engine_hazard_products(&products.hazards, resolution, radius_metres),
        "derivedHashes": derived_hashes,
        "forcing": historical_forcing_products(parameters),
        "history": {
            "derivationVersion": historical.metrics.derivation_version,
            "epochOffsetYears": historical.metrics.epoch_offset_years,
            "normalizedEpoch": historical.metrics.normalized_epoch,
            "temperatureOffsetCentiC": historical.metrics.temperature_offset_centi_c,
            "laggedTemperatureOffsetCentiC": historical.metrics.lagged_temperature_offset_centi_c,
            "landIceEquilibriumM3": historical.metrics.land_ice_equilibrium_m3,
            "landIceM3": historical.metrics.land_ice_m3,
            "thermalExpansionM3": historical.metrics.thermal_expansion_m3,
            "effectiveOceanWaterM3": historical.metrics.effective_ocean_water_m3,
            "conservedWaterM3": historical.metrics.conserved_water_m3,
            "balanceErrorM3": historical.metrics.balance_error_m3,
            "seaLevelMm": historical.metrics.sea_level_mm,
        },
    })
}

#[tauri::command]
pub(super) fn project_physical_climate(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical climate".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    let climate = job
        .result
        .as_ref()
        .map(|result| &result.climate)
        .ok_or_else(|| "physical job climate is not ready".to_string())?;
    Ok(engine_climate_products(climate))
}

#[tauri::command]
pub(super) fn project_physical_hydrology(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical hydrology".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    let result = job
        .result
        .as_ref()
        .ok_or_else(|| "physical job hydrology is not ready".to_string())?;
    Ok(engine_hydrology_products(&result.hydrology))
}

#[tauri::command]
pub(super) fn project_physical_hazards(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical hazards".to_string())?
        .root;
    let mut manager = jobs
        .lock()
        .map_err(|_| "physical job state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .filter(|job| manager.active_session_matches(&project_id, &job.session_id))
        .ok_or("physical job was not found, expired, or belongs to another session")?;
    let result = job
        .result
        .as_ref()
        .ok_or_else(|| "physical job hazards are not ready".to_string())?;
    let (resolution, radius_metres) = generation_source_grid(&result.generation)?;
    Ok(engine_hazard_products(
        &result.hazards,
        resolution,
        radius_metres,
    ))
}

#[tauri::command]
pub(super) async fn project_physical_accept(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
    name: String,
    request_id: Option<String>,
) -> Result<daena_core::AcceptedPhysicalMap, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before accepting a physical map".to_string())?
        .root;
    let result = {
        let mut manager = jobs
            .lock()
            .map_err(|_| "physical job state is unavailable".to_string())?;
        manager.reap_expired();
        let job = manager
            .jobs
            .get(&job_id)
            .ok_or_else(|| "physical job was not found or has expired".to_string())?;
        if job.project_id != project_id {
            return Err("physical job belongs to a different project".into());
        }
        if !manager.active_session_matches(&project_id, &job.session_id) {
            return Err("physical job belongs to another session".into());
        }
        if job.status.state != "completed" {
            return Err("physical job is not ready for acceptance".into());
        }
        job.result
            .clone()
            .ok_or_else(|| "physical job result is missing".to_string())?
    };
    let expected_identity = result.physical_identity.clone();
    let resolution = result
        .generation
        .get("settings")
        .and_then(|settings| settings.get("resolution"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "physical generation is missing its resolution".to_string())?;
    if !matches!(resolution, 512 | 1024) {
        return Err(format!(
            "{}: physical map resolution must be 512 or 1024",
            daena_core::maps::physical::CODE_INVALID_SOURCE
        ));
    }
    let accepted = with_core(state, move |core| {
        let accepted = core.project(trusted_shell())?.accept_physical_map(
            name,
            result.source,
            result.generation,
            request_id.as_deref(),
        )?;
        if accepted.physical_identity != expected_identity {
            return Err(CoreError::Validation(
                "physical identity changed between generation and acceptance".into(),
            ));
        }
        Ok(accepted)
    })
    .await?;
    jobs.lock()
        .map_err(|_| "physical job state is unavailable".to_string())?
        .jobs
        .remove(&job_id);
    Ok(accepted)
}

#[tauri::command]
pub(super) async fn project_physical_derived_geojson(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<String, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let planetary = planetary_from_generation(&generation);
        let physics = load_or_fill_engine_static(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            &validated.source,
            planetary,
        )
        .map_err(CoreError::Validation)?;
        Ok(physics.geojson)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_derived_climate(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<serde_json::Value, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let planetary = planetary_from_generation(&generation);
        let physics = load_or_fill_engine_static(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            &validated.source,
            planetary,
        )
        .map_err(CoreError::Validation)?;
        Ok(engine_climate_products(&physics.climate))
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_derived_hydrology(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<serde_json::Value, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let planetary = planetary_from_generation(&generation);
        let physics = load_or_fill_engine_static(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            &validated.source,
            planetary,
        )
        .map_err(CoreError::Validation)?;
        Ok(engine_hydrology_products(&physics.hydrology))
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_derived_hazards(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<serde_json::Value, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let planetary = planetary_from_generation(&generation);
        let physics = load_or_fill_engine_static(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            &validated.source,
            planetary,
        )
        .map_err(CoreError::Validation)?;
        let snapshot = validated.source.settings_snapshot();
        Ok(engine_hazard_products(
            &physics.hazards,
            snapshot.resolution,
            snapshot.radius_m,
        ))
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_derived_epoch(
    state: tauri::State<'_, SharedCore>,
    app: tauri::AppHandle,
    map_entity_id: String,
    epoch_offset_years: i64,
    request_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let (request_generation, expected_generation) = begin_historical_epoch_request(&map_entity_id)?;
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let physical_identity = validated.identity.clone();
        let planetary = planetary_from_generation(&generation);
        let forcing =
            historical_forcing_from_generation(&generation).map_err(CoreError::Validation)?;
        let normalized_epoch = daena_physical::history::normalize_epoch_offset(epoch_offset_years)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let source_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let cache_key = historical_cache_key(
            &physical_identity,
            forcing,
            normalized_epoch,
            planetary,
            daena_physical::engine::PRODUCT_VIEW_WIDTH,
            daena_physical::engine::PRODUCT_VIEW_HEIGHT,
        );
        {
            let cache = historical_epoch_cache()
                .lock()
                .map_err(|_| CoreError::Validation("historical cache is unavailable".into()))?;
            if let Some(value) = cache.get(&cache_key) {
                return Ok(value.clone());
            }
        }
        let physics = load_or_fill_engine_static(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &physical_identity,
            &validated.source,
            planetary,
        )
        .map_err(CoreError::Validation)?;
        let historical = daena_physical::history::derive_historical_products(
            &validated.source,
            &physics,
            forcing,
            normalized_epoch,
            planetary,
            &mut HistoricalProgress::with_reporter(
                request_generation,
                expected_generation,
                app,
                map_entity_id.clone(),
                request_id,
            ),
        )
        .map_err(|error| CoreError::Validation(error.to_string()))?;
        let snapshot = validated.source.settings_snapshot();
        let value = engine_historical_response(
            &source_hash,
            &physical_identity,
            cache_key.clone(),
            epoch_offset_years,
            &historical,
            forcing,
            snapshot.resolution,
            snapshot.radius_m,
        );
        let mut cache = historical_epoch_cache()
            .lock()
            .map_err(|_| CoreError::Validation("historical cache is unavailable".into()))?;
        while cache.len() >= HISTORICAL_CACHE_CAPACITY {
            let Some(oldest) = cache.keys().next().cloned() else {
                break;
            };
            cache.remove(&oldest);
        }
        cache.insert(cache_key, value.clone());
        Ok(value)
    })
    .await
}

fn physical_event_materialization_key(
    event: &daena_physical::events::MaterializedEvent,
    hazard_seed: u64,
    map_entity_id: &str,
    identity: &str,
) -> String {
    if event.event_kind == daena_physical::events::NaturalEventKind::Flood {
        let minimum_cell = event
            .sampled_center_id
            .and_then(|id| u32::try_from(id).ok())
            .unwrap_or(event.cell);
        daena_physical::events::flood_materialization_key(
            map_entity_id,
            identity,
            daena_physical::products::HYDROLOGY_DERIVATION_VERSION,
            minimum_cell,
            event.year_offset,
        )
    } else {
        format!(
            "{}:{}:{}",
            event.event_kind.label(),
            hazard_seed,
            event.ordinal
        )
    }
}

pub(super) fn deterministic_event_location_id(request_id: &str, ordinal: u32) -> String {
    let digest =
        Sha256::digest(format!("daena-physical-event-location:{request_id}:{ordinal}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // UUIDv4-shaped IDs are accepted by the canonical maps.locations schema;
    // the bytes remain deterministic for idempotent retries.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

pub(super) fn physical_materialize_events_response(
    project: &ProjectStore,
    map_entity_id: &str,
    request: &daena_physical::events::EventMaterializationRequest,
    request_id: Option<String>,
) -> Result<PhysicalEventMaterializationResult, CoreError> {
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let provider = project.map_provider_id(map_entity_id)?;
    if provider != daena_core::maps::PHYSICAL_PROVIDER {
        return Err(CoreError::Validation(
            "natural-event materialization requires a daena-physical map".into(),
        ));
    }
    let descriptor = project
        .list_fields(map_entity_id.to_string())?
        .into_iter()
        .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
        .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
    let source_id = descriptor
        .value
        .get("sourceAssetId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
    let generation = descriptor
        .value
        .get("generation")
        .cloned()
        .ok_or_else(|| CoreError::Validation("maps: physical generation is missing".into()))?;
    let bytes = project.asset_bytes(source_id.to_string())?;
    let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
    let source = &validated.source;
    let uses_epoch = matches!(
        request.event_kind,
        daena_physical::events::NaturalEventKind::Storm
            | daena_physical::events::NaturalEventKind::Flood
    );
    let epoch_solve = if uses_epoch {
        let forcing =
            historical_forcing_from_generation(&generation).map_err(CoreError::Validation)?;
        let normalized_epoch =
            daena_physical::history::normalize_epoch_offset(request.interval_midpoint_years())
                .map_err(|error| CoreError::Validation(error.to_string()))?;
        Some(
            daena_physical::history::solve_epoch(
                source,
                forcing,
                normalized_epoch,
                &daena_physical::NoopProgress,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))?,
        )
    } else {
        None
    };
    let climate_epoch = epoch_solve
        .as_ref()
        .map_or(0, |solve| solve.normalized_epoch);
    let events = source
        .sample_events(
            epoch_solve.as_ref(),
            request,
            &mut daena_physical::NoopProgress,
        )
        .map_err(|error| CoreError::Validation(error.to_string()))?
        .events;
    let settings = source.settings_snapshot();
    let resolution = settings.resolution;
    let source_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
    let mut existing_ids = vec![None; events.len()];
    if request.event_kind == daena_physical::events::NaturalEventKind::Flood {
        for (index, event) in events.iter().enumerate() {
            let key = physical_event_materialization_key(
                event,
                request.hazard_seed,
                map_entity_id,
                &validated.identity,
            );
            existing_ids[index] = project
                .entity_ids_with_field_value(
                    daena_core::maps::PHYSICAL_EVENT_NAMESPACE,
                    "materializationKey",
                    &serde_json::json!(key),
                )?
                .into_iter()
                .next();
        }
    }
    let entries = events
        .iter()
        .enumerate()
        .filter(|(index, _)| existing_ids[*index].is_none())
        .map(|(_, event)| {
            let strength = match event.event_kind {
                daena_physical::events::NaturalEventKind::Storm => {
                    format!("I {:.1}", f64::from(event.magnitude_milli) / 1_000.0)
                }
                daena_physical::events::NaturalEventKind::Flood => {
                    format!("spill {:.1}", f64::from(event.magnitude_milli) / 1_000.0)
                }
                _ => format!("M {:.3}", f64::from(event.magnitude_milli) / 1_000.0),
            };
            let name = format!(
                "{} · year {} · {}",
                event.event_kind.label(),
                event.year_offset,
                strength
            );
            let materialization_key = physical_event_materialization_key(
                event,
                request.hazard_seed,
                map_entity_id,
                &validated.identity,
            );
            let location_id = deterministic_event_location_id(&request_id, event.ordinal);
            let x = (f64::from(event.longitude_microdegrees) / 1_000_000.0 + 180.0) / 360.0;
            let y = (f64::from(event.latitude_microdegrees) / 1_000_000.0 + 90.0) / 180.0;
            let track_points = event
                .track_cells
                .iter()
                .map(|cell| {
                    let [longitude, latitude] = source
                        .cell_center_microdegrees(*cell)
                        .map_err(|error| CoreError::Validation(error.to_string()))?;
                    Ok([
                        (f64::from(longitude) / 1_000_000.0 + 180.0) / 360.0,
                        (f64::from(latitude) / 1_000_000.0 + 90.0) / 180.0,
                    ])
                })
                .collect::<Result<Vec<_>, CoreError>>()?;
            let anchor = match event.event_kind {
                daena_physical::events::NaturalEventKind::Flood => {
                    let minimum_cell = event
                        .sampled_center_id
                        .and_then(|id| u32::try_from(id).ok())
                        .unwrap_or(event.cell);
                    serde_json::json!({
                        "kind": "provider-feature",
                        "provider": daena_core::maps::PHYSICAL_PROVIDER,
                        "featureKind": daena_core::maps::PHYSICAL_LAKE_FEATURE_KIND,
                        "featureId": daena_physical::hydro_claim::canonical_lake_id(minimum_cell),
                        "fallbackPoint": [x, y],
                    })
                }
                daena_physical::events::NaturalEventKind::Storm if track_points.len() >= 2 => {
                    serde_json::json!({"kind": "path", "points": track_points})
                }
                _ => serde_json::json!({"kind": "point", "point": [x, y]}),
            };
            let mut provenance = serde_json::Map::new();
            provenance.insert(
                "materializationVersion".into(),
                serde_json::json!(event.event_kind.materialization_version()),
            );
            provenance.insert(
                "eventModel".into(),
                serde_json::json!(event.event_kind.model_label()),
            );
            provenance.insert(
                "eventKind".into(),
                serde_json::json!(event.event_kind.label()),
            );
            provenance.insert(
                "intervalStartYears".into(),
                serde_json::json!(request.interval_start_years),
            );
            provenance.insert(
                "intervalEndYears".into(),
                serde_json::json!(request.interval_end_years),
            );
            provenance.insert("yearOffset".into(), serde_json::json!(event.year_offset));
            provenance.insert("cell".into(), serde_json::json!(event.cell));
            provenance.insert("cellKind".into(), serde_json::json!("canonical-cube"));
            provenance.insert("sourceResolution".into(), serde_json::json!(resolution));
            provenance.insert(
                "longitudeMicrodegrees".into(),
                serde_json::json!(event.longitude_microdegrees),
            );
            provenance.insert(
                "latitudeMicrodegrees".into(),
                serde_json::json!(event.latitude_microdegrees),
            );
            provenance.insert(
                "magnitudeMilli".into(),
                serde_json::json!(event.magnitude_milli),
            );
            provenance.insert(
                "sampledCenterId".into(),
                serde_json::json!(event.sampled_center_id.map(|id| id.to_string())),
            );
            provenance.insert("trackCells".into(), serde_json::json!(event.track_cells));
            provenance.insert(
                "physicalIdentity".into(),
                serde_json::json!(validated.identity),
            );
            provenance.insert("requestId".into(), serde_json::json!(request_id));
            provenance.insert(
                "materializationKey".into(),
                serde_json::json!(materialization_key.clone()),
            );
            provenance.insert("sourceHash".into(), serde_json::json!(source_hash));
            provenance.insert(
                "generatorId".into(),
                serde_json::json!(daena_physical::GENERATOR_ID),
            );
            provenance.insert(
                "generatorVersion".into(),
                serde_json::json!(u64::from(daena_physical::engine::WORLD_ENGINE_VERSION)),
            );
            provenance.insert("prediction".into(), serde_json::json!(false));
            if event.event_kind == daena_physical::events::NaturalEventKind::Flood {
                provenance.insert(
                    "hydrologyDerivationVersion".into(),
                    serde_json::json!(daena_physical::products::HYDROLOGY_DERIVATION_VERSION),
                );
                provenance.insert(
                    "hydrologyEpochYears".into(),
                    serde_json::json!(climate_epoch),
                );
                provenance.insert("fillPpm".into(), serde_json::json!(event.hazard_ppm));
            } else {
                provenance.insert(
                    "hazardDerivationVersion".into(),
                    serde_json::json!(daena_physical::products::HAZARD_DERIVATION_VERSION),
                );
                provenance.insert("hazardSeed".into(), serde_json::json!(request.hazard_seed));
                provenance.insert(
                    "climateEpochYears".into(),
                    serde_json::json!(climate_epoch),
                );
                provenance.insert("hazardPpm".into(), serde_json::json!(event.hazard_ppm));
                provenance.insert(
                    "annualRateNano".into(),
                    serde_json::json!(event.annual_rate_nano),
                );
                provenance.insert(
                    "ratePerMillionYearsPpm".into(),
                    serde_json::json!(event.rate_per_million_years_ppm),
                );
                provenance.insert(
                    "volcanicSourceDerivationVersion".into(),
                    serde_json::json!(event.volcanic_source_derivation_version),
                );
            }
            let provenance = serde_json::Value::Object(provenance);
            let document = format!(
                "# {}\n\n- Relative time offset: {} years\n- Strength: {}\n- Location: {:.3}°, {:.3}°\n- Model: {}\n- Prediction: no; this is generated relative history.\n",
                name,
                event.year_offset,
                strength,
                f64::from(event.latitude_microdegrees) / 1_000_000.0,
                f64::from(event.longitude_microdegrees) / 1_000_000.0,
                event.event_kind.model_label(),
            );
            Ok(CreateEntry {
                name: name.clone(),
                entity_type: Some(daena_core::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
                document: Some(daena_core::CreateEntryDocument {
                    body: document,
                    format: Some("markdown".into()),
                }),
                fields: vec![
                    CreateEntryField {
                        namespace: daena_core::maps::PHYSICAL_EVENT_NAMESPACE.into(),
                        key: "provenance".into(),
                        value: provenance,
                    },
                    CreateEntryField {
                        namespace: daena_core::maps::PHYSICAL_EVENT_NAMESPACE.into(),
                        key: "materializationKey".into(),
                        value: serde_json::json!(materialization_key),
                    },
                    CreateEntryField {
                        namespace: daena_core::maps::MAP_NAMESPACE.into(),
                        key: daena_core::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY.into(),
                        value: serde_json::json!({
                            "contractVersion": 1,
                            "kind": "physical-offset-years",
                            "reference": "accepted-source",
                            "startOffsetYears": event.year_offset,
                            "endOffsetYears": event.year_offset,
                        }),
                    },
                    CreateEntryField {
                        namespace: daena_core::maps::MAP_NAMESPACE.into(),
                        key: "locations".into(),
                        value: serde_json::json!({
                            "schemaVersion": 1,
                            "locations": [{
                                "id": location_id,
                                "mapEntityId": map_entity_id,
                                "role": "physical-event",
                                "label": name,
                                 "anchor": anchor,
                                "validity": {"from": null, "to": null}
                            }]
                        }),
                    },
                ],
                relationships: vec![CreateEntryRelationship {
                    relationship_type: daena_core::maps::PHYSICAL_EVENT_ON_MAP_RELATIONSHIP.into(),
                    target_ids: vec![map_entity_id.to_string()],
                }],
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    let created = if entries.is_empty() {
        Vec::new()
    } else {
        project.create_entries_with_request(entries, Some(&request_id))?
    };
    let expected_new = existing_ids.iter().filter(|id| id.is_none()).count();
    if created.len() != expected_new {
        return Err(CoreError::Validation(
            "event materialization did not return one entity per new event".into(),
        ));
    }
    let mut created = created.into_iter();
    let mut materialized = Vec::with_capacity(events.len());
    for (index, event) in events.into_iter().enumerate() {
        let entity_id = if let Some(entity_id) = existing_ids[index].clone() {
            entity_id
        } else {
            created
                .next()
                .ok_or_else(|| {
                    CoreError::Validation("event materialization missed a new entity".into())
                })?
                .id
        };
        materialized.push(MaterializedPhysicalEvent { entity_id, event });
    }
    let (hazard_derivation_version, hydrology_derivation_version) =
        if request.event_kind == daena_physical::events::NaturalEventKind::Flood {
            (
                0,
                Some(daena_physical::products::HYDROLOGY_DERIVATION_VERSION),
            )
        } else {
            (daena_physical::products::HAZARD_DERIVATION_VERSION, None)
        };
    Ok(PhysicalEventMaterializationResult {
        request_id,
        map_entity_id: map_entity_id.to_owned(),
        materialization_version: request.event_kind.materialization_version(),
        hazard_derivation_version,
        hydrology_derivation_version,
        model_label: request.event_kind.model_label().to_owned(),
        prediction: false,
        events: materialized,
    })
}

#[tauri::command]
pub(super) async fn project_physical_materialize_events(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    request: daena_physical::events::EventMaterializationRequest,
    request_id: Option<String>,
) -> Result<PhysicalEventMaterializationResult, String> {
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        physical_materialize_events_response(project, &map_entity_id, &request, Some(request_id))
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_find_place(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    epoch_offset_years: i64,
    query: daena_physical::find_place::FindPlaceQuery,
) -> Result<daena_physical::find_place::FindPlaceResult, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        physical_find_place_response(
            &validated.identity,
            &validated.source,
            &generation,
            epoch_offset_years,
            &query,
        )
        .map_err(CoreError::Validation)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_suggest_routes(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    epoch_offset_years: i64,
    query: daena_physical::routing::RouteSuggestQuery,
) -> Result<daena_physical::routing::RouteSuggestResult, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        physical_routes_response(
            &validated.identity,
            &validated.source,
            &generation,
            epoch_offset_years,
            &query,
        )
        .map_err(CoreError::Validation)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_propose_region(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    epoch_offset_years: i64,
    lon_micro: i32,
    lat_micro: i32,
    detector: String,
) -> Result<crate::atlas_studio::AtlasRegionProposal, String> {
    with_read_project(state, move |project| {
        let descriptor = project
            .list_fields(map_entity_id.clone())?
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?;
        let source_id = descriptor
            .value
            .get("sourceAssetId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
        let generation =
            descriptor.value.get("generation").cloned().ok_or_else(|| {
                CoreError::Validation("maps: physical generation is missing".into())
            })?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        physical_region_response(
            &validated.identity,
            &validated.source,
            &generation,
            epoch_offset_years,
            lon_micro,
            lat_micro,
            &detector,
        )
        .map_err(CoreError::Validation)
    })
    .await
}

#[tauri::command]
pub(super) fn project_physical_clear_epoch_cache() -> Result<(), String> {
    clear_historical_epoch_cache()?;
    clear_tool_surface_cache()
}
