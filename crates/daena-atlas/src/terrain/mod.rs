//! Deterministic terrain detail on a canonical [`ToolSurface`](daena_physical::engine::ToolSurface).
//!
//! Elevations stay on the reference datum. Detail is hash-addressed gradient
//! noise in the `"terrain"` domain and fades with the scale-1 ground pixel.

mod noise;
mod render;

use std::sync::Arc;

use daena_physical::engine::{decode_source, ToolSurface, TOOL_SURFACE_NONE};
use daena_physical::PhysicalError;

pub use render::{ground_metres_per_pixel, lon_lat_to_tile, render_pixels};

use noise::{build_octaves, domain_seed, sample_octave, terrain_seed, Octave, RIDGED_NOISE_MEAN};

/// Pixel cover. Ice is a paint tint, not a cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainCover {
    Ocean,
    Lake,
    Land,
}

/// Canonical elevations, relief, and the terrain-domain octaves for one surface.
pub struct AtlasTerrain {
    surface: Arc<ToolSurface>,
    elevation_m: Vec<f32>,
    relief_m: Vec<f32>,
    /// Detail amplitude per canonical cell, interpolated like elevation.
    amplitude_m: Vec<f32>,
    ocean: Vec<u8>,
    ice: Vec<u8>,
    wet: Vec<u8>,
    lake_id: Vec<u32>,
    lake_level_m: Vec<f32>,
    cell_size_m: f64,
    radius_m: f64,
    sea_m: f64,
    sea_level_mm: i32,
    seed: u64,
    octaves: Vec<Octave>,
    /// Two octaves in the `"ice"` domain that roughen the ice edge.
    ice_octaves: Vec<Octave>,
    half: Vec<f64>,
}

impl AtlasTerrain {
    /// Snapshot `surface` and seed the terrain domain from `identity` bytes.
    #[must_use]
    pub fn new(surface: Arc<ToolSurface>, identity: &[u8]) -> Self {
        let count = surface.cell_count();
        let mut elevation_m = Vec::with_capacity(count);
        let mut relief_m = Vec::with_capacity(count);
        let mut ocean = Vec::with_capacity(count);
        let mut ice = Vec::with_capacity(count);
        let mut wet = Vec::with_capacity(count);
        let mut lake_id = Vec::with_capacity(count);
        let mut lake_level_m = Vec::with_capacity(count);
        for cell in 0..count as u32 {
            let mm = surface.elevation_mm[cell as usize];
            elevation_m.push(mm as f32 / 1000.0);
            let mut max_delta = 0.0f64;
            for neighbor in surface.neighbors(cell) {
                if neighbor == TOOL_SURFACE_NONE {
                    continue;
                }
                let delta = (i64::from(mm) - i64::from(surface.elevation_mm[neighbor as usize]))
                    .unsigned_abs() as f64;
                max_delta = max_delta.max(delta);
            }
            relief_m.push((max_delta / 1000.0) as f32);
            ocean.push(u8::from(surface.ocean(cell)));
            ice.push(u8::from(surface.ice(cell)));
            let is_wet = surface.wet_lake(cell);
            wet.push(u8::from(is_wet));
            if is_wet {
                let id = surface.lake_id(cell);
                let level_mm = surface.lake(id).map(|lake| lake.level_mm).unwrap_or(mm);
                lake_id.push(id);
                lake_level_m.push(level_mm as f32 / 1000.0);
            } else {
                lake_id.push(TOOL_SURFACE_NONE);
                lake_level_m.push(0.0);
            }
        }
        let sea_m = f64::from(surface.sea_level_mm) / 1000.0;
        let amplitude_m = (0..count)
            .map(|cell| {
                cell_amplitude(
                    f64::from(elevation_m[cell]),
                    f64::from(relief_m[cell]),
                    ocean[cell] != 0,
                    sea_m,
                ) as f32
            })
            .collect();
        let cell_size_m = surface.cell_size_m();
        let octave_count = octave_count_for(cell_size_m);
        let seed = terrain_seed(identity);
        let octaves = build_octaves(domain_seed(seed, "terrain"), octave_count);
        let ice_octaves = build_octaves(domain_seed(seed, "ice"), 2);
        let half = (0..octave_count).map(|k| 0.5f64.powi(k as i32)).collect();
        Self {
            sea_m,
            sea_level_mm: surface.sea_level_mm,
            radius_m: surface.radius_metres(),
            surface,
            elevation_m,
            relief_m,
            amplitude_m,
            ocean,
            ice,
            wet,
            lake_id,
            lake_level_m,
            cell_size_m,
            seed,
            octaves,
            ice_octaves,
            half,
        }
    }

    /// Decode `bytes`, derive the present or one epoch, and build the terrain.
    ///
    /// Identity is [`crate::spike_identity_from_source`]: daena-atlas has no
    /// daena-core manifest identity. `None`, `Some(0)`, and a solved epoch of
    /// zero use the present surface.
    pub fn from_source(bytes: &[u8], offset_years: Option<i64>) -> Result<Self, PhysicalError> {
        let decoded = decode_source(bytes)?;
        let identity = crate::spike_identity_from_source(bytes);
        let mut progress = daena_physical::NoopProgress;
        let surface = match offset_years {
            None => decoded.derive_tool_surface_present(&mut progress)?,
            Some(offset) => {
                let solve = daena_physical::history::solve_epoch(
                    &decoded,
                    daena_physical::history::HistoricalForcingParameters::default_for_seed(
                        decoded.seed(),
                    ),
                    offset,
                    &daena_physical::NoopProgress,
                )?;
                if solve.normalized_epoch == 0 {
                    decoded.derive_tool_surface_present(&mut progress)?
                } else {
                    decoded.derive_tool_surface_epoch(
                        solve.sea_level_mm,
                        f64::from(solve.temperature_offset_centi_c) / 100.0,
                        &mut progress,
                    )?
                }
            }
        };
        Ok(Self::new(Arc::new(surface), &identity))
    }

    #[must_use]
    pub fn surface(&self) -> &ToolSurface {
        &self.surface
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub fn elevation_m(&self) -> &[f32] {
        &self.elevation_m
    }

    #[must_use]
    pub fn relief_m(&self) -> &[f32] {
        &self.relief_m
    }

    #[must_use]
    pub fn sea_level_m(&self) -> f64 {
        self.sea_m
    }

    #[must_use]
    pub fn sea_level_mm(&self) -> i32 {
        self.sea_level_mm
    }

    #[must_use]
    pub fn cell_size_m(&self) -> f64 {
        self.cell_size_m
    }

    #[must_use]
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// Detailed elevation in metres at unit direction `p`.
    ///
    /// `ground_m` is the scale-1 ground pixel. Device scale does not enter.
    #[must_use]
    pub fn detailed_elevation(&self, direction: [f64; 3], ground_m: f64) -> f64 {
        self.sample_pixel(direction, ground_m).0
    }

    /// Elevation, water class, and ice tint for one direction.
    ///
    /// Bicubic, bilinear, and the class pull share one pair of stencils.
    #[must_use]
    pub fn sample_pixel(&self, direction: [f64; 3], ground_m: f64) -> (f64, TerrainCover, bool) {
        let p = unit(direction);
        let (bicubic_cells, bicubic_weights) = self.surface.bicubic_stencil(p);
        let mut canon = 0.0;
        for slot in 0..16 {
            canon +=
                bicubic_weights[slot] * f64::from(self.elevation_m[bicubic_cells[slot] as usize]);
        }
        let (cells, weights) = self.surface.bilinear_stencil(p);
        let mut relief = 0.0;
        for slot in 0..4 {
            relief += weights[slot] * f64::from(self.relief_m[cells[slot] as usize]);
        }
        let amplitude = self.amplitude_from(bicubic_cells, bicubic_weights);
        let mut height = canon + amplitude * self.detail(p, ground_m, relief, cells, weights);
        self.apply_class_pull(&mut height, p, cells, weights);
        let (cover, ice) = self.classify_weights(p, height, cells, weights);
        (height, cover, ice)
    }

    /// Canonical (no detail) elevation in metres.
    #[must_use]
    pub fn canonical_elevation(&self, direction: [f64; 3]) -> f64 {
        self.canon_and_relief(unit(direction)).0
    }

    /// Water class at `height`, plus whether the pixel paints as ice.
    #[must_use]
    pub fn classify(&self, direction: [f64; 3], height: f64) -> (TerrainCover, bool) {
        let p = unit(direction);
        let (cells, weights) = self.surface.bilinear_stencil(p);
        self.classify_weights(p, height, cells, weights)
    }

    fn classify_weights(
        &self,
        p: [f64; 3],
        height: f64,
        cells: [u32; 4],
        weights: [f64; 4],
    ) -> (TerrainCover, bool) {
        let nearest = self.surface.cell_at_direction(p) as usize;
        let nearest_lake = if self.wet[nearest] != 0 {
            Some(self.lake_id[nearest])
        } else {
            None
        };
        let mut ice_weight = 0.0;
        let mut ocean = false;
        let mut lakes = [(0u8, 0u32, 0.0f64); 4];
        let mut lake_count = 0usize;
        for slot in 0..4 {
            let weight = weights[slot];
            if weight <= WEIGHT_EPS {
                continue;
            }
            let cell = cells[slot] as usize;
            if self.ice[cell] != 0 {
                ice_weight += weight;
            }
            if height < self.sea_m && self.ocean[cell] != 0 {
                ocean = true;
            }
            if self.wet[cell] != 0 {
                let id = self.lake_id[cell];
                if (0..lake_count).all(|index| lakes[index].1 != id) {
                    let priority = u8::from(Some(id) != nearest_lake);
                    lakes[lake_count] = (priority, id, f64::from(self.lake_level_m[cell]));
                    lake_count += 1;
                }
            }
        }
        for index in 0..lake_count {
            let mut best = index;
            for other in index + 1..lake_count {
                if (lakes[other].0, lakes[other].1) < (lakes[best].0, lakes[best].1) {
                    best = other;
                }
            }
            lakes.swap(index, best);
        }
        let lake = (0..lake_count).any(|index| height < lakes[index].2);
        let cover = if ocean {
            TerrainCover::Ocean
        } else if lake {
            TerrainCover::Lake
        } else {
            TerrainCover::Land
        };
        (cover, ice_weight >= self.ice_threshold(p))
    }

    /// Ice paints where the bilinear ice weight passes this threshold. Noise
    /// keeps it within `0.5 ± 0.3`, so a cell centre (weight 0 or 1) keeps its
    /// canonical ice, and the edge stops tracing the cell grid.
    fn ice_threshold(&self, p: [f64; 3]) -> f64 {
        let coarse = self.radius_m / (0.5 * self.cell_size_m);
        let fine = self.radius_m / (0.125 * self.cell_size_m);
        let noise = 0.65 * sample_octave(&self.ice_octaves[0], p, coarse)
            + 0.35 * sample_octave(&self.ice_octaves[1], p, fine);
        0.5 + 0.3 * noise
    }

    /// Octaves whose fade is still positive at `ground_m`.
    #[must_use]
    pub fn octave_count(&self, ground_m: f64) -> usize {
        let mut lambda = self.cell_size_m;
        let mut count = 0usize;
        for _ in 0..self.octaves.len() {
            if octave_fade(lambda, ground_m) == 0.0 {
                break;
            }
            count += 1;
            lambda *= 0.5;
        }
        count
    }

    /// Land cell of highest relief within graph distance 2 of an ocean cell.
    /// Ties take the lowest cell id.
    #[must_use]
    pub fn auto_site_cell(&self) -> u32 {
        let count = self.elevation_m.len();
        let mut distance = vec![u8::MAX; count];
        let mut queue = std::collections::VecDeque::new();
        for cell in 0..count {
            if self.ocean[cell] != 0 {
                distance[cell] = 0;
                queue.push_back(cell as u32);
            }
        }
        while let Some(cell) = queue.pop_front() {
            let next = distance[cell as usize];
            if next >= 2 {
                continue;
            }
            for neighbor in self.surface.neighbors(cell) {
                if neighbor == TOOL_SURFACE_NONE {
                    continue;
                }
                if distance[neighbor as usize] == u8::MAX {
                    distance[neighbor as usize] = next + 1;
                    queue.push_back(neighbor);
                }
            }
        }
        let mut best: Option<(u32, f32)> = None;
        for cell in 0..count as u32 {
            let dist = distance[cell as usize];
            if !(1..=2).contains(&dist)
                || self.ocean[cell as usize] != 0
                || self.wet[cell as usize] != 0
            {
                continue;
            }
            let relief = self.relief_m[cell as usize];
            let replace = match best {
                None => true,
                Some((id, best_relief)) => {
                    relief > best_relief || (relief == best_relief && cell < id)
                }
            };
            if replace {
                best = Some((cell, relief));
            }
        }
        best.map(|(cell, _)| cell).unwrap_or(0)
    }

    fn canon_and_relief(&self, p: [f64; 3]) -> (f64, f64) {
        let (cells, weights) = self.surface.bicubic_stencil(p);
        let mut canon = 0.0;
        for slot in 0..16 {
            canon += weights[slot] * f64::from(self.elevation_m[cells[slot] as usize]);
        }
        let (cells, weights) = self.surface.bilinear_stencil(p);
        let mut relief = 0.0;
        for slot in 0..4 {
            relief += weights[slot] * f64::from(self.relief_m[cells[slot] as usize]);
        }
        (canon, relief)
    }

    /// Detail amplitude from the per-cell field on the bicubic stencil.
    ///
    /// The field is smooth in space. An amplitude switched on the interpolated
    /// elevation crossing sea level jumps within a pixel or two on a cliff
    /// coast, and the hillshade draws that step as a line offshore.
    fn amplitude_from(&self, cells: [u32; 16], weights: [f64; 16]) -> f64 {
        let mut amplitude = 0.0;
        for slot in 0..16 {
            amplitude += weights[slot] * f64::from(self.amplitude_m[cells[slot] as usize]);
        }
        amplitude.max(MIN_AMPLITUDE_M)
    }

    #[cfg(test)]
    fn amplitude_at(&self, p: [f64; 3]) -> f64 {
        let (cells, weights) = self.surface.bicubic_stencil(p);
        self.amplitude_from(cells, weights)
    }

    fn detail(
        &self,
        p: [f64; 3],
        ground_m: f64,
        relief: f64,
        cells: [u32; 4],
        weights: [f64; 4],
    ) -> f64 {
        let mountains = smoothstep(400.0, 1500.0, relief);
        let mut sum = 0.0;
        let mut lambda = self.cell_size_m;
        for (index, octave) in self.octaves.iter().enumerate() {
            let fade = octave_fade(lambda, ground_m);
            if fade == 0.0 {
                break;
            }
            let frequency = self.radius_m / lambda;
            let mut mixed = mix_noise(sample_octave(octave, p, frequency), mountains);
            // Wavelengths near the cell size are almost constant across a cell.
            // Subtract the bilinear of that octave at the stencil centres so the
            // cell mean stays on the canonical elevation.
            if lambda * 4.0 >= self.cell_size_m {
                let mut low = 0.0;
                for slot in 0..4 {
                    if weights[slot] == 0.0 {
                        continue;
                    }
                    let centre = self.surface.direction_of(cells[slot]);
                    low += weights[slot]
                        * mix_noise(sample_octave(octave, centre, frequency), mountains);
                }
                mixed -= low;
            }
            sum += fade * self.half[index] * mixed;
            lambda *= 0.5;
        }
        sum
    }

    /// Near a canonical cell centre, keep the detailed height on that cell's
    /// side of the water line. The clamp is a smooth min or max, so it adds no
    /// crease for the hillshade to draw. A land cell is only lifted where a
    /// water parent could otherwise claim the pixel, so a dry basin keeps its
    /// detail.
    fn apply_class_pull(
        &self,
        height: &mut f64,
        p: [f64; 3],
        stencil_cells: [u32; 4],
        stencil_weights: [f64; 4],
    ) {
        let cell = self.surface.cell_at_direction(p);
        let centre = self.surface.direction_of(cell);
        let dot = (p[0] * centre[0] + p[1] * centre[1] + p[2] * centre[2]).clamp(-1.0, 1.0);
        let cells = dot.acos() * self.radius_m / self.cell_size_m;
        let blend = smoothstep(0.35, 0.15, cells);
        if blend == 0.0 {
            return;
        }
        let index = cell as usize;
        let target = if self.ocean[index] != 0 {
            smooth_min(*height, self.sea_m - CLASS_MARGIN_M, CLASS_SOFTNESS_M)
        } else if self.wet[index] != 0 {
            let level = f64::from(self.lake_level_m[index]);
            smooth_min(*height, level - CLASS_MARGIN_M, CLASS_SOFTNESS_M)
        } else {
            let mut floor = f64::NEG_INFINITY;
            for slot in 0..4 {
                if stencil_weights[slot] <= WEIGHT_EPS {
                    continue;
                }
                let parent = stencil_cells[slot] as usize;
                if self.ocean[parent] != 0 {
                    floor = floor.max(self.sea_m);
                } else if self.wet[parent] != 0 {
                    floor = floor.max(f64::from(self.lake_level_m[parent]));
                }
            }
            if floor == f64::NEG_INFINITY {
                return;
            }
            smooth_max(*height, floor + CLASS_MARGIN_M, CLASS_SOFTNESS_M)
        };
        *height += blend * (target - *height);
    }

    /// Upper bound on `|h(g1) − h(g0)|` from fade changes alone.
    #[cfg(test)]
    fn fade_delta_bound(&self, direction: [f64; 3], g0: f64, g1: f64) -> f64 {
        let p = unit(direction);
        let amplitude = self.amplitude_at(p);
        let mut lambda = self.cell_size_m;
        let mut sum = 0.0;
        for (index, _) in self.octaves.iter().enumerate() {
            let fade0 = octave_fade(lambda, g0);
            let fade1 = octave_fade(lambda, g1);
            if fade0 == 0.0 && fade1 == 0.0 {
                break;
            }
            sum += (fade1 - fade0).abs() * self.half[index] * octave_cap(lambda, self.cell_size_m);
            lambda *= 0.5;
        }
        amplitude * sum
    }

    #[cfg(test)]
    fn max_detail_abs(&self) -> f64 {
        let mut lambda = self.cell_size_m;
        let mut sum = 0.0;
        for half in &self.half {
            sum += half * octave_cap(lambda, self.cell_size_m);
            lambda *= 0.5;
        }
        sum
    }
}

fn mix_noise(noise: f64, mountains: f64) -> f64 {
    let ridged = (1.0 - 2.0 * noise.abs()) - RIDGED_NOISE_MEAN;
    (1.0 - mountains) * noise + mountains * ridged
}

/// `|mixed|` for a raw octave, or `|mixed − bilinear(mixed)|` once the
/// cell-scale low-pass is removed.
#[cfg(test)]
fn octave_cap(lambda: f64, cell_size_m: f64) -> f64 {
    if lambda * 4.0 >= cell_size_m {
        2.0
    } else {
        1.0 + RIDGED_NOISE_MEAN.abs()
    }
}

/// Smallest detail amplitude, metres. A shelf keeps this much texture.
const MIN_AMPLITUDE_M: f64 = 4.0;

/// Detail amplitude of one canonical cell. Land and lake cells scale with
/// relief. Ocean cells scale with relief and fade in with depth, so a shallow
/// shelf stays at the floor.
fn cell_amplitude(elevation: f64, relief: f64, ocean: bool, sea_m: f64) -> f64 {
    if ocean {
        let depth = sea_m - elevation;
        ((0.1 * relief + 8.0).clamp(4.0, 300.0) * smoothstep(0.0, 200.0, depth))
            .max(MIN_AMPLITUDE_M)
    } else {
        (0.2 * relief + 6.0).clamp(6.0, 800.0)
    }
}

/// Bilinear weights below this are numerical leakage, not a real parent.
const WEIGHT_EPS: f64 = 1e-6;

/// Metres a pulled pixel keeps from the water line at a cell centre.
const CLASS_MARGIN_M: f64 = 2.0;

/// Width of the smooth min or max in the class pull, metres.
const CLASS_SOFTNESS_M: f64 = 8.0;

/// Polynomial smooth minimum. It never exceeds `min(a, b)` and is C1.
fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// Polynomial smooth maximum. It is never below `max(a, b)` and is C1.
fn smooth_max(a: f64, b: f64, k: f64) -> f64 {
    -smooth_min(-a, -b, k)
}

fn octave_count_for(cell_size_m: f64) -> usize {
    let mut count = 0usize;
    let mut lambda = cell_size_m;
    while lambda >= 50.0 && count < 24 {
        count += 1;
        lambda *= 0.5;
    }
    count
}

fn octave_fade(lambda: f64, ground_m: f64) -> f64 {
    if lambda < 50.0 {
        return 0.0;
    }
    smoothstep(1.5 * ground_m, 3.0 * ground_m, lambda)
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = if edge0 == edge1 {
        if x >= edge1 {
            1.0
        } else {
            0.0
        }
    } else {
        ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0)
    };
    t * t * (3.0 - 2.0 * t)
}

fn unit(p: [f64; 3]) -> [f64; 3] {
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    if len == 0.0 {
        [1.0, 0.0, 0.0]
    } else {
        [p[0] / len, p[1] / len, p[2] / len]
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use daena_physical::engine::{self, Settings, WorldPreset};
    use daena_physical::{NoopProgress, DEFAULT_RADIUS_METRES};
    use rayon::ThreadPoolBuilder;

    use super::*;
    use crate::style::{self, load_style, AtlasStyle};

    fn fixture() -> &'static AtlasTerrain {
        static TERRAIN: OnceLock<AtlasTerrain> = OnceLock::new();
        TERRAIN.get_or_init(|| {
            let settings = Settings::from_parts(
                831_429,
                WorldPreset::Earthlike,
                64,
                DEFAULT_RADIUS_METRES,
                0,
                daena_physical::planetary::PlanetaryConfiguration::earth_like(),
            );
            let products = engine::generate(settings, &mut NoopProgress).expect("n=64 world");
            AtlasTerrain::from_source(products.source(), None).expect("terrain")
        })
    }

    fn relief() -> AtlasStyle {
        load_style(style::RELIEF_STYLE_ID).expect("relief style").0
    }

    fn assert_rgba_eq(left: &[u8], right: &[u8]) {
        assert_eq!(left.len(), right.len(), "rgba length");
        if let Some(index) = left.iter().zip(right.iter()).position(|(a, b)| a != b) {
            panic!(
                "rgba differs at byte {index}: {} vs {}",
                left[index], right[index]
            );
        }
    }

    #[test]
    fn bicubic_through_tool_surface_matches_cell_centres() {
        let terrain = fixture();
        let surface = terrain.surface();
        for cell in 0..surface.cell_count() as u32 {
            let direction = surface.direction_of(cell);
            let (cells, weights) = surface.bicubic_stencil(direction);
            let mut elevation = 0.0;
            let mut weight_sum = 0.0;
            for slot in 0..16 {
                elevation += weights[slot] * f64::from(surface.elevation_mm[cells[slot] as usize]);
                weight_sum += weights[slot];
            }
            let expected = f64::from(surface.elevation_mm[cell as usize]);
            assert!(
                (elevation - expected).abs() < 1e-2,
                "cell {cell} bicubic {elevation} expected {expected}"
            );
            assert!(
                (weight_sum - 1.0).abs() < 1e-9,
                "cell {cell} weights {weight_sum}"
            );
            let (cells, weights) = surface.bilinear_stencil(direction);
            let mut bilinear_sum = 0.0;
            for weight in weights {
                bilinear_sum += weight;
            }
            assert!((bilinear_sum - 1.0).abs() < 1e-9);
            let centre_weight = cells
                .iter()
                .zip(weights)
                .find(|(id, _)| **id == cell)
                .map(|(_, weight)| weight)
                .unwrap_or(0.0);
            assert!(
                centre_weight > 1.0 - 1e-6,
                "cell {cell} bilinear centre weight {centre_weight}"
            );
        }
    }

    #[test]
    fn tile_bytes_do_not_depend_on_the_thread_count() {
        let terrain = fixture();
        let style = relief();
        let once = render_pixels(terrain, &style, 4, 256, 512, 256, 256, 1);
        let twice = render_pixels(terrain, &style, 4, 256, 512, 256, 256, 1);
        assert_rgba_eq(&once, &twice);
        let pool = ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("pool");
        let serial = pool.install(|| render_pixels(terrain, &style, 4, 256, 512, 256, 256, 1));
        assert_rgba_eq(&once, &serial);
    }

    #[test]
    fn scale_two_downsample_stays_near_scale_one() {
        let terrain = fixture();
        let style = relief();
        let width = 128u32;
        let height = 96u32;
        let coarse = render_pixels(terrain, &style, 6, 1_024, 768, width, height, 1);
        let fine = render_pixels(terrain, &style, 6, 1_024, 768, width, height, 2);
        let mut acc = 0.0;
        let mut count = 0.0;
        let fine_w = (width * 2) as usize;
        for y in 0..height as usize {
            for x in 0..width as usize {
                let mut average = [0.0; 3];
                for dy in 0..2usize {
                    for dx in 0..2usize {
                        let pixel = ((y * 2 + dy) * fine_w + (x * 2 + dx)) * 4;
                        for channel in 0..3 {
                            average[channel] += f64::from(fine[pixel + channel]);
                        }
                    }
                }
                let coarse_pixel = (y * width as usize + x) * 4;
                for channel in 0..3 {
                    average[channel] /= 4.0;
                    acc += (average[channel] - f64::from(coarse[coarse_pixel + channel])).abs();
                    count += 1.0;
                }
            }
        }
        let mean = acc / count;
        assert!(mean <= 3.0, "mean absolute RGB difference {mean}");
    }

    #[test]
    fn rendered_rectangles_match_adjacent_tiles() {
        let terrain = fixture();
        let style = relief();
        for z in [5u32, 11] {
            assert_seam(terrain, &style, z, 256, 256, 512, 256, false);
            assert_seam(terrain, &style, z, 256, 512, 256, 512, true);
            let world = 256u32 << z;
            assert_seam(terrain, &style, z, world - 256, 256, 512, 256, false);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_seam(
        terrain: &AtlasTerrain,
        style: &AtlasStyle,
        z: u32,
        gx0: u32,
        gy0: u32,
        width: u32,
        height: u32,
        vertical: bool,
    ) {
        let wide = render_pixels(terrain, style, z, gx0, gy0, width, height, 1);
        let (tile_w, tile_h, gx1, gy1) = if vertical {
            (width, height / 2, gx0, gy0 + height / 2)
        } else {
            (width / 2, height, gx0 + width / 2, gy0)
        };
        let first = render_pixels(terrain, style, z, gx0, gy0, tile_w, tile_h, 1);
        let second = render_pixels(terrain, style, z, gx1, gy1, tile_w, tile_h, 1);
        let mut stitched = vec![0u8; wide.len()];
        let tile_bytes_w = tile_w as usize * 4;
        if vertical {
            let row_bytes = width as usize * 4;
            let split = (height / 2) as usize * row_bytes;
            stitched[..split].copy_from_slice(&first);
            stitched[split..].copy_from_slice(&second);
        } else {
            for row in 0..height as usize {
                let wide_row = row * width as usize * 4;
                let tile_row = row * tile_bytes_w;
                stitched[wide_row..wide_row + tile_bytes_w]
                    .copy_from_slice(&first[tile_row..tile_row + tile_bytes_w]);
                stitched[wide_row + tile_bytes_w..wide_row + 2 * tile_bytes_w]
                    .copy_from_slice(&second[tile_row..tile_row + tile_bytes_w]);
            }
        }
        assert_rgba_eq(&wide, &stitched);
    }

    #[test]
    fn zoom_changes_only_by_the_octave_fade() {
        let terrain = fixture();
        let span = terrain.max_detail_abs();
        let mut directions = Vec::new();
        for cell in 0..terrain.surface().cell_count() as u32 {
            let direction = terrain.surface().direction_of(cell);
            let canon = terrain.canonical_elevation(direction);
            let amplitude = terrain.amplitude_at(direction);
            let index = cell as usize;
            let inactive = if terrain.ocean[index] != 0 {
                canon + amplitude * span < terrain.sea_m - CLASS_MARGIN_M - CLASS_SOFTNESS_M
            } else if terrain.wet[index] == 0 {
                canon - amplitude * span > terrain.sea_m + CLASS_MARGIN_M + CLASS_SOFTNESS_M
            } else {
                false
            };
            if inactive {
                directions.push(direction);
                if directions.len() == 1_000 {
                    break;
                }
            }
        }
        assert_eq!(directions.len(), 1_000, "need 1000 clamp-inactive centres");
        for direction in directions {
            let lat = direction[2].clamp(-1.0, 1.0).asin();
            for z in 4..=11u32 {
                let g0 = ground_metres_per_pixel(z, lat, terrain.radius_m());
                let g1 = ground_metres_per_pixel(z + 1, lat, terrain.radius_m());
                let h0 = terrain.detailed_elevation(direction, g0);
                let h1 = terrain.detailed_elevation(direction, g1);
                let bound = terrain.fade_delta_bound(direction, g0, g1);
                let delta = (h1 - h0).abs();
                assert!(delta <= bound + 1e-6, "z {z} delta {delta} bound {bound}");
            }
        }
    }

    #[test]
    fn canonical_class_holds_at_cell_centres() {
        let terrain = fixture();
        let surface = terrain.surface();
        for z in [6u32, 9, 12] {
            for cell in 0..surface.cell_count() as u32 {
                let direction = surface.direction_of(cell);
                let (lat, _) = surface.cell_lat_lon_degrees(cell);
                let ground = ground_metres_per_pixel(z, lat.to_radians(), terrain.radius_m());
                let height = terrain.detailed_elevation(direction, ground);
                let (cover, _ice) = terrain.classify(direction, height);
                let index = cell as usize;
                let expected = if terrain.ocean[index] != 0 {
                    TerrainCover::Ocean
                } else if terrain.wet[index] != 0 {
                    TerrainCover::Lake
                } else {
                    TerrainCover::Land
                };
                assert_eq!(cover, expected, "cell {cell} zoom {z} h {height}");
            }
        }
    }

    #[test]
    fn detail_is_zero_mean_on_high_land() {
        let terrain = fixture();
        let span = terrain.max_detail_abs();
        let mut cells = Vec::new();
        for cell in 0..terrain.elevation_m.len() as u32 {
            if terrain.ocean[cell as usize] != 0 || terrain.wet[cell as usize] != 0 {
                continue;
            }
            let elevation = f64::from(terrain.elevation_m[cell as usize]);
            let relief = f64::from(terrain.relief_m[cell as usize]);
            let amplitude = terrain.amplitude_at(terrain.surface().direction_of(cell));
            if elevation - amplitude * span - 2.0 * relief
                > terrain.sea_m + CLASS_MARGIN_M + CLASS_SOFTNESS_M
            {
                cells.push(cell);
                if cells.len() == 20 {
                    break;
                }
            }
        }
        assert_eq!(cells.len(), 20, "need 20 high land cells");
        let cell_angle = terrain.cell_size_m / terrain.radius_m;
        for cell in cells {
            let centre = terrain.surface().direction_of(cell);
            let (east, north) = tangent_axes(centre);
            let amplitude = terrain.amplitude_at(centre);
            let mut acc = 0.0;
            for y in 0..16 {
                for x in 0..16 {
                    let u = ((x as f64) + 0.5) / 16.0 - 0.5;
                    let v = ((y as f64) + 0.5) / 16.0 - 0.5;
                    let direction = unit([
                        centre[0] + (east[0] * u + north[0] * v) * cell_angle,
                        centre[1] + (east[1] * u + north[1] * v) * cell_angle,
                        centre[2] + (east[2] * u + north[2] * v) * cell_angle,
                    ]);
                    let height = terrain.detailed_elevation(direction, 1.0);
                    let canon = terrain.canonical_elevation(direction);
                    acc += height - canon;
                }
            }
            let mean = acc / 256.0;
            assert!(
                mean.abs() <= 0.25 * amplitude,
                "cell {cell} mean {mean} amplitude {amplitude}"
            );
        }
    }

    fn tangent_axes(up: [f64; 3]) -> ([f64; 3], [f64; 3]) {
        let reference = if up[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let east = unit([
            reference[1] * up[2] - reference[2] * up[1],
            reference[2] * up[0] - reference[0] * up[2],
            reference[0] * up[1] - reference[1] * up[0],
        ]);
        let north = [
            up[1] * east[2] - up[2] * east[1],
            up[2] * east[0] - up[0] * east[2],
            up[0] * east[1] - up[1] * east[0],
        ];
        (east, north)
    }

    #[test]
    fn auto_site_is_the_highest_coastal_land_cell() {
        let terrain = fixture();
        let site = terrain.auto_site_cell();
        assert_eq!(terrain.ocean[site as usize], 0);
        assert_eq!(terrain.wet[site as usize], 0);
        let mut distance = vec![u8::MAX; terrain.elevation_m.len()];
        let mut queue = std::collections::VecDeque::new();
        for cell in 0..terrain.elevation_m.len() {
            if terrain.ocean[cell] != 0 {
                distance[cell] = 0;
                queue.push_back(cell as u32);
            }
        }
        while let Some(cell) = queue.pop_front() {
            let next = distance[cell as usize];
            if next >= 2 {
                continue;
            }
            for neighbor in terrain.surface().neighbors(cell) {
                if neighbor != TOOL_SURFACE_NONE && distance[neighbor as usize] == u8::MAX {
                    distance[neighbor as usize] = next + 1;
                    queue.push_back(neighbor);
                }
            }
        }
        assert!((1..=2).contains(&distance[site as usize]));
        let site_relief = terrain.relief_m[site as usize];
        for cell in 0..terrain.elevation_m.len() as u32 {
            let dist = distance[cell as usize];
            if !(1..=2).contains(&dist)
                || terrain.ocean[cell as usize] != 0
                || terrain.wet[cell as usize] != 0
            {
                continue;
            }
            let relief = terrain.relief_m[cell as usize];
            assert!(
                relief < site_relief || (relief == site_relief && cell >= site),
                "cell {cell} relief {relief} beats site {site} {site_relief}"
            );
        }
    }

    #[test]
    #[ignore]
    fn terrain_tile_timing_n512_scale2() {
        let settings = Settings::from_parts(
            831_429,
            WorldPreset::Earthlike,
            512,
            DEFAULT_RADIUS_METRES,
            0,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let products = engine::generate(settings, &mut NoopProgress).expect("n=512");
        let decoded = engine::decode_source(products.source()).expect("decode");
        let surface = decoded
            .derive_tool_surface_present(&mut NoopProgress)
            .expect("surface");
        let identity = crate::spike_identity_from_source(products.source());
        let terrain = AtlasTerrain::new(Arc::new(surface), &identity);
        let style = relief();
        let (lat, lon) = terrain
            .surface()
            .cell_lat_lon_degrees(terrain.auto_site_cell());
        for z in [2u32, 5, 8, 10, 12] {
            let (x, y) = lon_lat_to_tile(lon, lat, z);
            let n = 1u32 << z;
            let mut times = Vec::with_capacity(9);
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let tile_x = (x as i32 + dx).rem_euclid(n as i32) as u32;
                    let tile_y = (y as i32 + dy).clamp(0, n as i32 - 1) as u32;
                    let started = std::time::Instant::now();
                    let _pixels =
                        render_pixels(&terrain, &style, z, tile_x * 256, tile_y * 256, 256, 256, 2);
                    times.push(started.elapsed().as_secs_f64() * 1000.0);
                }
            }
            times.sort_by(f64::total_cmp);
            let median = times[4];
            let worst = times[8];
            println!("zoom {z} median {median:.3} ms worst {worst:.3} ms target 50");
        }
    }
}
