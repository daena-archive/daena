// Physical-world commands and derived data.
use super::*;

pub(super) const HISTORICAL_CACHE_CAPACITY: usize = 16;
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
    let evolution_preset = daena_physical::evolution::EvolutionPreset::parse(
        input.evolution_preset.as_deref().unwrap_or("mature"),
    )
    .map_err(|error| error.to_string())?;
    input
        .settings
        .planetary
        .validate()
        .map_err(|error| error.to_string())?;
    if input.settings.planetary.radius_metres != input.settings.radius_metres {
        return Err("planetary.radiusMetres must match settings.radiusMetres".into());
    }
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
        let mut progress = PhysicalProgress {
            jobs: jobs_for_worker.clone(),
            job_id: worker_job_id.clone(),
            cancel: cancel.clone(),
        };
        let radius_metres = input.settings.planetary.radius_metres;
        let settings = daena_physical::GenerationSettings {
            width: input.settings.width,
            height: input.settings.height,
            radius_metres,
            target_land_fraction_ppm: input.settings.target_land_fraction_ppm,
        };
        let outcome = daena_physical::generate_world_with_evolution(
            settings,
            input.seed,
            input.retry_index,
            daena_physical::evolution::EvolutionSettings {
                preset: evolution_preset,
            },
            input.settings.planetary,
            &mut progress,
        );
        let mut manager = match jobs_for_worker.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(job) = manager.jobs.get_mut(&worker_job_id) else {
            return;
        };
        match outcome {
            Ok(world) => {
                let historical_forcing =
                    daena_physical::history::HistoricalForcingParameters::default_for(
                        input.seed,
                        input.retry_index,
                    );
                let generation = serde_json::json!({
                    "id": daena_core::maps::PHYSICAL_GENERATOR_ID,
                    "version": daena_core::maps::PHYSICAL_GENERATOR_VERSION,
                    "seed": input.seed,
                    "retryIndex": input.retry_index,
                    "settings": {
                        "width": input.settings.width,
                        "height": input.settings.height,
                        "radiusMetres": radius_metres,
                        "targetLandFractionPpm": input.settings.target_land_fraction_ppm,
                        "referenceWaterInventoryM3": world.report.reference_water_inventory_m3,
                        "plateCount": world.tectonics.settings.plate_count,
                        "continentalPlateCount": world.tectonics.settings.continental_plate_count,
                        "tectonicActivityPpm": world.tectonics.settings.tectonic_activity_ppm,
                        "islandActivityPpm": world.tectonics.settings.island_activity_ppm,
                        "evolutionPreset": evolution_preset.as_str(),
                        "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
                        "historicalForcing": historical_forcing_products(historical_forcing),
                        "planetary": input.settings.planetary,
                    }
                });
                let physical_identity =
                    match daena_core::maps::physical::validate_source(&world.source, &generation) {
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
                    source: world.source,
                    generation,
                    physical_identity,
                    derived_geojson: world.derived_geojson,
                    climate: world.climate,
                    evolution: world.evolution,
                    hydrology: world.hydrology,
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

pub(super) fn physical_climate_products(
    climate: &daena_physical::climate::ClimateField,
) -> serde_json::Value {
    serde_json::json!({
        "derivationVersion": climate.derivation_version,
        "width": climate.grid.width,
        "height": climate.grid.height,
        "temperatureCentiC": climate.temperature_centi_c,
        "temperatureNhSummerCentiC": climate.temperature_nh_summer_centi_c,
        "temperatureNhWinterCentiC": climate.temperature_nh_winter_centi_c,
        "moistureMmPerYear": climate.moisture_mm_per_year,
        "precipitationMmPerYear": climate.precipitation_mm_per_year,
        "runoffMmPerYear": climate.runoff_mm_per_year,
        "runoffVolumeM3PerYear": climate.runoff_volume_m3_per_year,
        "maritimeFactorPpm": climate.maritime_factor_ppm,
        "metrics": {
            "precipitationVolumeM3PerYear": climate.metrics.precipitation_volume_m3_per_year,
            "runoffVolumeM3PerYear": climate.metrics.runoff_volume_m3_per_year,
            "meanTemperatureCentiC": climate.metrics.mean_temperature_centi_c,
            "minimumTemperatureCentiC": climate.metrics.minimum_temperature_centi_c,
            "maximumTemperatureCentiC": climate.metrics.maximum_temperature_centi_c,
            "meanPrecipitationMmPerYear": climate.metrics.mean_precipitation_mm_per_year,
            "meanRunoffMmPerYear": climate.metrics.mean_runoff_mm_per_year,
            "wettestCellPrecipitationMmPerYear": climate.metrics.wettest_cell_precipitation_mm_per_year,
            "driestLandCellPrecipitationMmPerYear": climate.metrics.driest_land_cell_precipitation_mm_per_year,
            "transportIterations": climate.metrics.transport_iterations,
            "meanSeasonalRangeCentiC": climate.metrics.mean_seasonal_range_centi_c,
            "minimumSeasonalTemperatureCentiC": climate.metrics.minimum_seasonal_temperature_centi_c,
            "maximumSeasonalTemperatureCentiC": climate.metrics.maximum_seasonal_temperature_centi_c,
            "permanentlyFrozenLandPpm": climate.metrics.permanently_frozen_land_ppm,
            "seasonallyFrozenLandPpm": climate.metrics.seasonally_frozen_land_ppm,
        },
    })
}

pub(super) fn physical_evolution_products(
    evolution: &daena_physical::evolution::EvolutionField,
) -> serde_json::Value {
    serde_json::json!({
        "derivationVersion": evolution.derivation_version,
        "preset": evolution.preset.as_str(),
        "width": evolution.grid.width,
        "height": evolution.grid.height,
        "beforeElevationsMm": evolution.before_elevations_mm,
        "elevationsMm": evolution.elevations_mm,
        "routingElevationMm": evolution.drainage.routing_elevation_mm,
        "fillDepthMm": evolution.drainage.fill_depth_mm,
        "slopePpm": evolution.drainage.slope_ppm,
        "accumulationM3PerYear": evolution.drainage.accumulation_m3_per_year,
        "outletCells": evolution.drainage.outlet_cells,
        "edges": evolution.drainage.edges.iter().map(|edge| serde_json::json!({
            "sourceCell": edge.source_cell,
            "destinationCell": edge.destination_cell,
            "weightPpm": edge.weight_ppm,
            "distanceMetres": edge.distance_metres,
        })).collect::<Vec<_>>(),
        "drainageMetrics": {
            "directRunoffM3PerYear": evolution.drainage.metrics.direct_runoff_m3_per_year,
            "routedRunoffM3PerYear": evolution.drainage.metrics.routed_runoff_m3_per_year,
            "routedEdgeCount": evolution.drainage.metrics.routed_edge_count,
            "drainageDensityPpm": evolution.drainage.metrics.drainage_density_ppm,
            "gridAnisotropyPpm": evolution.drainage.metrics.grid_anisotropy_ppm,
            "convergencePpm": evolution.drainage.metrics.convergence_ppm,
            "outletCount": evolution.drainage.metrics.outlet_count,
            "routingSurfaceRaiseMaxMm": evolution.drainage.metrics.routing_surface_raise_max_mm,
        },
        "evolutionMetrics": {
            "initialReliefSpanMm": evolution.metrics.initial_relief_span_mm,
            "finalReliefSpanMm": evolution.metrics.final_relief_span_mm,
            "reliefChangeMm": evolution.metrics.relief_change_mm,
            "meanAbsoluteElevationChangeMm": evolution.metrics.mean_absolute_elevation_change_mm,
            "erosionWorkM3": evolution.metrics.erosion_work_m3,
            "upliftWorkM3": evolution.metrics.uplift_work_m3,
            "maxStepReliefLossMm": evolution.metrics.max_step_relief_loss_mm,
            "drainageDensityPpm": evolution.metrics.drainage_density_ppm,
            "gridAnisotropyPpm": evolution.metrics.grid_anisotropy_ppm,
            "convergencePpm": evolution.metrics.convergence_ppm,
            "tectonicRangeOrientationPpm": evolution.metrics.tectonic_range_orientation_ppm,
        },
    })
}

pub(super) fn physical_hydrology_products(
    hydrology: &daena_physical::hydrology::HydrologyField,
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
        "iceThicknessMm": hydrology.ice_thickness_mm,
        "shelfCells": hydrology.shelf_cells,
        "islandId": hydrology.island_id,
        "basins": hydrology.basins.iter().map(|basin| serde_json::json!({
            "id": basin.id,
            "minimumCell": basin.minimum_cell,
            "minimumElevationMm": basin.minimum_elevation_mm,
            "cellCount": basin.cell_count,
            "spillCell": basin.spill_cell,
            "spillElevationMm": basin.spill_elevation_mm,
            "volumeToSpillM3": basin.volume_to_spill_m3,
            "parentBasin": basin.parent_basin,
            "children": basin.children,
            "destination": basin.destination.label(),
            "waterLevelMm": basin.water_level_mm,
            "waterVolumeM3": basin.water_volume_m3,
            "inflowM3PerYear": basin.inflow_m3_per_year,
            "directPrecipitationM3PerYear": basin.direct_precipitation_m3_per_year,
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
            "totalWaterM3": hydrology.metrics.total_water_m3,
            "oceanWaterM3": hydrology.metrics.ocean_water_m3,
            "inlandWaterM3": hydrology.metrics.inland_water_m3,
            "landIceM3": hydrology.metrics.land_ice_m3,
            "balanceErrorM3": hydrology.metrics.balance_error_m3,
            "toleranceM3": hydrology.metrics.tolerance_m3,
            "fixedPointIterations": hydrology.metrics.fixed_point_iterations,
            "converged": hydrology.metrics.converged,
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
) -> String {
    format!(
        "{physical_identity}|history-v{}|hazards-v{}|epoch:{normalized_epoch}|forcing:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        daena_physical::history::HISTORICAL_DERIVATION_VERSION,
        daena_physical::hazards::HAZARD_DERIVATION_VERSION,
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

pub(super) fn historical_derived_hashes(
    world: &daena_physical::tectonics::TectonicWorld,
    source_hash: &str,
    geojson: &str,
    climate: &serde_json::Value,
    hydrology: &serde_json::Value,
) -> serde_json::Value {
    let field = world.physical_field();
    let elevation = serde_json::to_value(field.elevations_mm)
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

#[cfg(test)]
pub(super) fn derive_reopened_hydrology(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
) -> Result<(String, daena_physical::hydrology::HydrologyField), String> {
    let physics = compute_static_derived(world, generation, reference_water_inventory_m3)?;
    Ok((physics.geojson, physics.hydrology))
}

pub(super) fn compute_static_derived(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
) -> Result<daena_physical::derived_cache::StaticDerivedPhysics, String> {
    let field = world.physical_field();
    let mut progress = daena_physical::NoopProgress;
    let mut climate_settings = daena_physical::climate::ClimateSettings::default_for(field.grid);
    climate_settings.planetary = planetary_from_generation(generation);
    let climate = daena_physical::climate::derive_current_climate(
        &field,
        climate_settings,
        world.seed,
        world.retry_index,
        &mut progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let preset = generation
        .get("settings")
        .and_then(|settings| settings.get("evolutionPreset"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "evolutionPreset is required for physical sources".to_string())?;
    let preset = daena_physical::evolution::EvolutionPreset::parse(preset)
        .map_err(|error| error.to_string())?;
    let mut initial_progress = daena_physical::NoopProgress;
    let initial_world = daena_physical::tectonics::generate_tectonic_world(
        world.grid,
        world.settings,
        world.target_land_fraction_ppm,
        world.seed,
        world.retry_index,
        &mut initial_progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let evolution = daena_physical::evolution::diagnostics_from_before_after(
        initial_world.elevations_mm,
        &field,
        &climate,
        preset,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let hydrology = daena_physical::hydrology::derive_hydrology_with_crust(
        &field,
        &climate,
        &evolution.drainage,
        reference_water_inventory_m3,
        Some(&world.crust_by_cell),
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic = daena_physical::tectonics::to_diagnostic_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let hazards = daena_physical::hazards::to_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let water = daena_physical::hydrology::to_geojson(&hydrology)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic_with_hazards =
        daena_physical::merge_geojson_features_for_host(&diagnostic, &hazards)?;
    let geojson =
        daena_physical::merge_geojson_features_for_host(&diagnostic_with_hazards, &water)?;
    let ocean_curve = daena_physical::hydrology::ocean_volume_curve(&field)
        .map_err(|error| format!("maps.physical: {error}"))?;
    Ok(daena_physical::derived_cache::StaticDerivedPhysics {
        climate,
        evolution,
        hydrology,
        ocean_curve,
        geojson,
    })
}

pub(super) fn load_or_fill_static_derived(
    project_root: &str,
    identity: &str,
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
) -> Result<daena_physical::derived_cache::StaticDerivedPhysics, String> {
    let cache_dir = daena_core::maps::physical::physical_derived_cache_dir(
        Path::new(project_root),
        identity,
        planetary_from_generation(generation),
    )
    .map_err(|error| error.to_string())?;
    if let Some(hit) = daena_physical::derived_cache::load(&cache_dir)
        .map_err(|error| format!("maps.physical: {error}"))?
    {
        return Ok(hit);
    }
    let physics = compute_static_derived(world, generation, reference_water_inventory_m3)?;
    let _ = daena_physical::derived_cache::save(&cache_dir, &physics);
    Ok(physics)
}

pub(super) fn write_static_derived_from_job(
    project_root: &str,
    result: &PhysicalJobResult,
) -> Result<(), String> {
    let cache_dir = daena_core::maps::physical::physical_derived_cache_dir(
        Path::new(project_root),
        &result.physical_identity,
        planetary_from_generation(&result.generation),
    )
    .map_err(|error| error.to_string())?;
    let world = daena_physical::decode_source(&result.source)?;
    let ocean_curve = daena_physical::hydrology::ocean_volume_curve(&world.physical_field())
        .map_err(|error| format!("maps.physical: {error}"))?;
    daena_physical::derived_cache::save(
        &cache_dir,
        &daena_physical::derived_cache::StaticDerivedPhysics {
            climate: result.climate.clone(),
            evolution: result.evolution.clone(),
            hydrology: result.hydrology.clone(),
            ocean_curve,
            geojson: result.derived_geojson.clone(),
        },
    )
    .map_err(|error| format!("maps.physical: {error}"))
}

#[cfg(test)]
pub(super) fn derive_reopened_historical(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
    epoch_offset_years: i64,
    progress: &mut dyn daena_physical::ProgressSink,
) -> Result<
    (
        daena_physical::history::HistoricalWorld,
        daena_physical::history::HistoricalForcingParameters,
        String,
    ),
    String,
> {
    let parameters = historical_forcing_from_generation(generation)?;
    let field = world.physical_field();
    let historical = daena_physical::history::derive_historical_world_with_planet(
        &field,
        reference_water_inventory_m3,
        Some(&world.crust_by_cell),
        parameters,
        epoch_offset_years,
        planetary_from_generation(generation),
        progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic = daena_physical::tectonics::to_diagnostic_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let hazards = daena_physical::hazards::to_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let water = daena_physical::hydrology::to_geojson(&historical.hydrology)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic_with_hazards =
        daena_physical::merge_geojson_features_for_host(&diagnostic, &hazards)?;
    let geojson =
        daena_physical::merge_geojson_features_for_host(&diagnostic_with_hazards, &water)?;
    Ok((historical, parameters, geojson))
}

pub(super) fn derive_reopened_historical_from_static(
    world: &daena_physical::tectonics::TectonicWorld,
    generation: &serde_json::Value,
    reference_water_inventory_m3: u64,
    epoch_offset_years: i64,
    static_physics: &daena_physical::derived_cache::StaticDerivedPhysics,
    progress: &mut dyn daena_physical::ProgressSink,
) -> Result<
    (
        daena_physical::history::HistoricalWorld,
        daena_physical::history::HistoricalForcingParameters,
        String,
    ),
    String,
> {
    let parameters = historical_forcing_from_generation(generation)?;
    let field = world.physical_field();
    let historical = daena_physical::history::derive_historical_world_from_static(
        &field,
        static_physics,
        reference_water_inventory_m3,
        Some(&world.crust_by_cell),
        parameters,
        epoch_offset_years,
        progress,
    )
    .map_err(|error| format!("maps.physical: {error}"))?;
    if epoch_offset_years == 0 {
        return Ok((historical, parameters, static_physics.geojson.clone()));
    }
    let diagnostic = daena_physical::tectonics::to_diagnostic_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let hazards = daena_physical::hazards::to_geojson(world)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let water = daena_physical::hydrology::to_geojson(&historical.hydrology)
        .map_err(|error| format!("maps.physical: {error}"))?;
    let diagnostic_with_hazards =
        daena_physical::merge_geojson_features_for_host(&diagnostic, &hazards)?;
    let geojson =
        daena_physical::merge_geojson_features_for_host(&diagnostic_with_hazards, &water)?;
    Ok((historical, parameters, geojson))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn historical_response(
    world: &daena_physical::tectonics::TectonicWorld,
    source_hash: &str,
    physical_identity: &str,
    cache_key: String,
    epoch_offset_years: i64,
    normalized_epoch: i64,
    historical: &daena_physical::history::HistoricalWorld,
    parameters: daena_physical::history::HistoricalForcingParameters,
    geojson: String,
) -> serde_json::Value {
    let climate = physical_climate_products(&historical.climate);
    let hydrology = physical_hydrology_products(&historical.hydrology);
    let derived_hashes =
        historical_derived_hashes(world, source_hash, &geojson, &climate, &hydrology);
    serde_json::json!({
        "cacheKey": cache_key,
        "sourceHash": source_hash,
        "physicalIdentity": physical_identity,
        "epochOffsetYears": epoch_offset_years,
        "normalizedEpoch": normalized_epoch,
        "chronology": {
            "contractVersion": 1,
            "kind": "physical-offset-years",
            "reference": "accepted-source",
            "epochOffsetYears": epoch_offset_years,
        },
        "geojson": geojson,
        "climate": climate,
        "hydrology": hydrology,
        "hazards": {
            "derivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
            "volcanicSourceDerivationVersion": daena_physical::hazards::VOLCANIC_SOURCE_DERIVATION_VERSION,
            "model": "relative-generated-v1",
            "prediction": false,
        },
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
    Ok(physical_climate_products(climate))
}

#[tauri::command]
pub(super) fn project_physical_evolution(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .ok_or_else(|| "open a project before reading physical evolution".to_string())?
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
    let evolution = job
        .result
        .as_ref()
        .map(|result| &result.evolution)
        .ok_or_else(|| "physical job evolution is not ready".to_string())?;
    Ok(physical_evolution_products(evolution))
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
    let hydrology = job
        .result
        .as_ref()
        .map(|result| &result.hydrology)
        .ok_or_else(|| "physical job hydrology is not ready".to_string())?;
    Ok(physical_hydrology_products(hydrology))
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
    let _ = write_static_derived_from_job(&project_id, &result);
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
        let world = &validated.world;
        let report = validated.report;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            report.reference_water_inventory_m3,
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
        let world = &validated.world;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            validated.report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physical_climate_products(&physics.climate))
    })
    .await
}

#[tauri::command]
pub(super) async fn project_physical_derived_evolution(
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
        let world = &validated.world;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            validated.report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physical_evolution_products(&physics.evolution))
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
        let world = &validated.world;
        let report = validated.report;
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &validated.identity,
            world,
            &generation,
            report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        Ok(physical_hydrology_products(&physics.hydrology))
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
        let world = &validated.world;
        let report = validated.report;
        let physical_identity = validated.identity.clone();
        let forcing =
            historical_forcing_from_generation(&generation).map_err(CoreError::Validation)?;
        let normalized_epoch = daena_physical::history::normalize_epoch_offset(epoch_offset_years)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let source_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let cache_key = historical_cache_key(&physical_identity, forcing, normalized_epoch);
        {
            let cache = historical_epoch_cache()
                .lock()
                .map_err(|_| CoreError::Validation("historical cache is unavailable".into()))?;
            if let Some(value) = cache.get(&cache_key) {
                return Ok(value.clone());
            }
        }
        let physics = load_or_fill_static_derived(
            &project.info().ok_or(CoreError::ProjectNotOpen)?.root,
            &physical_identity,
            world,
            &generation,
            report.reference_water_inventory_m3,
        )
        .map_err(CoreError::Validation)?;
        let (historical, parameters, geojson) = derive_reopened_historical_from_static(
            world,
            &generation,
            report.reference_water_inventory_m3,
            normalized_epoch,
            &physics,
            &mut HistoricalProgress::with_reporter(
                request_generation,
                expected_generation,
                app,
                map_entity_id.clone(),
                request_id,
            ),
        )
        .map_err(CoreError::Validation)?;
        let value = historical_response(
            world,
            &source_hash,
            &physical_identity,
            cache_key.clone(),
            epoch_offset_years,
            normalized_epoch,
            &historical,
            parameters,
            geojson,
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

#[tauri::command]
pub(super) async fn project_physical_materialize_events(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    request: daena_physical::events::EventMaterializationRequest,
    request_id: Option<String>,
) -> Result<PhysicalEventMaterializationResult, String> {
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let response_map_id = map_entity_id.clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let provider = project.map_provider_id(&map_entity_id)?;
        if provider != daena_core::maps::PHYSICAL_PROVIDER {
            return Err(CoreError::Validation(
                "natural-event materialization requires a daena-physical map".into(),
            ));
        }
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
        let generation = descriptor
            .value
            .get("generation")
            .cloned()
            .ok_or_else(|| CoreError::Validation("maps: physical generation is missing".into()))?;
        let bytes = project.asset_bytes(source_id.to_string())?;
        let validated = daena_core::maps::physical::validate_source(&bytes, &generation)?;
        let world = &validated.world;
        let hazards = daena_physical::hazards::derive_hazards(world)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let events = daena_physical::events::sample_events(world, &hazards, &request)
            .map_err(CoreError::Validation)?;
        let source_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let generator_id = generation
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(daena_physical::GENERATOR_ID)
            .to_owned();
        let generator_version = generation
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(u64::from(daena_physical::GENERATOR_VERSION));
        let retry_index = generation
            .get("retryIndex")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(u64::from(world.retry_index));
        let entries = events
            .iter()
            .map(|event| {
                let name = format!(
                    "{} · year {} · M {:.3}",
                    event.event_kind.label(),
                    event.year_offset,
                    f64::from(event.magnitude_milli) / 1_000.0
                );
                let location_id = deterministic_event_location_id(&request_id, event.ordinal);
                let x = (f64::from(event.longitude_microdegrees) / 1_000_000.0 + 180.0) / 360.0;
                let y = (f64::from(event.latitude_microdegrees) / 1_000_000.0 + 90.0) / 180.0;
                let provenance = serde_json::json!({
                    "materializationVersion": daena_physical::events::EVENT_MATERIALIZATION_VERSION,
                    "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
                    "eventModel": event.event_kind.model_label(),
                    "eventKind": event.event_kind.label(),
                    "hazardSeed": request.hazard_seed,
                    "intervalStartYears": request.interval_start_years,
                    "intervalEndYears": request.interval_end_years,
                    "yearOffset": event.year_offset,
                    "cell": event.cell,
                    "longitudeMicrodegrees": event.longitude_microdegrees,
                    "latitudeMicrodegrees": event.latitude_microdegrees,
                    "magnitudeMilli": event.magnitude_milli,
                    "hazardPpm": event.hazard_ppm,
                    "annualRateNano": event.annual_rate_nano,
                    "ratePerMillionYearsPpm": event.rate_per_million_years_ppm,
                    "sampledCenterId": event.sampled_center_id,
                    "volcanicSourceDerivationVersion": event.volcanic_source_derivation_version,
                    "physicalIdentity": validated.identity,
                    "requestId": request_id,
                    "materializationKey": format!("{}:{}:{}", event.event_kind.label(), request.hazard_seed, event.ordinal),
                    "sourceHash": source_hash,
                    "generatorId": generator_id,
                    "generatorVersion": generator_version,
                    "sourceRetryIndex": retry_index,
                    "prediction": false,
                });
                let document = format!(
                    "# {}\n\n- Relative time offset: {} years\n- Magnitude/index: {:.3}\n- Location: {:.3}°, {:.3}°\n- Model: {}\n- Prediction: no; this is generated relative history.\n",
                    name,
                    event.year_offset,
                    f64::from(event.magnitude_milli) / 1_000.0,
                    f64::from(event.latitude_microdegrees) / 1_000_000.0,
                    f64::from(event.longitude_microdegrees) / 1_000_000.0,
                    event.event_kind.model_label(),
                );
                CreateEntry {
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
                            value: serde_json::json!(format!(
                                "{}:{}:{}",
                                event.event_kind.label(),
                                request.hazard_seed,
                                event.ordinal
                            )),
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
                                    "anchor": {"kind": "point", "point": [x, y]},
                                    "validity": {"from": null, "to": null}
                                }]
                            }),
                        },
                    ],
                    relationships: vec![CreateEntryRelationship {
                        relationship_type: daena_core::maps::PHYSICAL_EVENT_ON_MAP_RELATIONSHIP.into(),
                        target_ids: vec![map_entity_id.clone()],
                    }],
                }
            })
            .collect::<Vec<_>>();
        let entities = project.create_entries_with_request(entries, Some(&request_id))?;
        let materialized = events
            .into_iter()
            .zip(entities)
            .map(|(event, entity)| MaterializedPhysicalEvent {
                entity_id: entity.id,
                event,
            })
            .collect();
        Ok(PhysicalEventMaterializationResult {
            request_id,
            map_entity_id: response_map_id,
            materialization_version: daena_physical::events::EVENT_MATERIALIZATION_VERSION,
            hazard_derivation_version: daena_physical::hazards::HAZARD_DERIVATION_VERSION,
            prediction: false,
            events: materialized,
        })
    })
    .await
}

#[tauri::command]
pub(super) fn project_physical_clear_epoch_cache() -> Result<(), String> {
    clear_historical_epoch_cache()
}
