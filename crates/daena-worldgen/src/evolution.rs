//! Coarse-to-fine uplift–erosion on the cube-sphere.
//!
//! Levels run from `n = 32` up to the requested resolution. Each level
//! resamples the tectonic fields, routes flow, then applies implicit stream
//! power, a slope threshold and a Jacobi hillslope step.
//!
//! Roughness is added on every level, including the first. The formula is
//! stated once for the elevation that level starts from, and the coarse level
//! needs it to cut channels.

use std::ops::Range;
use std::time::Instant;

use rayon::prelude::*;

use crate::climate::{self, ClimateOptions, SurfaceWater};
use crate::grid::{resample_with_cancel, CubeSphere};
use crate::noise::{
    derive_seed, derive_seed_salt, fbm, fbm_fixed_norm, noise_on_sphere, SeedDomain,
};
use crate::routing::{
    accumulate_with_weight, fold_cells_mut, fold_cells_zip_mut, route_with_cancel,
    run_basin_kernel_gather, run_basin_kernel_keep, BasinBalance, BasinView, CarveStats, CellIndex,
    DomainIndex, RoutingScratch, SlotIndex,
};
use crate::sealevel;
use crate::tectonics::Tectonics;
use crate::{ClimatePass, ErosionParams, LevelStats, Progress, Stage, WorldError, WorldSettings};

/// Elevation and masks handed from one level to the next.
struct Carry {
    grid: CubeSphere,
    h: Vec<f32>,
    domain: Vec<u8>,
}

/// Fields of the finest level, before the sea-level shift.
pub(crate) struct LevelFields {
    /// Grid this level was evolved on.
    pub grid: CubeSphere,
    /// Elevation in metres, still relative to the erosion base level.
    pub h: Vec<f32>,
    /// `1` where `base_m > 0` after detail was added.
    pub domain: Vec<u8>,
    /// Pre-erosion surface, unshifted.
    pub base: Vec<f32>,
    /// Uplift in metres per year.
    pub uplift: Vec<f32>,
    /// Erodibility multiplier.
    pub k_rock: Vec<f32>,
    /// Plate index.
    pub plate: Vec<u8>,
    /// `1` continental, `0` oceanic.
    pub crust: Vec<u8>,
}

/// Result of the multi-level evolution, scratch buffers already dropped.
pub(crate) struct Evolved {
    /// Finest level.
    pub fields: LevelFields,
    /// One entry per level, coarse to fine.
    pub stats: Vec<LevelStats>,
    /// `(name, milliseconds)` for each level and each climate pass.
    pub timings: Vec<(&'static str, f64)>,
    /// Climate passes that drove erosion. The final pass is added later.
    pub passes: Vec<ClimatePass>,
    /// The finest level's routing scratch, reused by final routing.
    pub scratch: Option<RoutingScratch>,
}

/// Borrowed runoff weight, evaporation and ice for one erosion level.
#[derive(Clone, Copy)]
pub(crate) struct WaterIn<'a> {
    /// Drainage weight derived from runoff.
    pub weight: &'a [f32],
    /// Runoff, m/yr.
    pub runoff_m_yr: &'a [f32],
    /// Net lake evaporation, m/yr.
    pub evap_m_yr: &'a [f32],
    /// Ice balance, m water equivalent per year.
    pub ice_m_yr: &'a [f32],
}

/// Controls the optional per-step checks used by tests.
pub(crate) struct ErodeOptions {
    /// Iterations at this level.
    pub iterations: u32,
    /// Time step in years.
    pub dt_yr: f64,
    /// Stream power, threshold and smoothing.
    pub params: ErosionParams,
    /// Record max `|Δh|` of each iteration.
    pub record_steps: bool,
    /// Count stream-power violations of the `U = 0` bounds.
    pub check_bounds: bool,
    /// Progress stage. `None` skips reports.
    pub stage: Option<Stage>,
}

/// What one level's iteration loop did.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ErodeReport {
    /// Domain pits in the last iteration, before carving.
    pub pits_last: u32,
    /// Domain pits in the first iteration, before carving.
    pub pits_first: u32,
    /// Largest pass-edge buffer built during the loop.
    pub edges_peak: u32,
    /// Basins carved in the last iteration.
    pub carved_last: u32,
    /// Cells clamped to the elevation cap in the last iteration.
    pub clamped_last: u32,
    /// Closed pit basins in the last iteration.
    pub closed_last: u32,
    /// Glaciated domain cells in the last iteration.
    pub glaciated_last: u32,
    /// Max `|Δh|` per iteration when requested. Read by the erosion tests.
    #[allow(dead_code)]
    pub steps: Vec<f32>,
    /// Stream-power bound violations. Zero when the check is off. Read by tests.
    #[allow(dead_code)]
    pub violations: u32,
    /// Max domain elevation after the last iteration.
    pub max_domain_m: f32,
    /// Area-weighted mean domain elevation after the last iteration.
    pub mean_domain_m: f32,
    /// Ice discharge of the last iteration. Compared by the mode test.
    #[cfg(test)]
    pub q_ice: Vec<f64>,
}

/// Evolve every level from 32 up to `settings.resolution`.
///
/// A climate pass runs at the start of level 64 from the level-32 surface, and
/// at the start of level 256 from the level-128 surface. Later levels downscale
/// that pass. Levels before the first pass erode with weight 1 and no ice.
pub(crate) fn evolve(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    progress: &mut dyn Progress,
) -> Result<Evolved, WorldError> {
    let seed = settings.seed;
    let radius_m = settings.radius_m;
    let tect_grid = CubeSphere::new(tectonics.n, radius_m).map_err(WorldError::Internal)?;
    let mut parent: Option<Carry> = None;
    let mut stats = Vec::new();
    let mut timings = Vec::new();
    let mut final_fields: Option<LevelFields> = None;
    let mut final_scratch: Option<RoutingScratch> = None;
    let mut carved = CarveStats::default();
    let mut driving: Option<climate::ClimateField> = None;
    let mut passes = Vec::new();
    // Prepared fields of finer levels, built beside the erosion climate pass.
    let mut pending: Vec<(u32, Prepared)> = Vec::new();
    let levels = level_resolutions(settings.resolution);
    for (index, n) in levels.iter().copied().enumerate() {
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        let step = settings.schedule.step_for(n).ok_or_else(|| {
            WorldError::InvalidSettings(format!("schedule does not cover n = {n}"))
        })?;
        let grid = CubeSphere::new(n, radius_m).map_err(WorldError::Internal)?;
        let mut prepared = match pending.iter().position(|(level, _)| *level == n) {
            Some(position) => pending.swap_remove(position).1,
            None => prepare_fields(&tect_grid, tectonics, &grid, seed, &|| progress.cancelled())?,
        };
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        if n == 64 || n == 256 {
            let source = parent.as_ref().ok_or_else(|| {
                WorldError::Internal(format!("climate at n = {n} has no coarser surface"))
            })?;
            let expect = if n == 64 { 32 } else { 128 };
            if source.grid.n != expect {
                return Err(WorldError::Internal(format!(
                    "climate at n = {n} expected source {expect}, found {}",
                    source.grid.n
                )));
            }
            let source_n = source.grid.n;
            // Level 64 prepares the two levels up to the next pass; the
            // level-256 pass prepares the rest. Each level is prepared once.
            let future = &levels[index + 1..];
            let future = if n == 64 {
                &future[..future.len().min(2)]
            } else {
                future
            };
            // The prepared fields depend only on tectonics, so they build
            // beside the mostly serial climate pass.
            let (climate_side, prepared_side) = rayon::join(
                || -> Result<(climate::ClimateField, f64), WorldError> {
                    let started_c = Instant::now();
                    let (frame, ocean) = sealevel::sea_frame(
                        &source.grid,
                        &source.h,
                        &source.domain,
                        settings.land_fraction,
                        &|| progress.cancelled(),
                    )?;
                    let field = climate::coarse_with(
                        &source.grid,
                        &frame,
                        &ocean,
                        &settings.planet,
                        &ClimateOptions::erosion(),
                        &|| progress.cancelled(),
                    )?;
                    Ok((field, elapsed_ms(started_c)))
                },
                || -> Result<Vec<(u32, Prepared)>, WorldError> {
                    let mut out = Vec::with_capacity(future.len());
                    for &finer in future {
                        if progress.cancelled() {
                            return Err(WorldError::Cancelled);
                        }
                        let finer_grid =
                            CubeSphere::new(finer, radius_m).map_err(WorldError::Internal)?;
                        let fields =
                            prepare_fields(&tect_grid, tectonics, &finer_grid, seed, &|| {
                                progress.cancelled()
                            })?;
                        out.push((finer, fields));
                    }
                    Ok(out)
                },
            );
            let (field, wall_ms) = climate_side?;
            pending = prepared_side?;
            timings.push((climate_timing_name(n), wall_ms));
            passes.push(ClimatePass {
                label: climate_label(n),
                source_n,
                spinup_years: field.spinup_years,
                converged: field.converged,
                residual_k: field.spinup_residual_k,
                rms_k: field.spinup_rms_k,
                wall_ms,
            });
            driving = Some(field);
        }
        prepared.h = initial_elevation(&grid, &prepared, parent.as_ref(), seed)?;
        fill_single_cell_pits(&grid, &mut prepared.h, &prepared.domain);
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        parent = None;
        let started = Instant::now();
        let domain_index = DomainIndex::active(&grid, &prepared.domain);
        let surface = match driving.as_ref() {
            Some(field) => {
                let (frame, ocean) = sealevel::sea_frame(
                    &grid,
                    &prepared.h,
                    &prepared.domain,
                    settings.land_fraction,
                    &|| progress.cancelled(),
                )?;
                if progress.cancelled() {
                    return Err(WorldError::Cancelled);
                }
                let water = climate::surface_water(
                    &grid,
                    &frame,
                    &ocean,
                    &prepared.domain,
                    &domain_index,
                    &settings.planet,
                    field,
                    &|| progress.cancelled(),
                )?;
                if progress.cancelled() {
                    return Err(WorldError::Cancelled);
                }
                Some(water)
            }
            None => None,
        };
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        let mut scratch = RoutingScratch::with_index(&domain_index);
        let mut buffers = ErosionBuffers::new(domain_index.len());
        let stage = Stage::Level(n);
        let options = ErodeOptions {
            iterations: step.iterations,
            dt_yr: step.dt_yr,
            params: settings.erosion,
            record_steps: false,
            check_bounds: false,
            stage: Some(stage),
        };
        let water = surface.as_ref().map(water_in);
        let report = erode(
            ErodeJob {
                grid: &grid,
                index: &domain_index,
                h: &mut prepared.h,
                domain: &prepared.domain,
                uplift: &prepared.uplift,
                k_rock: &prepared.k_rock,
                buffers: &mut buffers,
                scratch: &mut scratch,
                options: &options,
                water,
            },
            progress,
        )?;
        if index + 1 == levels.len() {
            // The last routing pass already found every pit and its spill.
            // Overflowing basins get a visible outlet before the sea-level
            // shift, so the canonical surface is the breached one. Only active
            // subsidence and glacial overdeepening keep a basin closed.
            carved = scratch.carve_overflow_outlets(
                &domain_index,
                &mut prepared.h,
                &prepared.uplift,
                water.map(|water| (water.runoff_m_yr, water.evap_m_yr)),
                Some(&buffers.glaciated),
            );
        }
        if index + 1 == levels.len() {
            final_scratch = Some(scratch);
        }
        drop(buffers);
        drop(surface);
        let wall_ms = elapsed_ms(started);
        stats.push(LevelStats {
            n,
            iterations: step.iterations,
            pits: report.pits_last,
            pits_first: report.pits_first,
            carved_basins: report.carved_last,
            edges_peak: report.edges_peak,
            max_elevation_m: report.max_domain_m,
            mean_elevation_m: report.mean_domain_m,
            clamped_cells: report.clamped_last,
            closed_basins: report.closed_last,
            glaciated_cells: report.glaciated_last,
            breached_basins: carved.carved,
            kept_subsiding_basins: carved.kept_subsiding,
            kept_glacial_basins: carved.kept_glacial,
            kept_arid_basins: carved.kept_arid,
            wall_ms,
        });
        timings.push((level_timing_name(n), wall_ms));
        let is_last = index + 1 == levels.len();
        if is_last {
            final_fields = Some(prepared.into_fields(grid));
        } else {
            let (grid_c, h_c, domain_c) = prepared.into_carry(grid);
            parent = Some(Carry {
                grid: grid_c,
                h: h_c,
                domain: domain_c,
            });
        }
    }
    let fields =
        final_fields.ok_or_else(|| WorldError::Internal("no levels were evolved".to_string()))?;
    Ok(Evolved {
        fields,
        stats,
        timings,
        passes,
        scratch: final_scratch,
    })
}

/// Rebuild the final-grid pre-erosion fields and flood domain without erosion.
pub(crate) fn prepare_surface(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    cancelled: &dyn Fn() -> bool,
) -> Result<crate::SurfaceContext, WorldError> {
    let grid =
        CubeSphere::new(settings.resolution, settings.radius_m).map_err(WorldError::Internal)?;
    let tect_grid =
        CubeSphere::new(tectonics.n, settings.radius_m).map_err(WorldError::Internal)?;
    let prepared = prepare_fields(&tect_grid, tectonics, &grid, settings.seed, cancelled)?;
    Ok(crate::SurfaceContext {
        grid,
        domain: prepared.domain,
        base_m: prepared.base,
        uplift_m_yr: prepared.uplift,
        k_rock: prepared.k_rock,
        plate: prepared.plate,
        crust: prepared.crust,
    })
}

fn water_in(surface: &SurfaceWater) -> WaterIn<'_> {
    WaterIn {
        weight: &surface.weight,
        runoff_m_yr: &surface.runoff_m_yr,
        evap_m_yr: &surface.evap_m_yr,
        ice_m_yr: &surface.ice_m_yr,
    }
}

fn climate_label(n: u32) -> &'static str {
    match n {
        64 => "level_64",
        256 => "level_256",
        _ => "level_other",
    }
}

fn climate_timing_name(n: u32) -> &'static str {
    match n {
        64 => "climate_64",
        256 => "climate_256",
        _ => "climate_other",
    }
}

struct Prepared {
    h: Vec<f32>,
    domain: Vec<u8>,
    base: Vec<f32>,
    uplift: Vec<f32>,
    k_rock: Vec<f32>,
    plate: Vec<u8>,
    crust: Vec<u8>,
}

impl Prepared {
    fn into_fields(self, grid: CubeSphere) -> LevelFields {
        LevelFields {
            grid,
            h: self.h,
            domain: self.domain,
            base: self.base,
            uplift: self.uplift,
            k_rock: self.k_rock,
            plate: self.plate,
            crust: self.crust,
        }
    }

    fn into_carry(self, grid: CubeSphere) -> (CubeSphere, Vec<f32>, Vec<u8>) {
        (grid, self.h, self.domain)
    }
}

fn prepare_fields(
    tect_grid: &CubeSphere,
    tectonics: &Tectonics,
    grid: &CubeSphere,
    seed: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<Prepared, WorldError> {
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    let mut base = resample_with_cancel(tect_grid, &tectonics.base_m, grid, cancelled)?;
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    let mut uplift = resample_with_cancel(tect_grid, &tectonics.uplift_m_yr, grid, cancelled)?;
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    let mut k_rock = resample_with_cancel(tect_grid, &tectonics.k_rock, grid, cancelled)?;
    if cancelled() {
        return Err(WorldError::Cancelled);
    }
    add_detail(
        grid,
        tectonics.n,
        grid.radius_m,
        seed,
        &mut base,
        &mut uplift,
        &mut k_rock,
        cancelled,
    )?;
    let domain: Vec<u8> = base.iter().map(|v| u8::from(*v > 0.0)).collect();
    let plate = sample_labels(tect_grid, &tectonics.plate, grid);
    let crust = sample_labels(tect_grid, &tectonics.crust, grid);
    Ok(Prepared {
        h: Vec::new(),
        domain,
        base,
        uplift,
        k_rock,
        plate,
        crust,
    })
}

/// Cells per detail chunk. Six chunks at `n = 1024`; each stays under the
/// 100 ms cancellation budget in release on a 12-thread machine.
const DETAIL_CHUNK: usize = 1_048_576;

#[allow(clippy::too_many_arguments)]
fn add_detail(
    grid: &CubeSphere,
    n_t: u32,
    radius: f64,
    seed: u64,
    base: &mut [f32],
    uplift: &mut [f32],
    k_rock: &mut [f32],
    cancelled: &dyn Fn() -> bool,
) -> Result<(), WorldError> {
    let uplift_seed = derive_seed(seed, SeedDomain::UpliftDetail);
    let lith_seed = derive_seed_salt(seed, SeedDomain::Lithology, 1);
    let (detail_freq, detail_octaves) = detail_octaves(grid.n, n_t);
    let (uplift_freq, uplift_octaves) = uplift_octaves(grid.n);
    let finer = grid.n > n_t;
    let n = base.len();
    let chunk = crate::scaled_chunk(DETAIL_CHUNK);
    for start in (0..n).step_by(chunk) {
        if cancelled() {
            return Err(WorldError::Cancelled);
        }
        let end = (start + chunk).min(n);
        let base_slice = &mut base[start..end];
        let uplift_slice = &mut uplift[start..end];
        let rock_slice = &mut k_rock[start..end];
        base_slice
            .par_iter_mut()
            .zip(uplift_slice.par_iter_mut())
            .zip(rock_slice.par_iter_mut())
            .enumerate()
            .for_each(|(offset, ((b, u), k))| {
                let cell = start + offset;
                let p = grid.position(cell as u32);
                if finer {
                    let mut acc = 0.0f64;
                    let mut scale = n_t;
                    let mut oct = 0u64;
                    loop {
                        let freq = f64::from(scale) / std::f64::consts::PI;
                        let lambda = radius / freq;
                        let amp = 400.0 * (lambda / 80_000.0).powf(0.9);
                        let sample_seed = derive_seed_salt(seed, SeedDomain::BaseDetail, oct);
                        acc += amp * noise_on_sphere(sample_seed, p, freq);
                        if scale >= grid.n || oct > 32 {
                            break;
                        }
                        let next = scale.saturating_mul(2);
                        scale = if next > grid.n { grid.n } else { next };
                        oct += 1;
                    }
                    let weight = (-f64::from(b.abs()) / 1500.0).exp();
                    *b = (f64::from(*b) + weight * acc) as f32;
                    let rock = fbm(lith_seed, p, detail_freq, detail_octaves, 2.0, 0.5);
                    *k = (f64::from(*k) * (0.25 * rock).exp()) as f32;
                }
                let detail = fbm_fixed_norm(uplift_seed, p, uplift_freq, uplift_octaves, 2.0, 0.5);
                let before = f64::from(*u);
                let mut next = before * (1.0 + 0.35 * detail);
                if before >= 0.0 {
                    next = next.max(0.0);
                }
                *u = next as f32;
            });
    }
    Ok(())
}

/// Octave band for base detail and the fine `k_rock` multiplier.
///
/// Above `n_t` the band is the base-detail set (`n_t`, `2 n_t`, …, `n`).
/// At and below `n_t` it doubles from 32 up to `n`. The tectonic grid already
/// holds the coarser octaves, so this band must not repeat them.
fn detail_octaves(n: u32, n_t: u32) -> (f64, u32) {
    let start = if n > n_t { n_t } else { 32.min(n) };
    let octaves = (n / start).ilog2() + 1;
    (f64::from(start) / std::f64::consts::PI, octaves)
}

/// Uplift-modulation band from resolution 32 up to this level.
///
/// The base frequency stays 32, so each level's field is the previous level's
/// field plus the new octave. [`fbm_fixed_norm`] keeps the coarse weights fixed.
fn uplift_octaves(n: u32) -> (f64, u32) {
    let start = 32.min(n);
    let octaves = (n / start).ilog2() + 1;
    (f64::from(start) / std::f64::consts::PI, octaves)
}

fn sample_labels(src: &CubeSphere, labels: &[u8], dst: &CubeSphere) -> Vec<u8> {
    if src.n == dst.n {
        return labels.to_vec();
    }
    let mut out = vec![0u8; dst.cell_count()];
    if dst.n > src.n {
        let scale = dst.n / src.n;
        out.par_iter_mut().enumerate().for_each(|(cell, slot)| {
            let (face, i, j) = dst.decode(cell as u32);
            *slot = labels[src.id(face, i / scale, j / scale) as usize];
        });
    } else {
        out.par_iter_mut().enumerate().for_each(|(cell, slot)| {
            *slot = labels[src.cell_at(dst.position(cell as u32)) as usize];
        });
    }
    out
}

fn initial_elevation(
    grid: &CubeSphere,
    prepared: &Prepared,
    parent: Option<&Carry>,
    seed: u64,
) -> Result<Vec<f32>, WorldError> {
    let n = grid.cell_count();
    let mut h = vec![0.0f32; n];
    let rough_seed = derive_seed_salt(seed, SeedDomain::Roughness, u64::from(grid.n));
    let freq = f64::from(grid.n) / std::f64::consts::PI;
    let cell_m = grid.cell_size_m();
    let mut amp = 60.0 * (cell_m / 10_000.0).powf(0.8);
    if grid.n == 32 {
        amp *= 3.0;
    }
    if let Some(parent) = parent {
        if parent.grid.n * 2 != grid.n {
            return Err(WorldError::Internal(
                "elevation upsample expects the parent at half resolution".to_string(),
            ));
        }
        h.par_iter_mut().enumerate().for_each(|(cell, slot)| {
            if prepared.domain[cell] == 0 {
                *slot = prepared.base[cell];
                return;
            }
            let (face, i, j) = grid.decode(cell as u32);
            let fx = (f64::from(i) + 0.5) / 2.0;
            let fy = (f64::from(j) + 0.5) / 2.0;
            let inherited = parent
                .grid
                .sample_face_masked(&parent.h, &parent.domain, face, fx, fy);
            let mut value = f64::from(inherited.unwrap_or(prepared.base[cell]));
            value += roughness(
                grid,
                rough_seed,
                freq,
                amp,
                cell as u32,
                prepared.uplift[cell],
            );
            *slot = value.max(0.5) as f32;
        });
    } else {
        h.par_iter_mut().enumerate().for_each(|(cell, slot)| {
            if prepared.domain[cell] == 0 {
                *slot = prepared.base[cell];
                return;
            }
            let mut value = f64::from(prepared.base[cell]);
            value += roughness(
                grid,
                rough_seed,
                freq,
                amp,
                cell as u32,
                prepared.uplift[cell],
            );
            *slot = value.max(0.5) as f32;
        });
    }
    Ok(h)
}

fn roughness(grid: &CubeSphere, seed: u64, freq: f64, amp: f64, cell: u32, uplift: f32) -> f64 {
    let noise = noise_on_sphere(seed, grid.position(cell), freq);
    let activity = (f64::from(uplift) / 2.0e-3).clamp(0.0, 1.0);
    amp * (0.15 + activity) * noise
}

/// Large per-iteration buffers, reused across levels.
pub(crate) struct ErosionBuffers {
    /// Working elevation over active slots.
    he: Vec<f32>,
    /// Spare buffer: pit copy, kernel work, glacier weights and smoothing.
    work: Vec<f32>,
    /// Lake mask.
    lake: Vec<u8>,
    /// Ice discharge.
    q_ice: Vec<f64>,
    /// Glaciated mask.
    glaciated: Vec<u8>,
    /// Compact domain mask.
    domain: Vec<u8>,
}

impl ErosionBuffers {
    pub(crate) fn new(slots: usize) -> Self {
        Self {
            he: vec![0.0f32; slots],
            work: vec![0.0f32; slots],
            lake: vec![0u8; slots],
            q_ice: vec![0.0f64; slots],
            glaciated: vec![0u8; slots],
            domain: vec![0u8; slots],
        }
    }

    fn prepare(&mut self, index: &DomainIndex, full_domain: &[u8], h: &[f32]) {
        let slots = index.len();
        self.he.resize(slots, 0.0);
        self.work.resize(slots, 0.0);
        self.lake.resize(slots, 0);
        self.q_ice.resize(slots, 0.0);
        self.glaciated.resize(slots, 0);
        self.domain.resize(slots, 0);
        for (slot, &cell) in index.cells().iter().enumerate() {
            self.domain[slot] = full_domain[cell as usize];
            self.he[slot] = h[cell as usize];
        }
    }
}

/// Borrowed inputs for [`erode`]. Split out so the function stays small.
pub(crate) struct ErodeJob<'a> {
    /// Grid the elevation lives on.
    pub grid: &'a CubeSphere,
    /// Slot layout for erosion. Full index keeps every cell.
    pub index: &'a DomainIndex,
    /// Elevation, updated in place on domain cells.
    pub h: &'a mut Vec<f32>,
    /// Reused large buffers.
    pub buffers: &'a mut ErosionBuffers,
    /// `1` for cells that erode.
    pub domain: &'a [u8],
    /// Uplift in metres per year.
    pub uplift: &'a [f32],
    /// Erodibility multiplier.
    pub k_rock: &'a [f32],
    /// Routing buffers reused across iterations.
    pub scratch: &'a mut RoutingScratch,
    /// Step count, time step and checks.
    pub options: &'a ErodeOptions,
    /// Runoff, evaporation and ice. `None` keeps weight 1 and every basin open.
    pub water: Option<WaterIn<'a>>,
}

/// Run the erosion loop in place. Tests call this directly.
pub(crate) fn erode(
    mut job: ErodeJob<'_>,
    progress: &mut dyn Progress,
) -> Result<ErodeReport, WorldError> {
    let index = job.index;
    let full_domain = job.domain;
    job.buffers.prepare(index, full_domain, job.h);
    job.scratch.set_domain(index, full_domain);
    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    let mut steps = Vec::new();
    let mut violations = 0u32;
    let mut pits_last = 0u32;
    let mut pits_first = 0u32;
    let mut edges_peak = 0u32;
    let mut carved_last = 0u32;
    let mut clamped_last = 0u32;
    let mut closed_last = 0u32;
    let mut glaciated_last = 0u32;
    let iterations = job.options.iterations;
    let record_steps = job.options.record_steps;
    let stage = job.options.stage;
    let water = job.water;
    if progress.cancelled() {
        return Err(WorldError::Cancelled);
    }
    for iter in 0..iterations {
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        let before = if record_steps {
            job.buffers.he.clone()
        } else {
            Vec::new()
        };
        fill_single_cell_pits_active(
            job.grid,
            index,
            &mut job.buffers.he,
            &job.buffers.domain,
            &mut job.buffers.work,
        );
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        let balance = water.map(|water| BasinBalance {
            runoff_m_yr: water.runoff_m_yr,
            evap_m_yr: water.evap_m_yr,
        });
        let weight = water.map(|water| water.weight);
        let routed = route_with_cancel(
            job.grid,
            &job.buffers.he,
            full_domain,
            index,
            weight,
            job.scratch,
            balance.as_ref(),
            &|| progress.cancelled(),
        )?;
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        if iter == 0 {
            pits_first = routed.pits;
        }
        pits_last = routed.pits;
        edges_peak = edges_peak.max(routed.edges);
        carved_last = routed.carved;
        closed_last = routed.closed_basins;
        if let Some(water) = water {
            glaciated_last = apply_ice(
                job.grid,
                index,
                &job.buffers.domain,
                job.scratch,
                water.ice_m_yr,
                water.weight,
                &mut IceBuf {
                    boosted: &mut job.buffers.work,
                    q_ice: &mut job.buffers.q_ice,
                    glaciated: &mut job.buffers.glaciated,
                },
            );
        }
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        violations = violations.saturating_add(stream_power(&mut job));
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        if water.is_some() {
            overdeepen(
                &mut job.buffers.he,
                &job.buffers.domain,
                &job.buffers.q_ice,
                job.options.dt_yr,
                job.scratch.parallel_basins(),
            );
        }
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        hillslopes(
            &job.buffers.domain,
            &mut job.buffers.he,
            &job.buffers.lake,
            job.scratch,
            job.options.params.s_c,
            &mut job.buffers.work,
        );
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        smooth(
            job.grid,
            index,
            &job.buffers.domain,
            &mut job.buffers.he,
            &mut job.buffers.work,
            job.options.params.kappa,
        );
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        clamped_last = clamp_elevation(
            &mut job.buffers.he,
            &job.buffers.domain,
            job.options.params.max_elevation_m,
            job.scratch.parallel_basins(),
        );
        if progress.cancelled() {
            return Err(WorldError::Cancelled);
        }
        if record_steps {
            let mut max_step = 0.0f32;
            for (old, new) in before.iter().zip(job.buffers.he.iter()) {
                max_step = max_step.max((old - new).abs());
            }
            steps.push(max_step);
        }
        if let Some(stage) = stage {
            progress.report(stage, iter + 1, iterations);
        }
    }
    // Only domain cells changed; ring roots keep their original elevation.
    for (slot, &cell) in index.cells().iter().enumerate() {
        if job.buffers.domain[slot] == 1 {
            job.h[cell as usize] = job.buffers.he[slot];
        }
    }
    let (max_domain_m, mean_domain_m) = domain_stats(job.grid, job.h, job.domain);
    Ok(ErodeReport {
        pits_last,
        pits_first,
        edges_peak,
        carved_last,
        clamped_last,
        closed_last,
        glaciated_last,
        steps,
        violations,
        max_domain_m,
        mean_domain_m,
        #[cfg(test)]
        q_ice: job.buffers.q_ice.clone(),
    })
}

const GLACIER_WEIGHT: f32 = 2.0;
const GLACIER_K: f64 = 3.0;
const FJORD_BASE_M: f64 = -300.0;
const OVERDEEP_CAP_M: f64 = 1.0e-3;
const OVERDEEP_SCALE: f64 = 2.0e-4;
const OVERDEEP_Q_M3: f64 = 1.0e9;
const OVERDEEP_FLOOR_M: f64 = -500.0;

struct IceBuf<'a> {
    boosted: &'a mut [f32],
    q_ice: &'a mut [f64],
    glaciated: &'a mut [u8],
}

struct IceKernel<'a> {
    grid: &'a CubeSphere,
    index: &'a DomainIndex,
    stack: &'a [u32],
    donor_offset: &'a [u32],
    donors: &'a [u32],
    ice_m_yr: &'a [f32],
}

fn ice_kernel<const COMPUTE_BASE: bool, S: SlotIndex>(
    kernel: &IceKernel<'_>,
    range: Range<usize>,
    slots: S,
    values: &mut [f64],
) -> u32 {
    let stack = kernel.stack;
    let donor_offset = kernel.donor_offset;
    let donors = kernel.donors;
    let ice_m_yr = kernel.ice_m_yr;
    let grid = kernel.grid;
    let index = kernel.index;
    for idx in range.rev() {
        let slot = stack[idx] as usize;
        let value_slot = slots.current(idx, slot);
        let s = donor_offset[slot] as usize;
        let e = donor_offset[slot + 1] as usize;
        let mut sum = if COMPUTE_BASE {
            f64::from(ice_m_yr.get(slot).copied().unwrap_or(0.0)) * grid.area_m2(index.cell(slot))
        } else {
            values[value_slot]
        };
        for &donor in &donors[s..e] {
            sum += values[slots.at(donor as usize)].max(0.0);
        }
        values[value_slot] = sum;
    }
    0
}

/// Ice discharge on the current receivers, then glacier weights.
///
/// `q_ice = balance·area + Σ donors max(q_ice, 0)`, leaves first.
fn apply_ice(
    grid: &CubeSphere,
    index: &DomainIndex,
    domain: &[u8],
    scratch: &mut RoutingScratch,
    ice_m_yr: &[f32],
    base_weight: &[f32],
    buf: &mut IceBuf<'_>,
) -> u32 {
    debug_assert_eq!(buf.boosted.len(), buf.glaciated.len());
    let parallel = scratch.parallel_basins();
    let RoutingScratch {
        stack_f64,
        stack,
        pos,
        chunks,
        donor_offset,
        donors,
        ..
    } = scratch;
    let view = BasinView { stack, pos, chunks };
    let kernel = IceKernel {
        grid,
        index,
        stack,
        donor_offset,
        donors,
        ice_m_yr,
    };
    run_basin_kernel_gather(
        buf.q_ice,
        stack_f64,
        view,
        parallel,
        |slot, _| {
            f64::from(ice_m_yr.get(slot).copied().unwrap_or(0.0)) * grid.area_m2(index.cell(slot))
        },
        |range, values| ice_kernel::<true, _>(&kernel, range, CellIndex, values),
        |range, slots, values| ice_kernel::<false, _>(&kernel, range, slots, values),
    );
    let IceBuf {
        boosted,
        q_ice,
        glaciated,
    } = buf;
    let (count, any) = fold_cells_zip_mut(
        boosted,
        glaciated,
        parallel,
        (0u32, false),
        |slot, boosted_slot, glaciated_slot| {
            let ice = domain[slot] == 1 && q_ice[slot] > 0.0;
            *glaciated_slot = u8::from(ice);
            let mut weight = base_weight[slot];
            let mut counted = 0u32;
            let mut raised = false;
            if ice {
                counted = 1;
                if weight < GLACIER_WEIGHT {
                    weight = GLACIER_WEIGHT;
                    raised = true;
                }
            }
            *boosted_slot = weight;
            (counted, raised)
        },
        |a, b| (a.0 + b.0, a.1 || b.1),
    );
    if any {
        accumulate_with_weight(grid, index, scratch, boosted);
    }
    count
}

fn overdeepen(h: &mut [f32], domain: &[u8], q_ice: &[f64], dt_yr: f64, parallel: bool) {
    fold_cells_mut(
        h,
        parallel,
        (),
        |i, height| {
            if domain.get(i).copied().unwrap_or(0) == 0 {
                return;
            }
            let q = q_ice.get(i).copied().unwrap_or(0.0);
            if q <= 0.0 {
                return;
            }
            let rate = (OVERDEEP_SCALE * (q / OVERDEEP_Q_M3).sqrt()).min(OVERDEEP_CAP_M);
            let lowered = f64::from(*height) - rate * dt_yr;
            if lowered >= OVERDEEP_FLOOR_M {
                *height = lowered as f32;
            } else if f64::from(*height) > OVERDEEP_FLOOR_M {
                *height = OVERDEEP_FLOOR_M as f32;
            }
        },
        |(), ()| (),
    );
}

/// Raise each strict single-cell pit to 1 cm above its lowest neighbour.
///
/// A cell strictly below every neighbour cannot sit next to another such cell,
/// so the reads are safe in parallel. The write goes to a copy, then replaces
/// `h`. Non-domain cells are left alone.
pub(crate) fn fill_single_cell_pits(grid: &CubeSphere, h: &mut [f32], domain: &[u8]) {
    let mut next = vec![0.0f32; h.len()];
    fill_single_cell_pits_into(grid, h, domain, &mut next);
    h.copy_from_slice(&next);
}

fn fill_single_cell_pits_into(grid: &CubeSphere, h: &[f32], domain: &[u8], next: &mut [f32]) {
    next.par_iter_mut().enumerate().for_each(|(i, slot)| {
        let here = h[i];
        *slot = here;
        if domain.get(i).copied().unwrap_or(0) != 1 {
            return;
        }
        let mut min_nb = f32::MAX;
        let mut count = 0u32;
        for nb in grid.neighbors(i as u32) {
            if nb == crate::grid::NONE {
                continue;
            }
            let height = h[nb as usize];
            if height <= here {
                return;
            }
            count += 1;
            min_nb = min_nb.min(height);
        }
        if count > 0 {
            *slot = min_nb + 0.01;
        }
    });
}

/// Compact version of [`fill_single_cell_pits_into`] over the active slots.
fn fill_single_cell_pits_active(
    grid: &CubeSphere,
    index: &DomainIndex,
    h: &mut Vec<f32>,
    domain: &[u8],
    next: &mut Vec<f32>,
) {
    next.par_iter_mut().enumerate().for_each(|(slot, out)| {
        let here = h[slot];
        *out = here;
        if domain[slot] != 1 {
            return;
        }
        let cell = index.cell(slot);
        let mut min_nb = f32::MAX;
        let mut count = 0u32;
        for nb in grid.neighbors(cell) {
            if nb == crate::grid::NONE {
                continue;
            }
            let nb_slot = index.slot(nb as usize);
            debug_assert!(index.is_active(nb as usize));
            let height = h[nb_slot];
            if height <= here {
                return;
            }
            count += 1;
            min_nb = min_nb.min(height);
        }
        if count > 0 {
            *out = min_nb + 0.01;
        }
    });
    std::mem::swap(h, next);
}

fn stream_power(job: &mut ErodeJob<'_>) -> u32 {
    let m_exp = job.options.params.m;
    let k_coeff = job.options.params.k;
    let dt = job.options.dt_yr;
    let check_bounds = job.options.check_bounds;
    let parallel = job.scratch.parallel_basins();
    {
        let levels = job.scratch.lake_level.as_slice();
        let buffers = &mut job.buffers;
        fold_cells_mut(
            &mut buffers.lake,
            parallel,
            (),
            |slot, out| {
                *out = u8::from(buffers.domain[slot] == 1 && buffers.he[slot] < levels[slot]);
            },
            |(), ()| (),
        );
    }
    let RoutingScratch {
        stack,
        pos,
        chunks,
        receiver,
        receiver_dist,
        lake_level,
        area,
        ..
    } = job.scratch;
    let buffers = &mut job.buffers;
    let kernel = StreamKernel {
        index: job.index,
        stack,
        receiver,
        receiver_dist,
        lake_level,
        domain: &buffers.domain,
        lake: &buffers.lake,
        glaciated: &buffers.glaciated,
        uplift: job.uplift,
        k_rock: job.k_rock,
        area,
        m_exp,
        k_coeff,
        dt,
        check_bounds,
    };
    run_basin_kernel_keep(
        &mut buffers.he,
        &mut buffers.work,
        BasinView { stack, pos, chunks },
        parallel,
        |range, values| stream_kernel(&kernel, range, CellIndex, values),
        |range, slots, values| stream_kernel(&kernel, range, slots, values),
    )
}

struct StreamKernel<'a> {
    index: &'a DomainIndex,
    stack: &'a [u32],
    receiver: &'a [u32],
    receiver_dist: &'a [f32],
    lake_level: &'a [f32],
    domain: &'a [u8],
    lake: &'a [u8],
    glaciated: &'a [u8],
    uplift: &'a [f32],
    k_rock: &'a [f32],
    area: &'a [f64],
    m_exp: f64,
    k_coeff: f64,
    dt: f64,
    check_bounds: bool,
}

fn stream_kernel<S: SlotIndex>(
    kernel: &StreamKernel<'_>,
    range: Range<usize>,
    slots: S,
    values: &mut [f32],
) -> u32 {
    let stack = kernel.stack;
    let domain = kernel.domain;
    let receiver = kernel.receiver;
    let lake = kernel.lake;
    let mut violations = 0u32;
    for idx in range {
        let slot_cell = stack[idx] as usize;
        if domain[slot_cell] == 0 {
            continue;
        }
        let cell = kernel.index.cell(slot_cell);
        let value_slot = slots.current(idx, slot_cell);
        let r = receiver[slot_cell] as usize;
        let old = f64::from(values[value_slot]);
        let u_raw = f64::from(kernel.uplift[cell as usize]);
        let u_dt = subsidence_delta(u_raw, old, kernel.dt);
        if r == slot_cell || lake[slot_cell] == 1 {
            values[value_slot] = hold_subsidence_floor(u_raw, old + u_dt) as f32;
            continue;
        }
        let mut hr = f64::from(values[slots.at(r)]);
        if lake[r] == 1 {
            let level = f64::from(kernel.lake_level[r]);
            if level.is_finite() {
                hr = hr.max(level);
            }
        }
        // A rift floor near −1000 m must not become the base level for the
        // surrounding landscape. Cells with non-negative uplift grade to sea
        // level (0 in this pre-shift frame) when their receiver lies below it.
        // A glacier draining off the domain grades to −300 m instead.
        let fjord = kernel.glaciated.get(slot_cell).copied().unwrap_or(0) == 1 && domain[r] == 0;
        if domain[r] == 0 {
            hr = if fjord { FJORD_BASE_M } else { hr.max(0.0) };
        }
        if u_raw >= 0.0 && !fjord {
            hr = hr.max(0.0);
        }
        let dist = f64::from(kernel.receiver_dist[slot_cell]).max(1.0e-6);
        let k_scale = if kernel.glaciated.get(slot_cell).copied().unwrap_or(0) == 1 {
            GLACIER_K
        } else {
            1.0
        };
        let am = kernel.area[slot_cell].max(0.0).powf(kernel.m_exp) as f32;
        let f = kernel.k_coeff
            * k_scale
            * f64::from(kernel.k_rock[cell as usize])
            * kernel.dt
            * f64::from(am)
            / dist;
        let new_h = hold_subsidence_floor(u_raw, (old + u_dt + f * hr) / (1.0 + f));
        if kernel.check_bounds && u_dt.abs() <= 1.0e-15 && old + 1.0e-3 >= hr {
            let raised = new_h > old + 1.0e-2;
            let below = new_h < hr - 1.0e-2;
            if raised || below {
                violations = violations.saturating_add(1);
            }
        }
        values[value_slot] = new_h as f32;
    }
    violations
}

/// Rift floors stop subsiding as they approach −1000 m. One explicit step can
/// still cross that floor, so the result is held there.
const SUBSIDENCE_FLOOR_M: f64 = -1000.0;

fn subsidence_delta(uplift_m_yr: f64, height_m: f64, dt_yr: f64) -> f64 {
    if uplift_m_yr >= 0.0 {
        return uplift_m_yr * dt_yr;
    }
    let t = ((height_m - SUBSIDENCE_FLOOR_M) / 500.0).clamp(0.0, 1.0);
    let fade = t * t * (3.0 - 2.0 * t);
    uplift_m_yr * fade * dt_yr
}

fn hold_subsidence_floor(uplift_m_yr: f64, height_m: f64) -> f64 {
    if uplift_m_yr < 0.0 && height_m < SUBSIDENCE_FLOOR_M {
        SUBSIDENCE_FLOOR_M
    } else {
        height_m
    }
}

fn hillslopes(
    domain: &[u8],
    h: &mut [f32],
    lake: &[u8],
    scratch: &RoutingScratch,
    s_c: f64,
    work: &mut [f32],
) {
    let parallel = scratch.parallel_basins();
    let stack = scratch.stack.as_slice();
    let pos = scratch.pos.as_slice();
    let chunks = scratch.chunks.as_slice();
    let receiver = scratch.receiver.as_slice();
    let distance = scratch.receiver_dist.as_slice();
    let kernel = HillslopeKernel {
        stack,
        receiver,
        distance,
        domain,
        lake,
        s_c,
    };
    run_basin_kernel_keep(
        h,
        work,
        BasinView { stack, pos, chunks },
        parallel,
        |range, values| hillslope_kernel(&kernel, range, CellIndex, values),
        |range, slots, values| hillslope_kernel(&kernel, range, slots, values),
    );
}

struct HillslopeKernel<'a> {
    stack: &'a [u32],
    receiver: &'a [u32],
    distance: &'a [f32],
    domain: &'a [u8],
    lake: &'a [u8],
    s_c: f64,
}

fn hillslope_kernel<S: SlotIndex>(
    kernel: &HillslopeKernel<'_>,
    range: Range<usize>,
    slots: S,
    values: &mut [f32],
) -> u32 {
    let stack = kernel.stack;
    let domain = kernel.domain;
    let lake = kernel.lake;
    let receiver = kernel.receiver;
    let distance = kernel.distance;
    for idx in range {
        let slot_cell = stack[idx] as usize;
        if domain[slot_cell] == 0 || lake[slot_cell] == 1 {
            continue;
        }
        let r = receiver[slot_cell] as usize;
        if r == slot_cell {
            continue;
        }
        let value_slot = slots.current(idx, slot_cell);
        let dist = f64::from(distance[slot_cell]).max(0.0);
        let cap = f64::from(values[slots.at(r)]) + kernel.s_c * dist;
        if f64::from(values[value_slot]) > cap {
            values[value_slot] = cap as f32;
        }
    }
    0
}

fn smooth(
    grid: &CubeSphere,
    index: &DomainIndex,
    domain: &[u8],
    h: &mut Vec<f32>,
    aux: &mut Vec<f32>,
    kappa: f64,
) {
    let axis = crate::grid::axis_neighbor_slots();
    {
        let current = h.as_slice();
        aux.par_iter_mut().enumerate().for_each(|(slot, dest)| {
            if domain[slot] == 0 {
                *dest = current[slot];
                return;
            }
            let cell = index.cell(slot);
            let nbs = grid.neighbors(cell);
            let mut sum = 0.0f64;
            let mut count = 0.0f64;
            for slot_k in axis {
                let nb = nbs[slot_k];
                if nb == crate::grid::NONE {
                    continue;
                }
                let nb_slot = index.slot(nb as usize);
                let mut sample = f64::from(current[nb_slot]);
                if domain[nb_slot] == 0 {
                    sample = sample.max(0.0);
                }
                sum += sample;
                count += 1.0;
            }
            let mean = if count == 0.0 {
                f64::from(current[slot])
            } else {
                sum / count
            };
            *dest = (f64::from(current[slot]) + kappa * (mean - f64::from(current[slot]))) as f32;
        });
    }
    std::mem::swap(h, aux);
}

fn clamp_elevation(h: &mut [f32], domain: &[u8], max_elevation: f64, parallel: bool) -> u32 {
    let cap = max_elevation as f32;
    fold_cells_mut(
        h,
        parallel,
        0u32,
        |i, value| {
            if domain[i] == 1 && *value > cap {
                *value = cap;
                1
            } else {
                0
            }
        },
        |a, b| a + b,
    )
}

fn domain_stats(grid: &CubeSphere, h: &[f32], domain: &[u8]) -> (f32, f32) {
    let mut max_h = 0.0f32;
    let mut any = false;
    let mut weighted = 0.0f64;
    let mut area = 0.0f64;
    for (i, &value) in h.iter().enumerate() {
        if domain[i] == 0 {
            continue;
        }
        if !any || value > max_h {
            max_h = value;
        }
        any = true;
        let cell_area = grid.area_m2(i as u32);
        weighted += cell_area * f64::from(value);
        area += cell_area;
    }
    let mean = if area == 0.0 { 0.0 } else { weighted / area };
    (max_h, mean as f32)
}

fn level_resolutions(final_n: u32) -> Vec<u32> {
    let mut n = 32u32;
    let mut out = Vec::new();
    while n <= final_n {
        out.push(n);
        if n > final_n / 2 {
            break;
        }
        n *= 2;
    }
    out
}

pub(crate) fn level_timing_name(n: u32) -> &'static str {
    match n {
        32 => "level_32",
        64 => "level_64",
        128 => "level_128",
        256 => "level_256",
        512 => "level_512",
        1024 => "level_1024",
        2048 => "level_2048",
        _ => "level_other",
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}
