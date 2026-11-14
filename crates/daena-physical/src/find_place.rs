//! Deterministic Find Place search over derived physical fields.
//!
//! Search returns ranked candidate regions, not thousands of raw cells.
//! Earthquake and volcanic filters use tectonic hazard fields, which do not
//! change with climate epoch.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::climate::{biome_legend, ClimateField, BIOME_OCEAN};
use crate::engine::{ToolSurface, TOOL_SURFACE_NONE};
use crate::hazards::HazardField;
use crate::hydrology::HydrologyField;
use crate::products::BIOME_CLASS_COUNT;
use crate::{Grid, PhysicalError};

pub const FIND_PLACE_MAX_CANDIDATES: u32 = 12;
pub const FIND_PLACE_MIN_CELLS: u32 = 4;
const RISK_HAZARD_PPM: u32 = 400_000;
const RISK_STORM_PPM: u32 = 200_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FindPlaceQuery {
    #[serde(default = "default_true")]
    pub land_only: bool,
    #[serde(default)]
    pub altitude_m_min: Option<i32>,
    #[serde(default)]
    pub altitude_m_max: Option<i32>,
    #[serde(default)]
    pub latitude_milli_deg_min: Option<i32>,
    #[serde(default)]
    pub latitude_milli_deg_max: Option<i32>,
    #[serde(default)]
    pub slope_ppm_min: Option<u32>,
    #[serde(default)]
    pub slope_ppm_max: Option<u32>,
    #[serde(default)]
    pub temperature_centi_c_min: Option<i32>,
    #[serde(default)]
    pub temperature_centi_c_max: Option<i32>,
    #[serde(default)]
    pub precipitation_mm_min: Option<u32>,
    #[serde(default)]
    pub precipitation_mm_max: Option<u32>,
    #[serde(default)]
    pub aridity_ppm_min: Option<u32>,
    #[serde(default)]
    pub aridity_ppm_max: Option<u32>,
    #[serde(default)]
    pub biome_class: Option<u32>,
    #[serde(default)]
    pub island_id: Option<u32>,
    #[serde(default)]
    pub ice: Option<bool>,
    #[serde(default)]
    pub coast_distance_km_min: Option<u32>,
    #[serde(default)]
    pub coast_distance_km_max: Option<u32>,
    #[serde(default)]
    pub freshwater_distance_km_min: Option<u32>,
    #[serde(default)]
    pub freshwater_distance_km_max: Option<u32>,
    #[serde(default)]
    pub earthquake_hazard_ppm_min: Option<u32>,
    #[serde(default)]
    pub earthquake_hazard_ppm_max: Option<u32>,
    #[serde(default)]
    pub volcanic_hazard_ppm_min: Option<u32>,
    #[serde(default)]
    pub volcanic_hazard_ppm_max: Option<u32>,
    #[serde(default)]
    pub storm_suitability_ppm_min: Option<u32>,
    #[serde(default)]
    pub storm_suitability_ppm_max: Option<u32>,
    /// Criterion keys whose min/max are preferences, not hard filters.
    #[serde(default)]
    pub soft: Vec<String>,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: u32,
    #[serde(default = "default_min_cells")]
    pub min_cells: u32,
}

fn default_true() -> bool {
    true
}

fn default_max_candidates() -> u32 {
    FIND_PLACE_MAX_CANDIDATES
}

fn default_min_cells() -> u32 {
    FIND_PLACE_MIN_CELLS
}

impl Default for FindPlaceQuery {
    fn default() -> Self {
        Self {
            land_only: true,
            altitude_m_min: None,
            altitude_m_max: None,
            latitude_milli_deg_min: None,
            latitude_milli_deg_max: None,
            slope_ppm_min: None,
            slope_ppm_max: None,
            temperature_centi_c_min: None,
            temperature_centi_c_max: None,
            precipitation_mm_min: None,
            precipitation_mm_max: None,
            aridity_ppm_min: None,
            aridity_ppm_max: None,
            biome_class: None,
            island_id: None,
            ice: None,
            coast_distance_km_min: None,
            coast_distance_km_max: None,
            freshwater_distance_km_min: None,
            freshwater_distance_km_max: None,
            earthquake_hazard_ppm_min: None,
            earthquake_hazard_ppm_max: None,
            volcanic_hazard_ppm_min: None,
            volcanic_hazard_ppm_max: None,
            storm_suitability_ppm_min: None,
            storm_suitability_ppm_max: None,
            soft: Vec::new(),
            max_candidates: FIND_PLACE_MAX_CANDIDATES,
            min_cells: FIND_PLACE_MIN_CELLS,
        }
    }
}

impl FindPlaceQuery {
    fn has_criterion(&self) -> bool {
        self.altitude_m_min.is_some()
            || self.altitude_m_max.is_some()
            || self.latitude_milli_deg_min.is_some()
            || self.latitude_milli_deg_max.is_some()
            || self.slope_ppm_min.is_some()
            || self.slope_ppm_max.is_some()
            || self.temperature_centi_c_min.is_some()
            || self.temperature_centi_c_max.is_some()
            || self.precipitation_mm_min.is_some()
            || self.precipitation_mm_max.is_some()
            || self.aridity_ppm_min.is_some()
            || self.aridity_ppm_max.is_some()
            || self.biome_class.is_some()
            || self.island_id.is_some()
            || self.ice.is_some()
            || self.coast_distance_km_min.is_some()
            || self.coast_distance_km_max.is_some()
            || self.freshwater_distance_km_min.is_some()
            || self.freshwater_distance_km_max.is_some()
            || self.earthquake_hazard_ppm_min.is_some()
            || self.earthquake_hazard_ppm_max.is_some()
            || self.volcanic_hazard_ppm_min.is_some()
            || self.volcanic_hazard_ppm_max.is_some()
            || self.storm_suitability_ppm_min.is_some()
            || self.storm_suitability_ppm_max.is_some()
    }

    fn is_soft(&self, key: &str) -> bool {
        self.soft.iter().any(|item| item == key)
    }

    fn needs_coast(&self) -> bool {
        self.coast_distance_km_min.is_some() || self.coast_distance_km_max.is_some()
    }

    fn needs_freshwater(&self) -> bool {
        self.freshwater_distance_km_min.is_some() || self.freshwater_distance_km_max.is_some()
    }

    fn validate(&self, _hazards: Option<&HazardField>) -> Result<(), PhysicalError> {
        if !self.has_criterion() {
            return Err(PhysicalError::InvalidSettings(
                "Find Place needs at least one criterion".into(),
            ));
        }
        if !(1..=FIND_PLACE_MAX_CANDIDATES).contains(&self.max_candidates) {
            return Err(PhysicalError::InvalidSettings(format!(
                "maxCandidates must be between 1 and {FIND_PLACE_MAX_CANDIDATES}"
            )));
        }
        if self.min_cells == 0 {
            return Err(PhysicalError::InvalidSettings(
                "minCells must be at least 1".into(),
            ));
        }
        check_order_i32("altitude", self.altitude_m_min, self.altitude_m_max)?;
        check_order_i32(
            "latitude",
            self.latitude_milli_deg_min,
            self.latitude_milli_deg_max,
        )?;
        check_order_u32("slope", self.slope_ppm_min, self.slope_ppm_max)?;
        check_order_i32(
            "temperature",
            self.temperature_centi_c_min,
            self.temperature_centi_c_max,
        )?;
        check_order_u32(
            "precipitation",
            self.precipitation_mm_min,
            self.precipitation_mm_max,
        )?;
        check_order_u32("aridity", self.aridity_ppm_min, self.aridity_ppm_max)?;
        check_order_u32(
            "coast distance",
            self.coast_distance_km_min,
            self.coast_distance_km_max,
        )?;
        check_order_u32(
            "freshwater distance",
            self.freshwater_distance_km_min,
            self.freshwater_distance_km_max,
        )?;
        check_order_u32(
            "earthquake hazard",
            self.earthquake_hazard_ppm_min,
            self.earthquake_hazard_ppm_max,
        )?;
        check_order_u32(
            "volcanic hazard",
            self.volcanic_hazard_ppm_min,
            self.volcanic_hazard_ppm_max,
        )?;
        check_order_u32(
            "storm suitability",
            self.storm_suitability_ppm_min,
            self.storm_suitability_ppm_max,
        )?;
        if let Some(biome) = self.biome_class {
            if biome >= BIOME_CLASS_COUNT as u32 {
                return Err(PhysicalError::InvalidSettings(
                    "biome class is outside the versioned legend".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindPlaceMeans {
    pub altitude_m: i32,
    pub temperature_centi_c: i32,
    pub precipitation_mm: u32,
    pub aridity_ppm: u32,
    pub slope_ppm: u32,
    pub latitude_milli_deg: i32,
    pub coast_distance_km: Option<u32>,
    pub freshwater_distance_km: Option<u32>,
    pub earthquake_hazard_ppm: u32,
    pub volcanic_hazard_ppm: u32,
    pub storm_suitability_ppm: u32,
    pub biome_class: u32,
    pub island_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindPlaceCandidate {
    pub id: u32,
    pub cell_count: u32,
    pub area_km2: u32,
    pub score_ppm: u32,
    pub fit_ppm: u32,
    pub compactness_ppm: u32,
    pub size_ppm: u32,
    pub longitude_microdegrees: i32,
    pub latitude_microdegrees: i32,
    pub reasons: Vec<String>,
    pub satisfied: Vec<String>,
    pub preferences: Vec<String>,
    pub near_misses: Vec<String>,
    pub risks: Vec<String>,
    pub means: FindPlaceMeans,
    pub cells: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindPlaceResult {
    pub matched_cells: u32,
    pub candidate_count: u32,
    pub candidates: Vec<FindPlaceCandidate>,
}

#[allow(clippy::needless_range_loop)]
pub fn find_place(
    climate: &ClimateField,
    hydrology: &HydrologyField,
    hazards: Option<&HazardField>,
    query: &FindPlaceQuery,
) -> Result<FindPlaceResult, PhysicalError> {
    query.validate(hazards)?;
    let grid = climate.grid;
    let count = grid.sample_count();
    if hydrology.grid != grid
        || climate.temperature_centi_c.len() != count
        || hydrology.water_level_mm.len() != count
    {
        return Err(PhysicalError::InvalidSettings(
            "Find Place fields do not share a grid".into(),
        ));
    }
    if let Some(field) = hazards {
        if field.earthquake_hazard_ppm.len() != count {
            return Err(PhysicalError::InvalidSettings(
                "hazard field does not match the climate grid".into(),
            ));
        }
    }

    let river = river_mask(hydrology);
    let ocean = ocean_mask(hydrology);
    let freshwater: Vec<bool> = (0..count)
        .map(|cell| hydrology.lake_cells[cell] || river[cell])
        .collect();
    let coast_m = query.needs_coast().then(|| distance_metres(grid, &ocean));
    let fresh_m = query
        .needs_freshwater()
        .then(|| distance_metres(grid, &freshwater));

    let mut matched = Vec::new();
    for cell in 0..count {
        if query.land_only && (ocean[cell] || climate.biome_class[cell] == BIOME_OCEAN) {
            continue;
        }
        if cell_matches(
            grid,
            climate,
            hydrology,
            hazards,
            query,
            cell,
            coast_m.as_deref(),
            fresh_m.as_deref(),
        ) {
            matched.push(cell);
        }
    }

    let clusters = cluster_cells(grid, &matched, query.min_cells.max(1));
    let mut candidates = clusters
        .into_iter()
        .map(|cells| {
            summarize_candidate(
                grid,
                climate,
                hydrology,
                hazards,
                query,
                cells,
                coast_m.as_deref(),
                fresh_m.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score_ppm
            .cmp(&left.score_ppm)
            .then(right.cell_count.cmp(&left.cell_count))
            .then(
                left.longitude_microdegrees
                    .cmp(&right.longitude_microdegrees),
            )
            .then(left.latitude_microdegrees.cmp(&right.latitude_microdegrees))
    });
    candidates.truncate(query.max_candidates as usize);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.id = index as u32 + 1;
    }

    Ok(FindPlaceResult {
        matched_cells: matched.len() as u32,
        candidate_count: candidates.len() as u32,
        candidates,
    })
}

#[allow(clippy::too_many_arguments)]
fn cell_matches(
    grid: Grid,
    climate: &ClimateField,
    hydrology: &HydrologyField,
    hazards: Option<&HazardField>,
    query: &FindPlaceQuery,
    cell: usize,
    coast_m: Option<&[u32]>,
    fresh_m: Option<&[u32]>,
) -> bool {
    let altitude = altitude_m(hydrology, cell);
    let latitude = latitude_milli_deg(grid, cell);
    hard_i32(
        query,
        "altitude",
        altitude,
        query.altitude_m_min,
        query.altitude_m_max,
    ) && hard_i32(
        query,
        "latitude",
        latitude,
        query.latitude_milli_deg_min,
        query.latitude_milli_deg_max,
    ) && hard_u32(
        query,
        "slope",
        hydrology.slope_ppm[cell],
        query.slope_ppm_min,
        query.slope_ppm_max,
    ) && hard_i32(
        query,
        "temperature",
        climate.temperature_centi_c[cell],
        query.temperature_centi_c_min,
        query.temperature_centi_c_max,
    ) && hard_u32(
        query,
        "precipitation",
        climate.precipitation_mm_per_year[cell],
        query.precipitation_mm_min,
        query.precipitation_mm_max,
    ) && hard_u32(
        query,
        "aridity",
        climate.aridity_ppm[cell],
        query.aridity_ppm_min,
        query.aridity_ppm_max,
    ) && (query.is_soft("biome")
        || query
            .biome_class
            .is_none_or(|value| climate.biome_class[cell] == value))
        && (query.is_soft("island")
            || query
                .island_id
                .is_none_or(|value| hydrology.island_id[cell] == value))
        && (query.is_soft("ice")
            || query
                .ice
                .is_none_or(|value| hydrology.ice_cells[cell] == value))
        && coast_m.is_none_or(|distances| {
            hard_u32(
                query,
                "coast",
                metres_to_km(distances[cell]),
                query.coast_distance_km_min,
                query.coast_distance_km_max,
            )
        })
        && fresh_m.is_none_or(|distances| {
            hard_u32(
                query,
                "freshwater",
                metres_to_km(distances[cell]),
                query.freshwater_distance_km_min,
                query.freshwater_distance_km_max,
            )
        })
        && hazards
            .map(|field| {
                hard_u32(
                    query,
                    "earthquake",
                    field.earthquake_hazard_ppm[cell],
                    query.earthquake_hazard_ppm_min,
                    query.earthquake_hazard_ppm_max,
                ) && hard_u32(
                    query,
                    "volcanic",
                    field.volcanic_hazard_ppm[cell],
                    query.volcanic_hazard_ppm_min,
                    query.volcanic_hazard_ppm_max,
                )
            })
            .unwrap_or(true)
        && hard_u32(
            query,
            "storm",
            climate.storm_suitability_ppm[cell],
            query.storm_suitability_ppm_min,
            query.storm_suitability_ppm_max,
        )
}

#[allow(clippy::too_many_arguments)]
fn summarize_candidate(
    grid: Grid,
    climate: &ClimateField,
    hydrology: &HydrologyField,
    hazards: Option<&HazardField>,
    query: &FindPlaceQuery,
    cells: Vec<usize>,
    coast_m: Option<&[u32]>,
    fresh_m: Option<&[u32]>,
) -> FindPlaceCandidate {
    let n = cells.len().max(1) as i64;
    let mut altitude = 0i64;
    let mut temperature = 0i64;
    let mut precipitation = 0u64;
    let mut aridity = 0u64;
    let mut slope = 0u64;
    let mut coast = 0u64;
    let mut fresh = 0u64;
    let mut earthquake = 0u64;
    let mut volcanic = 0u64;
    let mut storm = 0u64;
    let mut sin_lon = 0.0;
    let mut cos_lon = 0.0;
    let mut sin_lat = 0.0;
    let mut area = 0.0;
    let mut biome_counts = [0u32; 10];
    let mut island_counts = std::collections::BTreeMap::<u32, u32>::new();
    let mut latitude = 0i64;
    let mut ice_cells = 0u32;
    for &cell in &cells {
        let (row, col) = grid.row_col(cell);
        let (lon, lat) = grid.center_radians(row, col);
        let cell_area = grid.cell_area(row);
        altitude += i64::from(altitude_m(hydrology, cell));
        temperature += i64::from(climate.temperature_centi_c[cell]);
        precipitation += u64::from(climate.precipitation_mm_per_year[cell]);
        aridity += u64::from(climate.aridity_ppm[cell]);
        slope += u64::from(hydrology.slope_ppm[cell]);
        latitude += i64::from(latitude_milli_deg(grid, cell));
        if let Some(distances) = coast_m {
            coast += u64::from(metres_to_km(distances[cell]));
        }
        if let Some(distances) = fresh_m {
            fresh += u64::from(metres_to_km(distances[cell]));
        }
        if let Some(field) = hazards {
            earthquake += u64::from(field.earthquake_hazard_ppm[cell]);
            volcanic += u64::from(field.volcanic_hazard_ppm[cell]);
        }
        storm += u64::from(climate.storm_suitability_ppm[cell]);
        sin_lon += lon.sin() * cell_area;
        cos_lon += lon.cos() * cell_area;
        sin_lat += lat.sin() * cell_area;
        area += cell_area;
        let biome = climate.biome_class[cell] as usize;
        if biome < biome_counts.len() {
            biome_counts[biome] += 1;
        }
        *island_counts.entry(hydrology.island_id[cell]).or_insert(0) += 1;
        if hydrology.ice_cells[cell] {
            ice_cells += 1;
        }
    }
    let biome_class = biome_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .map(|(id, _)| id as u32)
        .unwrap_or(0);
    let island_id = island_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(id, _)| id)
        .unwrap_or(u32::MAX);
    let means = FindPlaceMeans {
        altitude_m: (altitude / n) as i32,
        temperature_centi_c: (temperature / n) as i32,
        precipitation_mm: (precipitation / n as u64) as u32,
        aridity_ppm: (aridity / n as u64) as u32,
        slope_ppm: (slope / n as u64) as u32,
        latitude_milli_deg: (latitude / n) as i32,
        coast_distance_km: coast_m.map(|_| (coast / n as u64) as u32),
        freshwater_distance_km: fresh_m.map(|_| (fresh / n as u64) as u32),
        earthquake_hazard_ppm: (earthquake / n as u64) as u32,
        volcanic_hazard_ppm: (volcanic / n as u64) as u32,
        storm_suitability_ppm: (storm / n as u64) as u32,
        biome_class,
        island_id,
    };
    let compactness = compactness_ppm(grid, &cells);
    let size_ppm = ((cells.len() as u64).min(400) * 1_000_000 / 400) as u32;
    let fit_ppm = criterion_fit(query, &means);
    let score_ppm = fit_ppm / 2 + compactness / 4 + size_ppm / 4;
    let biome_name = biome_legend()
        .iter()
        .find(|entry| entry.id == biome_class)
        .map(|entry| entry.name)
        .unwrap_or("unclassified");
    let (satisfied, preferences) = explanations(query, &means, biome_name);
    let mut reasons = vec![
        format!(
            "rank fit {}% · compact {}% · size {}%",
            fit_ppm / 10_000,
            compactness / 10_000,
            size_ppm / 10_000
        ),
        format!("{} cells, {:.0} km²", cells.len(), area / 1_000_000.0),
    ];
    reasons.extend(satisfied.iter().cloned());
    if ice_cells > 0 {
        reasons.push(format!("{ice_cells} icy cells"));
    }
    let near_misses = near_misses(query, &means);
    let mut risks = Vec::new();
    if means.earthquake_hazard_ppm >= RISK_HAZARD_PPM {
        risks.push("elevated earthquake hazard".into());
    }
    if means.volcanic_hazard_ppm >= RISK_HAZARD_PPM {
        risks.push("elevated volcanic hazard".into());
    }
    if means.storm_suitability_ppm >= RISK_STORM_PPM {
        risks.push("storm-prone".into());
    }
    if ice_cells > 0 {
        risks.push("ice cover present".into());
    }
    let lon = sin_lon.atan2(cos_lon);
    let lat = (sin_lat / area.max(1.0)).clamp(-1.0, 1.0).asin();
    FindPlaceCandidate {
        id: 0,
        cell_count: cells.len() as u32,
        area_km2: (area / 1_000_000.0).round().clamp(0.0, f64::from(u32::MAX)) as u32,
        score_ppm,
        fit_ppm,
        compactness_ppm: compactness,
        size_ppm,
        longitude_microdegrees: (lon.to_degrees() * 1_000_000.0).round() as i32,
        latitude_microdegrees: (lat.to_degrees() * 1_000_000.0).round() as i32,
        reasons,
        satisfied,
        preferences,
        near_misses,
        risks,
        means,
        cells: cells.into_iter().map(|cell| cell as u32).collect(),
    }
}

fn hard_i32(
    query: &FindPlaceQuery,
    key: &str,
    value: i32,
    min: Option<i32>,
    max: Option<i32>,
) -> bool {
    query.is_soft(key) || in_range_i32(value, min, max)
}

fn hard_u32(
    query: &FindPlaceQuery,
    key: &str,
    value: u32,
    min: Option<u32>,
    max: Option<u32>,
) -> bool {
    query.is_soft(key) || in_range_u32(value, min, max)
}

fn criterion_fit(query: &FindPlaceQuery, means: &FindPlaceMeans) -> u32 {
    let mut parts = Vec::new();
    push_fit_i32(
        &mut parts,
        means.altitude_m,
        query.altitude_m_min,
        query.altitude_m_max,
    );
    push_fit_i32(
        &mut parts,
        means.latitude_milli_deg,
        query.latitude_milli_deg_min,
        query.latitude_milli_deg_max,
    );
    push_fit_u32(
        &mut parts,
        means.slope_ppm,
        query.slope_ppm_min,
        query.slope_ppm_max,
    );
    push_fit_i32(
        &mut parts,
        means.temperature_centi_c,
        query.temperature_centi_c_min,
        query.temperature_centi_c_max,
    );
    push_fit_u32(
        &mut parts,
        means.precipitation_mm,
        query.precipitation_mm_min,
        query.precipitation_mm_max,
    );
    push_fit_u32(
        &mut parts,
        means.aridity_ppm,
        query.aridity_ppm_min,
        query.aridity_ppm_max,
    );
    if let Some(km) = means.coast_distance_km {
        push_fit_u32(
            &mut parts,
            km,
            query.coast_distance_km_min,
            query.coast_distance_km_max,
        );
    }
    if let Some(km) = means.freshwater_distance_km {
        push_fit_u32(
            &mut parts,
            km,
            query.freshwater_distance_km_min,
            query.freshwater_distance_km_max,
        );
    }
    push_fit_u32(
        &mut parts,
        means.storm_suitability_ppm,
        query.storm_suitability_ppm_min,
        query.storm_suitability_ppm_max,
    );
    push_fit_u32(
        &mut parts,
        means.earthquake_hazard_ppm,
        query.earthquake_hazard_ppm_min,
        query.earthquake_hazard_ppm_max,
    );
    push_fit_u32(
        &mut parts,
        means.volcanic_hazard_ppm,
        query.volcanic_hazard_ppm_min,
        query.volcanic_hazard_ppm_max,
    );
    if parts.is_empty() {
        1_000_000
    } else {
        (parts.iter().sum::<u32>() / parts.len() as u32).min(1_000_000)
    }
}

fn push_fit_i32(parts: &mut Vec<u32>, value: i32, min: Option<i32>, max: Option<i32>) {
    if min.is_none() && max.is_none() {
        return;
    }
    parts.push(fit_i32(value, min, max));
}

fn push_fit_u32(parts: &mut Vec<u32>, value: u32, min: Option<u32>, max: Option<u32>) {
    push_fit_i32(
        parts,
        value as i32,
        min.map(|v| v as i32),
        max.map(|v| v as i32),
    );
}

fn fit_i32(value: i32, min: Option<i32>, max: Option<i32>) -> u32 {
    match (min, max) {
        (Some(lo), Some(hi)) if hi > lo => {
            if value < lo || value > hi {
                let dist = if value < lo { lo - value } else { value - hi };
                let span = (hi - lo).max(1);
                500_000u32.saturating_sub(
                    ((i64::from(dist) * 500_000) / i64::from(span)).clamp(0, 500_000) as u32,
                )
            } else {
                let mid = (lo + hi) / 2;
                let half = ((hi - lo) / 2).max(1);
                let delta = (value - mid).unsigned_abs();
                1_000_000
                    - ((u64::from(delta) * 500_000) / u64::from(half as u32)).min(500_000) as u32
            }
        }
        (Some(lo), None) => {
            if value < lo {
                250_000
            } else {
                1_000_000
            }
        }
        (None, Some(hi)) if value > hi => 250_000,
        (None, Some(_)) => 1_000_000,
        _ => 1_000_000,
    }
}

fn explanations(
    query: &FindPlaceQuery,
    means: &FindPlaceMeans,
    biome_name: &str,
) -> (Vec<String>, Vec<String>) {
    let mut satisfied = Vec::new();
    let mut preferences = Vec::new();
    let mut push = |key: &str, line: String| {
        if query.is_soft(key) {
            preferences.push(format!("prefer {line}"));
        } else {
            satisfied.push(line);
        }
    };
    if query.altitude_m_min.is_some() || query.altitude_m_max.is_some() {
        push(
            "altitude",
            format!(
                "altitude {} m{}",
                means.altitude_m,
                range_note_i32(query.altitude_m_min, query.altitude_m_max)
            ),
        );
    }
    if query.latitude_milli_deg_min.is_some() || query.latitude_milli_deg_max.is_some() {
        push(
            "latitude",
            format!(
                "latitude {:.1}°{}",
                f64::from(means.latitude_milli_deg) / 1_000.0,
                range_note_i32(
                    query.latitude_milli_deg_min.map(|v| v / 1_000),
                    query.latitude_milli_deg_max.map(|v| v / 1_000)
                )
            ),
        );
    }
    if query.slope_ppm_min.is_some() || query.slope_ppm_max.is_some() {
        push(
            "slope",
            format!(
                "slope {}%{}",
                means.slope_ppm / 10_000,
                range_note_u32(
                    query.slope_ppm_min.map(|v| v / 10_000),
                    query.slope_ppm_max.map(|v| v / 10_000)
                )
            ),
        );
    }
    if query.temperature_centi_c_min.is_some() || query.temperature_centi_c_max.is_some() {
        push(
            "temperature",
            format!(
                "temperature {:.1} °C{}",
                f64::from(means.temperature_centi_c) / 100.0,
                range_note_f(
                    query.temperature_centi_c_min,
                    query.temperature_centi_c_max,
                    100.0
                )
            ),
        );
    }
    if query.precipitation_mm_min.is_some() || query.precipitation_mm_max.is_some() {
        push(
            "precipitation",
            format!(
                "rainfall {} mm/year{}",
                means.precipitation_mm,
                range_note_u32(query.precipitation_mm_min, query.precipitation_mm_max)
            ),
        );
    }
    if query.aridity_ppm_min.is_some() || query.aridity_ppm_max.is_some() {
        push(
            "aridity",
            format!(
                "aridity {}%{}",
                means.aridity_ppm / 10_000,
                range_note_u32(
                    query.aridity_ppm_min.map(|v| v / 10_000),
                    query.aridity_ppm_max.map(|v| v / 10_000)
                )
            ),
        );
    }
    if query.biome_class.is_some() {
        push("biome", format!("biome {biome_name}"));
    }
    if query.island_id.is_some() {
        push("island", format!("landmass {}", means.island_id));
    }
    if let Some(km) = means.coast_distance_km {
        push(
            "coast",
            format!(
                "{km} km from coast{}",
                range_note_u32(query.coast_distance_km_min, query.coast_distance_km_max)
            ),
        );
    }
    if let Some(km) = means.freshwater_distance_km {
        push(
            "freshwater",
            format!(
                "{km} km from fresh water{}",
                range_note_u32(
                    query.freshwater_distance_km_min,
                    query.freshwater_distance_km_max
                )
            ),
        );
    }
    if query.earthquake_hazard_ppm_min.is_some() || query.earthquake_hazard_ppm_max.is_some() {
        push(
            "earthquake",
            format!(
                "earthquake hazard {}%{}",
                means.earthquake_hazard_ppm / 10_000,
                range_note_u32(
                    query.earthquake_hazard_ppm_min.map(|v| v / 10_000),
                    query.earthquake_hazard_ppm_max.map(|v| v / 10_000)
                )
            ),
        );
    }
    if query.volcanic_hazard_ppm_min.is_some() || query.volcanic_hazard_ppm_max.is_some() {
        push(
            "volcanic",
            format!(
                "volcanic hazard {}%{}",
                means.volcanic_hazard_ppm / 10_000,
                range_note_u32(
                    query.volcanic_hazard_ppm_min.map(|v| v / 10_000),
                    query.volcanic_hazard_ppm_max.map(|v| v / 10_000)
                )
            ),
        );
    }
    if query.storm_suitability_ppm_min.is_some() || query.storm_suitability_ppm_max.is_some() {
        push(
            "storm",
            format!(
                "storm exposure {}%{}",
                means.storm_suitability_ppm / 10_000,
                range_note_u32(
                    query.storm_suitability_ppm_min.map(|v| v / 10_000),
                    query.storm_suitability_ppm_max.map(|v| v / 10_000)
                )
            ),
        );
    }
    (satisfied, preferences)
}

fn range_note_i32(min: Option<i32>, max: Option<i32>) -> String {
    match (min, max) {
        (Some(lo), Some(hi)) => format!(" (asked {lo}–{hi})"),
        (Some(lo), None) => format!(" (asked ≥ {lo})"),
        (None, Some(hi)) => format!(" (asked ≤ {hi})"),
        (None, None) => String::new(),
    }
}

fn range_note_u32(min: Option<u32>, max: Option<u32>) -> String {
    range_note_i32(min.map(|v| v as i32), max.map(|v| v as i32))
}

fn range_note_f(min: Option<i32>, max: Option<i32>, scale: f64) -> String {
    match (min, max) {
        (Some(lo), Some(hi)) => format!(
            " (asked {:.1}–{:.1})",
            f64::from(lo) / scale,
            f64::from(hi) / scale
        ),
        (Some(lo), None) => format!(" (asked ≥ {:.1})", f64::from(lo) / scale),
        (None, Some(hi)) => format!(" (asked ≤ {:.1})", f64::from(hi) / scale),
        (None, None) => String::new(),
    }
}

fn near_misses(query: &FindPlaceQuery, means: &FindPlaceMeans) -> Vec<String> {
    let mut misses = Vec::new();
    push_near(
        &mut misses,
        "altitude",
        means.altitude_m,
        query.altitude_m_min,
        query.altitude_m_max,
    );
    push_near(
        &mut misses,
        "latitude",
        means.latitude_milli_deg,
        query.latitude_milli_deg_min,
        query.latitude_milli_deg_max,
    );
    push_near_u(
        &mut misses,
        "slope",
        means.slope_ppm,
        query.slope_ppm_min,
        query.slope_ppm_max,
    );
    push_near(
        &mut misses,
        "temperature",
        means.temperature_centi_c,
        query.temperature_centi_c_min,
        query.temperature_centi_c_max,
    );
    push_near_u(
        &mut misses,
        "rainfall",
        means.precipitation_mm,
        query.precipitation_mm_min,
        query.precipitation_mm_max,
    );
    push_near_u(
        &mut misses,
        "aridity",
        means.aridity_ppm,
        query.aridity_ppm_min,
        query.aridity_ppm_max,
    );
    if let Some(km) = means.coast_distance_km {
        push_near_u(
            &mut misses,
            "coast distance",
            km,
            query.coast_distance_km_min,
            query.coast_distance_km_max,
        );
    }
    if let Some(km) = means.freshwater_distance_km {
        push_near_u(
            &mut misses,
            "freshwater distance",
            km,
            query.freshwater_distance_km_min,
            query.freshwater_distance_km_max,
        );
    }
    push_near_u(
        &mut misses,
        "storm exposure",
        means.storm_suitability_ppm,
        query.storm_suitability_ppm_min,
        query.storm_suitability_ppm_max,
    );
    push_near_u(
        &mut misses,
        "earthquake hazard",
        means.earthquake_hazard_ppm,
        query.earthquake_hazard_ppm_min,
        query.earthquake_hazard_ppm_max,
    );
    push_near_u(
        &mut misses,
        "volcanic hazard",
        means.volcanic_hazard_ppm,
        query.volcanic_hazard_ppm_min,
        query.volcanic_hazard_ppm_max,
    );
    misses
}

fn push_near(out: &mut Vec<String>, label: &str, value: i32, min: Option<i32>, max: Option<i32>) {
    let Some(span) = bound_span_i32(min, max) else {
        return;
    };
    if let Some(lo) = min {
        if value - lo <= span {
            out.push(format!("{label} sits near the minimum"));
        }
    }
    if let Some(hi) = max {
        if hi - value <= span {
            out.push(format!("{label} sits near the maximum"));
        }
    }
}

fn push_near_u(out: &mut Vec<String>, label: &str, value: u32, min: Option<u32>, max: Option<u32>) {
    push_near(
        out,
        label,
        value as i32,
        min.map(|value| value as i32),
        max.map(|value| value as i32),
    );
}

fn bound_span_i32(min: Option<i32>, max: Option<i32>) -> Option<i32> {
    match (min, max) {
        (Some(lo), Some(hi)) => Some(((hi - lo).unsigned_abs() / 10).max(1) as i32),
        (Some(lo), None) => Some((lo.unsigned_abs() / 10).max(1) as i32),
        (None, Some(hi)) => Some((hi.unsigned_abs() / 10).max(1) as i32),
        (None, None) => None,
    }
}

fn compactness_ppm(grid: Grid, cells: &[usize]) -> u32 {
    let mut min_row = u32::MAX;
    let mut max_row = 0;
    let mut cols = Vec::with_capacity(cells.len());
    for &cell in cells {
        let (row, col) = grid.row_col(cell);
        min_row = min_row.min(row);
        max_row = max_row.max(row);
        cols.push(col);
    }
    cols.sort_unstable();
    let col_span = wrapped_span(&cols, grid.width).max(1);
    let row_span = (max_row - min_row + 1).max(1);
    let bbox = u64::from(row_span) * u64::from(col_span);
    ((cells.len() as u64) * 1_000_000 / bbox.max(1)).min(1_000_000) as u32
}

fn wrapped_span(sorted_cols: &[u32], width: u32) -> u32 {
    if sorted_cols.is_empty() {
        return 1;
    }
    let linear = sorted_cols[sorted_cols.len() - 1] - sorted_cols[0] + 1;
    let mut gap = 0u32;
    for window in sorted_cols.windows(2) {
        gap = gap.max(window[1] - window[0]);
    }
    let wrap_gap = sorted_cols[0] + width - sorted_cols[sorted_cols.len() - 1];
    gap = gap.max(wrap_gap);
    linear.min(width - gap + 1)
}

fn cluster_cells(grid: Grid, matched: &[usize], min_cells: u32) -> Vec<Vec<usize>> {
    let count = grid.sample_count();
    let mut wanted = vec![false; count];
    for &cell in matched {
        wanted[cell] = true;
    }
    let mut seen = vec![false; count];
    let mut clusters = Vec::new();
    for &start in matched {
        if seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut cluster = Vec::new();
        seen[start] = true;
        while let Some(cell) = stack.pop() {
            cluster.push(cell);
            for neighbor in adjacent_neighbors(grid, cell) {
                if wanted[neighbor] && !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        if cluster.len() as u32 >= min_cells {
            cluster.sort_unstable();
            clusters.push(cluster);
        }
    }
    clusters
}

fn adjacent_neighbors(grid: Grid, index: usize) -> Vec<usize> {
    let (row, col) = grid.row_col(index);
    let west = (col + grid.width - 1) % grid.width;
    let east = (col + 1) % grid.width;
    let mut neighbors = vec![grid.index(row, west), grid.index(row, east)];
    if row > 0 {
        neighbors.push(grid.index(row - 1, col));
        neighbors.push(grid.index(row - 1, west));
        neighbors.push(grid.index(row - 1, east));
    }
    if row + 1 < grid.height {
        neighbors.push(grid.index(row + 1, col));
        neighbors.push(grid.index(row + 1, west));
        neighbors.push(grid.index(row + 1, east));
    }
    neighbors
}

fn cardinal_neighbors(grid: Grid, index: usize) -> Vec<usize> {
    let (row, col) = grid.row_col(index);
    let mut neighbors = vec![
        grid.index(row, (col + grid.width - 1) % grid.width),
        grid.index(row, (col + 1) % grid.width),
    ];
    if row > 0 {
        neighbors.push(grid.index(row - 1, col));
    }
    if row + 1 < grid.height {
        neighbors.push(grid.index(row + 1, col));
    }
    neighbors
}

fn distance_metres(grid: Grid, sources: &[bool]) -> Vec<u32> {
    let count = grid.sample_count();
    let mut distance = vec![u32::MAX; count];
    let mut queue = VecDeque::new();
    for (cell, is_source) in sources.iter().enumerate() {
        if *is_source {
            distance[cell] = 0;
            queue.push_back(cell);
        }
    }
    while let Some(cell) = queue.pop_front() {
        let here = distance[cell];
        for neighbor in cardinal_neighbors(grid, cell) {
            let step = grid
                .great_circle_distance(
                    grid.center_radians(grid.row_col(cell).0, grid.row_col(cell).1),
                    grid.center_radians(grid.row_col(neighbor).0, grid.row_col(neighbor).1),
                )
                .round()
                .clamp(0.0, f64::from(u32::MAX / 4)) as u32;
            let next = here.saturating_add(step.max(1));
            if next < distance[neighbor] {
                distance[neighbor] = next;
                queue.push_back(neighbor);
            }
        }
    }
    distance
}

fn ocean_mask(hydrology: &HydrologyField) -> Vec<bool> {
    hydrology
        .water_level_mm
        .iter()
        .map(|level| *level <= hydrology.sea_level_mm)
        .collect()
}

fn river_mask(hydrology: &HydrologyField) -> Vec<bool> {
    let mut mask = vec![false; hydrology.grid.sample_count()];
    for river in &hydrology.rivers {
        if river.source_cell < mask.len() {
            mask[river.source_cell] = true;
        }
        if river.mouth_cell < mask.len() {
            mask[river.mouth_cell] = true;
        }
    }
    for path in &hydrology.river_coordinates {
        for [lon, lat] in path {
            let cell = cell_from_microdegrees(hydrology.grid, *lon, *lat);
            mask[cell] = true;
        }
    }
    mask
}

fn cell_from_microdegrees(grid: Grid, lon: i32, lat: i32) -> usize {
    let col = ((i64::from(lon) + 180_000_000) * i64::from(grid.width) / 360_000_000)
        .clamp(0, i64::from(grid.width) - 1) as u32;
    let row = ((i64::from(lat) + 90_000_000) * i64::from(grid.height) / 180_000_000)
        .clamp(0, i64::from(grid.height) - 1) as u32;
    grid.index(row, col)
}

fn altitude_m(hydrology: &HydrologyField, cell: usize) -> i32 {
    (hydrology.water_level_mm[cell] - hydrology.sea_level_mm) / 1_000
}

fn latitude_milli_deg(grid: Grid, cell: usize) -> i32 {
    let (row, col) = grid.row_col(cell);
    let (_, lat) = grid.center_radians(row, col);
    (lat.to_degrees() * 1_000.0).round() as i32
}

fn metres_to_km(metres: u32) -> u32 {
    if metres == u32::MAX {
        u32::MAX / 1_000
    } else {
        metres / 1_000
    }
}

fn in_range_i32(value: i32, min: Option<i32>, max: Option<i32>) -> bool {
    min.is_none_or(|lo| value >= lo) && max.is_none_or(|hi| value <= hi)
}

fn in_range_u32(value: u32, min: Option<u32>, max: Option<u32>) -> bool {
    min.is_none_or(|lo| value >= lo) && max.is_none_or(|hi| value <= hi)
}

fn check_order_i32(label: &str, min: Option<i32>, max: Option<i32>) -> Result<(), PhysicalError> {
    if let (Some(lo), Some(hi)) = (min, max) {
        if lo > hi {
            return Err(PhysicalError::InvalidSettings(format!(
                "{label} minimum cannot exceed maximum"
            )));
        }
    }
    Ok(())
}

fn check_order_u32(label: &str, min: Option<u32>, max: Option<u32>) -> Result<(), PhysicalError> {
    check_order_i32(
        label,
        min.map(|value| value as i32),
        max.map(|value| value as i32),
    )
}

/// Find Place over the canonical cube cells of a tool surface.
///
/// Every criterion reads one canonical field. Coast and freshwater distances
/// are multi-source shortest paths over the eight cube neighbours with true
/// great-circle step lengths. Candidate `cells` are canonical cube ids.
pub fn find_place_canonical(
    surface: &ToolSurface,
    query: &FindPlaceQuery,
) -> Result<FindPlaceResult, PhysicalError> {
    query.validate(None)?;
    let count = surface.cell_count();
    let coast_m = query
        .needs_coast()
        .then(|| surface_distance_metres(surface, &|cell| surface.ocean(cell)));
    let freshwater: Vec<bool> = (0..count as u32)
        .map(|cell| surface.fresh_lake(cell) || surface.river(cell))
        .collect();
    let fresh_m = query
        .needs_freshwater()
        .then(|| surface_distance_metres(surface, &|cell| freshwater[cell as usize]));

    let mut matched = Vec::new();
    for cell in 0..count as u32 {
        if query.land_only && surface.ocean(cell) {
            continue;
        }
        if cell_matches_surface(surface, query, cell, coast_m.as_deref(), fresh_m.as_deref()) {
            matched.push(cell);
        }
    }

    let clusters = cluster_surface_cells(surface, &matched, query.min_cells.max(1));
    let mut candidates = clusters
        .into_iter()
        .map(|cells| {
            summarize_surface_candidate(
                surface,
                query,
                cells,
                coast_m.as_deref(),
                fresh_m.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score_ppm
            .cmp(&left.score_ppm)
            .then(right.cell_count.cmp(&left.cell_count))
            .then(
                left.longitude_microdegrees
                    .cmp(&right.longitude_microdegrees),
            )
            .then(left.latitude_microdegrees.cmp(&right.latitude_microdegrees))
    });
    candidates.truncate(query.max_candidates as usize);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.id = index as u32 + 1;
    }

    Ok(FindPlaceResult {
        matched_cells: matched.len() as u32,
        candidate_count: candidates.len() as u32,
        candidates,
    })
}

fn surface_latitude_milli_deg(surface: &ToolSurface, cell: u32) -> i32 {
    surface.center_microdegrees(cell)[1] / 1_000
}

fn surface_altitude_m(surface: &ToolSurface, cell: u32) -> i32 {
    (surface.elevation_mm[cell as usize] - surface.sea_level_mm) / 1_000
}

fn surface_aridity_ppm(surface: &ToolSurface, cell: u32) -> u32 {
    if surface.ocean(cell) {
        return 0;
    }
    let pet = u64::from(surface.pet_mm[cell as usize]);
    if pet == 0 {
        return 0;
    }
    let precipitation = u64::from(surface.precipitation_mm[cell as usize]);
    1_000_000u64
        .saturating_sub(precipitation.saturating_mul(1_000_000) / pet)
        .min(1_000_000) as u32
}

fn cell_matches_surface(
    surface: &ToolSurface,
    query: &FindPlaceQuery,
    cell: u32,
    coast_m: Option<&[u32]>,
    fresh_m: Option<&[u32]>,
) -> bool {
    hard_i32(
        query,
        "altitude",
        surface_altitude_m(surface, cell),
        query.altitude_m_min,
        query.altitude_m_max,
    ) && hard_i32(
        query,
        "latitude",
        surface_latitude_milli_deg(surface, cell),
        query.latitude_milli_deg_min,
        query.latitude_milli_deg_max,
    ) && hard_u32(
        query,
        "slope",
        surface.slope_ppm[cell as usize],
        query.slope_ppm_min,
        query.slope_ppm_max,
    ) && hard_i32(
        query,
        "temperature",
        surface.temperature_centi_c[cell as usize],
        query.temperature_centi_c_min,
        query.temperature_centi_c_max,
    ) && hard_u32(
        query,
        "precipitation",
        surface.precipitation_mm[cell as usize],
        query.precipitation_mm_min,
        query.precipitation_mm_max,
    ) && hard_u32(
        query,
        "aridity",
        surface_aridity_ppm(surface, cell),
        query.aridity_ppm_min,
        query.aridity_ppm_max,
    ) && (query.is_soft("biome")
        || query
            .biome_class
            .is_none_or(|value| u32::from(surface.biome_class[cell as usize]) == value))
        && (query.is_soft("island")
            || query
                .island_id
                .is_none_or(|value| surface.landmass_id(cell) == value))
        && (query.is_soft("ice") || query.ice.is_none_or(|value| surface.ice(cell) == value))
        && coast_m.is_none_or(|distances| {
            hard_u32(
                query,
                "coast",
                metres_to_km(distances[cell as usize]),
                query.coast_distance_km_min,
                query.coast_distance_km_max,
            )
        })
        && fresh_m.is_none_or(|distances| {
            hard_u32(
                query,
                "freshwater",
                metres_to_km(distances[cell as usize]),
                query.freshwater_distance_km_min,
                query.freshwater_distance_km_max,
            )
        })
        && hard_u32(
            query,
            "earthquake",
            crate::products::hazard_ppm(surface.earthquake_hazard[cell as usize]),
            query.earthquake_hazard_ppm_min,
            query.earthquake_hazard_ppm_max,
        )
        && hard_u32(
            query,
            "volcanic",
            crate::products::hazard_ppm(surface.volcanic_hazard[cell as usize]),
            query.volcanic_hazard_ppm_min,
            query.volcanic_hazard_ppm_max,
        )
        && hard_u32(
            query,
            "storm",
            surface.storm_genesis_ppm[cell as usize],
            query.storm_suitability_ppm_min,
            query.storm_suitability_ppm_max,
        )
}

/// Multi-source shortest paths over cube neighbours with true step lengths.
fn surface_distance_metres(
    surface: &ToolSurface,
    is_source: &(dyn Fn(u32) -> bool + Sync),
) -> Vec<u32> {
    let count = surface.cell_count();
    let mut distance = vec![u32::MAX; count];
    let mut heap = BinaryHeap::new();
    for cell in 0..count as u32 {
        if is_source(cell) {
            distance[cell as usize] = 0;
            heap.push(Reverse((0u32, cell)));
        }
    }
    while let Some(Reverse((here, cell))) = heap.pop() {
        if here != distance[cell as usize] {
            continue;
        }
        for (index, neighbor) in surface.neighbors(cell).into_iter().enumerate() {
            if neighbor == TOOL_SURFACE_NONE {
                continue;
            }
            let step = surface.neighbor_distance_metres(cell, index);
            let next = here.saturating_add(step);
            if next < distance[neighbor as usize] {
                distance[neighbor as usize] = next;
                heap.push(Reverse((next, neighbor)));
            }
        }
    }
    distance
}

fn cluster_surface_cells(surface: &ToolSurface, matched: &[u32], min_cells: u32) -> Vec<Vec<u32>> {
    let count = surface.cell_count();
    let mut wanted = vec![false; count];
    for &cell in matched {
        wanted[cell as usize] = true;
    }
    let mut seen = vec![false; count];
    let mut clusters = Vec::new();
    for &start in matched {
        if seen[start as usize] {
            continue;
        }
        let mut stack = vec![start];
        let mut cluster = Vec::new();
        seen[start as usize] = true;
        while let Some(cell) = stack.pop() {
            cluster.push(cell);
            for neighbor in surface.neighbors(cell) {
                if neighbor != TOOL_SURFACE_NONE
                    && wanted[neighbor as usize]
                    && !seen[neighbor as usize]
                {
                    seen[neighbor as usize] = true;
                    stack.push(neighbor);
                }
            }
        }
        if cluster.len() as u32 >= min_cells {
            cluster.sort_unstable();
            clusters.push(cluster);
        }
    }
    clusters
}

fn compactness_surface_ppm(surface: &ToolSurface, cells: &[u32]) -> u32 {
    let n = surface.resolution();
    let mut min_row = u32::MAX;
    let mut max_row = 0u32;
    let mut cols = Vec::with_capacity(cells.len());
    for &cell in cells {
        let [lon, lat] = surface.center_microdegrees(cell);
        let row = ((i64::from(lat) + 90_000_000) * i64::from(n) / 180_000_000)
            .clamp(0, i64::from(n) - 1) as u32;
        let col = ((i64::from(lon) + 180_000_000) * i64::from(n) / 360_000_000)
            .clamp(0, i64::from(n) - 1) as u32;
        min_row = min_row.min(row);
        max_row = max_row.max(row);
        cols.push(col);
    }
    cols.sort_unstable();
    let col_span = wrapped_span(&cols, n).max(1);
    let row_span = (max_row - min_row + 1).max(1);
    let bbox = u64::from(row_span) * u64::from(col_span);
    ((cells.len() as u64) * 1_000_000 / bbox.max(1)).min(1_000_000) as u32
}

fn summarize_surface_candidate(
    surface: &ToolSurface,
    query: &FindPlaceQuery,
    cells: Vec<u32>,
    coast_m: Option<&[u32]>,
    fresh_m: Option<&[u32]>,
) -> FindPlaceCandidate {
    let n = cells.len().max(1) as u64;
    let mut altitude = 0i64;
    let mut temperature = 0i64;
    let mut precipitation = 0u64;
    let mut aridity = 0u64;
    let mut slope = 0u64;
    let mut coast = 0u64;
    let mut fresh = 0u64;
    let mut earthquake = 0u64;
    let mut volcanic = 0u64;
    let mut storm = 0u64;
    let mut sin_lon = 0.0;
    let mut cos_lon = 0.0;
    let mut sin_lat = 0.0;
    let mut area = 0.0;
    let mut biome_counts = [0u32; BIOME_CLASS_COUNT];
    let mut island_counts = std::collections::BTreeMap::<u32, u32>::new();
    let mut latitude = 0i64;
    let mut ice_cells = 0u32;
    for &cell in &cells {
        let (lat_deg, lon_deg) = surface.cell_lat_lon_degrees(cell);
        let lat = lat_deg.to_radians();
        let lon = lon_deg.to_radians();
        let cell_area = surface.area_m2(cell);
        altitude += i64::from(surface_altitude_m(surface, cell));
        temperature += i64::from(surface.temperature_centi_c[cell as usize]);
        precipitation += u64::from(surface.precipitation_mm[cell as usize]);
        aridity += u64::from(surface_aridity_ppm(surface, cell));
        slope += u64::from(surface.slope_ppm[cell as usize]);
        latitude += i64::from(surface_latitude_milli_deg(surface, cell));
        if let Some(distances) = coast_m {
            coast += u64::from(metres_to_km(distances[cell as usize]));
        }
        if let Some(distances) = fresh_m {
            fresh += u64::from(metres_to_km(distances[cell as usize]));
        }
        earthquake += u64::from(crate::products::hazard_ppm(
            surface.earthquake_hazard[cell as usize],
        ));
        volcanic += u64::from(crate::products::hazard_ppm(
            surface.volcanic_hazard[cell as usize],
        ));
        storm += u64::from(surface.storm_genesis_ppm[cell as usize]);
        sin_lon += lon.sin() * cell_area;
        cos_lon += lon.cos() * cell_area;
        sin_lat += lat.sin() * cell_area;
        area += cell_area;
        let biome = surface.biome_class[cell as usize] as usize;
        if biome < biome_counts.len() {
            biome_counts[biome] += 1;
        }
        *island_counts.entry(surface.landmass_id(cell)).or_insert(0) += 1;
        if surface.ice(cell) {
            ice_cells += 1;
        }
    }
    let biome_class = biome_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .map(|(id, _)| id as u32)
        .unwrap_or(0);
    let island_id = island_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(id, _)| id)
        .unwrap_or(TOOL_SURFACE_NONE);
    let means = FindPlaceMeans {
        altitude_m: (altitude / n as i64) as i32,
        temperature_centi_c: (temperature / n as i64) as i32,
        precipitation_mm: (precipitation / n) as u32,
        aridity_ppm: (aridity / n) as u32,
        slope_ppm: (slope / n) as u32,
        latitude_milli_deg: (latitude / n as i64) as i32,
        coast_distance_km: coast_m.map(|_| (coast / n) as u32),
        freshwater_distance_km: fresh_m.map(|_| (fresh / n) as u32),
        earthquake_hazard_ppm: (earthquake / n) as u32,
        volcanic_hazard_ppm: (volcanic / n) as u32,
        storm_suitability_ppm: (storm / n) as u32,
        biome_class,
        island_id,
    };
    let compactness = compactness_surface_ppm(surface, &cells);
    let size_ppm = ((cells.len() as u64).min(400) * 1_000_000 / 400) as u32;
    let fit_ppm = criterion_fit(query, &means);
    let score_ppm = fit_ppm / 2 + compactness / 4 + size_ppm / 4;
    let biome_name = crate::products::biome_legend()
        .into_iter()
        .find(|(id, _, _)| *id == biome_class)
        .map(|(_, name, _)| name)
        .unwrap_or("unclassified");
    let (satisfied, preferences) = explanations(query, &means, biome_name);
    let mut reasons = vec![
        format!(
            "rank fit {}% · compact {}% · size {}%",
            fit_ppm / 10_000,
            compactness / 10_000,
            size_ppm / 10_000
        ),
        format!("{} cells, {:.0} km²", cells.len(), area / 1_000_000.0),
    ];
    reasons.extend(satisfied.iter().cloned());
    if ice_cells > 0 {
        reasons.push(format!("{ice_cells} icy cells"));
    }
    let near_misses = near_misses(query, &means);
    let mut risks = Vec::new();
    if means.earthquake_hazard_ppm >= RISK_HAZARD_PPM {
        risks.push("elevated earthquake hazard".into());
    }
    if means.volcanic_hazard_ppm >= RISK_HAZARD_PPM {
        risks.push("elevated volcanic hazard".into());
    }
    if means.storm_suitability_ppm >= RISK_STORM_PPM {
        risks.push("storm-prone".into());
    }
    if ice_cells > 0 {
        risks.push("ice cover present".into());
    }
    let lon = sin_lon.atan2(cos_lon);
    let lat = (sin_lat / area.max(1.0)).clamp(-1.0, 1.0).asin();
    FindPlaceCandidate {
        id: 0,
        cell_count: cells.len() as u32,
        area_km2: (area / 1_000_000.0).round().clamp(0.0, f64::from(u32::MAX)) as u32,
        score_ppm,
        fit_ppm,
        compactness_ppm: compactness,
        size_ppm,
        longitude_microdegrees: (lon.to_degrees() * 1_000_000.0).round() as i32,
        latitude_microdegrees: (lat.to_degrees() * 1_000_000.0).round() as i32,
        reasons,
        satisfied,
        preferences,
        near_misses,
        risks,
        means,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::ClimateMetrics;
    use crate::hydrology::{Basin, WaterBalanceMetrics};
    use crate::planetary::PlanetaryConfiguration;
    use crate::{Segment, DEFAULT_RADIUS_METRES};

    fn empty_climate(grid: Grid) -> ClimateField {
        let count = grid.sample_count();
        ClimateField {
            grid,
            derivation_version: crate::climate::CLIMATE_DERIVATION_VERSION,
            planetary: PlanetaryConfiguration::earth_like(),
            temperature_centi_c: vec![0; count],
            temperature_nh_summer_centi_c: vec![0; count],
            temperature_nh_winter_centi_c: vec![0; count],
            moisture_mm_per_year: vec![0; count],
            precipitation_mm_per_year: vec![0; count],
            runoff_mm_per_year: vec![0; count],
            runoff_volume_m3_per_year: vec![0; count],
            maritime_factor_ppm: vec![0; count],
            wind_east_milli: vec![0; count],
            wind_north_milli: vec![0; count],
            wind_east_nh_summer_milli: vec![0; count],
            wind_north_nh_summer_milli: vec![0; count],
            wind_east_nh_winter_milli: vec![0; count],
            wind_north_nh_winter_milli: vec![0; count],
            wind_divergence_ppm: vec![0; count],
            wind_divergence_nh_summer_ppm: vec![0; count],
            wind_divergence_nh_winter_ppm: vec![0; count],
            wind_band: vec![0; count],
            wind_band_nh_summer: vec![0; count],
            wind_band_nh_winter: vec![0; count],
            current_east_milli: vec![0; count],
            current_north_milli: vec![0; count],
            humidity_ppm: vec![0; count],
            aridity_ppm: vec![0; count],
            precipitation_nh_summer_mm: vec![0; count],
            precipitation_nh_winter_mm: vec![0; count],
            biome_class: vec![BIOME_OCEAN; count],
            storm_suitability_ppm: vec![0; count],
            storm_track_ppm: vec![0; count],
            storm_intensity_ppm: vec![0; count],
            metrics: ClimateMetrics::default(),
        }
    }

    fn empty_hydrology(grid: Grid) -> HydrologyField {
        let count = grid.sample_count();
        HydrologyField {
            grid,
            derivation_version: crate::hydrology::HYDROLOGY_DERIVATION_VERSION,
            sea_level_mm: 0,
            water_level_mm: vec![-2_000_000; count],
            lake_level_mm: vec![0; count],
            slope_ppm: vec![0; count],
            hillshade_ppm: vec![0; count],
            bathymetry_mm: vec![2_000_000; count],
            watershed_id: vec![0; count],
            basin_by_cell: vec![0; count],
            lake_cells: vec![false; count],
            ice_cells: vec![false; count],
            ice_thickness_mm: vec![0; count],
            shelf_cells: vec![false; count],
            island_id: vec![u32::MAX; count],
            basins: Vec::<Basin>::new(),
            rivers: Vec::new(),
            river_coordinates: Vec::new(),
            coastline_segments: Vec::<Segment>::new(),
            lake_polygons: Vec::new(),
            watershed_polygons: Vec::new(),
            land_polygons: Vec::new(),
            ocean_polygons: Vec::new(),
            shelf_polygons: Vec::new(),
            island_polygons: Vec::new(),
            ice_polygons: Vec::new(),
            bathymetry_contours: Vec::new(),
            metrics: WaterBalanceMetrics {
                total_water_m3: 0,
                ocean_water_m3: 0,
                inland_water_m3: 0,
                land_ice_m3: 0,
                balance_error_m3: 0,
                tolerance_m3: 0,
                fixed_point_iterations: 0,
                converged: true,
                lake_count: 0,
                river_count: 0,
                watershed_count: 0,
                coastline_segment_count: 0,
                land_polygon_count: 0,
                ocean_polygon_count: 0,
                shelf_cell_count: 0,
                bathymetry_contour_count: 0,
                island_count: 0,
            },
        }
    }

    fn query() -> FindPlaceQuery {
        FindPlaceQuery {
            precipitation_mm_min: Some(800),
            temperature_centi_c_min: Some(1_500),
            ..FindPlaceQuery::default()
        }
    }

    fn paint_land(
        climate: &mut ClimateField,
        hydrology: &mut HydrologyField,
        cols: std::ops::Range<u32>,
        rows: std::ops::Range<u32>,
        island: u32,
        precip: u32,
        temp: i32,
    ) {
        for row in rows {
            for col in cols.clone() {
                let cell = climate.grid.index(row, col);
                hydrology.water_level_mm[cell] = 120_000;
                hydrology.island_id[cell] = island;
                climate.biome_class[cell] = crate::climate::BIOME_TEMPERATE_FOREST;
                climate.precipitation_mm_per_year[cell] = precip;
                climate.temperature_centi_c[cell] = temp;
            }
        }
    }

    #[test]
    fn empty_query_is_rejected() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let climate = empty_climate(grid);
        let hydrology = empty_hydrology(grid);
        let query = FindPlaceQuery::default();
        assert!(find_place(&climate, &hydrology, None, &query).is_err());
    }

    #[test]
    fn warm_wet_query_keeps_the_east_land_and_drops_the_dry_west() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut climate = empty_climate(grid);
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut climate, &mut hydrology, 1..5, 2..6, 1, 120, 200);
        paint_land(&mut climate, &mut hydrology, 11..15, 2..6, 2, 1_200, 2_200);
        let result = find_place(&climate, &hydrology, None, &query()).unwrap();
        assert_eq!(result.candidate_count, 1);
        assert_eq!(result.candidates[0].means.island_id, 2);
        assert!(result.candidates[0].means.precipitation_mm >= 800);
        assert!(result.candidates[0].longitude_microdegrees > 0);
        assert!(!result.candidates[0].reasons.is_empty());
        assert_eq!(result.candidates[0].means.coast_distance_km, None);
        assert_eq!(result.candidates[0].means.freshwater_distance_km, None);
    }

    #[test]
    fn disconnected_matches_rank_as_separate_candidates() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut climate = empty_climate(grid);
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut climate, &mut hydrology, 1..4, 2..5, 1, 1_200, 2_200);
        paint_land(&mut climate, &mut hydrology, 12..15, 2..6, 2, 1_400, 2_400);
        let result = find_place(&climate, &hydrology, None, &query()).unwrap();
        assert_eq!(result.candidate_count, 2);
        assert_ne!(
            result.candidates[0].means.island_id,
            result.candidates[1].means.island_id
        );
    }

    #[test]
    fn storm_minimum_keeps_exposed_cells() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut climate = empty_climate(grid);
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut climate, &mut hydrology, 1..5, 2..6, 1, 1_200, 2_200);
        paint_land(&mut climate, &mut hydrology, 11..15, 2..6, 2, 1_200, 2_200);
        for row in 2..6 {
            for col in 11..15 {
                climate.storm_suitability_ppm[grid.index(row, col)] = 700_000;
            }
        }
        let query = FindPlaceQuery {
            storm_suitability_ppm_min: Some(400_000),
            ..FindPlaceQuery::default()
        };
        let result = find_place(&climate, &hydrology, None, &query).unwrap();
        assert_eq!(result.candidate_count, 1);
        assert!(result.candidates[0].means.storm_suitability_ppm >= 400_000);
        assert!(result.candidates[0]
            .satisfied
            .iter()
            .any(|line| line.contains("storm")));
    }

    #[test]
    fn soft_altitude_does_not_exclude_out_of_range_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut climate = empty_climate(grid);
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut climate, &mut hydrology, 11..15, 2..6, 2, 1_200, 2_200);
        let query = FindPlaceQuery {
            precipitation_mm_min: Some(800),
            altitude_m_min: Some(400),
            altitude_m_max: Some(500),
            soft: vec!["altitude".into()],
            ..FindPlaceQuery::default()
        };
        let result = find_place(&climate, &hydrology, None, &query).unwrap();
        assert_eq!(result.candidate_count, 1);
        assert!(result.candidates[0].means.altitude_m < 400);
        assert!(result.candidates[0]
            .preferences
            .iter()
            .any(|line| line.contains("altitude")));
    }

    #[test]
    fn missing_hazards_ignore_earthquake_filters() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut climate = empty_climate(grid);
        let mut hydrology = empty_hydrology(grid);
        paint_land(&mut climate, &mut hydrology, 11..15, 2..6, 2, 1_200, 2_200);
        let query = FindPlaceQuery {
            precipitation_mm_min: Some(800),
            earthquake_hazard_ppm_max: Some(10),
            ..FindPlaceQuery::default()
        };
        let result = find_place(&climate, &hydrology, None, &query).unwrap();
        assert_eq!(result.candidate_count, 1);
    }
}

#[cfg(test)]
mod canonical_tests {
    use super::*;
    use crate::engine::{ToolSurface, TOOL_SURFACE_NONE};

    const BIOME_TEMPERATE_FOREST: u8 = 17;

    fn uniform_surface(n: u32) -> ToolSurface {
        let mut surface = ToolSurface::synthetic(n);
        let count = surface.cell_count();
        for cell in 0..count as u32 {
            surface.test_set_land(cell);
            surface.elevation_mm[cell as usize] = 120_000;
            surface.slope_ppm[cell as usize] = 5_000;
            surface.biome_class[cell as usize] = BIOME_TEMPERATE_FOREST;
            surface.temperature_centi_c[cell as usize] = 2_000;
            surface.precipitation_mm[cell as usize] = 1_200;
            surface.pet_mm[cell as usize] = 1_500;
            surface.earthquake_hazard[cell as usize] = 128;
            surface.volcanic_hazard[cell as usize] = 51;
            surface.storm_genesis_ppm[cell as usize] = 700_000;
            surface.landmass_id[cell as usize] = 0;
        }
        surface
    }

    #[test]
    fn canonical_means_match_hand_computed_values() {
        let surface = uniform_surface(32);
        let query = FindPlaceQuery {
            precipitation_mm_min: Some(800),
            temperature_centi_c_min: Some(1_500),
            aridity_ppm_max: Some(250_000),
            biome_class: Some(17),
            island_id: Some(0),
            ice: Some(false),
            earthquake_hazard_ppm_min: Some(500_000),
            volcanic_hazard_ppm_max: Some(250_000),
            storm_suitability_ppm_min: Some(600_000),
            slope_ppm_min: Some(4_000),
            altitude_m_min: Some(100),
            altitude_m_max: Some(200),
            ..FindPlaceQuery::default()
        };
        let result = find_place_canonical(&surface, &query).unwrap();
        assert_eq!(result.matched_cells, surface.cell_count() as u32);
        assert_eq!(result.candidate_count, 1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.means.altitude_m, 120);
        assert_eq!(candidate.means.temperature_centi_c, 2_000);
        assert_eq!(candidate.means.precipitation_mm, 1_200);
        assert_eq!(candidate.means.aridity_ppm, 200_000);
        assert_eq!(candidate.means.slope_ppm, 5_000);
        assert_eq!(candidate.means.earthquake_hazard_ppm, 501_960);
        assert_eq!(candidate.means.volcanic_hazard_ppm, 200_000);
        assert_eq!(candidate.means.storm_suitability_ppm, 700_000);
        assert_eq!(candidate.means.biome_class, 17);
        assert_eq!(candidate.means.island_id, 0);
        assert!(candidate.means.latitude_milli_deg.abs() < 2);
        assert!((candidate.area_km2 as f64 - 510_064_472.0).abs() < 2_000.0);
        assert_eq!(candidate.cell_count, surface.cell_count() as u32);
        assert!(candidate
            .cells
            .iter()
            .all(|cell| (*cell as usize) < surface.cell_count()));
        assert_eq!(candidate.cells.len(), surface.cell_count());
        assert_eq!(surface.center_microdegrees(candidate.cells[0]).len(), 2);
    }

    #[test]
    fn canonical_criteria_reject_out_of_range_values_one_to_one() {
        let surface = uniform_surface(32);
        let queries = [
            FindPlaceQuery {
                altitude_m_min: Some(121),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                slope_ppm_max: Some(4_999),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                temperature_centi_c_max: Some(1_999),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                precipitation_mm_max: Some(1_199),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                aridity_ppm_max: Some(199_999),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                biome_class: Some(18),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                island_id: Some(1),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                ice: Some(true),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                earthquake_hazard_ppm_max: Some(501_959),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                volcanic_hazard_ppm_min: Some(200_001),
                ..FindPlaceQuery::default()
            },
            FindPlaceQuery {
                storm_suitability_ppm_max: Some(699_999),
                ..FindPlaceQuery::default()
            },
        ];
        for query in queries {
            let result = find_place_canonical(&surface, &query).unwrap();
            assert_eq!(result.matched_cells, 0, "{query:?}");
            assert_eq!(result.candidate_count, 0, "{query:?}");
        }
    }

    #[test]
    fn coast_distance_counts_the_ocean_source_cell() {
        let mut surface = uniform_surface(16);
        let ocean = 10u32;
        surface.test_set_ocean(ocean);
        let query = FindPlaceQuery {
            land_only: false,
            coast_distance_km_max: Some(0),
            min_cells: 1,
            ..FindPlaceQuery::default()
        };
        let result = find_place_canonical(&surface, &query).unwrap();
        assert_eq!(result.matched_cells, 1);
        assert_eq!(result.candidates[0].cells, vec![ocean]);
        assert_eq!(result.candidates[0].means.coast_distance_km, Some(0));

        let query = FindPlaceQuery {
            coast_distance_km_max: Some(0),
            min_cells: 1,
            ..FindPlaceQuery::default()
        };
        assert_eq!(
            find_place_canonical(&surface, &query)
                .unwrap()
                .matched_cells,
            0
        );
    }

    #[test]
    fn freshwater_distance_counts_lake_cells_and_rivers() {
        let mut surface = uniform_surface(16);
        let lake = 20u32;
        let river = 30u32;
        let saline = 40u32;
        surface.test_set_lake(lake, 7, true);
        surface.test_set_lake(saline, 8, false);
        surface.test_set_river(river);
        let query = FindPlaceQuery {
            freshwater_distance_km_max: Some(0),
            min_cells: 1,
            ..FindPlaceQuery::default()
        };
        let result = find_place_canonical(&surface, &query).unwrap();
        assert_eq!(result.matched_cells, 2);
        let mut cells = result
            .candidates
            .iter()
            .flat_map(|candidate| candidate.cells.iter().copied())
            .collect::<Vec<_>>();
        cells.sort_unstable();
        assert_eq!(cells, vec![lake, river]);
    }

    #[test]
    fn coast_distance_is_the_true_neighbour_length_on_the_cube_grid() {
        let surface = uniform_surface(16);
        let source = 100u32;
        let mut sources = vec![false; surface.cell_count()];
        sources[source as usize] = true;
        let distances = surface_distance_metres(&surface, &|cell| sources[cell as usize]);
        assert_eq!(distances[source as usize], 0);
        let neighbors = surface.neighbors(source);
        for (index, neighbor) in neighbors.iter().enumerate() {
            if *neighbor == TOOL_SURFACE_NONE {
                continue;
            }
            assert_eq!(
                distances[*neighbor as usize],
                surface.neighbor_distance_metres(source, index)
            );
        }
        let far = (0..surface.cell_count() as u32)
            .find(|cell| *cell != source && !neighbors.contains(cell) && !surface.ocean(*cell))
            .unwrap();
        assert!(distances[far as usize] > 0);
    }

    #[test]
    fn no_ocean_inputs_leave_coast_distance_undefined_but_defined() {
        let mut surface = uniform_surface(16);
        for cell in 0..surface.cell_count() as u32 {
            surface.test_set_land(cell);
        }
        let query = FindPlaceQuery {
            coast_distance_km_max: Some(u32::MAX / 1_000),
            min_cells: 1,
            ..FindPlaceQuery::default()
        };
        let result = find_place_canonical(&surface, &query).unwrap();
        assert_eq!(result.matched_cells, surface.cell_count() as u32);
        assert_eq!(
            result.candidates[0].means.coast_distance_km,
            Some(u32::MAX / 1_000)
        );
    }
}
