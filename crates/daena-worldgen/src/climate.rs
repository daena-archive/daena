//! Seasonal climate on a coarse cube-sphere, then a lapse onto the terrain grid.
//!
//! The energy balance and the moisture sweeps read the finished surface. They
//! do not write elevation. Reductions that feed a stored field run in cell order.
//! Per-cell maps are the only parallel work, so one thread and many threads match.

use rayon::prelude::*;

use crate::grid::{cross, dot3, normalize, ChartStencil, CubeSphere, NONE};
use crate::{PlanetSettings, WorldError};

/// Month index of the northern-hemisphere summer solstice. Month 0 is the spring equinox.
pub const NH_SUMMER_SOLSTICE: usize = 3;
/// Month index of the northern-hemisphere winter solstice.
pub const NH_WINTER_SOLSTICE: usize = 9;

const MONTHS: usize = 12;
const BANDS: usize = 32;
const LON_BINS: usize = 64;
const SPINUP_LIMIT: u32 = 40;
const SPINUP_TOLERANCE_K: f64 = 0.02;
const B_FEEDBACK: f64 = 2.09;
/// Ocean heat transport between bands, W/m²/K in `sin φ` units like the
/// atmospheric diffusivity, scaled by the sea the two bands share.
const OCEAN_DIFFUSION: f64 = 0.12;
/// Alternating atmosphere and ocean solves per month.
const TRANSPORT_SWEEPS: usize = 3;
/// Share of the ocean's extra heat capacity hidden by full sea-ice cover.
const SEA_ICE_INSULATION: f64 = 0.7;
/// Zonal mixing inside a latitude band, W/m²/K. Not an upwind coupling.
/// Zonal flow crosses a continent in days, so land and sea in one band
/// exchange heat about as strongly as they radiate (`B`). At 1 W/m²/K polar
/// land under the summer sun reached 20 °C next to perennial sea ice.
const GAMMA_ZONAL: f64 = 3.0;
const LAPSE_K_PER_M: f64 = 0.0065;
const LAND_HEAT: f64 = 1.0e7;
const OCEAN_HEAT: f64 = 2.1e8;
/// Share of the ocean–land heat-capacity gap added to coastal land.
const MARITIME_BLEND: f64 = 0.25;
/// Cover relaxes halfway toward the temperature target each month.
const COVER_STEP: f64 = 0.5;
const THERMAL_SCALE: f64 = 3.0e5;
const THERMAL_CAP_MS: f64 = 5.0;
const MOISTURE_SWEEPS: usize = 48;
/// Discarded moisture years before the published one.
const MOISTURE_WARMUP_YEARS: usize = 1;
const STEP_SECONDS: f64 = 6.0 * 3600.0;
const FU_W: f64 = 2.6;
const PDD_SIGMA_K: f64 = 4.5;
const MELT_MM_PER_PDD: f64 = 4.0;
// Spec cold-land albedo is 0.62 and the snow target ends at -12 °C. Albedo
// 0.62 cannot melt at 45 °N in summer, so continents lock cold. 0.45 still
// brightens snow. The land target runs to -22 °C. Ocean ice still aims at
// 0.62 over [-1.8, -8] °C. Cover lags that target by a month or two.
const LAND_ALBEDO_COLD: f64 = 0.45;
const LAND_SNOW_RAMP_C: f64 = -22.0;
/// Ice-sheet albedo at the climate-cell scale (ice, firn, margins and cloud).
const LAND_ICE_ALBEDO: f64 = 0.5;
// Spec evaporation is 9/4 mm per 6 h at a unit deficit and 25 °C (9 mm/day).
// That equilibrium is about 500 mm/yr here, short of 600–1400. 4 mm per 6 h
// is 16 mm/day at full deficit and lands near the middle of the range.
const EVAP_MM_PER_6H: f64 = 4.0;

/// Area-weighted climate numbers for the JSON report and the calibration tests.
#[derive(Clone, Copy, Debug)]
pub struct ClimateStats {
    /// Global area-mean annual temperature, °C.
    pub t_global_c: f64,
    /// Area-mean annual temperature for `|lat| < 10°`.
    pub t_equator_c: f64,
    /// Area-mean annual temperature for `|lat| > 70°`.
    pub t_polar_c: f64,
    /// Global area-mean annual precipitation, mm.
    pub p_global_mm: f64,
    /// Area-mean annual precipitation for `|lat| < 10°`.
    pub p_equator_mm: f64,
    /// Area-mean annual precipitation on land with `|lat|` in `15°..=35°`.
    pub p_subtropical_land_mm: f64,
    /// Years integrated during spin-up, including the year that converged.
    pub spinup_years: u32,
    /// True when the same-month change fell below `0.02 K`.
    pub spinup_converged: bool,
    /// Largest same-month change on the last raw year, K.
    pub spinup_residual_k: f64,
    /// Hadley-cell edge, degrees.
    pub phi_h_deg: f64,
}

/// Monthly coarse climate. Layouts are month-major and cell-major.
///
/// `t_c`, `p_mm`, `q_wm2`, `s_land` and `s_ocean` use index `month * cell_count + cell`.
/// `wind` stores tangent `(x, y, z)` in planes: `(month * 3 + component) * cell_count + cell`.
/// `itcz_lat_deg` uses `month * 64 + longitude_bin`. `maritime` is one value per cell.
#[derive(Clone, Debug)]
pub struct ClimateField {
    /// Coarse grid, `min(resolution, 64)`.
    pub grid: CubeSphere,
    /// Sea-level temperature, °C, twelve months.
    pub t_c: Vec<f32>,
    /// Precipitation, mm, twelve months.
    pub p_mm: Vec<f32>,
    /// Surface wind, m/s, tangent vectors, twelve months.
    pub wind: Vec<f32>,
    /// Daily-mean insolation, W/m², twelve months.
    pub q_wm2: Vec<f32>,
    /// Hadley edge, degrees.
    pub phi_h_deg: f64,
    /// Ferrel edge, degrees.
    pub phi_f_deg: f64,
    /// Spin-up years.
    pub spinup_years: u32,
    /// Largest same-month change on the last raw spin-up year, K.
    pub spinup_residual_k: f64,
    /// True when that change fell below `0.02 K`.
    pub converged: bool,
    /// Raw previous-year then last-year temperature, only when the run did not
    /// converge. Empty after a converged spin-up. Test-only.
    #[cfg(test)]
    pub(crate) spinup_pair_t: Vec<f32>,
    /// Smoothed ITCZ latitude, degrees, twelve months by 64 longitude bins.
    pub itcz_lat_deg: Vec<f32>,
    /// Land snow cover after each month of the final spin-up year, in `[0, 1]`.
    pub s_land: Vec<f32>,
    /// Sea-ice cover after each month of the final spin-up year, in `[0, 1]`.
    pub s_ocean: Vec<f32>,
    /// Maritime index from land, sea, and the annual band wind. One value per cell.
    pub maritime: Vec<f32>,
}

/// Runoff, lake evaporation and ice balance on one erosion level.
pub(crate) struct SurfaceWater {
    /// Runoff, mm/yr. Weights use this directly.
    pub runoff_mm: Vec<f32>,
    /// Runoff, m/yr.
    pub runoff_m_yr: Vec<f32>,
    /// `max(0, PET − P)`, m/yr.
    pub evap_m_yr: Vec<f32>,
    /// Ice balance, m water equivalent per year. Negative is ablation.
    pub ice_m_yr: Vec<f32>,
}

/// Full-resolution climate summary. Temperatures are surface values after the lapse.
#[derive(Clone, Debug)]
pub struct ClimateSummary {
    /// Annual mean temperature, °C.
    pub t_annual_c: Vec<f32>,
    /// Warmest month, °C.
    pub t_warm_c: Vec<f32>,
    /// Coldest month, °C.
    pub t_cold_c: Vec<f32>,
    /// Annual precipitation, mm.
    pub p_annual_mm: Vec<f32>,
    /// Driest month, mm.
    pub p_dry_month_mm: Vec<f32>,
    /// Annual potential evapotranspiration, mm.
    pub pet_annual_mm: Vec<f32>,
    /// Annual runoff, mm.
    pub runoff_mm: Vec<f32>,
    /// Annual ice mass balance, mm water equivalent.
    pub ice_balance_mm: Vec<f32>,
    /// Ocean months with sea-level temperature below `-1.8 °C`. Land is `0`.
    pub sea_ice_months: Vec<u8>,
    /// Area-weighted report.
    pub stats: ClimateStats,
}

impl ClimateSummary {
    pub(crate) fn hash_bits(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for field in [
            self.t_annual_c.as_slice(),
            self.t_warm_c.as_slice(),
            self.t_cold_c.as_slice(),
            self.p_annual_mm.as_slice(),
            self.p_dry_month_mm.as_slice(),
            self.pet_annual_mm.as_slice(),
            self.runoff_mm.as_slice(),
            self.ice_balance_mm.as_slice(),
        ] {
            hash = fnv_f32(hash, field);
        }
        for byte in &self.sea_ice_months {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}

#[derive(Clone, Copy)]
struct Geo {
    lat: f64,
    lon: f64,
    area: f64,
    band: usize,
    pos: [f64; 3],
    east: [f64; 3],
    north: [f64; 3],
    /// Grid neighbours, `NONE` where missing.
    neighbors: [u32; 8],
}

#[derive(Clone, Copy)]
struct CoarseLand {
    land_frac: f64,
    elev_mean: f64,
    elev_barrier: f64,
}

#[derive(Clone, Copy)]
struct Rotation {
    phi_h_deg: f64,
    phi_f_deg: f64,
    diffusion: f64,
}

#[derive(Clone, Copy)]
struct Frame {
    pos: [f64; 3],
    east: [f64; 3],
    north: [f64; 3],
}

struct Spinup {
    temperature: Vec<f64>,
    /// End-of-month land snow cover for the final spin-up year.
    s_land: Vec<f64>,
    /// End-of-month sea-ice cover for the final spin-up year.
    s_ocean: Vec<f64>,
    years: u32,
    delta_k: f64,
    converged: bool,
    #[cfg(test)]
    pair_t: Vec<f32>,
}

struct MoistureCell {
    upwind: [f64; 3],
    calm: bool,
    /// `2.25 · (W_sat(T) / 50) · (1 − ice) · (1 − land_frac) · precipitation_scale`.
    ocean_factor: f64,
    /// Sea-level saturation, mm. The deficit ratio divides by this.
    w_sat: f64,
    land_pet: f64,
    land_weight: f64,
    convect: f64,
    rh_crit: f64,
    w_sat_barrier: f64,
}

/// Coarse monthly climate. `n_c = min(N, 64)`.
///
/// Every pass spins up from 15 °C with no snow or sea ice. Cold planets can be
/// bistable, so a shared warm start keeps the branch independent of how the
/// terrain was built.
pub(crate) fn coarse(
    fine: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    planet: &PlanetSettings,
) -> Result<ClimateField, WorldError> {
    if elevation.len() != fine.cell_count() || ocean.len() != fine.cell_count() {
        return Err(WorldError::Internal(
            "climate inputs do not match the terrain grid".to_string(),
        ));
    }
    let n_c = fine.n.min(64);
    let grid = CubeSphere::new(n_c, fine.radius_m).map_err(WorldError::Internal)?;
    let land = aggregate(fine, elevation, ocean, &grid)?;
    let geo = build_geo(&grid);
    let n = geo.len();
    let rotation = rotation_of(planet.rotation_hours);
    let maritime = maritime_index(&grid, &geo, &land, &rotation);
    let capacity: Vec<f64> = land
        .iter()
        .zip(&maritime)
        .map(|(cell, maritime)| heat_capacity(cell.land_frac, *maritime))
        .collect();
    let insolation = monthly_insolation(&geo, planet);
    let spin = spin_up(planet, &geo, &land, &capacity, &insolation, &rotation);
    let (wind, itcz) = diagnose_circulation(
        &grid,
        &geo,
        &spin.temperature,
        planet.axial_tilt_deg,
        &rotation,
    );
    let precip = precipitate(&PrecipIn {
        planet,
        grid: &grid,
        geo: &geo,
        land: &land,
        temperature: &spin.temperature,
        insolation: &insolation,
        wind: &wind,
        itcz: &itcz,
        phi_h_deg: rotation.phi_h_deg,
        phi_f_deg: rotation.phi_f_deg,
    });
    Ok(ClimateField {
        grid: grid.clone(),
        t_c: to_f32(&spin.temperature),
        p_mm: to_f32(&precip),
        wind: planar_wind(&wind, n),
        q_wm2: to_f32(&insolation),
        phi_h_deg: rotation.phi_h_deg,
        phi_f_deg: rotation.phi_f_deg,
        spinup_years: spin.years,
        spinup_residual_k: spin.delta_k,
        converged: spin.converged,
        #[cfg(test)]
        spinup_pair_t: spin.pair_t,
        itcz_lat_deg: itcz_to_f32(&itcz),
        s_land: to_f32(&spin.s_land),
        s_ocean: to_f32(&spin.s_ocean),
        maritime: to_f32(&maritime),
    })
}

/// Spin-up on `grid` with sea-level land of the given fraction per cell.
/// Returns the published monthly temperature, `month * cells + cell`.
#[cfg(test)]
pub(crate) fn spin_up_on_land(
    grid: &CubeSphere,
    land_frac: &[f64],
    planet: &PlanetSettings,
) -> Vec<f64> {
    let land: Vec<CoarseLand> = land_frac
        .iter()
        .map(|frac| CoarseLand {
            land_frac: *frac,
            elev_mean: 0.0,
            elev_barrier: 0.0,
        })
        .collect();
    let geo = build_geo(grid);
    let rotation = rotation_of(planet.rotation_hours);
    let maritime = maritime_index(grid, &geo, &land, &rotation);
    let capacity: Vec<f64> = land
        .iter()
        .zip(&maritime)
        .map(|(cell, maritime)| heat_capacity(cell.land_frac, *maritime))
        .collect();
    let insolation = monthly_insolation(&geo, planet);
    spin_up(planet, &geo, &land, &capacity, &insolation, &rotation).temperature
}

/// Runoff, net lake evaporation and ice balance on `fine`, using `field`.
///
/// `elevation` is already relative to this level's sea level. The lapse and the
/// orographic factor read that height, through the same chart stencil as [`downscale`].
pub(crate) fn surface_water(
    fine: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    planet: &PlanetSettings,
    field: &ClimateField,
) -> SurfaceWater {
    let summary = downscale(fine, elevation, ocean, planet, field);
    let n = summary.runoff_mm.len();
    let mut runoff_m_yr = Vec::with_capacity(n);
    let mut evap_m_yr = Vec::with_capacity(n);
    let mut ice_m_yr = Vec::with_capacity(n);
    for cell in 0..n {
        let runoff = f64::from(summary.runoff_mm[cell]).max(0.0);
        let pet = f64::from(summary.pet_annual_mm[cell]);
        let rain = f64::from(summary.p_annual_mm[cell]);
        runoff_m_yr.push((runoff / 1000.0) as f32);
        evap_m_yr.push(((pet - rain).max(0.0) / 1000.0) as f32);
        ice_m_yr.push((f64::from(summary.ice_balance_mm[cell]) / 1000.0) as f32);
    }
    SurfaceWater {
        runoff_mm: summary.runoff_mm,
        runoff_m_yr,
        evap_m_yr,
        ice_m_yr,
    }
}

/// Lapse, orographic precipitation and the annual water balance on the terrain grid.
pub(crate) fn downscale(
    fine: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    planet: &PlanetSettings,
    field: &ClimateField,
) -> ClimateSummary {
    let n = fine.cell_count();
    let h_pos: Vec<f64> = elevation
        .iter()
        .map(|value| f64::from(value.max(0.0)))
        .collect();
    let month_days = planet.month_seconds() / 86_400.0;
    // Cell centres are reused by the stencil, the orographic gradient and the
    // area report. Computing each one on demand repeated the face tangents.
    let positions: Vec<[f64; 3]> = (0..n)
        .into_par_iter()
        .map(|cell| fine.position(cell as u32))
        .collect();
    // One stencil per fine cell. Every month and field reuses it.
    let stencils: Vec<ChartStencil> = (0..n)
        .into_par_iter()
        .map(|cell| {
            let (face, fx, fy) = field.grid.face_coords(positions[cell]);
            field.grid.chart_stencil(face, fx, fy)
        })
        .collect();
    let ctx = DownCtx {
        fine,
        field,
        h_pos: &h_pos,
        positions: &positions,
        ocean,
        month_days,
        stencils: &stencils,
    };
    // Contiguous columns, in cell order. A later array-of-structs transpose
    // was slower than writing each field once.
    let starts: Vec<usize> = (0..n).step_by(4096).collect();
    let parts: Vec<Column> = starts
        .into_par_iter()
        .map(|start| {
            let end = (start + 4096).min(n);
            let mut column = Column::with_capacity(end - start);
            for cell in start..end {
                column.push(downscale_cell(&ctx, cell));
            }
            column
        })
        .collect();
    let mut summary = summary_from_columns(&parts);
    summary.stats = area_stats(fine, ocean, &summary, field, &positions);
    summary
}

#[derive(Clone, Copy)]
struct Downscaled {
    t_annual: f32,
    t_warm: f32,
    t_cold: f32,
    p_annual: f32,
    p_dry: f32,
    pet: f32,
    runoff: f32,
    ice: f32,
    sea_ice: u8,
}

struct Column {
    t_annual: Vec<f32>,
    t_warm: Vec<f32>,
    t_cold: Vec<f32>,
    p_annual: Vec<f32>,
    p_dry: Vec<f32>,
    pet: Vec<f32>,
    runoff: Vec<f32>,
    ice: Vec<f32>,
    sea_ice: Vec<u8>,
}

impl Column {
    fn with_capacity(n: usize) -> Self {
        Self {
            t_annual: Vec::with_capacity(n),
            t_warm: Vec::with_capacity(n),
            t_cold: Vec::with_capacity(n),
            p_annual: Vec::with_capacity(n),
            p_dry: Vec::with_capacity(n),
            pet: Vec::with_capacity(n),
            runoff: Vec::with_capacity(n),
            ice: Vec::with_capacity(n),
            sea_ice: Vec::with_capacity(n),
        }
    }

    fn push(&mut self, row: Downscaled) {
        self.t_annual.push(row.t_annual);
        self.t_warm.push(row.t_warm);
        self.t_cold.push(row.t_cold);
        self.p_annual.push(row.p_annual);
        self.p_dry.push(row.p_dry);
        self.pet.push(row.pet);
        self.runoff.push(row.runoff);
        self.ice.push(row.ice);
        self.sea_ice.push(row.sea_ice);
    }
}

fn summary_from_columns(parts: &[Column]) -> ClimateSummary {
    let n = parts.iter().map(|part| part.t_annual.len()).sum();
    let mut summary = ClimateSummary {
        t_annual_c: Vec::with_capacity(n),
        t_warm_c: Vec::with_capacity(n),
        t_cold_c: Vec::with_capacity(n),
        p_annual_mm: Vec::with_capacity(n),
        p_dry_month_mm: Vec::with_capacity(n),
        pet_annual_mm: Vec::with_capacity(n),
        runoff_mm: Vec::with_capacity(n),
        ice_balance_mm: Vec::with_capacity(n),
        sea_ice_months: Vec::with_capacity(n),
        stats: ClimateStats {
            t_global_c: 0.0,
            t_equator_c: 0.0,
            t_polar_c: 0.0,
            p_global_mm: 0.0,
            p_equator_mm: 0.0,
            p_subtropical_land_mm: 0.0,
            spinup_years: 0,
            spinup_converged: false,
            spinup_residual_k: 0.0,
            phi_h_deg: 0.0,
        },
    };
    for part in parts {
        summary.t_annual_c.extend_from_slice(&part.t_annual);
        summary.t_warm_c.extend_from_slice(&part.t_warm);
        summary.t_cold_c.extend_from_slice(&part.t_cold);
        summary.p_annual_mm.extend_from_slice(&part.p_annual);
        summary.p_dry_month_mm.extend_from_slice(&part.p_dry);
        summary.pet_annual_mm.extend_from_slice(&part.pet);
        summary.runoff_mm.extend_from_slice(&part.runoff);
        summary.ice_balance_mm.extend_from_slice(&part.ice);
        summary.sea_ice_months.extend_from_slice(&part.sea_ice);
    }
    summary
}

fn aggregate(
    fine: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    coarse: &CubeSphere,
) -> Result<Vec<CoarseLand>, WorldError> {
    if !fine.n.is_multiple_of(coarse.n) {
        return Err(WorldError::Internal(format!(
            "climate grid {} does not nest in {}",
            coarse.n, fine.n
        )));
    }
    let scale = fine.n / coarse.n;
    Ok((0..coarse.cell_count())
        .into_par_iter()
        .map(|cell| coarse_land(fine, elevation, ocean, coarse, scale, cell as u32))
        .collect())
}

fn coarse_land(
    fine: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    coarse: &CubeSphere,
    scale: u32,
    cell: u32,
) -> CoarseLand {
    let (face, i, j) = coarse.decode(cell);
    let mut area_sum = 0.0;
    let mut land_sum = 0.0;
    let mut elev_sum = 0.0;
    let mut elev_max = 0.0f64;
    for dj in 0..scale {
        for di in 0..scale {
            let id = fine.id(face, i * scale + di, j * scale + dj);
            let area = fine.area_m2(id);
            let elev = f64::from(elevation[id as usize].max(0.0));
            let land = if ocean[id as usize] == 0 { 1.0 } else { 0.0 };
            area_sum += area;
            land_sum += area * land;
            elev_sum += area * elev;
            elev_max = elev_max.max(elev);
        }
    }
    let (land_frac, elev_mean) = if area_sum > 0.0 {
        (land_sum / area_sum, elev_sum / area_sum)
    } else {
        (0.0, 0.0)
    };
    CoarseLand {
        land_frac,
        elev_mean,
        elev_barrier: 0.5 * elev_mean + 0.5 * elev_max,
    }
}

fn build_geo(grid: &CubeSphere) -> Vec<Geo> {
    (0..grid.cell_count())
        .into_par_iter()
        .map(|cell| {
            let pos = grid.position(cell as u32);
            let (east, north) = east_north(pos);
            Geo {
                lat: pos[2].clamp(-1.0, 1.0).asin(),
                lon: pos[1].atan2(pos[0]),
                area: grid.area_m2(cell as u32),
                band: band_index(pos[2]),
                pos,
                east,
                north,
                neighbors: grid.neighbors(cell as u32),
            }
        })
        .collect()
}

fn monthly_insolation(geo: &[Geo], planet: &PlanetSettings) -> Vec<f64> {
    let n = geo.len();
    let mut q = vec![0.0; MONTHS * n];
    let tilt = planet.axial_tilt_deg.to_radians();
    let peri = planet.perihelion_deg.to_radians();
    let solar = planet.solar_constant();
    for month in 0..MONTHS {
        let lambda = std::f64::consts::TAU * (month as f64) / MONTHS as f64;
        let delta = declination(tilt, lambda);
        let scale = solar * distance_factor(planet.eccentricity, lambda, peri);
        let slice = &mut q[month * n..(month + 1) * n];
        slice.par_iter_mut().enumerate().for_each(|(cell, dest)| {
            *dest = daily_insolation(scale, geo[cell].lat, delta);
        });
    }
    q
}

fn rotation_of(hours: f64) -> Rotation {
    let scale = (hours / 24.0).sqrt();
    let phi_h = (30.0 * scale).clamp(12.0, 70.0);
    Rotation {
        phi_h_deg: phi_h,
        phi_f_deg: phi_h + 0.5 * (90.0 - phi_h),
        // Atmospheric share of Earth's meridional transport; the ocean channel
        // (`OCEAN_DIFFUSION`) carries the rest.
        diffusion: 0.375 * scale.clamp(0.5, 3.0),
    }
}

fn spin_up(
    planet: &PlanetSettings,
    geo: &[Geo],
    land: &[CoarseLand],
    capacity: &[f64],
    insolation: &[f64],
    rotation: &Rotation,
) -> Spinup {
    let n = geo.len();
    let dt = planet.month_seconds();
    let a_coeff = 203.3 - planet.greenhouse_wm2 - B_FEEDBACK * planet.temperature_offset_c;
    let mut state = vec![15.0; n];
    let mut snow = vec![0.0; n];
    let mut ice = vec![0.0; n];
    let mut glacier = vec![0.0; n];
    // Neighbourhood temperature of the last twelve months, a rolling window.
    let mut regional = vec![15.0; MONTHS * n];
    let mut temperature = vec![15.0; MONTHS * n];
    let mut s_land = vec![0.0; MONTHS * n];
    let mut s_ocean = vec![0.0; MONTHS * n];
    let mut previous_t = temperature.clone();
    let mut years = 0;
    let mut delta_k = f64::MAX;
    let mut converged = false;
    for year in 1..=SPINUP_LIMIT {
        years = year;
        let mut max_change = 0.0f64;
        for month in 0..MONTHS {
            let next = step_temperature(&EnergyIn {
                geo,
                land,
                capacity,
                temperature: &state,
                snow: &snow,
                ice: &ice,
                glacier: &glacier,
                insolation: &insolation[month * n..(month + 1) * n],
                dt,
                a_coeff,
                diffusion: rotation.diffusion,
            });
            relax_cover(geo, land, &next, &mut snow, &mut ice);
            for cell in 0..n {
                regional[month * n + cell] = neighbourhood_mean(&geo[cell].neighbors, &next, cell);
            }
            relax_glacier(land, &regional, &mut glacier);
            let base = month * n;
            for cell in 0..n {
                let idx = base + cell;
                max_change = max_change.max((next[cell] - previous_t[idx]).abs());
                temperature[idx] = next[cell];
                s_land[idx] = snow[cell];
                s_ocean[idx] = ice[cell];
            }
            state = next;
        }
        delta_k = max_change;
        if max_change < SPINUP_TOLERANCE_K {
            converged = true;
            break;
        }
        if year == SPINUP_LIMIT {
            break;
        }
        previous_t.copy_from_slice(&temperature);
    }
    let published = publish_spin(&temperature, &previous_t, converged);
    Spinup {
        temperature: published.temperature,
        s_land,
        s_ocean,
        years,
        delta_k,
        converged,
        #[cfg(test)]
        pair_t: published.pair_t,
    }
}

pub(crate) struct PublishedSpin {
    pub(crate) temperature: Vec<f64>,
    #[cfg(test)]
    pair_t: Vec<f32>,
}

/// A converged year is stored as itself. Otherwise each month is the mean of
/// the last two raw years. Winds are diagnosed from that stored temperature.
pub(crate) fn publish_spin(
    temperature: &[f64],
    previous_t: &[f64],
    converged: bool,
) -> PublishedSpin {
    if converged {
        return PublishedSpin {
            temperature: temperature.to_vec(),
            #[cfg(test)]
            pair_t: Vec::new(),
        };
    }
    PublishedSpin {
        temperature: mean_slice(temperature, previous_t),
        #[cfg(test)]
        pair_t: concat_f32(previous_t, temperature),
    }
}

fn mean_slice(current: &[f64], previous: &[f64]) -> Vec<f64> {
    current
        .iter()
        .zip(previous)
        .map(|(now, old)| 0.5 * (now + old))
        .collect()
}

#[cfg(test)]
fn concat_f32(previous: &[f64], current: &[f64]) -> Vec<f32> {
    let mut out = to_f32(previous);
    out.extend(to_f32(current));
    out
}

/// Perennial land ice, relaxed each month toward the warmest of the last
/// twelve months. Land whose warmest month stays
/// near freezing at its height keeps its snow through summer and turns into an
/// ice sheet, which then reflects like ice. Without it polar continents had
/// warm summers and no ice sheets.
fn relax_glacier(land: &[CoarseLand], regional: &[f64], glacier: &mut [f64]) {
    let n = land.len();
    for cell in 0..n {
        if land[cell].land_frac <= 0.0 {
            glacier[cell] = 0.0;
            continue;
        }
        let mut warm = f64::MIN;
        for month in 0..MONTHS {
            warm = warm.max(regional[month * n + cell]);
        }
        let t_warm = warm - LAPSE_K_PER_M * land[cell].elev_mean;
        let target = smoothstep(0.0, -4.0, t_warm);
        glacier[cell] += COVER_STEP * (target - glacier[cell]);
    }
}

fn relax_cover(
    geo: &[Geo],
    land: &[CoarseLand],
    temperature: &[f64],
    snow: &mut [f64],
    ice: &mut [f64],
) {
    // Cover follows the 3×3 neighbourhood temperature, not the cell alone. The
    // albedo jump is worth about 15 K to one cell, so a target from its own
    // temperature made every cell bistable: neighbours at the same latitude
    // settled 16 K apart and the sea-ice edge came out as alternating rows.
    for cell in 0..geo.len() {
        let regional = neighbourhood_mean(&geo[cell].neighbors, temperature, cell);
        let t_elev = regional - LAPSE_K_PER_M * land[cell].elev_mean;
        let land_target = smoothstep(-2.0, LAND_SNOW_RAMP_C, t_elev);
        let ocean_target = smoothstep(-1.8, -8.0, regional);
        snow[cell] += COVER_STEP * (land_target - snow[cell]);
        ice[cell] += COVER_STEP * (ocean_target - ice[cell]);
    }
}

/// Mean of `cell` and its existing grid neighbours.
fn neighbourhood_mean(neighbors: &[u32; 8], field: &[f64], cell: usize) -> f64 {
    let mut sum = field[cell];
    let mut count = 1.0;
    for nb in neighbors {
        if *nb != NONE {
            sum += field[*nb as usize];
            count += 1.0;
        }
    }
    sum / count
}

struct EnergyIn<'a> {
    geo: &'a [Geo],
    land: &'a [CoarseLand],
    capacity: &'a [f64],
    temperature: &'a [f64],
    snow: &'a [f64],
    ice: &'a [f64],
    glacier: &'a [f64],
    insolation: &'a [f64],
    dt: f64,
    a_coeff: f64,
    diffusion: f64,
}

fn step_temperature(input: &EnergyIn<'_>) -> Vec<f64> {
    // Meridional transport reaches every cell as the same flux per unit area,
    // solved implicitly together with the local balance. A uniform temperature
    // increment would hand each cell energy in proportion to its heat
    // capacity: in a polar band that is mostly land, the few ocean cells took
    // the whole band's convergence and ran away past 200 °C.
    //
    // Each cell is `T = (num + F) / k` for its band's flux `F`, so the band's
    // area mean is `a + g F` with `a = <num / k>` and `g = <1 / k>`. Diffusing
    // that mean implicitly is the tridiagonal solve below with capacity
    // `dt / g`, which gives `F` for every band at once. Zonal mixing reads the
    // incoming band mean. Both are interpolated in sin φ so a cell on a band
    // edge does not take a step.
    let (t_bar, _) = band_means(input.geo, input.capacity, input.temperature);
    let local: Vec<LocalBalance> = (0..input.geo.len())
        .into_par_iter()
        .map(|cell| {
            let land_frac = input.land[cell].land_frac;
            let albedo = cover_albedo(
                land_frac,
                input.snow[cell],
                input.ice[cell],
                input.glacier[cell],
            );
            let band_mean = sample_band_delta(input.geo[cell].pos[2], &t_bar);
            local_balance(
                insulated_capacity(input.capacity[cell], land_frac, input.ice[cell]),
                input.temperature[cell],
                input.insolation[cell],
                albedo,
                band_mean,
                input.dt,
                input.a_coeff,
            )
        })
        .collect();
    let flux = band_transport(input.geo, input.land, &local, &t_bar, input.diffusion);
    (0..input.geo.len())
        .into_par_iter()
        .map(|cell| {
            let item = local[cell];
            let z = input.geo[cell].pos[2];
            let sea_share = 1.0 - input.land[cell].land_frac;
            let air = sample_band_delta(z, &flux.air);
            let sea = sea_share * sample_band_delta(z, &flux.sea);
            (item.num + air + sea) / item.k
        })
        .collect()
}

/// Implicit month for one cell, before transport: `T = (num + F) / k`.
#[derive(Clone, Copy)]
struct LocalBalance {
    num: f64,
    k: f64,
}

fn local_balance(
    capacity: f64,
    incoming: f64,
    insolation: f64,
    albedo: f64,
    band_mean: f64,
    dt: f64,
    a_coeff: f64,
) -> LocalBalance {
    let c_dt = capacity / dt;
    LocalBalance {
        num: c_dt * incoming + insolation * (1.0 - albedo) - a_coeff + GAMMA_ZONAL * band_mean,
        k: c_dt + B_FEEDBACK + GAMMA_ZONAL,
    }
}

/// Meridional heat convergence per band, W/m².
struct BandFlux {
    /// Atmosphere, per unit of band area. Every cell takes the same flux.
    air: [f64; BANDS],
    /// Ocean, per unit of ocean area. A cell takes it times its ocean share.
    sea: [f64; BANDS],
}

/// Atmosphere and ocean transport, solved implicitly with the local balances.
///
/// A cell is `T = (num + F_air + w F_sea) / k`, with `w` its ocean share. Band
/// means are then linear in the two fluxes, so each channel is one implicit
/// band exchange; the two are coupled by a few alternating solves.
fn band_transport(
    geo: &[Geo],
    land: &[CoarseLand],
    local: &[LocalBalance],
    t_bar: &[f64; BANDS],
    diffusion: f64,
) -> BandFlux {
    let mut area = [0.0; BANDS];
    let mut sea_area = [0.0; BANDS];
    let mut occupied = [false; BANDS];
    for (place, cell) in geo.iter().zip(land) {
        area[place.band] += place.area;
        sea_area[place.band] += place.area * (1.0 - cell.land_frac);
        occupied[place.band] = true;
    }
    // Area means: T_bar = a + g F_air + h F_sea. Ocean means (weights A w):
    // T_sea = a_o + g_o F_air + h_o F_sea, with F_sea per unit of band area.
    let mut a = [0.0; BANDS];
    let mut g = [0.0; BANDS];
    let mut h = [0.0; BANDS];
    let mut a_o = [0.0; BANDS];
    let mut g_o = [0.0; BANDS];
    let mut h_o = [0.0; BANDS];
    for ((place, cell), item) in geo.iter().zip(land).zip(local) {
        let b = place.band;
        let w = 1.0 - cell.land_frac;
        let share = if sea_area[b] > 0.0 {
            w * area[b] / sea_area[b]
        } else {
            0.0
        };
        a[b] += place.area * item.num / item.k;
        g[b] += place.area / item.k;
        h[b] += place.area * share / item.k;
        a_o[b] += place.area * w * item.num / item.k;
        g_o[b] += place.area * w / item.k;
        h_o[b] += place.area * w * share / item.k;
    }
    let mut sea_frac = [0.0; BANDS];
    for b in 0..BANDS {
        if area[b] > 0.0 {
            a[b] /= area[b];
            g[b] /= area[b];
            h[b] /= area[b];
            sea_frac[b] = sea_area[b] / area[b];
        }
        if sea_area[b] > 0.0 {
            a_o[b] /= sea_area[b];
            g_o[b] /= sea_area[b];
            h_o[b] /= sea_area[b];
        }
    }
    fill_empty_bands(&mut a, &mut g, &occupied);
    let mut air_conductance = meridional_conductance(BANDS, diffusion);
    let mut sea_conductance = meridional_conductance(BANDS, OCEAN_DIFFUSION);
    let dx = 2.0 / BANDS as f64;
    for i in 0..BANDS - 1 {
        let x = -1.0 + (i + 1) as f64 * dx;
        let t_face = 0.5 * (t_bar[i] + t_bar[i + 1]);
        air_conductance[i] *= moist_scale(t_face, x);
        // Ocean heat moves only as far as the sea between the bands reaches.
        sea_conductance[i] *= sea_frac[i].min(sea_frac[i + 1]);
    }
    let air_inertia: Vec<f64> = g.iter().map(|value| 1.0 / value.max(1e-12)).collect();
    let has_sea: Vec<bool> = (0..BANDS)
        .map(|b| sea_area[b] > 0.0 && h_o[b] > 0.0)
        .collect();
    let sea_inertia: Vec<f64> = (0..BANDS)
        .map(|b| if has_sea[b] { 1.0 / h_o[b] } else { 1.0 })
        .collect();
    let mut air = [0.0; BANDS];
    let mut sea_band = [0.0; BANDS];
    for _ in 0..TRANSPORT_SWEEPS {
        let base: Vec<f64> = (0..BANDS).map(|b| a[b] + h[b] * sea_band[b]).collect();
        let mixed = exchange_bands(&air_inertia, &base, &air_conductance).unwrap_or(base.clone());
        for b in 0..BANDS {
            air[b] = (mixed[b] - base[b]) * air_inertia[b];
        }
        let base: Vec<f64> = (0..BANDS)
            .map(|b| {
                if has_sea[b] {
                    a_o[b] + g_o[b] * air[b]
                } else {
                    0.0
                }
            })
            .collect();
        let mixed = exchange_bands(&sea_inertia, &base, &sea_conductance).unwrap_or(base.clone());
        for b in 0..BANDS {
            sea_band[b] = if has_sea[b] {
                (mixed[b] - base[b]) * sea_inertia[b]
            } else {
                0.0
            };
        }
    }
    let mut sea = [0.0; BANDS];
    for b in 0..BANDS {
        if sea_frac[b] > 0.0 {
            sea[b] = sea_band[b] / sea_frac[b];
        }
    }
    BandFlux { air, sea }
}

/// `D (1 − x²) / dx²` at the `n − 1` inner interfaces of `n` equal-area bands.
fn meridional_conductance(n: usize, diffusion: f64) -> Vec<f64> {
    let dx = 2.0 / n as f64;
    (1..n)
        .map(|i| {
            let x = -1.0 + i as f64 * dx;
            diffusion * (1.0 - x * x) / (dx * dx)
        })
        .collect()
}

/// Implicit exchange `inertia_b (T_b − base_b) = Σ c (T_neighbour − T_b)` with
/// `conductance[i]` between bands `i` and `i + 1`. Conserves `Σ inertia T`.
fn exchange_bands(inertia: &[f64], base: &[f64], conductance: &[f64]) -> Option<Vec<f64>> {
    let n = base.len();
    if n == 0 || inertia.len() != n || conductance.len() + 1 != n {
        return None;
    }
    let mut lower = vec![0.0; n];
    let mut diag = vec![0.0; n];
    let mut upper = vec![0.0; n];
    let mut rhs = vec![0.0; n];
    for i in 0..n {
        let left = if i == 0 { 0.0 } else { conductance[i - 1] };
        let right = if i + 1 == n { 0.0 } else { conductance[i] };
        lower[i] = -left;
        upper[i] = -right;
        diag[i] = inertia[i] + left + right;
        rhs[i] = inertia[i] * base[i];
    }
    thomas(&lower, &diag, &upper, &rhs)
}

/// Latent-heat weight of moist transport, `(L / c_p) RH dq_s/dT`, dimensionless.
fn moist_chi(t_c: f64) -> f64 {
    let t = t_c.clamp(-60.0, 60.0);
    let e_s = 611.2 * (17.27 * t / (t + 237.3)).exp();
    let q_s = 0.622 * e_s / 1.0e5;
    2490.0 * 0.8 * q_s * 17.27 * 237.3 / ((t + 237.3) * (t + 237.3))
}

/// Annual zonal-mean temperature of the Earth-like calibration, °C, at
/// `x = sin φ`. The base diffusivity already carries this climate's moisture.
fn earth_reference_c(x: f64) -> f64 {
    28.0 - 44.0 * x * x
}

/// Moist transport relative to the calibration climate. Warm air carries more
/// latent heat (Clausius–Clapeyron), so warm planets flatten their pole-to-
/// equator contrast and cold planets sharpen it.
fn moist_scale(t_face_c: f64, x: f64) -> f64 {
    ((1.0 + moist_chi(t_face_c)) / (1.0 + moist_chi(earth_reference_c(x)))).clamp(0.4, 3.0)
}

fn cover_albedo(land_frac: f64, snow: f64, ice: f64, glacier: f64) -> f64 {
    let seasonal = 0.32 + (LAND_ALBEDO_COLD - 0.32) * snow;
    let land = seasonal + (LAND_ICE_ALBEDO - seasonal) * glacier;
    let ocean = 0.28 + (0.62 - 0.28) * ice;
    land_frac * land + (1.0 - land_frac) * ocean
}

/// Linear interpolation of the band increment in `x = sin φ`, clamped to the
/// end-band centres so a cell on a band edge does not take a step.
fn sample_band_delta(sin_phi: f64, delta: &[f64]) -> f64 {
    let n = delta.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return delta[0];
    }
    let dx = 2.0 / n as f64;
    let x = sin_phi.clamp(-1.0, 1.0);
    let first = -1.0 + 0.5 * dx;
    let last = 1.0 - 0.5 * dx;
    if x <= first {
        return delta[0];
    }
    if x >= last {
        return delta[n - 1];
    }
    let pos = (x - first) / dx;
    let i = (pos.floor() as usize).min(n - 2);
    let frac = pos - i as f64;
    delta[i] * (1.0 - frac) + delta[i + 1] * frac
}

fn band_means(geo: &[Geo], capacity: &[f64], temperature: &[f64]) -> ([f64; BANDS], [f64; BANDS]) {
    let mut sum_t = [0.0; BANDS];
    let mut sum_c = [0.0; BANDS];
    let mut sum_a = [0.0; BANDS];
    let mut occupied = [false; BANDS];
    for (cell, place) in geo.iter().enumerate() {
        let band = place.band;
        sum_t[band] += place.area * temperature[cell];
        sum_c[band] += place.area * capacity[cell];
        sum_a[band] += place.area;
        occupied[band] = true;
    }
    let mut t_bar = [0.0; BANDS];
    let mut c_bar = [0.0; BANDS];
    for band in 0..BANDS {
        if sum_a[band] > 0.0 {
            t_bar[band] = sum_t[band] / sum_a[band];
            c_bar[band] = sum_c[band] / sum_a[band];
        }
    }
    // A sinφ band with no cell centre would otherwise have zero heat capacity and
    // couple the tridiagonal solve to an undefined temperature. Copy the nearest
    // occupied band so meridional transport stays well posed. Occupied bands keep
    // their area-weighted means.
    fill_empty_bands(&mut t_bar, &mut c_bar, &occupied);
    (t_bar, c_bar)
}

fn fill_empty_bands(t_bar: &mut [f64], c_bar: &mut [f64], occupied: &[bool]) {
    if occupied.iter().all(|flag| *flag) {
        return;
    }
    let snapshot_t = t_bar.to_vec();
    let snapshot_c = c_bar.to_vec();
    for band in 0..BANDS {
        if occupied[band] {
            continue;
        }
        if let Some(source) = nearest_occupied(occupied, band) {
            t_bar[band] = snapshot_t[source];
            c_bar[band] = snapshot_c[source];
        }
    }
}

fn nearest_occupied(occupied: &[bool], band: usize) -> Option<usize> {
    for dist in 1..BANDS {
        let left = band as isize - dist as isize;
        if left >= 0 && occupied[left as usize] {
            return Some(left as usize);
        }
        let right = band + dist;
        if right < BANDS && occupied[right] {
            return Some(right);
        }
    }
    None
}

#[cfg(test)]
/// Implicit diffusion `C (T' − T) / Δt = D d/dx [(1 − x²) dT'/dx]` on equal-area
/// `x = sin φ` bands. No flux through the poles. The stencil conserves `Σ C T`
/// in exact arithmetic: interface fluxes cancel and the polar conductances are zero.
#[cfg(test)]
pub(crate) fn diffuse_meridional(
    capacity: &[f64],
    temperature: &[f64],
    diffusion: f64,
    dt: f64,
) -> Vec<f64> {
    let n = temperature.len();
    if n == 0 || capacity.len() != n || !(dt.is_finite() && dt.abs() > 0.0) {
        return temperature.to_vec();
    }
    let inertia: Vec<f64> = capacity.iter().map(|c| c / dt).collect();
    exchange_bands(&inertia, temperature, &meridional_conductance(n, diffusion))
        .unwrap_or_else(|| temperature.to_vec())
}

fn thomas(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = rhs.len();
    if n == 0 || diag[0].abs() < 1e-30 {
        return None;
    }
    let mut c_prime = vec![0.0; n];
    let mut d_prime = vec![0.0; n];
    c_prime[0] = upper[0] / diag[0];
    d_prime[0] = rhs[0] / diag[0];
    for i in 1..n {
        let pivot = diag[i] - lower[i] * c_prime[i - 1];
        if pivot.abs() < 1e-30 {
            return None;
        }
        c_prime[i] = if i + 1 == n { 0.0 } else { upper[i] / pivot };
        d_prime[i] = (rhs[i] - lower[i] * d_prime[i - 1]) / pivot;
    }
    let mut out = vec![0.0; n];
    out[n - 1] = d_prime[n - 1];
    for i in (0..n - 1).rev() {
        out[i] = d_prime[i] - c_prime[i] * out[i + 1];
    }
    if out.iter().any(|value| !value.is_finite()) {
        None
    } else {
        Some(out)
    }
}

fn itcz_latitudes(
    geo: &[Geo],
    temperature: &[f64],
    window_deg: f64,
    tilt_deg: f64,
) -> [f64; LON_BINS] {
    let mut max_t = [f64::NEG_INFINITY; LON_BINS];
    let mut found = [false; LON_BINS];
    for (cell, place) in geo.iter().enumerate() {
        let lat_deg = place.lat.to_degrees();
        if lat_deg.abs() > window_deg {
            continue;
        }
        let bin = longitude_bin(place.lon);
        found[bin] = true;
        if temperature[cell] > max_t[bin] {
            max_t[bin] = temperature[cell];
        }
    }
    let mut sum_w = [0.0; LON_BINS];
    let mut sum_lat = [0.0; LON_BINS];
    for (cell, place) in geo.iter().enumerate() {
        let lat_deg = place.lat.to_degrees();
        if lat_deg.abs() > window_deg {
            continue;
        }
        let bin = longitude_bin(place.lon);
        let weight = (temperature[cell] - (max_t[bin] - 3.0)).max(0.0);
        sum_w[bin] += weight;
        sum_lat[bin] += weight * lat_deg;
    }
    let mut lat = [0.0; LON_BINS];
    for bin in 0..LON_BINS {
        if sum_w[bin] > 0.0 {
            lat[bin] = sum_lat[bin] / sum_w[bin];
        }
    }
    fill_itcz_gaps(&mut lat, &found);
    smooth_itcz(lat, (tilt_deg + 5.0).min(35.0))
}

fn fill_itcz_gaps(lat: &mut [f64; LON_BINS], found: &[bool; LON_BINS]) {
    let snapshot = *lat;
    for bin in 0..LON_BINS {
        if found[bin] {
            continue;
        }
        for dist in 1..LON_BINS {
            let left = (bin + LON_BINS - dist) % LON_BINS;
            if found[left] {
                lat[bin] = snapshot[left];
                break;
            }
            let right = (bin + dist) % LON_BINS;
            if found[right] {
                lat[bin] = snapshot[right];
                break;
            }
        }
    }
}

fn smooth_itcz(lat: [f64; LON_BINS], cap: f64) -> [f64; LON_BINS] {
    let mut out = [0.0; LON_BINS];
    for (bin, slot) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        for offset in 0..5 {
            let index = (bin + LON_BINS - 2 + offset) % LON_BINS;
            sum += lat[index];
        }
        *slot = (sum / 5.0).clamp(-cap, cap);
    }
    out
}

fn interpolate_itcz(bins: &[f64; LON_BINS], lon: f64) -> f64 {
    let u = ((lon + std::f64::consts::PI) / std::f64::consts::TAU * LON_BINS as f64 - 0.5)
        .rem_euclid(LON_BINS as f64);
    let i0 = u.floor() as usize;
    let frac = u - i0 as f64;
    let i1 = (i0 + 1) % LON_BINS;
    bins[i0] * (1.0 - frac) + bins[i1] * frac
}

/// ITCZ, band winds, and a capped thermal term, from the stored monthly
/// temperature. Precipitation is the only consumer.
fn diagnose_circulation(
    grid: &CubeSphere,
    geo: &[Geo],
    temperature: &[f64],
    tilt_deg: f64,
    rotation: &Rotation,
) -> (Vec<f64>, [[f64; LON_BINS]; MONTHS]) {
    let n = geo.len();
    let window = tilt_deg + 10.0;
    let mut wind = vec![0.0; MONTHS * n * 3];
    let mut itcz = [[0.0; LON_BINS]; MONTHS];
    for month in 0..MONTHS {
        let slice = &temperature[month * n..(month + 1) * n];
        let month_itcz = itcz_latitudes(geo, slice, window, tilt_deg);
        itcz[month] = month_itcz;
        let anomaly = zonal_anomaly(geo, slice);
        let smooth = smooth_two_rings(grid, &anomaly);
        let month_wind: Vec<[f64; 3]> = (0..n)
            .into_par_iter()
            .map(|cell| surface_wind(grid, geo, &smooth, cell, &month_itcz, rotation))
            .collect();
        for (cell, flow) in month_wind.iter().enumerate() {
            let base = (month * n + cell) * 3;
            wind[base] = flow[0];
            wind[base + 1] = flow[1];
            wind[base + 2] = flow[2];
        }
    }
    (wind, itcz)
}

fn zonal_anomaly(geo: &[Geo], temperature: &[f64]) -> Vec<f64> {
    let ones = vec![1.0; geo.len()];
    let (mean, _) = band_means(geo, &ones, temperature);
    temperature
        .iter()
        .enumerate()
        .map(|(cell, value)| value - sample_band_delta(geo[cell].pos[2], &mean))
        .collect()
}

/// Equal average of the cell and every unique neighbour within two rings.
fn smooth_two_rings(grid: &CubeSphere, field: &[f64]) -> Vec<f64> {
    (0..field.len())
        .into_par_iter()
        .map(|cell| {
            let mut ids = Vec::with_capacity(32);
            ids.push(cell as u32);
            let mut start = 0usize;
            for _ring in 0..2 {
                let end = ids.len();
                for index in start..end {
                    for neighbour in grid.neighbors(ids[index]) {
                        if neighbour != NONE && !ids.contains(&neighbour) {
                            ids.push(neighbour);
                        }
                    }
                }
                start = end;
            }
            let mut sum = 0.0;
            for id in &ids {
                sum += field[*id as usize];
            }
            sum / ids.len() as f64
        })
        .collect()
}

fn surface_wind(
    grid: &CubeSphere,
    geo: &[Geo],
    anomaly: &[f64],
    cell: usize,
    itcz: &[f64; LON_BINS],
    rotation: &Rotation,
) -> [f64; 3] {
    let place = &geo[cell];
    let phi_itcz = interpolate_itcz(itcz, place.lon);
    let phi_p = place.lat.to_degrees() - phi_itcz * place.lat.cos().powi(2);
    let (mut zonal, mut meridional) = band_wind(phi_p, rotation.phi_h_deg, rotation.phi_f_deg);
    let frame = Frame {
        pos: place.pos,
        east: place.east,
        north: place.north,
    };
    let (gx, gy) = tangent_gradient(grid, anomaly, cell, &frame, |nb| grid.position(nb));
    let scale = THERMAL_SCALE * place.lat.cos().powi(2);
    let mut thermal_u = scale * gx;
    let mut thermal_v = scale * gy;
    let magnitude = (thermal_u * thermal_u + thermal_v * thermal_v).sqrt();
    if magnitude > THERMAL_CAP_MS {
        let capped = THERMAL_CAP_MS / magnitude;
        thermal_u *= capped;
        thermal_v *= capped;
    }
    zonal += thermal_u;
    meridional += thermal_v;
    [
        zonal * place.east[0] + meridional * place.north[0],
        zonal * place.east[1] + meridional * place.north[1],
        zonal * place.east[2] + meridional * place.north[2],
    ]
}

/// Fractional position inside the Ferrel band, `0` outside it.
///
/// This is the `s` used by [`band_wind`]: `( |φ'| − φ_H ) / (φ_F − φ_H)`.
/// Poleward eddy moisture speed, m/s: rises from the Hadley edge to the middle
/// of the Ferrel band, then fades to zero at the pole.
fn storm_track_eddy(magnitude: f64, phi_h: f64, phi_f: f64) -> f64 {
    let mid = 0.5 * (phi_h + phi_f);
    if magnitude <= phi_h {
        0.0
    } else if magnitude < mid {
        3.0 * (std::f64::consts::FRAC_PI_2 * (magnitude - phi_h) / (mid - phi_h).max(1e-6)).sin()
    } else {
        3.0 * (std::f64::consts::FRAC_PI_2 * ((magnitude - mid) / (90.0 - mid).max(1e-6)).min(1.0))
            .cos()
    }
}

fn band_wind(phi_p_deg: f64, phi_h: f64, phi_f: f64) -> (f64, f64) {
    let magnitude = phi_p_deg.abs();
    let sign = if phi_p_deg > 0.0 {
        1.0
    } else if phi_p_deg < 0.0 {
        -1.0
    } else {
        0.0
    };
    if magnitude < phi_h {
        let wave = (std::f64::consts::PI * (magnitude / phi_h.max(1e-6)).clamp(0.0, 1.0)).sin();
        (-6.0 * wave, -sign * 2.0 * wave)
    } else if magnitude < phi_f {
        let span = (phi_f - phi_h).max(1e-6);
        let wave = (std::f64::consts::PI * ((magnitude - phi_h) / span).clamp(0.0, 1.0)).sin();
        (8.0 * wave, sign * wave)
    } else {
        let span = (90.0 - phi_f).max(1e-6);
        let wave = (std::f64::consts::PI * ((magnitude - phi_f) / span).clamp(0.0, 1.0)).sin();
        (-4.0 * wave, -sign * wave)
    }
}

#[derive(Clone, Copy)]
struct PrecipIn<'a> {
    planet: &'a PlanetSettings,
    grid: &'a CubeSphere,
    geo: &'a [Geo],
    land: &'a [CoarseLand],
    temperature: &'a [f64],
    insolation: &'a [f64],
    wind: &'a [f64],
    itcz: &'a [[f64; LON_BINS]; MONTHS],
    phi_h_deg: f64,
    phi_f_deg: f64,
}

fn precipitate(input: &PrecipIn<'_>) -> Vec<f64> {
    let PrecipIn {
        planet,
        grid,
        geo,
        land,
        temperature,
        insolation,
        wind,
        itcz,
        phi_h_deg,
        phi_f_deg,
    } = *input;
    let n = geo.len();
    let steps = planet.month_seconds() / STEP_SECONDS;
    let mut monthly = vec![0.0; MONTHS * n];
    // Vapour and soil water carry from month to month. Restarting each month
    // at half saturation left that start-up vapour still raining out in the
    // dry belt after 48 sweeps, a floor of about 200 mm a year that the
    // evaporation supply could not lower. Warm-up years from the last month's
    // saturation are discarded; each published month is the mean rain of its
    // sweeps.
    let last = &temperature[(MONTHS - 1) * n..];
    let mut column: Vec<f64> = last
        .iter()
        .map(|value| 0.5 * saturation_mm(*value))
        .collect();
    // Soil-water memory per cell. Land evaporation reads the mean of previous
    // steps, then the mean takes in the step that just fell.
    let mut p_mean = vec![0.0; n];
    for year in 0..=MOISTURE_WARMUP_YEARS {
        for month in 0..MONTHS {
            let prep = moisture_prep(
                &MoistureCtx {
                    grid,
                    itcz: &itcz[month],
                    phi_h_deg,
                    phi_f_deg,
                    precip_scale: planet.precipitation_scale,
                },
                geo,
                land,
                &temperature[month * n..(month + 1) * n],
                &insolation[month * n..(month + 1) * n],
                wind,
                month,
            );
            let mut rain = vec![0.0; n];
            for _ in 0..MOISTURE_SWEEPS {
                let stepped = moisture_sweep(grid, &prep, &column, &p_mean);
                for cell in 0..n {
                    column[cell] = stepped[cell].0;
                    let p_step = stepped[cell].1;
                    rain[cell] += p_step;
                    p_mean[cell] = 0.8 * p_mean[cell] + 0.2 * p_step;
                }
            }
            if year == MOISTURE_WARMUP_YEARS {
                for cell in 0..n {
                    monthly[month * n + cell] = rain[cell] / MOISTURE_SWEEPS as f64 * steps;
                }
            }
        }
    }
    monthly
}

struct MoistureCtx<'a> {
    grid: &'a CubeSphere,
    itcz: &'a [f64; LON_BINS],
    phi_h_deg: f64,
    phi_f_deg: f64,
    precip_scale: f64,
}

fn moisture_prep(
    ctx: &MoistureCtx<'_>,
    geo: &[Geo],
    land: &[CoarseLand],
    temperature: &[f64],
    insolation: &[f64],
    wind: &[f64],
    month: usize,
) -> Vec<MoistureCell> {
    let n = geo.len();
    (0..n)
        .into_par_iter()
        .map(|cell| {
            moisture_cell(
                ctx,
                &geo[cell],
                &land[cell],
                temperature[cell],
                insolation[cell],
                wind_at(wind, month, cell, n),
            )
        })
        .collect()
}

fn moisture_cell(
    ctx: &MoistureCtx<'_>,
    place: &Geo,
    land: &CoarseLand,
    temperature: f64,
    insolation: f64,
    wind: [f64; 3],
) -> MoistureCell {
    let phi_p =
        place.lat.to_degrees() - interpolate_itcz(ctx.itcz, place.lon) * place.lat.cos().powi(2);
    let (convect, rh_crit) = regime(phi_p, ctx.phi_h_deg);
    let w_sat = saturation_mm(temperature);
    let ice = smoothstep(-1.8, -8.0, temperature);
    // The 850–700 hPa layer carries vapour faster than the surface wind.
    // Surface winds stored on the field are left unchanged. Storm-track eddies
    // add a poleward flux from the Hadley edge to the pole; stopping it at the
    // Ferrel edge left land above 60° with almost no rain.
    let sign = if phi_p > 0.0 {
        1.0
    } else if phi_p < 0.0 {
        -1.0
    } else {
        0.0
    };
    let eddy = sign * storm_track_eddy(phi_p.abs(), ctx.phi_h_deg, ctx.phi_f_deg);
    let steered = [
        wind[0] * 1.6 + eddy * place.north[0],
        wind[1] * 1.6 + eddy * place.north[1],
        wind[2] * 1.6 + eddy * place.north[2],
    ];
    let speed = horizontal_speed(place.pos, steered);
    let calm = speed < 1e-8;
    let upwind = if calm {
        place.pos
    } else {
        march_upwind(place.pos, steered, speed * STEP_SECONDS, ctx.grid.radius_m)
    };
    MoistureCell {
        upwind,
        calm,
        ocean_factor: EVAP_MM_PER_6H
            * (w_sat / 50.0)
            * (1.0 - ice)
            * (1.0 - land.land_frac)
            * ctx.precip_scale,
        w_sat,
        land_pet: pet_mm(STEP_SECONDS / 86_400.0, temperature, insolation),
        land_weight: land.land_frac * ctx.precip_scale,
        convect,
        rh_crit,
        w_sat_barrier: saturation_mm(temperature - LAPSE_K_PER_M * land.elev_barrier),
    }
}

fn moisture_sweep(
    grid: &CubeSphere,
    prep: &[MoistureCell],
    column: &[f64],
    p_mean: &[f64],
) -> Vec<(f64, f64)> {
    (0..prep.len())
        .into_par_iter()
        .map(|cell| {
            let item = &prep[cell];
            let w_up = if item.calm {
                column[cell]
            } else {
                sample_f64(grid, column, item.upwind).max(0.0)
            };
            column_step(item, w_up, p_mean[cell])
        })
        .collect()
}

fn column_step(item: &MoistureCell, w_up: f64, p_mean: f64) -> (f64, f64) {
    let deficit = (1.0 - w_up / item.w_sat).max(0.0);
    let land_evap = item.land_pet.min(0.65 * p_mean.max(0.0));
    let evap = item.ocean_factor * deficit + land_evap * item.land_weight;
    let w_prime = (w_up + evap).max(0.0);
    let excess = (w_prime - item.rh_crit * item.w_sat_barrier).max(0.0);
    let mut rain = excess * (1.0 - (-1.0_f64).exp()) + item.convect * w_prime;
    if rain > w_prime {
        rain = w_prime;
    }
    if rain < 0.0 {
        rain = 0.0;
    }
    (w_prime - rain, rain)
}

fn regime(phi_p_deg: f64, phi_h_deg: f64) -> (f64, f64) {
    let abs_phi = phi_p_deg.abs();
    let mut convect = 0.10 * (-(phi_p_deg / 8.0).powi(2)).exp();
    let storm = (abs_phi - (phi_h_deg + 12.0)) / 12.0;
    convect += 0.03 * (-storm.powi(2)).exp();
    // Wide shoulders. A narrow window left a crisp slanted edge on the
    // subtropical ocean where the dry regime switched on across one cell.
    let s = smoothstep(0.35 * phi_h_deg, 0.60 * phi_h_deg, abs_phi)
        * smoothstep(1.40 * phi_h_deg, 1.10 * phi_h_deg, abs_phi);
    (convect * (1.0 - 0.8 * s), 0.85 + 0.15 * s)
}

struct DownCtx<'a> {
    fine: &'a CubeSphere,
    field: &'a ClimateField,
    h_pos: &'a [f64],
    positions: &'a [[f64; 3]],
    ocean: &'a [u8],
    month_days: f64,
    stencils: &'a [ChartStencil],
}

fn downscale_cell(ctx: &DownCtx<'_>, cell: usize) -> Downscaled {
    let pos = ctx.positions[cell];
    let (east, north) = east_north(pos);
    let (gx, gy) = tangent_gradient(
        ctx.fine,
        ctx.h_pos,
        cell,
        &Frame { pos, east, north },
        |nb| ctx.positions[nb as usize],
    );
    let stencil = &ctx.stencils[cell];
    let n = ctx.field.grid.cell_count();
    let ocean = ctx.ocean.get(cell).copied().unwrap_or(1);
    let mut t_sum = 0.0;
    let mut t_warm = f64::MIN;
    let mut t_cold = f64::MAX;
    let mut p_sum = 0.0;
    let mut p_dry = f64::MAX;
    let mut pet_sum = 0.0;
    let mut snow = 0.0;
    let mut degree_days = 0.0;
    let mut ice_months = 0u8;
    for month in 0..MONTHS {
        let sample = sample_month(ctx.field, stencil, n, month);
        let wind = tangent_part(pos, sample.wind);
        let zonal = dot3(wind, east);
        let meridional = dot3(wind, north);
        let speed = (zonal * zonal + meridional * meridional).sqrt();
        let factor = orographic_factor(speed, zonal, meridional, gx, gy);
        let t_m = sample.t_sl - LAPSE_K_PER_M * ctx.h_pos[cell];
        let p_m = (sample.precip * factor).max(0.0);
        t_sum += t_m;
        t_warm = t_warm.max(t_m);
        t_cold = t_cold.min(t_m);
        p_sum += p_m;
        p_dry = p_dry.min(p_m);
        pet_sum += pet_mm(ctx.month_days, t_m, sample.insolation);
        snow += p_m * smoothstep(2.0, -2.0, t_m);
        degree_days += positive_degree_days(ctx.month_days, t_m);
        if ocean == 1 && sample.t_sl < -1.8 {
            ice_months = ice_months.saturating_add(1);
        }
    }
    let t_annual = t_sum / MONTHS as f64;
    let pet = pet_sum.max(0.0);
    let rain = p_sum.max(0.0);
    let aet = fu_aet(rain, pet);
    Downscaled {
        t_annual: t_annual as f32,
        t_warm: t_warm as f32,
        t_cold: t_cold as f32,
        p_annual: rain as f32,
        p_dry: p_dry.max(0.0) as f32,
        pet: pet as f32,
        runoff: (rain - aet).max(0.0) as f32,
        ice: (snow - MELT_MM_PER_PDD * degree_days) as f32,
        sea_ice: ice_months,
    }
}

struct MonthSample {
    t_sl: f64,
    precip: f64,
    insolation: f64,
    wind: [f64; 3],
}

fn sample_month(
    field: &ClimateField,
    stencil: &ChartStencil,
    n: usize,
    month: usize,
) -> MonthSample {
    let start = month * n;
    let plane = month * 3 * n;
    MonthSample {
        t_sl: stencil.sample(&field.t_c[start..start + n]),
        precip: stencil.sample(&field.p_mm[start..start + n]),
        insolation: stencil.sample(&field.q_wm2[start..start + n]),
        wind: [
            stencil.sample(&field.wind[plane..plane + n]),
            stencil.sample(&field.wind[plane + n..plane + 2 * n]),
            stencil.sample(&field.wind[plane + 2 * n..plane + 3 * n]),
        ],
    }
}

fn orographic_factor(speed: f64, zonal: f64, meridional: f64, gx: f64, gy: f64) -> f64 {
    if speed < 0.5 {
        1.0
    } else {
        let upslope = (zonal * gx + meridional * gy) / (speed * 0.01);
        (1.0 + 0.6 * upslope).clamp(0.4, 2.5)
    }
}

/// Coldest-month sea-level temperature on land in the westerlies.
///
/// Coastal cells reach ocean within 500 km upwind. Inland cells stay on land
/// for 1500 km upwind. Both limits use the coldest month's wind.
#[cfg(test)]
pub(crate) struct CoastContrast {
    pub coast_c: f64,
    pub inland_c: f64,
    pub coast_n: usize,
    pub inland_n: usize,
}

#[cfg(test)]
pub(crate) fn westerly_coast_contrast(
    grid: &CubeSphere,
    ocean: &[u8],
    monthly_t: &[f32],
    wind: &[f32],
) -> CoastContrast {
    let n = grid.cell_count();
    let mut coast = Accum::default();
    let mut inland = Accum::default();
    let mut coast_n = 0usize;
    let mut inland_n = 0usize;
    if ocean.len() != n || monthly_t.len() != MONTHS * n || wind.len() != MONTHS * 3 * n {
        return CoastContrast {
            coast_c: 0.0,
            inland_c: 0.0,
            coast_n: 0,
            inland_n: 0,
        };
    }
    for cell in 0..n {
        if ocean[cell] != 0 {
            continue;
        }
        let lat = grid.lat_lon(cell as u32).0.to_degrees().abs();
        if !(40.0..=60.0).contains(&lat) {
            continue;
        }
        let month = coldest_month(monthly_t, n, cell);
        let flow = wind_components(wind, n, month, cell);
        let pos = grid.position(cell as u32);
        let (east, _) = east_north(pos);
        let zonal = flow[0] * east[0] + flow[1] * east[1] + flow[2] * east[2];
        if zonal <= 0.0 {
            continue;
        }
        let reached = upwind_ocean_m(grid, ocean, pos, flow);
        let temperature = f64::from(monthly_t[month * n + cell]);
        let area = grid.area_m2(cell as u32);
        if reached <= 500_000.0 {
            coast.add(temperature, area);
            coast_n += 1;
        } else if reached >= 1_500_000.0 {
            inland.add(temperature, area);
            inland_n += 1;
        }
    }
    CoastContrast {
        coast_c: coast.mean(),
        inland_c: inland.mean(),
        coast_n,
        inland_n,
    }
}

#[cfg(test)]
fn coldest_month(monthly_t: &[f32], n: usize, cell: usize) -> usize {
    let mut best = 0usize;
    let mut cold = f32::MAX;
    for month in 0..MONTHS {
        let value = monthly_t[month * n + cell];
        if value < cold {
            cold = value;
            best = month;
        }
    }
    best
}

#[cfg(test)]
fn wind_components(wind: &[f32], n: usize, month: usize, cell: usize) -> [f64; 3] {
    let base = month * 3 * n + cell;
    [
        f64::from(wind[base]),
        f64::from(wind[base + n]),
        f64::from(wind[base + 2 * n]),
    ]
}

#[cfg(test)]
fn upwind_ocean_m(grid: &CubeSphere, ocean: &[u8], pos: [f64; 3], wind: [f64; 3]) -> f64 {
    let step = 50_000.0;
    let mut dist = 0.0;
    while dist < 2_000_000.0 {
        dist += step;
        let point = march_upwind(pos, wind, dist, grid.radius_m);
        if ocean[grid.cell_at(point) as usize] == 1 {
            return dist;
        }
    }
    f64::INFINITY
}

fn area_stats(
    fine: &CubeSphere,
    ocean: &[u8],
    summary: &ClimateSummary,
    field: &ClimateField,
    positions: &[[f64; 3]],
) -> ClimateStats {
    let mut global_t = Accum::default();
    let mut equator_t = Accum::default();
    let mut polar_t = Accum::default();
    let mut global_p = Accum::default();
    let mut equator_p = Accum::default();
    let mut subtrop_p = Accum::default();
    for (cell, (pos, (temperature, precip))) in positions
        .iter()
        .zip(summary.t_annual_c.iter().zip(&summary.p_annual_mm))
        .enumerate()
    {
        let lat = pos[2].clamp(-1.0, 1.0).asin();
        let lat_deg = lat.to_degrees().abs();
        let area = fine.area_m2(cell as u32);
        let temperature = f64::from(*temperature);
        let precip = f64::from(*precip);
        global_t.add(temperature, area);
        global_p.add(precip, area);
        if lat_deg < 10.0 {
            equator_t.add(temperature, area);
            equator_p.add(precip, area);
        }
        if lat_deg > 70.0 {
            polar_t.add(temperature, area);
        }
        if (15.0..=35.0).contains(&lat_deg) && ocean.get(cell).copied().unwrap_or(1) == 0 {
            subtrop_p.add(precip, area);
        }
    }
    ClimateStats {
        t_global_c: global_t.mean(),
        t_equator_c: equator_t.mean(),
        t_polar_c: polar_t.mean(),
        p_global_mm: global_p.mean(),
        p_equator_mm: equator_p.mean(),
        p_subtropical_land_mm: subtrop_p.mean(),
        spinup_years: field.spinup_years,
        spinup_converged: field.converged,
        spinup_residual_k: field.spinup_residual_k,
        phi_h_deg: field.phi_h_deg,
    }
}

#[derive(Clone, Copy, Default)]
struct Accum {
    sum: f64,
    area: f64,
}

impl Accum {
    fn add(&mut self, value: f64, area: f64) {
        self.sum += value * area;
        self.area += area;
    }

    fn mean(self) -> f64 {
        if self.area > 0.0 {
            self.sum / self.area
        } else {
            0.0
        }
    }
}

/// Daily-mean top-of-atmosphere insolation, W/m².
///
/// `h0 = acos(clamp(−tan φ · tan δ, −1, 1))`. Latitudes or declinations at
/// `±90°` skip the tangent: polar day when `φ` and `δ` share a sign, polar night otherwise.
pub(crate) fn daily_insolation(solar: f64, lat: f64, decl: f64) -> f64 {
    let phi = lat.clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
    let delta = decl.clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
    let h0 = hour_angle(phi, delta);
    let q = (solar / std::f64::consts::PI)
        * (h0 * phi.sin() * delta.sin() + phi.cos() * delta.cos() * h0.sin());
    if q.is_finite() {
        q.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn declination(tilt: f64, lambda: f64) -> f64 {
    (tilt.sin() * lambda.sin()).asin()
}

pub(crate) fn distance_factor(eccentricity: f64, lambda: f64, perihelion: f64) -> f64 {
    let e = eccentricity.clamp(0.0, 0.99);
    let numer = 1.0 + e * (lambda - perihelion).cos();
    let denom = 1.0 - e * e;
    (numer * numer) / (denom * denom)
}

fn hour_angle(phi: f64, delta: f64) -> f64 {
    let cos_phi = phi.cos();
    let cos_delta = delta.cos();
    if cos_phi.abs() < 1e-10 || cos_delta.abs() < 1e-10 {
        return if phi.sin() * delta.sin() >= 0.0 {
            std::f64::consts::PI
        } else {
            0.0
        };
    }
    let argument = (-(phi.sin() / cos_phi) * (delta.sin() / cos_delta)).clamp(-1.0, 1.0);
    argument.acos()
}

fn heat_capacity(land_frac: f64, maritime: f64) -> f64 {
    let land = LAND_HEAT + MARITIME_BLEND * maritime * (OCEAN_HEAT - LAND_HEAT);
    land_frac * land + (1.0 - land_frac) * OCEAN_HEAT
}

/// Annual band wind, no ITCZ shift and no thermal term. `m` is the weighted
/// ocean fraction upwind of the cell.
fn maritime_index(
    grid: &CubeSphere,
    geo: &[Geo],
    land: &[CoarseLand],
    rotation: &Rotation,
) -> Vec<f64> {
    let ocean_frac: Vec<f64> = land.iter().map(|cell| 1.0 - cell.land_frac).collect();
    let distances = [250_000.0, 500_000.0, 1_000_000.0, 1_500_000.0];
    let weights = [0.4, 0.3, 0.2, 0.1];
    (0..geo.len())
        .into_par_iter()
        .map(|cell| {
            let place = &geo[cell];
            let (zonal, meridional) = band_wind(
                place.lat.to_degrees(),
                rotation.phi_h_deg,
                rotation.phi_f_deg,
            );
            let flow = [
                zonal * place.east[0] + meridional * place.north[0],
                zonal * place.east[1] + meridional * place.north[1],
                zonal * place.east[2] + meridional * place.north[2],
            ];
            let mut maritime = 0.0;
            for (distance, weight) in distances.iter().zip(weights) {
                let point = march_upwind(place.pos, flow, *distance, grid.radius_m);
                maritime += weight * sample_f64(grid, &ocean_frac, point);
            }
            maritime
        })
        .collect()
}

fn saturation_mm(temperature: f64) -> f64 {
    (50.0 * (0.068 * (temperature - 25.0)).exp()).max(1.0e-6)
}

fn pet_mm(days: f64, temperature: f64, insolation: f64) -> f64 {
    if temperature > 0.0 {
        days * 0.0135 * (temperature + 17.8) * 0.408 * 0.0864 * 0.6 * insolation.max(0.0)
    } else {
        0.0
    }
}

fn fu_aet(precip: f64, pet: f64) -> f64 {
    if precip <= 0.0 || pet <= 0.0 {
        return 0.0;
    }
    let phi = (pet / precip).min(1.0e6);
    let aet = precip * (1.0 + phi - (1.0 + phi.powf(FU_W)).powf(1.0 / FU_W));
    aet.clamp(0.0, precip)
}

fn positive_degree_days(days: f64, temperature: f64) -> f64 {
    let z = temperature / PDD_SIGMA_K;
    let density = (PDD_SIGMA_K / (std::f64::consts::TAU).sqrt()) * (-0.5 * z * z).exp();
    (days * (density + temperature * normal_cdf(z))).max(0.0)
}

fn normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// Abramowitz and Stegun 7.1.26. Maximum absolute error is about `1.5e-7`.
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let poly = ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t;
    sign * (1.0 - poly * (-x * x).exp())
}

/// Hermite step. `a > b` is the "cold" direction: `0` at `a`, `1` at `b`.
fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    if (b - a).abs() < 1e-15 {
        return if x >= b { 1.0 } else { 0.0 };
    }
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn tangent_gradient(
    grid: &CubeSphere,
    field: &[f64],
    cell: usize,
    frame: &Frame,
    position: impl Fn(u32) -> [f64; 3],
) -> (f64, f64) {
    let mut a00 = 0.0;
    let mut a01 = 0.0;
    let mut a11 = 0.0;
    let mut b0 = 0.0;
    let mut b1 = 0.0;
    let centre = field[cell];
    for (k, &nb) in grid.neighbors(cell as u32).iter().enumerate() {
        if nb == NONE {
            continue;
        }
        let dist = f64::from(grid.neighbor_distance(cell as u32, k));
        if dist <= 0.0 {
            continue;
        }
        let pk = position(nb);
        let proj = dot3(pk, frame.pos);
        let along = [
            pk[0] - frame.pos[0] * proj,
            pk[1] - frame.pos[1] * proj,
            pk[2] - frame.pos[2] * proj,
        ];
        let dx = dot3(along, frame.east);
        let dy = dot3(along, frame.north);
        let horiz = (dx * dx + dy * dy).sqrt();
        if horiz < 1e-12 {
            continue;
        }
        let xm = dx / horiz * dist;
        let ym = dy / horiz * dist;
        let dh = field[nb as usize] - centre;
        a00 += xm * xm;
        a01 += xm * ym;
        a11 += ym * ym;
        b0 += xm * dh;
        b1 += ym * dh;
    }
    let det = a00 * a11 - a01 * a01;
    if det.abs() < 1e-6 {
        (0.0, 0.0)
    } else {
        ((b0 * a11 - a01 * b1) / det, (a00 * b1 - a01 * b0) / det)
    }
}

fn march_upwind(pos: [f64; 3], wind: [f64; 3], dist: f64, radius: f64) -> [f64; 3] {
    let tangent = tangent_part(pos, wind);
    let speed = norm(tangent);
    if speed < 1e-12 || radius <= 0.0 {
        return pos;
    }
    let inv = 1.0 / speed;
    let dir = [-tangent[0] * inv, -tangent[1] * inv, -tangent[2] * inv];
    let theta = dist / radius;
    let (cosine, sine) = (theta.cos(), theta.sin());
    normalize([
        pos[0] * cosine + dir[0] * sine,
        pos[1] * cosine + dir[1] * sine,
        pos[2] * cosine + dir[2] * sine,
    ])
}

fn sample_f64(grid: &CubeSphere, field: &[f64], point: [f64; 3]) -> f64 {
    grid.sample_f64(field, point)
}

fn horizontal_speed(pos: [f64; 3], wind: [f64; 3]) -> f64 {
    norm(tangent_part(pos, wind))
}

fn tangent_part(up: [f64; 3], value: [f64; 3]) -> [f64; 3] {
    let radial = dot3(value, up);
    [
        value[0] - up[0] * radial,
        value[1] - up[1] * radial,
        value[2] - up[2] * radial,
    ]
}

fn east_north(up: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut east = cross([0.0, 0.0, 1.0], up);
    let mut len = norm(east);
    if len < 1e-8 {
        east = cross([0.0, 1.0, 0.0], up);
        len = norm(east);
    }
    if len < 1e-12 {
        return ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    }
    east = [east[0] / len, east[1] / len, east[2] / len];
    (east, cross(up, east))
}

fn norm(value: [f64; 3]) -> f64 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn band_index(z: f64) -> usize {
    let x = z.clamp(-1.0, 1.0);
    let band = ((x + 1.0) * 0.5 * BANDS as f64).floor() as isize;
    band.clamp(0, BANDS as isize - 1) as usize
}

fn longitude_bin(lon: f64) -> usize {
    let scaled = ((lon + std::f64::consts::PI) / std::f64::consts::TAU * LON_BINS as f64).floor();
    let bin = scaled as isize;
    bin.clamp(0, LON_BINS as isize - 1) as usize
}

fn wind_at(wind: &[f64], month: usize, cell: usize, n: usize) -> [f64; 3] {
    let base = (month * n + cell) * 3;
    [wind[base], wind[base + 1], wind[base + 2]]
}

fn to_f32(values: &[f64]) -> Vec<f32> {
    values.iter().map(|value| *value as f32).collect()
}

fn planar_wind(interleaved: &[f64], n: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; MONTHS * 3 * n];
    for month in 0..MONTHS {
        for comp in 0..3 {
            for cell in 0..n {
                let src = (month * n + cell) * 3 + comp;
                out[(month * 3 + comp) * n + cell] = interleaved[src] as f32;
            }
        }
    }
    out
}

fn itcz_to_f32(itcz: &[[f64; LON_BINS]; MONTHS]) -> Vec<f32> {
    let mut out = Vec::with_capacity(MONTHS * LON_BINS);
    for month in itcz {
        for value in month {
            out.push(*value as f32);
        }
    }
    out
}

fn fnv_f32(mut hash: u64, values: &[f32]) -> u64 {
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// Heat capacity seen by the air. Sea ice insulates the ocean below it, so an
/// ice-covered sea responds much like land: cold winters over the pack, and ice
/// that melts back in summer. With the full ocean capacity under the ice, polar
/// seas barely cooled in winter and almost all sea ice was perennial.
fn insulated_capacity(capacity: f64, land_frac: f64, ice: f64) -> f64 {
    capacity - SEA_ICE_INSULATION * (1.0 - land_frac) * ice * (OCEAN_HEAT - LAND_HEAT)
}
