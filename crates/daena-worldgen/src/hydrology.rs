//! Priority-flood drainage, lakes, rivers and ice on the finished surface.
//!
//! Elevation is read and never written. The heap key is `(filled elevation,
//! push sequence)`, so equal levels pop in push order and the network does
//! not depend on thread count. Sums run in that fixed order.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::grid::{CubeSphere, NONE};
use crate::{ClimateSummary, WorldError};

const YEAR_SECONDS: f64 = 3.15576e7;
const SPILL_GAP_M: f64 = 1.0e-3;
const ICE_SHEET_M2: f64 = 2.0e11;

/// Fresh water that spills, a closed salt lake, or a salt flat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LakeKind {
    Fresh = 0,
    Saline = 1,
    SaltFlat = 2,
}

/// One settled lake. Volumes are cubic metres per year except `volume_m3`.
#[derive(Clone, Debug)]
pub struct Lake {
    pub id: u32,
    pub kind: LakeKind,
    /// Water surface, metres. The spill elevation when the lake is full.
    pub level_m: f64,
    pub area_m2: f64,
    /// `Σ (level − h) · area` over the flooded cells.
    pub volume_m3: f64,
    /// Water removed before the remainder is passed downstream, m³/yr.
    pub evaporated_m3: f64,
    /// m³/yr passed to the spill cell. Zero when the lake is closed.
    pub outflow_m3: f64,
    /// m³/yr that reached the depression exit before evaporation.
    pub inflow_m3: f64,
}

/// Where a traced river stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RiverEnd {
    Ocean = 0,
    Lake = 1,
    Confluence = 2,
}

/// One path from a river head to the ocean, a lake, or a cell already traced.
#[derive(Clone, Debug)]
pub struct RiverSegment {
    pub cells: Vec<u32>,
    pub end: RiverEnd,
    pub max_discharge_m3s: f64,
}

/// Drainage products. `receiver[k]` is the downstream cell, or `k` at a root.
#[derive(Clone, Debug)]
pub struct Hydrology {
    pub lakes: Vec<Lake>,
    pub rivers: Vec<RiverSegment>,
    /// `NONE` when the cell is not under a settled lake.
    pub lake_id: Vec<u32>,
    /// Discharge after lake settlement, m³/s. Ocean cells hold what flowed in.
    pub discharge_m3s: Vec<f64>,
    /// `0` none, `1` glacier, `2` ice sheet.
    pub ice_cover: Vec<u8>,
    /// Priority-flood receiver. Ocean roots point at themselves.
    pub receiver: Vec<u32>,
    /// Priority-flood surface, metres.
    pub filled_m: Vec<f32>,
    /// Pop order. Receivers stand before their donors. Read by the unit tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) pop_order: Vec<u32>,
    /// Exit cell of a settled lake. Read by the unit tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) lake_exit: Vec<u8>,
}

/// `(filled elevation, push sequence)` min-heap. `Ord` is reversed for [`BinaryHeap`].
#[derive(Clone, Copy)]
struct FloodKey {
    elev: f64,
    seq: u64,
    cell: u32,
}

impl PartialEq for FloodKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for FloodKey {}

impl PartialOrd for FloodKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FloodKey {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .elev
            .total_cmp(&self.elev)
            .then_with(|| other.seq.cmp(&self.seq))
            .then_with(|| other.cell.cmp(&self.cell))
    }
}

struct Depression {
    spill: u32,
    exit: u32,
    cells: Vec<u32>,
}

struct Flood {
    receiver: Vec<u32>,
    filled: Vec<f64>,
    pop_order: Vec<u32>,
    depressions: Vec<Depression>,
}

/// Build lakes, rivers and ice. `river_min_m3s` is the river threshold.
pub(crate) fn build(
    grid: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    climate: &ClimateSummary,
    river_min_m3s: f64,
) -> Result<Hydrology, WorldError> {
    let n = grid.cell_count();
    if elevation.len() != n
        || ocean.len() != n
        || climate.runoff_mm.len() != n
        || climate.p_annual_mm.len() != n
        || climate.pet_annual_mm.len() != n
        || climate.ice_balance_mm.len() != n
    {
        return Err(WorldError::Internal(
            "hydrology inputs do not match the terrain grid".to_string(),
        ));
    }
    let flood = priority_flood(grid, elevation, ocean)?;
    let (lakes, lake_id, lake_exit, discharge) =
        settle_water(grid, elevation, ocean, climate, &flood);
    let ice_cover = cover_ice(grid, ocean, &climate.ice_balance_mm, &flood);
    let rivers = trace_rivers(ocean, &lake_id, &flood.receiver, &discharge, river_min_m3s);
    let filled_m: Vec<f32> = flood.filled.iter().map(|value| *value as f32).collect();
    Ok(Hydrology {
        lakes,
        rivers,
        lake_id,
        discharge_m3s: discharge,
        ice_cover,
        receiver: flood.receiver,
        filled_m,
        pop_order: flood.pop_order,
        lake_exit,
    })
}

fn priority_flood(grid: &CubeSphere, elevation: &[f32], ocean: &[u8]) -> Result<Flood, WorldError> {
    let n = grid.cell_count();
    let mut receiver = vec![NONE; n];
    let mut filled = vec![0.0; n];
    let mut visited = vec![false; n];
    let mut heap = BinaryHeap::new();
    let mut seq = 0u64;
    let mut seeded = false;
    for cell in 0..n {
        if ocean[cell] != 1 || !touches_land(grid, ocean, cell as u32) {
            continue;
        }
        seeded = true;
        visited[cell] = true;
        let elev = f64::from(elevation[cell]);
        filled[cell] = elev;
        receiver[cell] = cell as u32;
        seq += 1;
        heap.push(FloodKey {
            elev,
            seq,
            cell: cell as u32,
        });
    }
    if !seeded {
        if let Some(root) = lowest_land(elevation, ocean) {
            seeded = true;
            visited[root] = true;
            let elev = f64::from(elevation[root]);
            filled[root] = elev;
            receiver[root] = root as u32;
            seq += 1;
            heap.push(FloodKey {
                elev,
                seq,
                cell: root as u32,
            });
        }
    }
    let mut pop_order = Vec::with_capacity(n);
    let mut depressions: Vec<Depression> = Vec::new();
    let mut member = vec![NONE; n];
    while let Some(item) = heap.pop() {
        let c = item.cell;
        let ci = c as usize;
        pop_order.push(c);
        let mut fresh = Vec::new();
        for nb in grid.neighbors(c) {
            if nb == NONE {
                continue;
            }
            let k = nb as usize;
            if visited[k] || ocean[k] == 1 {
                continue;
            }
            visited[k] = true;
            let hk = f64::from(elevation[k]);
            let fk = hk.max(filled[ci]);
            filled[k] = fk;
            receiver[k] = c;
            seq += 1;
            heap.push(FloodKey {
                elev: fk,
                seq,
                cell: nb,
            });
            if hk < filled[ci] - SPILL_GAP_M {
                fresh.push(nb);
            }
        }
        if fresh.is_empty() {
            continue;
        }
        // Cells pushed while popping `c` share one depression. If `c` is
        // already inside one, they join it, so a nested pit takes the
        // outermost spill. Direct children of a new spill are re-pointed at
        // the first of them: a jump across the lake to that exit cell.
        if member[ci] != NONE {
            let id = member[ci];
            for k in fresh {
                member[k as usize] = id;
                depressions[id as usize].cells.push(k);
            }
        } else {
            let exit = fresh[0];
            let id = depressions.len() as u32;
            for (i, k) in fresh.iter().copied().enumerate() {
                member[k as usize] = id;
                if i > 0 {
                    receiver[k as usize] = exit;
                }
            }
            depressions.push(Depression {
                spill: c,
                exit,
                cells: fresh,
            });
        }
    }
    if seeded {
        for cell in 0..n {
            if ocean[cell] == 0 && !visited[cell] {
                return Err(WorldError::Internal(
                    "priority flood missed a land cell".to_string(),
                ));
            }
        }
    }
    Ok(Flood {
        receiver,
        filled,
        pop_order,
        depressions,
    })
}

fn touches_land(grid: &CubeSphere, ocean: &[u8], cell: u32) -> bool {
    grid.neighbors(cell)
        .into_iter()
        .any(|nb| nb != NONE && ocean[nb as usize] == 0)
}

fn lowest_land(elevation: &[f32], ocean: &[u8]) -> Option<usize> {
    let mut best: Option<(f64, usize)> = None;
    for (cell, &h) in elevation.iter().enumerate() {
        if ocean[cell] != 0 {
            continue;
        }
        let height = f64::from(h);
        match best {
            None => best = Some((height, cell)),
            Some((old, id)) => {
                let order = height.total_cmp(&old);
                if order == Ordering::Less || (order == Ordering::Equal && cell < id) {
                    best = Some((height, cell));
                }
            }
        }
    }
    best.map(|(_, cell)| cell)
}

fn settle_water(
    grid: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    climate: &ClimateSummary,
    flood: &Flood,
) -> (Vec<Lake>, Vec<u32>, Vec<u8>, Vec<f64>) {
    let n = grid.cell_count();
    let mut q = vec![0.0; n];
    let mut lakes = Vec::new();
    let mut lake_id = vec![NONE; n];
    let mut lake_exit = vec![0u8; n];
    let mut exit_of = vec![NONE; n];
    for (id, dep) in flood.depressions.iter().enumerate() {
        exit_of[dep.exit as usize] = id as u32;
    }
    for &cell in flood.pop_order.iter().rev() {
        let c = cell as usize;
        if ocean[c] == 0 {
            q[c] += runoff_m3(grid, climate, c);
        }
        if exit_of[c] != NONE {
            let dep = &flood.depressions[exit_of[c] as usize];
            let inflow = q[c];
            let basin = BasinClimate {
                grid,
                elevation,
                pet: &climate.pet_annual_mm,
                precip: &climate.p_annual_mm,
                spill_level: flood.filled[dep.spill as usize],
            };
            if let Some(lake) =
                settle_depression(&basin, dep, inflow, lakes.len() as u32, &mut lake_id)
            {
                lake_exit[c] = 1;
                let dest = flood.receiver[c];
                if dest != cell {
                    q[dest as usize] += lake.outflow_m3;
                }
                lakes.push(lake);
            } else if flood.receiver[c] != cell {
                let dest = flood.receiver[c] as usize;
                q[dest] += q[c];
            }
        } else if flood.receiver[c] != cell {
            let dest = flood.receiver[c] as usize;
            q[dest] += q[c];
        }
    }
    let discharge = q.iter().map(|value| value / YEAR_SECONDS).collect();
    (lakes, lake_id, lake_exit, discharge)
}

fn runoff_m3(grid: &CubeSphere, climate: &ClimateSummary, cell: usize) -> f64 {
    f64::from(climate.runoff_mm[cell]).max(0.0) / 1000.0 * grid.area_m2(cell as u32)
}

struct BasinClimate<'a> {
    grid: &'a CubeSphere,
    elevation: &'a [f32],
    pet: &'a [f32],
    precip: &'a [f32],
    spill_level: f64,
}

fn settle_depression(
    basin: &BasinClimate<'_>,
    dep: &Depression,
    inflow: f64,
    id: u32,
    lake_id: &mut [u32],
) -> Option<Lake> {
    if inflow == 0.0 || dep.cells.is_empty() {
        return None;
    }
    let mut deficit = 0.0;
    let mut area_full = 0.0;
    for &cell in &dep.cells {
        let i = cell as usize;
        deficit += f64::from(basin.pet[i]) - f64::from(basin.precip[i]);
        area_full += basin.grid.area_m2(cell);
    }
    let e_net = (deficit / dep.cells.len() as f64).max(0.0) / 1000.0;
    let demand = e_net * area_full;
    if demand <= inflow {
        let level = basin.spill_level;
        let mut volume = 0.0;
        for &cell in &dep.cells {
            lake_id[cell as usize] = id;
            let depth = level - f64::from(basin.elevation[cell as usize]);
            if depth > 0.0 {
                volume += depth * basin.grid.area_m2(cell);
            }
        }
        return Some(Lake {
            id,
            kind: LakeKind::Fresh,
            level_m: level,
            area_m2: area_full,
            volume_m3: volume,
            evaporated_m3: demand,
            outflow_m3: inflow - demand,
            inflow_m3: inflow,
        });
    }
    let mut order: Vec<u32> = dep.cells.clone();
    order.sort_by(|&a, &b| {
        let ha = f64::from(basin.elevation[a as usize]);
        let hb = f64::from(basin.elevation[b as usize]);
        ha.total_cmp(&hb).then_with(|| a.cmp(&b))
    });
    let smallest = basin.grid.area_m2(order[0]);
    if e_net <= 0.0 {
        return None;
    }
    let target = inflow / e_net;
    if target < smallest {
        let cell = order[0];
        lake_id[cell as usize] = id;
        return Some(Lake {
            id,
            kind: LakeKind::SaltFlat,
            level_m: f64::from(basin.elevation[cell as usize]),
            area_m2: smallest,
            volume_m3: 0.0,
            evaporated_m3: inflow,
            outflow_m3: 0.0,
            inflow_m3: inflow,
        });
    }
    let mut area = 0.0;
    let mut taken = Vec::new();
    let mut level = f64::from(basin.elevation[order[0] as usize]);
    for cell in order {
        area += basin.grid.area_m2(cell);
        level = f64::from(basin.elevation[cell as usize]);
        taken.push(cell);
        if area >= target {
            break;
        }
    }
    let mut volume = 0.0;
    for cell in &taken {
        lake_id[*cell as usize] = id;
        let depth = level - f64::from(basin.elevation[*cell as usize]);
        if depth > 0.0 {
            volume += depth * basin.grid.area_m2(*cell);
        }
    }
    Some(Lake {
        id,
        kind: LakeKind::Saline,
        level_m: level,
        area_m2: area,
        volume_m3: volume,
        evaporated_m3: inflow,
        outflow_m3: 0.0,
        inflow_m3: inflow,
    })
}

/// `q_ice` follows the flood receivers. A positive-balance region of at least
/// `2·10^11 m²` is an ice sheet; other cells with `q_ice > 0` are glaciers.
fn cover_ice(grid: &CubeSphere, ocean: &[u8], ice_balance_mm: &[f32], flood: &Flood) -> Vec<u8> {
    let n = grid.cell_count();
    let mut q = vec![0.0; n];
    for &cell in flood.pop_order.iter().rev() {
        let c = cell as usize;
        if ocean[c] == 1 {
            continue;
        }
        q[c] += f64::from(ice_balance_mm[c]) / 1000.0 * grid.area_m2(cell);
        if q[c] > 0.0 {
            let dest = flood.receiver[c];
            if dest != cell {
                q[dest as usize] += q[c];
            }
        }
    }
    let mut cover = vec![0u8; n];
    let mut seen = vec![false; n];
    let mut region = Vec::new();
    for start in 0..n {
        if seen[start] || ocean[start] == 1 || ice_balance_mm[start] <= 0.0 {
            continue;
        }
        region.clear();
        region.push(start);
        seen[start] = true;
        let mut area = 0.0;
        let mut head = 0usize;
        while head < region.len() {
            let c = region[head];
            head += 1;
            area += grid.area_m2(c as u32);
            for nb in grid.neighbors(c as u32) {
                if nb == NONE {
                    continue;
                }
                let j = nb as usize;
                if seen[j] || ocean[j] == 1 || ice_balance_mm[j] <= 0.0 {
                    continue;
                }
                seen[j] = true;
                region.push(j);
            }
        }
        if area >= ICE_SHEET_M2 {
            for &c in &region {
                cover[c] = 2;
            }
        }
    }
    for cell in 0..n {
        if cover[cell] == 0 && ocean[cell] == 0 && q[cell] > 0.0 {
            cover[cell] = 1;
        }
    }
    cover
}

fn trace_rivers(
    ocean: &[u8],
    lake_id: &[u32],
    receiver: &[u32],
    discharge: &[f64],
    river_min_m3s: f64,
) -> Vec<RiverSegment> {
    let n = ocean.len();
    let mut is_river = vec![false; n];
    for cell in 0..n {
        is_river[cell] = ocean[cell] == 0
            && lake_id[cell] == NONE
            && discharge[cell] >= river_min_m3s
            && discharge[cell].is_finite();
    }
    let mut has_donor = vec![false; n];
    for cell in 0..n {
        if !is_river[cell] {
            continue;
        }
        let dest = receiver[cell] as usize;
        if dest < n && is_river[dest] {
            has_donor[dest] = true;
        }
    }
    let mut traced = vec![false; n];
    let mut rivers = Vec::new();
    for cell in 0..n {
        if !is_river[cell] || has_donor[cell] || traced[cell] {
            continue;
        }
        let mut cells = vec![cell as u32];
        traced[cell] = true;
        let mut max_q = discharge[cell];
        let mut cursor = cell;
        let mut end = RiverEnd::Confluence;
        for _ in 0..n {
            let next = receiver[cursor] as usize;
            if next >= n || next == cursor {
                break;
            }
            if ocean[next] == 1 {
                cells.push(next as u32);
                end = RiverEnd::Ocean;
                break;
            }
            if lake_id[next] != NONE {
                cells.push(next as u32);
                end = RiverEnd::Lake;
                break;
            }
            if !is_river[next] {
                end = RiverEnd::Confluence;
                break;
            }
            max_q = max_q.max(discharge[next]);
            if traced[next] {
                cells.push(next as u32);
                end = RiverEnd::Confluence;
                break;
            }
            cells.push(next as u32);
            traced[next] = true;
            cursor = next;
        }
        rivers.push(RiverSegment {
            cells,
            end,
            max_discharge_m3s: max_q,
        });
    }
    rivers.sort_by(|a, b| {
        a.cells
            .first()
            .cmp(&b.cells.first())
            .then_with(|| (a.end as u8).cmp(&(b.end as u8)))
    });
    rivers
}

impl Hydrology {
    pub(crate) fn hash_bits(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for lake in &self.lakes {
            hash = fnv_byte(hash, lake.kind as u8);
            hash = fnv_f64(hash, lake.level_m);
            hash = fnv_f64(hash, lake.area_m2);
            hash = fnv_f64(hash, lake.volume_m3);
            hash = fnv_f64(hash, lake.evaporated_m3);
            hash = fnv_f64(hash, lake.outflow_m3);
            hash = fnv_f64(hash, lake.inflow_m3);
        }
        for river in &self.rivers {
            hash = fnv_byte(hash, river.end as u8);
            hash = fnv_f64(hash, river.max_discharge_m3s);
            for cell in &river.cells {
                hash = fnv_u32(hash, *cell);
            }
        }
        for id in &self.lake_id {
            hash = fnv_u32(hash, *id);
        }
        for value in &self.discharge_m3s {
            hash = fnv_f64(hash, *value);
        }
        for flag in &self.ice_cover {
            hash = fnv_byte(hash, *flag);
        }
        for cell in &self.receiver {
            hash = fnv_u32(hash, *cell);
        }
        hash
    }

    /// Great-circle length of every segment, metres.
    pub fn river_length_m(&self, grid: &CubeSphere) -> f64 {
        let mut total = 0.0;
        for river in &self.rivers {
            for pair in river.cells.windows(2) {
                total += cell_distance(grid, pair[0], pair[1]);
            }
        }
        total
    }
}

fn cell_distance(grid: &CubeSphere, a: u32, b: u32) -> f64 {
    let nbs = grid.neighbors(a);
    for (k, nb) in nbs.iter().enumerate() {
        if *nb == b {
            return f64::from(grid.neighbor_distance(a, k));
        }
    }
    let pa = grid.position(a);
    let pb = grid.position(b);
    let dot = (pa[0] * pb[0] + pa[1] * pb[1] + pa[2] * pb[2]).clamp(-1.0, 1.0);
    dot.acos() * grid.radius_m
}

fn fnv_byte(mut hash: u64, byte: u8) -> u64 {
    hash ^= u64::from(byte);
    hash.wrapping_mul(0x0000_0100_0000_01b3)
}

fn fnv_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash = fnv_byte(hash, byte);
    }
    hash
}

fn fnv_f64(mut hash: u64, value: f64) -> u64 {
    for byte in value.to_bits().to_le_bytes() {
        hash = fnv_byte(hash, byte);
    }
    hash
}
