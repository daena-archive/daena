//! Engine rivers carved into `h(p)`, on one centre-line with the vector tiles.
//!
//! Each run is the cell-centre polyline from [`river_runs`], smoothed with two
//! Chaikin passes. A hash-addressed meander in the `"meander"` domain offsets
//! that line. The raster evaluates the offset on the fly. Vector tiles sample
//! the same function.

use daena_physical::engine::{ToolSurface, TOOL_SURFACE_NONE};
use daena_physical::hydro_claim::canonical_river_id;

use super::noise::{domain_seed, splitmix64};
use super::smoothstep;

/// Strahler orders 1 through 5. Higher orders grow by [`CHANNEL_WIDTH_GROWTH`].
const CHANNEL_WIDTH_M: [f64; 5] = [12.0, 30.0, 70.0, 160.0, 350.0];
const CHANNEL_WIDTH_GROWTH: f64 = 2.2;
/// Meander wavelength is `12 · w`, and never shorter than this.
const MIN_MEANDER_M: f64 = 1_000.0;
/// `|d(offset)/ds|` stays under this for a sine of wavelength `λ`.
const MEANDER_SLOPE_LIMIT: f64 = 0.8;
/// Second-octave weight. The sum is normalised so `|offset| ≤ A`.
const MEANDER_OCTAVE_2: f64 = 0.5;
const MEANDER_OCTAVE_2_MIX: u64 = 0xD1B5_4A32_D192_ED03;
/// Box radius for the along-run slope, in cells.
const SLOPE_SMOOTH_CELLS: f64 = 2.0;
/// Plains meander below this rise/run; mountains are straight above the top.
const FLAT_SLOPE_LO: f64 = 0.002;
const FLAT_SLOPE_HI: f64 = 0.02;
/// Bed stays at least this far above sea level, except the ocean-mouth ramp.
const BED_ABOVE_SEA_M: f64 = 1.0;
/// Detail factor `k` in `canonical − k · amplitude`.
///
/// On the n=64 fixture the 1st percentile of `(uncarved − canonical) / amplitude`
/// is −0.3343 at ground 1 m (−0.3342 at 40 m, −0.3347 at 150 m). The minimum
/// of those 14_040 land samples is −0.8003. Valley shoulders still reach about
/// −1.10, so `k` sits above that excursion.
const BED_DETAIL_K: f64 = 1.20;
/// Coarse vector pieces hold about this many samples, so a tile span stays local.
const EXPORT_PIECE_POINTS: usize = 8;

/// Channel width for a Strahler order. Order 0 is treated as order 1.
pub(super) fn channel_width_m(order: u16) -> f64 {
    match order {
        0 | 1 => CHANNEL_WIDTH_M[0],
        2 => CHANNEL_WIDTH_M[1],
        3 => CHANNEL_WIDTH_M[2],
        4 => CHANNEL_WIDTH_M[3],
        5 => CHANNEL_WIDTH_M[4],
        order => {
            let extra = i32::from(order - 5);
            CHANNEL_WIDTH_M[4] * CHANNEL_WIDTH_GROWTH.powi(extra)
        }
    }
}

#[derive(Clone, Copy)]
struct Seg {
    /// Index of the segment start in the point arrays. The end is `i0 + 1`.
    i0: u32,
    run: u32,
}

struct RunRec {
    start: u32,
    count: u32,
    river: u32,
    length_m: f32,
    half_cell_m: f32,
    width_m: f32,
    depth_m: f32,
    valley_m: f32,
    lambda_m: f32,
    amp_cap_m: f32,
    reach_rad_sq: f32,
    seed: u64,
}

struct RiverRec {
    source_cell: u32,
    order: u16,
    width_m: f32,
    run_start: u32,
    run_count: u32,
}

struct RawRun {
    points: Vec<[f64; 3]>,
    ocean_mouth: bool,
}

/// Smoothed, meandered centre-lines and the per-cell segment index.
pub(super) struct RiverGeometry {
    cell_offsets: Vec<u32>,
    seg_ids: Vec<u32>,
    segs: Vec<Seg>,
    xyz: Vec<[f32; 3]>,
    /// Unit vertex normals. Along a chord these are interpolated, so the
    /// displaced curve does not jump at a Chaikin vertex.
    normal: Vec<[f32; 3]>,
    arc: Vec<f32>,
    bed: Vec<f32>,
    slope: Vec<f32>,
    runs: Vec<RunRec>,
    rivers: Vec<RiverRec>,
    radius_m: f64,
    sea_m: f64,
    cell_m: f64,
    max_valley_m: f32,
    max_width_m: f32,
}

impl RiverGeometry {
    pub(super) fn build(
        surface: &ToolSurface,
        elevation_m: &[f32],
        amplitude_m: &[f32],
        terrain_seed: u64,
        cell_m: f64,
        radius_m: f64,
        sea_m: f64,
    ) -> Self {
        let domain = domain_seed(terrain_seed, "meander");
        let mut xyz = Vec::new();
        let mut normal = Vec::new();
        let mut arc = Vec::new();
        let mut bed = Vec::new();
        let mut slope = Vec::new();
        let mut runs = Vec::new();
        let mut rivers = Vec::new();
        let mut segs = Vec::new();
        let mut max_valley_m = 0.0f32;
        let mut max_width_m = 0.0f32;

        for river in &surface.rivers {
            let raw_runs = river_runs(surface, river);
            if raw_runs.is_empty() {
                continue;
            }
            let width = channel_width_m(river.strahler_order);
            let run_start = runs.len() as u32;
            let seed = meander_seed(domain, river.source_cell);
            for raw in raw_runs {
                let smoothed = chaikin_twice(&raw.points);
                if smoothed.len() < 2 {
                    continue;
                }
                push_run(
                    surface,
                    elevation_m,
                    amplitude_m,
                    &smoothed,
                    raw.ocean_mouth,
                    RunBuild {
                        river: rivers.len() as u32,
                        width,
                        seed,
                        cell_m,
                        radius_m,
                        sea_m,
                    },
                    &mut xyz,
                    &mut normal,
                    &mut arc,
                    &mut bed,
                    &mut slope,
                    &mut runs,
                    &mut segs,
                    &mut max_valley_m,
                    &mut max_width_m,
                );
            }
            let run_count = runs.len() as u32 - run_start;
            if run_count == 0 {
                continue;
            }
            rivers.push(RiverRec {
                source_cell: river.source_cell,
                order: river.strahler_order.max(1),
                width_m: width as f32,
                run_start,
                run_count,
            });
        }

        let (cell_offsets, seg_ids) = build_index(surface, &xyz, &segs, &runs, cell_m, radius_m);
        Self {
            cell_offsets,
            seg_ids,
            segs,
            xyz,
            normal,
            arc,
            bed,
            slope,
            runs,
            rivers,
            radius_m,
            sea_m,
            cell_m,
            max_valley_m,
            max_width_m,
        }
    }

    pub(super) fn storage_bytes(&self) -> usize {
        let mut bytes = 0usize;
        bytes += self.cell_offsets.capacity() * std::mem::size_of::<u32>();
        bytes += self.seg_ids.capacity() * std::mem::size_of::<u32>();
        bytes += self.segs.capacity() * std::mem::size_of::<Seg>();
        bytes += self.xyz.capacity() * std::mem::size_of::<[f32; 3]>();
        bytes += self.normal.capacity() * std::mem::size_of::<[f32; 3]>();
        bytes += self.arc.capacity() * std::mem::size_of::<f32>();
        bytes += self.bed.capacity() * std::mem::size_of::<f32>();
        bytes += self.slope.capacity() * std::mem::size_of::<f32>();
        bytes += self.runs.capacity() * std::mem::size_of::<RunRec>();
        bytes += self.rivers.capacity() * std::mem::size_of::<RiverRec>();
        bytes
    }

    pub(super) fn river_count(&self) -> usize {
        self.rivers.len()
    }

    /// Carved height. `h` is returned unchanged far from every river, in a wet
    /// lake, or where the channel and the valley are both narrower than a pixel.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn carve(
        &self,
        p: [f64; 3],
        h: f64,
        relief: f64,
        ground_m: f64,
        cell: u32,
        wet: u8,
        canon: f64,
        amplitude: f64,
    ) -> f64 {
        if self.runs.is_empty() || wet != 0 {
            return h;
        }
        if ground_m >= f64::from(self.max_valley_m) && ground_m >= 2.0 * f64::from(self.max_width_m)
        {
            return h;
        }
        let cell = cell as usize;
        if cell + 1 >= self.cell_offsets.len() {
            return h;
        }
        let start = self.cell_offsets[cell] as usize;
        let end = self.cell_offsets[cell + 1] as usize;
        if start == end {
            return h;
        }
        let Some(hit) = self.closest(p, start, end) else {
            return h;
        };
        let run = &self.runs[hit.run as usize];
        let valley = f64::from(run.valley_m);
        let width = f64::from(run.width_m);
        if hit.distance >= valley || valley <= 0.0 {
            return h;
        }
        let valley_fade = smoothstep(ground_m, 3.0 * ground_m, valley);
        let channel_fade = smoothstep(0.5 * ground_m, 2.0 * ground_m, width);
        if valley_fade == 0.0 && channel_fade == 0.0 {
            return h;
        }
        let sea_floor = self.sea_m + BED_ABOVE_SEA_M;
        let local = canon - BED_DETAIL_K * amplitude;
        // The stored bed is floored at sea + 1 m. Use the local sink when it
        // stays above that floor. Where the uncarved ground is already below
        // the floor, the floor is not a cut, so keep the local sink.
        let bed = if h < sea_floor || canon < self.sea_m {
            hit.bed.min(local)
        } else {
            let floor = hit.bed.min(sea_floor);
            hit.bed.min(local).max(floor)
        };
        let hit = Hit {
            distance: hit.distance,
            bed,
            run: hit.run,
            position: hit.position,
        };
        let half = 0.5 * width;
        let bank = (hit.distance - half).max(0.0);
        // The floor meets this pixel's uncarved height by `0.6 V`, so the
        // blend from there out to `V` has no step. A deep bed (running minimum
        // in mountains) makes a steep wall; a plains bed only `k · a` down stays shallow.
        let span = (0.6 * valley - half).max(1.0);
        let rise = (h - hit.bed).max(0.0);
        let wall = (relief / self.cell_m).max(0.0).max(rise / span);
        let valley_floor = hit.bed + wall * bank;
        let lowered = h.min(valley_floor);
        let mut carved = h + valley_fade * (lowered - h);
        if hit.distance < half {
            let bottom = hit.bed - f64::from(run.depth_m) * channel_fade;
            if carved > bottom {
                carved = bottom;
            }
        }
        let blend = smoothstep(0.6 * valley, valley, hit.distance);
        carved + blend * (h - carved)
    }

    /// Absolute distance in metres from `p` to the displaced centre-line.
    /// A point with no indexed segment returns a large value.
    #[cfg(test)]
    pub(super) fn displaced_distance(&self, p: [f64; 3], cell: u32) -> f64 {
        let cell = cell as usize;
        if cell + 1 >= self.cell_offsets.len() {
            return 1.0e9;
        }
        let start = self.cell_offsets[cell] as usize;
        let end = self.cell_offsets[cell + 1] as usize;
        self.closest(p, start, end)
            .map(|hit| hit.distance)
            .unwrap_or(1.0e9)
    }

    /// Distance to this run, ignoring the cell index. Tests use it when the
    /// index misses.
    #[cfg(test)]
    fn distance_along_run(&self, p: [f64; 3], run_index: usize) -> f64 {
        let Some(run) = self.runs.get(run_index) else {
            return 1.0e9;
        };
        let start = run.start as usize;
        let last = start + run.count as usize - 1;
        let mut best = 1.0e9f64;
        for index in start..last {
            let a = f64_3(self.xyz[index]);
            let b = f64_3(self.xyz[index + 1]);
            let (t, _) = chord_t_d2(p, a, b);
            let Some((t, centre, normal)) = self.located_frame(index, p, t) else {
                continue;
            };
            let signed = dot(sub(p, centre), normal) * self.radius_m;
            let s0 = f64::from(self.arc[index]);
            let s1 = f64::from(self.arc[index + 1]);
            let s = s0 + t * (s1 - s0);
            let slope = lerp(
                f64::from(self.slope[index]),
                f64::from(self.slope[index + 1]),
                t,
            );
            let offset = meander_offset(run, s, slope);
            best = best.min((signed - offset).abs());
        }
        best
    }

    #[cfg(test)]
    fn listed_segments(&self, cell: u32) -> usize {
        let cell = cell as usize;
        if cell + 1 >= self.cell_offsets.len() {
            return 0;
        }
        (self.cell_offsets[cell + 1] - self.cell_offsets[cell]) as usize
    }

    /// `(max segments in one cell, nonempty cells, total listings)`.
    #[cfg(test)]
    pub(super) fn segment_list_stats(&self) -> (u32, u32, u64) {
        let mut max = 0u32;
        let mut nonempty = 0u32;
        let mut sum = 0u64;
        for window in self.cell_offsets.windows(2) {
            let count = window[1] - window[0];
            if count == 0 {
                continue;
            }
            nonempty += 1;
            sum += u64::from(count);
            max = max.max(count);
        }
        (max, nonempty, sum)
    }

    /// Every indexed run at `cell` is narrower than a z pixel, so carving is a no-op.
    #[cfg(test)]
    fn cell_is_all_faded(&self, cell: u32, ground_m: f64) -> bool {
        let cell = cell as usize;
        if cell + 1 >= self.cell_offsets.len() {
            return false;
        }
        let start = self.cell_offsets[cell] as usize;
        let end = self.cell_offsets[cell + 1] as usize;
        if start == end {
            return false;
        }
        for slot in start..end {
            let seg = self.segs[self.seg_ids[slot] as usize];
            let run = &self.runs[seg.run as usize];
            let valley_fade = smoothstep(ground_m, 3.0 * ground_m, f64::from(run.valley_m));
            let channel_fade = smoothstep(0.5 * ground_m, 2.0 * ground_m, f64::from(run.width_m));
            if valley_fade != 0.0 || channel_fade != 0.0 {
                return false;
            }
        }
        true
    }

    pub(super) fn midpoint_lonlat(&self, order: Option<u16>) -> Option<(f64, f64)> {
        let run = self.longest_run(order)?;
        let length = f64::from(self.runs[run].length_m);
        let direction = self.displaced_direction(run, length * 0.5)?;
        Some(lonlat_tuple(direction))
    }

    /// Displaced lon/lat samples from `s_start` to `s_end`, including both ends.
    pub(super) fn sample_run_span(
        &self,
        run_index: u32,
        step_m: f64,
        s_start: f64,
        s_end: f64,
    ) -> Vec<[f64; 2]> {
        self.samples_marked(run_index, step_m, s_start, s_end)
            .into_iter()
            .map(|(_, lonlat)| lonlat)
            .collect()
    }

    fn samples_marked(
        &self,
        run_index: u32,
        step_m: f64,
        s_start: f64,
        s_end: f64,
    ) -> Vec<(f64, [f64; 2])> {
        let Some(run) = self.runs.get(run_index as usize) else {
            return Vec::new();
        };
        let length = f64::from(run.length_m);
        if length <= 0.0 || run.count < 2 {
            return Vec::new();
        }
        let s_start = s_start.clamp(0.0, length);
        let s_end = s_end.clamp(s_start, length);
        let step = step_m.max(1.0);
        let mut samples = Vec::new();
        let mut s = s_start;
        while s < s_end {
            if let Some(direction) = self.displaced_direction(run_index as usize, s) {
                samples.push((s, direction_lonlat(direction)));
            }
            s += step;
        }
        if let Some(direction) = self.displaced_direction(run_index as usize, s_end) {
            let end = direction_lonlat(direction);
            if samples.last().is_none_or(|last| last.1 != end) {
                samples.push((s_end, end));
            }
        }
        samples
    }

    /// Coarse displaced polylines for tile rejection. One entry per river that
    /// has a run.
    pub(super) fn export_rivers(&self, step_m: f64) -> Vec<ExportedRiver> {
        let mut out = Vec::with_capacity(self.rivers.len());
        for river in &self.rivers {
            let mut pieces = Vec::new();
            for run_index in river.run_start..river.run_start + river.run_count {
                let run = &self.runs[run_index as usize];
                let length = f64::from(run.length_m);
                let marked = self.samples_marked(run_index, step_m, 0.0, length);
                let pad_m = f64::from(run.amp_cap_m) + f64::from(run.valley_m);
                push_local_pieces(&marked, run_index, pad_m, self.radius_m, &mut pieces);
            }
            if pieces.is_empty() {
                continue;
            }
            out.push(ExportedRiver {
                id: canonical_river_id(river.source_cell),
                order: river.order,
                channel_width_m: f64::from(river.width_m),
                run_start: river.run_start,
                run_count: river.run_count,
                pieces,
            });
        }
        out
    }

    fn longest_run(&self, order: Option<u16>) -> Option<usize> {
        let mut best: Option<(usize, f32, u32)> = None;
        for (index, run) in self.runs.iter().enumerate() {
            let river = &self.rivers[run.river as usize];
            let keep = match order {
                None => river.order >= 4,
                Some(order) => river.order == order,
            };
            if !keep || run.count < 2 {
                continue;
            }
            let replace = match best {
                None => true,
                Some((best_index, length, source)) => {
                    run.length_m > length
                        || (run.length_m == length && river.source_cell < source)
                        || (run.length_m == length
                            && river.source_cell == source
                            && index < best_index)
                }
            };
            if replace {
                best = Some((index, run.length_m, river.source_cell));
            }
        }
        best.map(|(index, _, _)| index)
    }

    fn closest(&self, p: [f64; 3], start: usize, end: usize) -> Option<Hit> {
        // Chord distance is cheap. The meander is not, so the second pass
        // keeps only chords that can still win once each curve shifts by `A`.
        // `d2` is a unit-sphere residual, so `A` is converted by the radius.
        let mut best_d2 = f64::INFINITY;
        let mut best_amp = 0.0f64;
        for slot in start..end {
            let seg = self.segs[self.seg_ids[slot] as usize];
            let run = &self.runs[seg.run as usize];
            let a = f64_3(self.xyz[seg.i0 as usize]);
            let b = f64_3(self.xyz[seg.i0 as usize + 1]);
            let (_, d2) = chord_t_d2(p, a, b);
            if d2 > f64::from(run.reach_rad_sq) {
                continue;
            }
            if d2 < best_d2 {
                best_d2 = d2;
                best_amp = f64::from(run.amp_cap_m);
            }
        }
        if !best_d2.is_finite() {
            return None;
        }
        let best_chord = best_d2.sqrt();
        // Rough hits. `located_frame` then corrects `t` on chords still within
        // 16 m: on a long chord the projection is off by more than that before
        // the correction, and the analytic samples sit inside 1 m after it.
        const NEAR: usize = 48;
        let mut near = [(0usize, 0.0f64, 0u32, f64::INFINITY); NEAR];
        let mut near_n = 0usize;
        let mut rough_best = f64::INFINITY;
        for slot in start..end {
            let seg = self.segs[self.seg_ids[slot] as usize];
            let run = &self.runs[seg.run as usize];
            let i0 = seg.i0 as usize;
            let a = f64_3(self.xyz[i0]);
            let b = f64_3(self.xyz[i0 + 1]);
            let (t, d2) = chord_t_d2(p, a, b);
            if d2 > f64::from(run.reach_rad_sq) {
                continue;
            }
            let amp = f64::from(run.amp_cap_m).max(best_amp);
            let limit = best_chord + 2.0 * amp / self.radius_m;
            if d2 > limit * limit {
                continue;
            }
            let Some(hit) = self.hit_at(i0, t, seg.run, p) else {
                continue;
            };
            rough_best = rough_best.min(hit.distance);
            if near_n < NEAR {
                near[near_n] = (i0, t, seg.run, hit.distance);
                near_n += 1;
            }
        }
        if near_n == 0 {
            return None;
        }
        let mut best: Option<Hit> = None;
        for (i0, t, run, rough) in near.into_iter().take(near_n) {
            if rough > rough_best + 16.0 {
                continue;
            }
            let t = self.located_frame(i0, p, t).map(|(t, _, _)| t).unwrap_or(t);
            let Some(hit) = self.hit_at(i0, t, run, p) else {
                continue;
            };
            let replace = best
                .as_ref()
                .is_none_or(|kept| hit.distance < kept.distance);
            if replace {
                best = Some(hit);
            }
        }
        best
    }

    fn hit_at(&self, i0: usize, t: f64, run_index: u32, p: [f64; 3]) -> Option<Hit> {
        let (centre, normal) = self.vertex_frame(i0, t)?;
        let run = &self.runs[run_index as usize];
        let signed = dot(sub(p, centre), normal) * self.radius_m;
        let s0 = f64::from(self.arc[i0]);
        let s1 = f64::from(self.arc[i0 + 1]);
        let s = s0 + t * (s1 - s0);
        let slope = lerp(f64::from(self.slope[i0]), f64::from(self.slope[i0 + 1]), t);
        let offset = meander_offset(run, s, slope);
        // A foot clamped to a chord end leaves `p` beyond that end. Count the
        // overshoot too, or the clamped chord under-reports the distance and
        // notches the bank at every vertex.
        let tangent = cross(normal, centre);
        let along = dot(sub(p, centre), tangent) * self.radius_m;
        let overshoot = if (t <= 0.0 && along < 0.0) || (t >= 1.0 && along > 0.0) {
            along
        } else {
            0.0
        };
        let position = normalize(add(centre, scale(normal, offset / self.radius_m)));
        Some(Hit {
            distance: (signed - offset).hypot(overshoot),
            bed: lerp(f64::from(self.bed[i0]), f64::from(self.bed[i0 + 1]), t),
            run: run_index,
            position,
        })
    }

    /// Nearest point on a displaced centre-line indexed at `cells`.
    ///
    /// `amp_m` is the meander cap `A` and `width_m` is the channel width `w`.
    /// Stream outlets use the band `A + w`.
    pub(super) fn nearest_centreline(
        &self,
        cells: &[u32],
        point: [f64; 3],
    ) -> Option<CentrelineHit> {
        let mut best: Option<Hit> = None;
        for &cell in cells {
            let cell = cell as usize;
            if cell + 1 >= self.cell_offsets.len() {
                continue;
            }
            let start = self.cell_offsets[cell] as usize;
            let end = self.cell_offsets[cell + 1] as usize;
            if start == end {
                continue;
            }
            let Some(hit) = self.closest(point, start, end) else {
                continue;
            };
            let replace = match &best {
                None => true,
                Some(kept) => {
                    hit.distance < kept.distance
                        || (hit.distance == kept.distance && hit.run < kept.run)
                }
            };
            if replace {
                best = Some(hit);
            }
        }
        best.map(|hit| {
            let run = &self.runs[hit.run as usize];
            CentrelineHit {
                direction: hit.position,
                distance_m: hit.distance,
                amp_m: f64::from(run.amp_cap_m),
                width_m: f64::from(run.width_m),
            }
        })
    }

    /// Chord projection is a fraction of a chord off the parameter once the
    /// normal is not perpendicular to the chord. One or two steps along the
    /// tangent bring `t` back onto the displaced sample.
    fn located_frame(
        &self,
        index: usize,
        p: [f64; 3],
        t: f64,
    ) -> Option<(f64, [f64; 3], [f64; 3])> {
        let mut t = t;
        let mut frame = self.vertex_frame(index, t)?;
        let ds = (f64::from(self.arc[index + 1]) - f64::from(self.arc[index])).max(1.0);
        for _ in 0..2 {
            let (centre, normal) = frame;
            let tangent = cross(normal, centre);
            let tlen = dot(tangent, tangent).sqrt();
            if tlen < 1e-12 {
                break;
            }
            let tangent = scale(tangent, 1.0 / tlen);
            let along = dot(sub(p, centre), tangent) * self.radius_m;
            let next = (t + along / ds).clamp(0.0, 1.0);
            t = next;
            frame = self.vertex_frame(index, t)?;
            if along.abs() < 0.05 {
                break;
            }
        }
        Some((t, frame.0, frame.1))
    }

    /// Centre and cross-track normal with the vertex normal interpolated in `t`.
    fn vertex_frame(&self, index: usize, t: f64) -> Option<([f64; 3], [f64; 3])> {
        let a = f64_3(self.xyz[index]);
        let b = f64_3(self.xyz[index + 1]);
        let centre = normalize(mix(a, b, t));
        let n0 = f64_3(self.normal[index]);
        let n1 = f64_3(self.normal[index + 1]);
        let mixed = normalize(mix(n0, n1, t));
        let tangent = sub(mixed, scale(centre, dot(mixed, centre)));
        let len2 = dot(tangent, tangent);
        if len2 < 1e-24 {
            return chord_frame(a, b, t);
        }
        Some((centre, scale(tangent, 1.0 / len2.sqrt())))
    }

    fn displaced_direction(&self, run_index: usize, s: f64) -> Option<[f64; 3]> {
        let run = self.runs.get(run_index)?;
        if run.count < 2 {
            return None;
        }
        let length = f64::from(run.length_m);
        let s = s.clamp(0.0, length);
        let (index, t) = self.span_at(run, s);
        let (centre, normal) = self.vertex_frame(index, t)?;
        let slope = lerp(
            f64::from(self.slope[index]),
            f64::from(self.slope[index + 1]),
            t,
        );
        let offset = meander_offset(run, s, slope);
        Some(normalize(add(
            centre,
            scale(normal, offset / self.radius_m),
        )))
    }

    /// Frame of the displaced centre-line: position, cross-track normal, valley.
    #[cfg(test)]
    fn frame_at(&self, run_index: usize, s: f64) -> Option<LineFrame> {
        let run = self.runs.get(run_index)?;
        let length = f64::from(run.length_m);
        let s = s.clamp(0.0, length);
        let (index, t) = self.span_at(run, s);
        let (centre, normal) = self.vertex_frame(index, t)?;
        let slope = lerp(
            f64::from(self.slope[index]),
            f64::from(self.slope[index + 1]),
            t,
        );
        let offset = meander_offset(run, s, slope);
        let position = normalize(add(centre, scale(normal, offset / self.radius_m)));
        let radial = dot(normal, position);
        let normal = normalize(sub(normal, scale(position, radial)));
        Some(LineFrame {
            position,
            normal,
            valley_m: f64::from(run.valley_m),
        })
    }

    fn span_at(&self, run: &RunRec, s: f64) -> (usize, f64) {
        let start = run.start as usize;
        let last = start + run.count as usize - 1;
        let mut lo = start;
        let mut hi = last;
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if f64::from(self.arc[mid]) <= s {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if lo >= last {
            lo = last - 1;
        }
        let s0 = f64::from(self.arc[lo]);
        let s1 = f64::from(self.arc[lo + 1]);
        let t = if s1 > s0 {
            ((s - s0) / (s1 - s0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (lo, t)
    }
}

struct Hit {
    distance: f64,
    bed: f64,
    run: u32,
    position: [f64; 3],
}

pub(super) struct CentrelineHit {
    pub direction: [f64; 3],
    pub distance_m: f64,
    pub amp_m: f64,
    pub width_m: f64,
}

#[cfg(test)]
struct LineFrame {
    position: [f64; 3],
    normal: [f64; 3],
    valley_m: f64,
}

struct RunBuild {
    river: u32,
    width: f64,
    seed: u64,
    cell_m: f64,
    radius_m: f64,
    sea_m: f64,
}

#[allow(clippy::too_many_arguments)]
fn push_run(
    surface: &ToolSurface,
    elevation_m: &[f32],
    amplitude_m: &[f32],
    points: &[[f64; 3]],
    ocean_mouth: bool,
    build: RunBuild,
    xyz: &mut Vec<[f32; 3]>,
    normal_out: &mut Vec<[f32; 3]>,
    arc_out: &mut Vec<f32>,
    bed_out: &mut Vec<f32>,
    slope_out: &mut Vec<f32>,
    runs: &mut Vec<RunRec>,
    segs: &mut Vec<Seg>,
    max_valley_m: &mut f32,
    max_width_m: &mut f32,
) {
    let start = xyz.len() as u32;
    let mut canon = Vec::with_capacity(points.len());
    let mut sunk = Vec::with_capacity(points.len());
    let mut arc = Vec::with_capacity(points.len());
    arc.push(0.0);
    for (index, point) in points.iter().enumerate() {
        let elevation = canonical_elevation(surface, elevation_m, *point);
        let amplitude = detail_amplitude(surface, amplitude_m, *point);
        canon.push(elevation);
        sunk.push(elevation - BED_DETAIL_K * amplitude);
        if index + 1 < points.len() {
            let step = great_circle(points[index], points[index + 1], build.radius_m);
            arc.push(arc[index] + step);
        }
    }
    let length = *arc.last().unwrap_or(&0.0);
    if length < 1.0 {
        return;
    }
    let half_cell = 0.5 * build.cell_m;
    // Running minimum of the sunk series. Slope still uses the raw canonical.
    let bed = bed_profile(&sunk, &arc, build.sea_m, half_cell, ocean_mouth);
    let slope = smooth_slope(&canon, &arc, SLOPE_SMOOTH_CELLS * build.cell_m);
    let lambda = (12.0 * build.width).max(MIN_MEANDER_M);
    let slope_limit = MEANDER_SLOPE_LIMIT * lambda / std::f64::consts::TAU;
    let mut amp_cap = (2.5 * build.width).min(0.2 * build.cell_m).min(slope_limit);
    let valley = (8.0 * build.width).min(0.3 * build.cell_m).max(0.0);
    if amp_cap + valley > 0.5 * build.cell_m {
        amp_cap = (0.5 * build.cell_m - valley).max(0.0);
    }
    let depth = 1.0 + build.width / 60.0;
    let reach = amp_cap + valley;
    let reach_rad = (reach * 1.2) / build.radius_m;
    for point in points {
        xyz.push([point[0] as f32, point[1] as f32, point[2] as f32]);
    }
    for normal in vertex_normals(points) {
        normal_out.push([normal[0] as f32, normal[1] as f32, normal[2] as f32]);
    }
    for value in &arc {
        arc_out.push(*value as f32);
    }
    bed_out.extend(bed.iter().map(|value| *value as f32));
    slope_out.extend(slope.iter().map(|value| *value as f32));
    let run_index = runs.len() as u32;
    for index in 0..points.len() - 1 {
        segs.push(Seg {
            i0: start + index as u32,
            run: run_index,
        });
    }
    *max_valley_m = max_valley_m.max(valley as f32);
    *max_width_m = max_width_m.max(build.width as f32);
    runs.push(RunRec {
        start,
        count: points.len() as u32,
        river: build.river,
        length_m: length as f32,
        half_cell_m: half_cell as f32,
        width_m: build.width as f32,
        depth_m: depth as f32,
        valley_m: valley as f32,
        lambda_m: lambda as f32,
        amp_cap_m: amp_cap as f32,
        reach_rad_sq: (reach_rad * reach_rad) as f32,
        seed: build.seed,
    });
}

fn bed_profile(
    canon: &[f64],
    arc: &[f64],
    sea_m: f64,
    half_cell: f64,
    ocean_mouth: bool,
) -> Vec<f64> {
    let mut running = f64::INFINITY;
    let mut bed = Vec::with_capacity(canon.len());
    for value in canon {
        running = running.min(*value);
        bed.push(running.max(sea_m + BED_ABOVE_SEA_M));
    }
    if ocean_mouth && half_cell > 0.0 {
        let end = *arc.last().unwrap_or(&0.0);
        for (index, value) in bed.iter_mut().enumerate() {
            let from_end = end - arc[index];
            if from_end < half_cell {
                let t = from_end / half_cell;
                *value = sea_m + t * (*value - sea_m);
            }
        }
    }
    let mut floor = f64::INFINITY;
    for value in &mut bed {
        floor = floor.min(*value);
        *value = floor;
    }
    bed
}

fn smooth_slope(canon: &[f64], arc: &[f64], radius_m: f64) -> Vec<f64> {
    let n = canon.len();
    if n == 0 {
        return Vec::new();
    }
    let mut raw = vec![0.0; n];
    let mut weight = vec![0.0; n];
    for index in 0..n.saturating_sub(1) {
        let ds = (arc[index + 1] - arc[index]).max(1e-3);
        let slope = (canon[index + 1] - canon[index]).abs() / ds;
        raw[index] += slope;
        raw[index + 1] += slope;
        weight[index] += 1.0;
        weight[index + 1] += 1.0;
    }
    for index in 0..n {
        if weight[index] > 0.0 {
            raw[index] /= weight[index];
        }
    }
    let mut out = vec![0.0; n];
    for index in 0..n {
        let s = arc[index];
        let mut lo = index;
        while lo > 0 && s - arc[lo - 1] <= radius_m {
            lo -= 1;
        }
        let mut hi = index;
        while hi + 1 < n && arc[hi + 1] - s <= radius_m {
            hi += 1;
        }
        let mut sum = 0.0;
        let mut count = 0.0;
        for value in raw.iter().take(hi + 1).skip(lo) {
            sum += value;
            count += 1.0;
        }
        out[index] = if count > 0.0 { sum / count } else { 0.0 };
    }
    out
}

fn detail_amplitude(surface: &ToolSurface, amplitude_m: &[f32], direction: [f64; 3]) -> f64 {
    let (cells, weights) = surface.bicubic_stencil(direction);
    let mut amplitude = 0.0;
    for slot in 0..16 {
        amplitude += weights[slot] * f64::from(amplitude_m[cells[slot] as usize]);
    }
    amplitude.max(super::MIN_AMPLITUDE_M)
}

fn vertex_normals(points: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut acc = vec![[0.0; 3]; points.len()];
    for index in 0..points.len().saturating_sub(1) {
        let Some((_, normal)) = chord_frame(points[index], points[index + 1], 0.5) else {
            continue;
        };
        acc[index] = add(acc[index], normal);
        acc[index + 1] = add(acc[index + 1], normal);
    }
    acc.into_iter().map(normalize).collect()
}

fn canonical_elevation(surface: &ToolSurface, elevation_m: &[f32], direction: [f64; 3]) -> f64 {
    let (cells, weights) = surface.bicubic_stencil(direction);
    let mut height = 0.0;
    for slot in 0..16 {
        height += weights[slot] * f64::from(elevation_m[cells[slot] as usize]);
    }
    height
}

fn build_index(
    surface: &ToolSurface,
    xyz: &[[f32; 3]],
    segs: &[Seg],
    runs: &[RunRec],
    cell_m: f64,
    radius_m: f64,
) -> (Vec<u32>, Vec<u32>) {
    let mut pairs = Vec::new();
    let mut cells = Vec::new();
    for (seg_id, seg) in segs.iter().enumerate() {
        let run = &runs[seg.run as usize];
        let a = f64_3(xyz[seg.i0 as usize]);
        let b = f64_3(xyz[seg.i0 as usize + 1]);
        let reach = f64::from(run.amp_cap_m) + f64::from(run.valley_m);
        cells.clear();
        let steps = if great_circle(a, b, radius_m) > 0.5 * cell_m {
            4
        } else {
            2
        };
        for step in 0..=steps {
            let t = f64::from(step) / f64::from(steps);
            let point = normalize(mix(a, b, t));
            consider_cell(surface, point, &mut cells);
        }
        for &cell in &cells {
            if !segment_reaches_cell(surface, cell, a, b, reach, radius_m) {
                continue;
            }
            pairs.push((cell, seg_id as u32));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    let count = surface.cell_count();
    let mut tallies = vec![0u32; count];
    for &(cell, _) in &pairs {
        tallies[cell as usize] += 1;
    }
    let mut cell_offsets = Vec::with_capacity(count + 1);
    cell_offsets.push(0u32);
    for tally in tallies {
        let next = cell_offsets.last().copied().unwrap_or(0) + tally;
        cell_offsets.push(next);
    }
    let mut cursor = cell_offsets[..count].to_vec();
    let mut seg_ids = vec![0u32; pairs.len()];
    for (cell, seg) in pairs {
        let slot = cursor[cell as usize] as usize;
        seg_ids[slot] = seg;
        cursor[cell as usize] += 1;
    }
    (cell_offsets, seg_ids)
}

fn consider_cell(surface: &ToolSurface, point: [f64; 3], cells: &mut Vec<u32>) {
    let cell = surface.cell_at_direction(point);
    push_unique(cells, cell);
    for neighbor in surface.neighbors(cell) {
        if neighbor != TOOL_SURFACE_NONE {
            push_unique(cells, neighbor);
        }
    }
}

fn push_unique(cells: &mut Vec<u32>, cell: u32) {
    if !cells.contains(&cell) {
        cells.push(cell);
    }
}

/// A segment can carve a cell when it comes within `reach` of that cell.
/// The cell's own radius is half the distance to its farthest neighbour:
/// nominal `cell_size` underestimates cells at cube-face corners.
fn segment_reaches_cell(
    surface: &ToolSurface,
    cell: u32,
    a: [f64; 3],
    b: [f64; 3],
    reach_m: f64,
    radius_m: f64,
) -> bool {
    let centre = surface.direction_of(cell);
    let (_, d2) = chord_t_d2(centre, a, b);
    let mut neighbor = 0.0f64;
    for slot in 0..8 {
        neighbor = neighbor.max(f64::from(surface.neighbor_distance_metres(cell, slot)));
    }
    let cover = if neighbor == 0.0 {
        0.0
    } else {
        0.55 * neighbor
    };
    let limit = (reach_m + cover) / radius_m;
    d2 <= limit * limit
}

fn meander_offset(run: &RunRec, s: f64, slope: f64) -> f64 {
    let flat = 1.0 - smoothstep(FLAT_SLOPE_LO, FLAT_SLOPE_HI, slope);
    if flat == 0.0 {
        return 0.0;
    }
    let taper = end_taper(s, f64::from(run.length_m), f64::from(run.half_cell_m));
    if taper == 0.0 {
        return 0.0;
    }
    let amp = f64::from(run.amp_cap_m) * flat * taper;
    if amp == 0.0 {
        return 0.0;
    }
    let lambda = f64::from(run.lambda_m);
    let n1 = value_noise(run.seed, s / lambda);
    let n2 = value_noise(run.seed ^ MEANDER_OCTAVE_2_MIX, s / (lambda / 2.7));
    let n = (n1 + MEANDER_OCTAVE_2 * n2) / (1.0 + MEANDER_OCTAVE_2);
    amp * n
}

fn end_taper(s: f64, length: f64, half_cell: f64) -> f64 {
    if half_cell <= 0.0 {
        return 1.0;
    }
    smoothstep(0.0, half_cell, s) * smoothstep(0.0, half_cell, length - s)
}

fn value_noise(seed: u64, x: f64) -> f64 {
    let i = x.floor() as i64;
    let f = x - i as f64;
    let u = f * f * (3.0 - 2.0 * f);
    let h0 = hash_unit(seed, i);
    let h1 = hash_unit(seed, i + 1);
    h0 + (h1 - h0) * u
}

fn hash_unit(seed: u64, index: i64) -> f64 {
    let mixed = splitmix64(seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    (mixed >> 11) as f64 / ((1u64 << 53) as f64) * 2.0 - 1.0
}

fn meander_seed(domain: u64, source_cell: u32) -> u64 {
    let mut hash = domain;
    for byte in b"river:c" {
        hash = splitmix64(hash ^ u64::from(*byte));
    }
    for byte in source_cell.to_string().bytes() {
        hash = splitmix64(hash ^ u64::from(byte));
    }
    hash
}

/// Cell-centre runs. A run ends on the first ocean or lake centre and the next
/// run starts again on that lake centre, matching the vector lines.
fn river_runs(surface: &ToolSurface, river: &daena_physical::engine::SurfaceRiver) -> Vec<RawRun> {
    let mut runs = Vec::new();
    let mut current = Vec::new();
    let mut previous = None;
    let mut last_lake = None;
    for &cell in &river.cells {
        if previous == Some(cell) {
            continue;
        }
        previous = Some(cell);
        if surface.ocean(cell) {
            if !current.is_empty() {
                current.push(surface.direction_of(cell));
                keep_run(&mut runs, &mut current, true);
            }
            break;
        }
        if surface.wet_lake(cell) {
            if !current.is_empty() {
                current.push(surface.direction_of(cell));
                keep_run(&mut runs, &mut current, false);
            }
            last_lake = Some(cell);
            continue;
        }
        if current.is_empty() {
            if let Some(lake) = last_lake.take() {
                current.push(surface.direction_of(lake));
            }
        }
        current.push(surface.direction_of(cell));
    }
    keep_run(&mut runs, &mut current, false);
    runs
}

fn keep_run(runs: &mut Vec<RawRun>, current: &mut Vec<[f64; 3]>, ocean_mouth: bool) {
    if current.len() >= 2 {
        runs.push(RawRun {
            points: std::mem::take(current),
            ocean_mouth,
        });
    } else {
        current.clear();
    }
}

fn chaikin_twice(points: &[[f64; 3]]) -> Vec<[f64; 3]> {
    chaikin_once(&chaikin_once(points))
}

fn chaikin_once(points: &[[f64; 3]]) -> Vec<[f64; 3]> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len() * 2);
    out.push(points[0]);
    for pair in points.windows(2) {
        out.push(normalize(mix(pair[0], pair[1], 0.25)));
        out.push(normalize(mix(pair[0], pair[1], 0.75)));
    }
    out.push(*points.last().expect("run has an end"));
    out
}

pub(super) struct ExportedPiece {
    pub lonlat: Vec<[f64; 2]>,
    pub min_lon: f64,
    pub max_lon: f64,
    pub min_lat: f64,
    pub max_lat: f64,
    pub run: u32,
    pub s0: f64,
    pub s1: f64,
}

pub(super) struct ExportedRiver {
    pub id: String,
    pub order: u16,
    pub channel_width_m: f64,
    pub run_start: u32,
    pub run_count: u32,
    pub pieces: Vec<ExportedPiece>,
}

fn push_local_pieces(
    samples: &[(f64, [f64; 2])],
    run: u32,
    pad_m: f64,
    radius_m: f64,
    pieces: &mut Vec<ExportedPiece>,
) {
    for part in split_marked(samples) {
        if part.len() < 2 {
            continue;
        }
        let mut start = 0usize;
        while start + 1 < part.len() {
            let end = (start + EXPORT_PIECE_POINTS - 1).min(part.len() - 1);
            let slice = &part[start..=end];
            let lonlat = slice.iter().map(|(_, lonlat)| *lonlat).collect();
            if let Some(piece) = exported_piece(
                lonlat,
                pad_m,
                radius_m,
                run,
                slice[0].0,
                slice[slice.len() - 1].0,
            ) {
                pieces.push(piece);
            }
            if end + 1 >= part.len() {
                break;
            }
            start = end;
        }
    }
}

fn split_marked(line: &[(f64, [f64; 2])]) -> Vec<Vec<(f64, [f64; 2])>> {
    if line.len() < 2 {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut current = vec![line[0]];
    for pair in line.windows(2) {
        if (pair[0].1[0] - pair[1].1[0]).abs() <= 180.0 {
            current.push(pair[1]);
            continue;
        }
        if current.len() >= 2 {
            parts.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
        current.push(pair[1]);
    }
    if current.len() >= 2 {
        parts.push(current);
    }
    parts
}

fn exported_piece(
    lonlat: Vec<[f64; 2]>,
    pad_m: f64,
    radius_m: f64,
    run: u32,
    s0: f64,
    s1: f64,
) -> Option<ExportedPiece> {
    if lonlat.len() < 2 {
        return None;
    }
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    for point in &lonlat {
        min_lon = min_lon.min(point[0]);
        max_lon = max_lon.max(point[0]);
        min_lat = min_lat.min(point[1]);
        max_lat = max_lat.max(point[1]);
    }
    let mid_lat = ((min_lat + max_lat) * 0.5).to_radians();
    let dlat = (pad_m / radius_m).to_degrees();
    let dlon = pad_m / (radius_m * mid_lat.cos().max(0.05));
    let dlon = dlon.to_degrees();
    Some(ExportedPiece {
        lonlat,
        min_lon: min_lon - dlon,
        max_lon: max_lon + dlon,
        min_lat: min_lat - dlat,
        max_lat: max_lat + dlat,
        run,
        s0,
        s1,
    })
}

fn f64_3(point: [f32; 3]) -> [f64; 3] {
    [
        f64::from(point[0]),
        f64::from(point[1]),
        f64::from(point[2]),
    ]
}

fn chord_t_d2(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> (f64, f64) {
    let abx = b[0] - a[0];
    let aby = b[1] - a[1];
    let abz = b[2] - a[2];
    let apx = p[0] - a[0];
    let apy = p[1] - a[1];
    let apz = p[2] - a[2];
    let ab2 = abx * abx + aby * aby + abz * abz;
    if ab2 < 1e-24 {
        return (0.0, apx * apx + apy * apy + apz * apz);
    }
    let t = ((apx * abx + apy * aby + apz * abz) / ab2).clamp(0.0, 1.0);
    let dx = apx - t * abx;
    let dy = apy - t * aby;
    let dz = apz - t * abz;
    (t, dx * dx + dy * dy + dz * dz)
}

fn chord_frame(a: [f64; 3], b: [f64; 3], t: f64) -> Option<([f64; 3], [f64; 3])> {
    let centre = normalize(mix(a, b, t));
    let ab = sub(b, a);
    let radial = dot(ab, centre);
    let tangent = sub(ab, scale(centre, radial));
    let len2 = dot(tangent, tangent);
    if len2 < 1e-24 {
        return None;
    }
    let tangent = scale(tangent, 1.0 / len2.sqrt());
    let normal = cross(centre, tangent);
    Some((centre, normal))
}

fn great_circle(a: [f64; 3], b: [f64; 3], radius_m: f64) -> f64 {
    dot(a, b).clamp(-1.0, 1.0).acos() * radius_m
}

fn direction_lonlat(direction: [f64; 3]) -> [f64; 2] {
    let point = normalize(direction);
    let lat = point[2].clamp(-1.0, 1.0).asin().to_degrees();
    let lon = point[1].atan2(point[0]).to_degrees();
    [lon, lat]
}

fn lonlat_tuple(direction: [f64; 3]) -> (f64, f64) {
    let lonlat = direction_lonlat(direction);
    (lonlat[0], lonlat[1])
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    let u = 1.0 - t;
    [
        a[0] * u + b[0] * t,
        a[1] * u + b[1] * t,
        a[2] * u + b[2] * t,
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(point: [f64; 3]) -> [f64; 3] {
    let len = dot(point, point).sqrt();
    if len == 0.0 {
        point
    } else {
        scale(point, 1.0 / len)
    }
}

#[cfg(test)]
fn shift_metres(position: [f64; 3], normal: [f64; 3], metres: f64, radius_m: f64) -> [f64; 3] {
    let angle = metres / radius_m;
    normalize(add(
        scale(position, angle.cos()),
        scale(normal, angle.sin()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{
        ground_metres_per_pixel, lon_lat_to_tile, render_imagery_pixels, render_pixels,
        render_terrain_vector_tile, terrain_fixture, TerrainVectorLayer, TerrainVectors,
    };

    /// `BED_DETAIL_K` stays at least the measured 1st-percentile excursion.
    #[test]
    fn detail_factor_percentile_on_the_fixture() {
        let terrain = terrain_fixture();
        let radius = terrain.radius_m();
        let cell = terrain.cell_size_m();
        let count = terrain.surface().cell_count();
        let mut ratios = Vec::new();
        for cell_id in 0..count {
            let cell_id = cell_id as u32;
            if terrain.surface().ocean(cell_id) || terrain.surface().wet_lake(cell_id) {
                continue;
            }
            let centre = terrain.surface().direction_of(cell_id);
            for direction in [centre, offset_direction(centre, 0.25 * cell, radius)] {
                let amplitude = terrain.amplitude_at(direction);
                let height = terrain.detailed_elevation_uncarved(direction, 1.0);
                let canon = terrain.canonical_elevation(direction);
                ratios.push((height - canon) / amplitude);
            }
        }
        ratios.sort_by(f64::total_cmp);
        let n = ratios.len();
        let index = ((0.01 * n as f64).ceil() as usize).saturating_sub(1);
        let percentile = ratios[index];
        let minimum = ratios[0];
        println!(
            "detail/amplitude ground 1 n {n} min {minimum:.4} p01 {percentile:.4} k {BED_DETAIL_K}"
        );
        assert!(
            minimum >= -BED_DETAIL_K,
            "minimum {minimum:.4} is below -k ({BED_DETAIL_K})"
        );
        assert!(
            (-0.35..-0.30).contains(&percentile),
            "1st percentile {percentile:.4} drifted from the measured -0.334"
        );
    }

    #[test]
    fn channel_widths_follow_the_table() {
        assert_eq!(channel_width_m(1), 12.0);
        assert_eq!(channel_width_m(2), 30.0);
        assert_eq!(channel_width_m(3), 70.0);
        assert_eq!(channel_width_m(4), 160.0);
        assert_eq!(channel_width_m(5), 350.0);
        assert!((channel_width_m(6) - 350.0 * 2.2).abs() < 1e-9);
        assert!((channel_width_m(7) - 350.0 * 2.2 * 2.2).abs() < 1e-6);
    }

    #[test]
    fn amplitude_and_valley_stay_inside_half_a_cell() {
        let terrain = terrain_fixture();
        let cell = terrain.cell_size_m();
        for run in &terrain.rivers.runs {
            let amp = f64::from(run.amp_cap_m);
            let valley = f64::from(run.valley_m);
            let lambda = f64::from(run.lambda_m);
            assert!(amp + valley <= 0.5 * cell + 1e-6);
            assert!(amp * std::f64::consts::TAU / lambda <= MEANDER_SLOPE_LIMIT + 1e-9);
            assert!(lambda + 1e-6 >= MIN_MEANDER_M);
            assert!(lambda + 1e-6 >= 12.0 * f64::from(run.width_m));
        }
        let bytes = terrain.river_geometry_bytes();
        assert!(bytes < 8 * 1024 * 1024, "n=64 river geometry {bytes} bytes");
    }

    #[test]
    fn meander_holds_the_run_ends() {
        let terrain = terrain_fixture();
        for (index, run) in terrain.rivers.runs.iter().enumerate() {
            let length = f64::from(run.length_m);
            let start = terrain
                .rivers
                .displaced_direction(index, 0.0)
                .expect("start");
            let end = terrain
                .rivers
                .displaced_direction(index, length)
                .expect("end");
            let start_xyz = normalize(f64_3(terrain.rivers.xyz[run.start as usize]));
            let end_xyz = normalize(f64_3(
                terrain.rivers.xyz[(run.start + run.count - 1) as usize],
            ));
            assert!(
                great_circle(start, start_xyz, terrain.radius_m()) < 1.0,
                "run {index} start moved"
            );
            assert!(
                great_circle(end, end_xyz, terrain.radius_m()) < 1.0,
                "run {index} end moved"
            );
        }
    }

    #[test]
    fn analytic_samples_sit_on_the_carve_centre_line() {
        let terrain = terrain_fixture();
        let mut checked = 0usize;
        for (index, run) in terrain.rivers.runs.iter().enumerate() {
            if terrain.rivers.rivers[run.river as usize].order < 2 {
                continue;
            }
            let length = f64::from(run.length_m);
            let mut s = 0.0;
            while s <= length {
                let direction = terrain
                    .rivers
                    .displaced_direction(index, s)
                    .expect("sample");
                let cell = terrain.surface().cell_at_direction(direction);
                let distance = terrain.rivers.displaced_distance(direction, cell);
                let along = terrain.rivers.distance_along_run(direction, index);
                let listed = terrain.rivers.listed_segments(cell);
                let wet = terrain.surface().wet_lake(cell);
                let ocean = terrain.surface().ocean(cell);
                assert!(
                    distance < 1.0,
                    "run {index} s {s} indexed {distance} m along {along} m cell {cell} wet {wet} ocean {ocean} segs {listed}"
                );
                checked += 1;
                s += 250.0;
            }
        }
        assert!(checked > 20, "expected order >= 2 samples, got {checked}");
    }

    #[test]
    fn order_one_rivers_do_not_carve_at_zoom_5() {
        let terrain = terrain_fixture();
        let mut checked = 0usize;
        for (index, run) in terrain.rivers.runs.iter().enumerate() {
            if terrain.rivers.rivers[run.river as usize].order != 1 {
                continue;
            }
            let length = f64::from(run.length_m);
            let mut s = length * 0.5;
            if length > terrain.cell_size_m() {
                s = terrain.cell_size_m();
            }
            let Some(direction) = terrain.rivers.displaced_direction(index, s) else {
                continue;
            };
            let cell = terrain.surface().cell_at_direction(direction);
            if terrain.surface().wet_lake(cell) || terrain.surface().ocean(cell) {
                continue;
            }
            let (lat, _) = lonlat_of(direction);
            let ground = ground_metres_per_pixel(5, lat.to_radians(), terrain.radius_m());
            // A wider river in the same cell may still carve. Order 1 itself
            // is below both fade thresholds at every Web Mercator z5 pixel.
            if !terrain.rivers.cell_is_all_faded(cell, ground) {
                continue;
            }
            let carved = terrain.sample_pixel(direction, ground).0;
            let plain = terrain.detailed_elevation_uncarved(direction, ground);
            assert!(
                (carved - plain).abs() < 1e-4,
                "order-1 run {index} changed by {} m at z5",
                carved - plain
            );
            checked += 1;
        }
        assert!(checked > 0, "fixture has no order-1 river");
    }

    /// Where the valley is wider than 3 px (`V > 3 · ground`), the carved centre
    /// is at most both uncarved shoulders at `±V`, within 0.5 m.
    ///
    /// The sea + 1 m floor holds some thalwegs. A shoulder there can sit a few
    /// metres lower, and the cut does not go through that floor. Those samples
    /// are counted on their own.
    #[test]
    fn rivers_stay_in_valleys_at_high_zoom() {
        let terrain = terrain_fixture();
        let (max_segs, nonempty, listed) = terrain.rivers.segment_list_stats();
        let mean = if nonempty == 0 {
            0.0
        } else {
            listed as f64 / f64::from(nonempty)
        };
        println!("n=64 index max {max_segs} nonempty {nonempty} mean {mean:.2}");
        let sea_floor = terrain.sea_m + BED_ABOVE_SEA_M;
        let mut checked = 0usize;
        let mut failed = 0usize;
        let mut floor_bound = 0usize;
        let mut worst = 0.0f64;
        let mut worst_msg = String::new();
        for zoom in [10u32, 12] {
            for (index, run) in terrain.rivers.runs.iter().enumerate() {
                let order = terrain.rivers.rivers[run.river as usize].order;
                if order < 2 {
                    continue;
                }
                let length = f64::from(run.length_m);
                let mut s = 0.0;
                while s <= length {
                    let Some(frame) = terrain.rivers.frame_at(index, s) else {
                        s += 400.0;
                        continue;
                    };
                    let (lat, _) = lonlat_of(frame.position);
                    let ground =
                        ground_metres_per_pixel(zoom, lat.to_radians(), terrain.radius_m());
                    if frame.valley_m <= 3.0 * ground {
                        s += 400.0;
                        continue;
                    }
                    let centre_cell = terrain.surface().cell_at_direction(frame.position);
                    if terrain.surface().wet_lake(centre_cell)
                        || terrain.surface().ocean(centre_cell)
                    {
                        s += 400.0;
                        continue;
                    }
                    let plus = shift_metres(
                        frame.position,
                        frame.normal,
                        frame.valley_m,
                        terrain.radius_m(),
                    );
                    let minus = shift_metres(
                        frame.position,
                        frame.normal,
                        -frame.valley_m,
                        terrain.radius_m(),
                    );
                    if water_cell(terrain, plus) || water_cell(terrain, minus) {
                        s += 400.0;
                        continue;
                    }
                    let (plain, carved) = terrain.carve_pair(frame.position, ground);
                    assert!(
                        carved <= plain + 1e-3,
                        "carve raised z{zoom} order {order} s {s:.0} by {}",
                        carved - plain
                    );
                    // Shoulders are the uncarved field. At `±V` this river's blend is
                    // already back to that field; a second river must not decide it.
                    let plus_h = terrain.carve_pair(plus, ground).0;
                    let minus_h = terrain.carve_pair(minus, ground).0;
                    checked += 1;
                    let over = (carved - plus_h).max(carved - minus_h);
                    if over > 0.5 {
                        let held = plain > sea_floor && carved <= sea_floor + 0.5;
                        if held {
                            floor_bound += 1;
                        } else {
                            failed += 1;
                        }
                        if over > worst {
                            worst = over;
                            worst_msg = format!(
                                "z{zoom} order {order} s {s:.0} carved {carved:.2} plain {plain:.2} +V {plus_h:.2} -V {minus_h:.2} over {over:.2} floor {held}"
                            );
                        }
                    }
                    s += 400.0;
                }
            }
        }
        let floor_rate = if checked == 0 {
            1.0
        } else {
            floor_bound as f64 / checked as f64
        };
        println!(
            "valley checks {checked} unbound failures {failed} sea-floor {floor_bound} ({floor_rate:.6}) worst {worst:.2} m {worst_msg}"
        );
        assert!(checked > 10, "valley checks {checked}");
        assert!(
            failed == 0,
            "valley failures {failed}/{checked} worst {worst_msg}"
        );
    }

    #[test]
    fn vector_line_matches_the_carve_centre_line() {
        let terrain = terrain_fixture();
        let vectors = TerrainVectors::new(terrain);
        let run = terrain
            .rivers
            .runs
            .iter()
            .enumerate()
            .filter(|(_, run)| terrain.rivers.rivers[run.river as usize].order >= 2)
            .max_by(|(_, a), (_, b)| a.length_m.total_cmp(&b.length_m))
            .map(|(index, _)| index)
            .expect("order >= 2 river");
        let length = f64::from(terrain.rivers.runs[run].length_m);
        let direction = terrain
            .rivers
            .displaced_direction(run, length * 0.5)
            .expect("midpoint");
        let (lon, lat) = lonlat_of(direction);
        let (x, y) = lon_lat_to_tile(lon, lat, 12);
        let bytes =
            render_terrain_vector_tile(terrain, &vectors, TerrainVectorLayer::Rivers, 12, x, y)
                .expect("tile");
        let features = serde_json::from_slice::<serde_json::Value>(&bytes).expect("json");
        let features = features["features"].as_array().expect("features");
        assert!(!features.is_empty(), "river tile {x}/{y} is empty");
        let mut checked = 0usize;
        for feature in features {
            let width = feature["properties"]["channel_width_m"].as_f64().unwrap();
            assert!(width > 0.0);
            let line = feature["geometry"]["coordinates"].as_array().unwrap();
            for pair in line.windows(2) {
                for t in [0.0, 0.5, 1.0] {
                    let lon = pair[0][0].as_f64().unwrap()
                        + t * (pair[1][0].as_f64().unwrap() - pair[0][0].as_f64().unwrap());
                    let lat = pair[0][1].as_f64().unwrap()
                        + t * (pair[1][1].as_f64().unwrap() - pair[0][1].as_f64().unwrap());
                    let direction = direction_from_degrees(lon, lat);
                    let cell = terrain.surface().cell_at_direction(direction);
                    let distance = terrain.rivers.displaced_distance(direction, cell);
                    let ground = ground_metres_per_pixel(12, lat.to_radians(), terrain.radius_m());
                    assert!(
                        distance <= 0.5 * ground + 2.0,
                        "vector point {lon},{lat} is {distance:.1} m off the centre-line (half pixel {:.1})",
                        0.5 * ground
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 4, "checked {checked} vector samples");
    }

    #[test]
    fn river_tiles_match_across_threads_and_seams() {
        use rayon::ThreadPoolBuilder;

        let terrain = terrain_fixture();
        let (lon, lat) = terrain
            .river_site(Some(2))
            .or_else(|| terrain.river_site(None))
            .expect("fixture has a river");
        let z = 11u32;
        let (x, y) = lon_lat_to_tile(lon, lat, z);
        let once = render_pixels(terrain, &relief_style(), z, x * 256, y * 256, 256, 256, 1);
        let pool = ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("pool");
        let serial = pool
            .install(|| render_pixels(terrain, &relief_style(), z, x * 256, y * 256, 256, 256, 1));
        assert_eq!(
            once, serial,
            "relief river tile depends on the thread count"
        );
        let imagery = render_imagery_pixels(terrain, z, x * 256, y * 256, 256, 256, 1);
        let imagery_serial =
            pool.install(|| render_imagery_pixels(terrain, z, x * 256, y * 256, 256, 256, 1));
        assert_eq!(
            imagery, imagery_serial,
            "imagery river tile depends on the thread count"
        );

        let gx = x.saturating_sub(0) * 256;
        let gy = y * 256;
        let wide = render_pixels(terrain, &relief_style(), z, gx, gy, 512, 256, 1);
        let left = render_pixels(terrain, &relief_style(), z, gx, gy, 256, 256, 1);
        let right_x = gx + 256;
        let right = render_pixels(terrain, &relief_style(), z, right_x, gy, 256, 256, 1);
        for row in 0..256usize {
            let wide_row = row * 512 * 4;
            assert_eq!(
                &wide[wide_row..wide_row + 256 * 4],
                &left[row * 256 * 4..(row + 1) * 256 * 4]
            );
            assert_eq!(
                &wide[wide_row + 256 * 4..wide_row + 512 * 4],
                &right[row * 256 * 4..(row + 1) * 256 * 4]
            );
        }
    }

    fn relief_style() -> crate::style::AtlasStyle {
        crate::style::load_style(crate::style::RELIEF_STYLE_ID)
            .expect("relief")
            .0
    }

    fn lonlat_of(direction: [f64; 3]) -> (f64, f64) {
        let lonlat = direction_lonlat(direction);
        (lonlat[0], lonlat[1])
    }

    fn offset_direction(direction: [f64; 3], metres: f64, radius_m: f64) -> [f64; 3] {
        let axis = if direction[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let tangent = normalize(cross(direction, axis));
        let angle = metres / radius_m;
        normalize(add(
            scale(direction, angle.cos()),
            scale(tangent, angle.sin()),
        ))
    }

    fn direction_from_degrees(lon: f64, lat: f64) -> [f64; 3] {
        let lat = lat.to_radians();
        let lon = lon.to_radians();
        let cos_lat = lat.cos();
        [cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()]
    }

    fn water_cell(terrain: &crate::terrain::AtlasTerrain, direction: [f64; 3]) -> bool {
        let cell = terrain.surface().cell_at_direction(direction);
        terrain.surface().wet_lake(cell) || terrain.surface().ocean(cell)
    }
}
