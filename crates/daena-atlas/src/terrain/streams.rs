//! Tributary trees below engine resolution.
//!
//! One deterministic tree per canonical land cell (not ocean, not a wet lake,
//! not ice). Nodes sit on a jittered `k × k` grid in the tangent frame at the
//! cell centre, `k = clamp(round(cell_m / 1200), 4, 16)`. Heights come from
//! [`AtlasTerrain::sample_pixel`] at `ground_m = spacing / 2`, so the tree does
//! not depend on zoom. Each grid node drains by steepest descent. A pit, which
//! has no lower neighbour, breaches to the receiver edge, an engine river within
//! `A + w`, or detailed water by a priority flood: minimum spill, then fewest
//! steps, then node index. Pure inflow nodes are not targets of that routing.
//! An inflow carries the upstream cell's flow, two cells deep, and drains by
//! its own steepest step into the grid or an outlet. The raster is not carved.
//!
//! `R0` and the exit weight sit in a side map. A cold cell samples its own
//! grid, then `R0` of its inflow neighbours and of their inflow neighbours.
//! Finished trees sit in a 2048-cell LRU. The geometry does not depend on
//! which tile asked, on the cache contents, or on the thread count.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use daena_physical::engine::TOOL_SURFACE_NONE;
use rayon::prelude::*;

use super::noise::{domain_seed, splitmix64};
use super::{AtlasTerrain, TerrainCover};

const SPACING_TARGET_M: f64 = 1_200.0;
const K_MIN: u32 = 4;
const K_MAX: u32 = 16;
const JITTER: f64 = 0.35;
const EDGE_SLIDE: f64 = 0.25;
const ARID_MOISTURE: f64 = 0.3;
/// 2048 trees are about 52 MiB at `k = 16` (n=512), and Studio keeps two
/// terrains, so the cap is kept well under the 4096 a z8 viewport could use.
const CACHE_CAP: usize = 2048;
/// Grid samples reused while exit weights are built. Dropped once the tree
/// for that cell is stored. 2048 cores is the scratch cap, not a second tree.
const CORE_CAP: usize = 2048;
/// `R0` and exit weight, 8 bytes of payload per cell. Large enough for the
/// n=64 fixture and for a tile plus its two-cell halo.
const FLOW_CAP: usize = 65_536;
pub(super) const STREAM_NONE: u32 = u32::MAX;

pub(super) const FLAG_INFLOW: u8 = 1;
pub(super) const FLAG_RECEIVER: u8 = 2;
pub(super) const FLAG_RIVER: u8 = 4;
pub(super) const FLAG_WATER: u8 = 8;

#[derive(Clone)]
pub(super) struct StreamNode {
    pub dir: [f64; 3],
    pub height: f64,
    pub flags: u8,
    pub river_at: Option<[f64; 3]>,
    grid: Option<(u16, u16)>,
    /// Upstream cell when this node is that cell's inflow.
    source: Option<u32>,
    /// Package `w` on an inflow. Grid nodes contribute 1. The receiver
    /// outlet contributes 0. Draw accumulation uses this.
    weight: u32,
    /// `R0` of `source`. Boundary accumulation uses this, not `weight`,
    /// so the exit stays two cells deep.
    r0_source: u32,
    source_arid: bool,
}

impl StreamNode {
    pub(super) fn is_outlet(&self) -> bool {
        self.flags & (FLAG_RECEIVER | FLAG_RIVER | FLAG_WATER) != 0
    }

    pub(super) fn is_inflow(&self) -> bool {
        self.flags & FLAG_INFLOW != 0
    }

    fn pinned(&self) -> bool {
        self.is_outlet() || self.is_inflow()
    }
}

pub(super) struct StreamTree {
    pub nodes: Vec<StreamNode>,
    pub down: Vec<u32>,
    /// Boundary accumulation. Inflow seeds are `R0(source)`, grid seeds are
    /// 1, and the receiver outlet seed is 0. `acc[receiver] == exit_w`.
    pub acc: Vec<u32>,
    /// Draw accumulation. Inflow seeds are the upstream package `w`, so a
    /// stream that entered the cell stays visible. Edges into the receiver
    /// emit from `acc` instead, so the package cannot cross `P` on its own.
    pub draw: Vec<u32>,
    pub k: u32,
    pub arid: bool,
    /// Grid nodes that drain to this cell's receiver under grid-only routing.
    pub r0: u32,
    /// `R0(self) + Σ R0(u)` for inflow neighbours whose node reaches the
    /// receiver. This is the weight the downstream cell starts with.
    pub exit_w: u32,
    pub receiver_arid: Option<bool>,
    /// Nodes the flood could not reach, then tied to the nearest outlet.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fallback: u32,
}

/// Grid, receiver outlet, and grid-only routing. No inflow nodes.
struct GridCore {
    nodes: Vec<StreamNode>,
    down: Vec<u32>,
    r0: u32,
    fallback: u32,
    k: u32,
    arid: bool,
    receiver_arid: Option<bool>,
}

struct InflowSpec {
    source: u32,
    dir: [f64; 3],
    weight: u32,
    r0: u32,
    arid: bool,
}

pub(super) struct StreamLine {
    pub id: String,
    pub order: u16,
    pub accumulation: u32,
    pub lonlat: Vec<[f64; 2]>,
}

pub(super) struct StreamCache {
    map: HashMap<u32, Arc<StreamTree>>,
    order: VecDeque<u32>,
    cores: HashMap<u32, Arc<GridCore>>,
    core_order: VecDeque<u32>,
    r0: HashMap<u32, u32>,
    exit_w: HashMap<u32, u32>,
    flow_order: VecDeque<u32>,
}

impl StreamCache {
    pub(super) fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            cores: HashMap::new(),
            core_order: VecDeque::new(),
            r0: HashMap::new(),
            exit_w: HashMap::new(),
            flow_order: VecDeque::new(),
        }
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.cores.clear();
        self.core_order.clear();
        self.r0.clear();
        self.exit_w.clear();
        self.flow_order.clear();
    }

    fn touch(&mut self, cell: u32) -> Option<Arc<StreamTree>> {
        let hit = self.map.get(&cell)?.clone();
        if let Some(pos) = self.order.iter().position(|id| *id == cell) {
            self.order.remove(pos);
        }
        self.order.push_back(cell);
        Some(hit)
    }

    fn insert(&mut self, cell: u32, tree: Arc<StreamTree>) {
        if let Some(slot) = self.map.get_mut(&cell) {
            *slot = tree;
            return;
        }
        while self.order.len() >= CACHE_CAP {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            self.map.remove(&old);
        }
        self.order.push_back(cell);
        self.map.insert(cell, tree);
        self.forget_core(cell);
    }

    fn insert_core(&mut self, cell: u32, core: Arc<GridCore>) {
        if let Some(slot) = self.cores.get_mut(&cell) {
            *slot = core;
            return;
        }
        while self.core_order.len() >= CORE_CAP {
            let Some(old) = self.core_order.pop_front() else {
                break;
            };
            self.cores.remove(&old);
        }
        self.core_order.push_back(cell);
        self.cores.insert(cell, core);
    }

    fn forget_core(&mut self, cell: u32) {
        if self.cores.remove(&cell).is_none() {
            return;
        }
        if let Some(pos) = self.core_order.iter().position(|id| *id == cell) {
            self.core_order.remove(pos);
        }
    }

    fn remember(&mut self, cell: u32, r0: u32, exit_w: u32) {
        self.r0.insert(cell, r0);
        self.exit_w.insert(cell, exit_w);
        self.touch_flow(cell);
    }

    fn remember_r0(&mut self, cell: u32, r0: u32) {
        self.r0.insert(cell, r0);
        self.touch_flow(cell);
    }

    fn touch_flow(&mut self, cell: u32) {
        if let Some(pos) = self.flow_order.iter().position(|id| *id == cell) {
            self.flow_order.remove(pos);
        }
        self.flow_order.push_back(cell);
        while self.flow_order.len() > FLOW_CAP {
            let Some(old) = self.flow_order.pop_front() else {
                break;
            };
            self.r0.remove(&old);
            self.exit_w.remove(&old);
        }
    }
}

fn lock(cache: &Mutex<StreamCache>) -> MutexGuard<'_, StreamCache> {
    cache.lock().unwrap_or_else(|err| err.into_inner())
}

#[cfg(test)]
pub(super) fn clear_cache(terrain: &AtlasTerrain) {
    lock(&terrain.stream_cache).clear();
}

pub(super) fn is_stream_cell(terrain: &AtlasTerrain, cell: u32) -> bool {
    let surface = terrain.surface();
    (cell as usize) < surface.cell_count()
        && !surface.ocean(cell)
        && !surface.wet_lake(cell)
        && !surface.ice(cell)
}

/// `None` below zoom 8. Arid cells (`P/PET < 0.3`) double the threshold.
///
/// z8–9 keep the main stem (`k²/2`). z10 and up use `k`. A lower z12
/// threshold drew two-node links as straight spikes and star junctions on
/// the 1.2 km grid, so z12 shows the z10 network. Arid cells double each
/// value.
pub(super) fn stream_threshold(z: u32, k: u32, arid: bool) -> Option<u32> {
    let base = match z {
        0..=7 => return None,
        8 | 9 => k.saturating_mul(k) / 2,
        _ => k.max(4),
    };
    Some(if arid { base.saturating_mul(2) } else { base })
}

pub(super) fn tree(terrain: &AtlasTerrain, cell: u32) -> Option<Arc<StreamTree>> {
    if !is_stream_cell(terrain, cell) {
        return None;
    }
    if let Some(hit) = lock(&terrain.stream_cache).touch(cell) {
        return Some(hit);
    }
    let built = Arc::new(assemble(terrain, cell, true));
    let mut cache = lock(&terrain.stream_cache);
    if let Some(hit) = cache.touch(cell) {
        return Some(hit);
    }
    cache.remember(cell, built.r0, built.exit_w);
    cache.insert(cell, Arc::clone(&built));
    Some(built)
}

fn arid_of(terrain: &AtlasTerrain, cell: u32) -> bool {
    terrain
        .moisture
        .get(cell as usize)
        .is_some_and(|moisture| f64::from(*moisture) < ARID_MOISTURE)
}

fn is_pure_inflow(node: &StreamNode) -> bool {
    node.is_inflow() && !node.is_outlet()
}

fn blank_node(dir: [f64; 3], flags: u8, grid: Option<(u16, u16)>) -> StreamNode {
    StreamNode {
        dir,
        height: 0.0,
        flags,
        river_at: None,
        grid,
        source: None,
        weight: if flags & FLAG_RECEIVER != 0 { 0 } else { 1 },
        r0_source: 0,
        source_arid: false,
    }
}

fn receiver_index(nodes: &[StreamNode]) -> Option<usize> {
    nodes
        .iter()
        .position(|node| node.flags & FLAG_RECEIVER != 0)
}

fn agreed(k: u32, arid_a: bool, arid_b: bool, z: u32) -> Option<u32> {
    let left = stream_threshold(z, k, arid_a)?;
    let right = stream_threshold(z, k, arid_b)?;
    Some(left.max(right))
}

pub(super) fn grid_k(cell_m: f64) -> u32 {
    let rounded = (cell_m / SPACING_TARGET_M).round() as i32;
    rounded.clamp(K_MIN as i32, K_MAX as i32) as u32
}

fn ground_m(terrain: &AtlasTerrain, k: u32) -> f64 {
    terrain.cell_size_m / f64::from(k) * 0.5
}

/// Grid-only core. Does not read a neighbour's tree, so `R0` can be
/// published before any exit weight asks for it.
fn core(terrain: &AtlasTerrain, cell: u32) -> Option<Arc<GridCore>> {
    if !is_stream_cell(terrain, cell) {
        return None;
    }
    if let Some(hit) = lock(&terrain.stream_cache).cores.get(&cell).cloned() {
        return Some(hit);
    }
    let built = Arc::new(build_core(terrain, cell));
    let mut cache = lock(&terrain.stream_cache);
    if let Some(hit) = cache.cores.get(&cell).cloned() {
        return Some(hit);
    }
    let r0 = built.r0;
    cache.insert_core(cell, Arc::clone(&built));
    cache.remember_r0(cell, r0);
    Some(built)
}

fn r0_of(terrain: &AtlasTerrain, cell: u32) -> u32 {
    if !is_stream_cell(terrain, cell) {
        return 0;
    }
    {
        let cache = lock(&terrain.stream_cache);
        if let Some(value) = cache.r0.get(&cell) {
            return *value;
        }
        if let Some(core) = cache.cores.get(&cell) {
            return core.r0;
        }
    }
    core(terrain, cell).map(|core| core.r0).unwrap_or(0)
}

/// Exit weight of `cell`. Uses `R0` of inflow neighbours, never their exit
/// weight, so the depth stays at two cells and the cache lock is not re-entered.
fn boundary_exit(terrain: &AtlasTerrain, cell: u32) -> u32 {
    if !is_stream_cell(terrain, cell) {
        return 0;
    }
    if let Some(value) = lock(&terrain.stream_cache).exit_w.get(&cell).copied() {
        return value;
    }
    let built = assemble(terrain, cell, false);
    let mut cache = lock(&terrain.stream_cache);
    if let Some(value) = cache.exit_w.get(&cell).copied() {
        return value;
    }
    cache.remember(cell, built.r0, built.exit_w);
    built.exit_w
}

fn inflow_neighbors(terrain: &AtlasTerrain, cell: u32) -> Vec<u32> {
    let surface = terrain.surface();
    let mut neighbors = Vec::new();
    for neighbor in surface.neighbors(cell) {
        if neighbor == TOOL_SURFACE_NONE || neighbor == cell {
            continue;
        }
        if surface.receiver(neighbor) != cell || !is_stream_cell(terrain, neighbor) {
            continue;
        }
        neighbors.push(neighbor);
    }
    neighbors
}

fn inflow_specs(terrain: &AtlasTerrain, cell: u32, package: bool) -> Vec<InflowSpec> {
    let neighbors = inflow_neighbors(terrain, cell);
    let mut specs: Vec<InflowSpec> = neighbors
        .par_iter()
        .map(|neighbor| {
            let r0 = r0_of(terrain, *neighbor);
            let weight = if package {
                boundary_exit(terrain, *neighbor)
            } else {
                r0
            };
            InflowSpec {
                source: *neighbor,
                dir: shared_edge_direction(terrain, *neighbor, cell),
                weight: if package { weight } else { 0 },
                r0,
                arid: arid_of(terrain, *neighbor),
            }
        })
        .filter(|spec| {
            if package {
                spec.weight > 0
            } else {
                spec.r0 > 0
            }
        })
        .collect();
    specs.sort_by_key(|spec| spec.source);
    specs
}

/// `package` is the full tree: inflows carry `w`. Otherwise inflows carry
/// `R0` only, which is what the exit weight is allowed to see.
fn assemble(terrain: &AtlasTerrain, cell: u32, package: bool) -> StreamTree {
    let core = core(terrain, cell).expect("stream cell");
    let mut nodes = core.nodes.clone();
    let base = nodes.len();
    for spec in inflow_specs(terrain, cell, package) {
        push_inflow(&mut nodes, &spec);
    }
    paint_heights(terrain, &mut nodes, ground_m(terrain, core.k), base);
    let mut down = core.down.clone();
    down.resize(nodes.len(), STREAM_NONE);
    let fallback = core
        .fallback
        .saturating_add(route_inflows(&nodes, &mut down));
    let acc = accumulate(&nodes, &down, boundary_seed);
    let draw = accumulate(&nodes, &down, draw_seed);
    let exit_w = receiver_index(&nodes).map(|index| acc[index]).unwrap_or(0);
    StreamTree {
        nodes,
        down,
        acc,
        draw,
        k: core.k,
        arid: core.arid,
        r0: core.r0,
        exit_w,
        receiver_arid: core.receiver_arid,
        fallback,
    }
}

fn build_core(terrain: &AtlasTerrain, cell: u32) -> GridCore {
    let surface = terrain.surface();
    let k = grid_k(terrain.cell_size_m);
    let spacing = terrain.cell_size_m / f64::from(k);
    let ground = spacing * 0.5;
    let domain = domain_seed(terrain.seed, "streams");
    let centre = surface.direction_of(cell);
    let (east, north) = tangent_basis(centre);
    let radius = terrain.radius_m();
    let mut nodes = Vec::new();
    for j in 0..k {
        for i in 0..k {
            let u = (f64::from(i) + 0.5) / f64::from(k) - 0.5;
            let v = (f64::from(j) + 0.5) / f64::from(k) - 0.5;
            let ju = (hash01(domain, cell, i, j.wrapping_mul(2)) * 2.0 - 1.0) * JITTER * spacing;
            let jv = (hash01(domain, cell, i, j.wrapping_mul(2).wrapping_add(1)) * 2.0 - 1.0)
                * JITTER
                * spacing;
            let east_m = u * terrain.cell_size_m + ju;
            let north_m = v * terrain.cell_size_m + jv;
            let dir = offset_dir(centre, east, north, east_m, north_m, radius);
            if surface.cell_at_direction(dir) != cell {
                continue;
            }
            nodes.push(blank_node(dir, 0, Some((i as u16, j as u16))));
        }
    }
    let receiver = surface.receiver(cell);
    if receiver != TOOL_SURFACE_NONE
        && receiver != cell
        && (receiver as usize) < surface.cell_count()
    {
        push_extra(
            &mut nodes,
            shared_edge_direction(terrain, receiver, cell),
            FLAG_RECEIVER,
        );
    }
    paint_heights(terrain, &mut nodes, ground, 0);
    let mut river_cells = vec![cell];
    for neighbor in surface.neighbors(cell) {
        if neighbor != TOOL_SURFACE_NONE {
            river_cells.push(neighbor);
        }
    }
    // `A + w` is narrower than the grid, so a node may sit up to one
    // spacing off the centre-line and still be the node on the river.
    // Only a local minimum of that distance becomes an outlet. A fat
    // band would turn every nearby node into a terminal and the tree
    // would collapse into spokes.
    let mut river_dist = vec![f64::INFINITY; nodes.len()];
    let mut river_hit: Vec<Option<[f64; 3]>> = vec![None; nodes.len()];
    let mut river_band = vec![0.0f64; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if node.grid.is_none() {
            continue;
        }
        let Some(hit) = terrain.rivers.nearest_centreline(&river_cells, node.dir) else {
            continue;
        };
        river_dist[index] = hit.distance_m;
        river_hit[index] = Some(hit.direction);
        river_band[index] = (hit.amp_m + hit.width_m).max(spacing);
    }
    let mut slot = vec![STREAM_NONE; (k * k) as usize];
    for (index, node) in nodes.iter().enumerate() {
        if let Some((i, j)) = node.grid {
            slot[(u32::from(j) * k + u32::from(i)) as usize] = index as u32;
        }
    }
    let k_i = k as i32;
    for index in 0..nodes.len() {
        let Some((i, j)) = nodes[index].grid else {
            continue;
        };
        let dist = river_dist[index];
        if !dist.is_finite() || dist > river_band[index] {
            continue;
        }
        let mut closer = false;
        for dj in -1i32..=1 {
            for di in -1i32..=1 {
                if di == 0 && dj == 0 {
                    continue;
                }
                let ii = i32::from(i) + di;
                let jj = i32::from(j) + dj;
                if ii < 0 || jj < 0 || ii >= k_i || jj >= k_i {
                    continue;
                }
                let other = slot[(jj as u32 * k + ii as u32) as usize];
                if other != STREAM_NONE && river_dist[other as usize] < dist {
                    closer = true;
                }
            }
        }
        if closer {
            continue;
        }
        nodes[index].flags |= FLAG_RIVER;
        nodes[index].river_at = river_hit[index];
    }
    let adj = adjacency(&nodes, k);
    let (down, fallback) = route(&nodes, &adj);
    let arid = arid_of(terrain, cell);
    let r0 = count_r0(&nodes, &down, receiver_index(&nodes));
    let receiver_arid = if receiver != TOOL_SURFACE_NONE
        && receiver != cell
        && (receiver as usize) < surface.cell_count()
    {
        Some(arid_of(terrain, receiver))
    } else {
        None
    };
    GridCore {
        nodes,
        down,
        r0,
        fallback,
        k,
        arid,
        receiver_arid,
    }
}

fn push_extra(nodes: &mut Vec<StreamNode>, dir: [f64; 3], flags: u8) {
    if let Some(existing) = nodes.iter_mut().find(|node| angle(node.dir, dir) < 1e-9) {
        existing.flags |= flags;
        if existing.grid.is_some() && flags & (FLAG_RECEIVER | FLAG_INFLOW) != 0 {
            existing.dir = dir;
            existing.grid = None;
        }
        if flags & FLAG_RECEIVER != 0 {
            existing.weight = 0;
            existing.r0_source = 0;
        }
        return;
    }
    nodes.push(blank_node(dir, flags, None));
}

/// An inflow that lands on the receiver outlet joins that outlet. One that
/// lands on a grid node stays a separate node, so the grid target set does
/// not change.
fn push_inflow(nodes: &mut Vec<StreamNode>, spec: &InflowSpec) {
    let mut outlet = None;
    let mut prior = None;
    for (index, node) in nodes.iter().enumerate() {
        if angle(node.dir, spec.dir) >= 1e-9 {
            continue;
        }
        if node.is_outlet() {
            outlet = Some(index);
            break;
        }
        if node.is_inflow() && prior.is_none() {
            prior = Some(index);
        }
    }
    if let Some(index) = outlet {
        let existing = &mut nodes[index];
        existing.flags |= FLAG_INFLOW;
        existing.source = Some(spec.source);
        existing.weight = spec.weight;
        existing.r0_source = 0;
        existing.source_arid = spec.arid;
        return;
    }
    if let Some(index) = prior {
        let existing = &mut nodes[index];
        existing.weight = existing.weight.saturating_add(spec.weight);
        existing.r0_source = existing.r0_source.saturating_add(spec.r0);
        existing.source_arid |= spec.arid;
        return;
    }
    nodes.push(StreamNode {
        dir: spec.dir,
        height: 0.0,
        flags: FLAG_INFLOW,
        river_at: None,
        grid: None,
        source: Some(spec.source),
        weight: spec.weight,
        r0_source: spec.r0,
        source_arid: spec.arid,
    });
}

fn paint_heights(terrain: &AtlasTerrain, nodes: &mut [StreamNode], ground: f64, start: usize) {
    if start >= nodes.len() {
        return;
    }
    let dirs: Vec<[f64; 3]> = nodes[start..].iter().map(|node| node.dir).collect();
    let sampled: Vec<(f64, TerrainCover)> = dirs
        .par_iter()
        .map(|dir| {
            let (height, cover, _) = terrain.sample_pixel(*dir, ground);
            (height, cover)
        })
        .collect();
    for (node, (height, cover)) in nodes[start..].iter_mut().zip(sampled) {
        node.height = height;
        // A pure inflow is the shared edge point. Marking it water would
        // make it a terminal and the downstream cell would not leave `P`.
        if cover != TerrainCover::Land && !node.is_inflow() {
            node.flags |= FLAG_WATER;
        }
    }
}

/// Great-circle midpoint of the two centres, slid along the shared edge.
///
/// The slide is a hash of the unordered pair, at most a quarter of the
/// centre-to-centre distance. Both cells therefore use the same point.
pub(super) fn shared_edge_direction(terrain: &AtlasTerrain, a: u32, b: u32) -> [f64; 3] {
    let surface = terrain.surface();
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    let p_lo = surface.direction_of(lo);
    let p_hi = surface.direction_of(hi);
    let mid = normalize(add(p_lo, p_hi));
    let across = sub(p_hi, p_lo);
    let mut tangent = cross(mid, across);
    let tlen = norm(tangent);
    if tlen < 1e-18 {
        return mid;
    }
    tangent = scale(tangent, 1.0 / tlen);
    let edge = angle(p_lo, p_hi) * terrain.radius_m();
    let domain = domain_seed(terrain.seed, "streams");
    let unit = hash01(domain, lo, hi, 0x000A_11CE);
    let slide = (unit * 2.0 - 1.0) * EDGE_SLIDE * edge;
    normalize(add(mid, scale(tangent, slide / terrain.radius_m())))
}

fn adjacency(nodes: &[StreamNode], k: u32) -> Vec<Vec<u32>> {
    let n = nodes.len();
    let mut adj = vec![Vec::new(); n];
    for i in 0..n {
        if is_pure_inflow(&nodes[i]) {
            continue;
        }
        let mut best = [(u64::MAX, STREAM_NONE); 8];
        for (j, node) in nodes.iter().enumerate() {
            if i == j || is_pure_inflow(node) {
                continue;
            }
            let key = (ordered_bits(chord_d2(nodes[i].dir, node.dir)), j as u32);
            if key >= best[7] {
                continue;
            }
            best[7] = key;
            best.sort_unstable();
        }
        for (_, j) in best {
            if j != STREAM_NONE {
                link(&mut adj, i as u32, j);
            }
        }
    }
    let mut slot = vec![STREAM_NONE; (k * k) as usize];
    for (index, node) in nodes.iter().enumerate() {
        if let Some((i, j)) = node.grid {
            slot[(u32::from(j) * k + u32::from(i)) as usize] = index as u32;
        }
    }
    let k_i = k as i32;
    for j in 0..k_i {
        for i in 0..k_i {
            let a = slot[(j as u32 * k + i as u32) as usize];
            if a == STREAM_NONE {
                continue;
            }
            for dj in -1i32..=1 {
                for di in -1i32..=1 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let ii = i + di;
                    let jj = j + dj;
                    if ii < 0 || jj < 0 || ii >= k_i || jj >= k_i {
                        continue;
                    }
                    let b = slot[(jj as u32 * k + ii as u32) as usize];
                    if b != STREAM_NONE {
                        link(&mut adj, a, b);
                    }
                }
            }
        }
    }
    adj
}

fn link(adj: &mut [Vec<u32>], a: u32, b: u32) {
    if a == b {
        return;
    }
    if !adj[a as usize].contains(&b) {
        adj[a as usize].push(b);
        adj[b as usize].push(a);
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct FloodKey {
    spill: u64,
    hop: u32,
    via: u32,
    node: u32,
}

/// Downstream neighbour of every node.
///
/// Steepest descent keeps streams in valleys. A pure flood from a high
/// outlet assigns one spill to the whole depression and the index tie-break
/// then walks uphill, so pits breach instead: one minimum-spill path climbs
/// out, and the rest of the basin keeps its downhill edges.
fn route(nodes: &[StreamNode], adj: &[Vec<u32>]) -> (Vec<u32>, u32) {
    let n = nodes.len();
    let mut down = vec![STREAM_NONE; n];
    for i in 0..n {
        if nodes[i].is_outlet() {
            continue;
        }
        let self_key = (ordered_bits(nodes[i].height), i as u32);
        let mut best: Option<(u64, u32)> = None;
        for &next in &adj[i] {
            let key = (ordered_bits(nodes[next as usize].height), next);
            if key < self_key && best.is_none_or(|kept| key < kept) {
                best = Some(key);
            }
        }
        if let Some((_, next)) = best {
            down[i] = next;
        }
    }

    let reach = reaches_outlet(nodes, &down);
    let mut cost = vec![u64::MAX; n];
    let mut cost_h = vec![f64::INFINITY; n];
    let mut hop = vec![u32::MAX; n];
    let mut parent = vec![STREAM_NONE; n];
    let mut heap = BinaryHeap::new();
    for i in 0..n {
        if !reach[i] && !nodes[i].is_outlet() {
            continue;
        }
        let spill = ordered_bits(nodes[i].height);
        cost[i] = spill;
        cost_h[i] = nodes[i].height;
        hop[i] = 0;
        heap.push(Reverse(FloodKey {
            spill,
            hop: 0,
            via: STREAM_NONE,
            node: i as u32,
        }));
    }
    let mut settled = vec![false; n];
    while let Some(Reverse(key)) = heap.pop() {
        let u = key.node as usize;
        if settled[u] || cost[u] != key.spill || hop[u] != key.hop || parent[u] != key.via {
            continue;
        }
        settled[u] = true;
        for &next in &adj[u] {
            let v = next as usize;
            if reach[v] || nodes[v].is_outlet() {
                continue;
            }
            let spill_h = higher(nodes[v].height, cost_h[u]);
            let spill = ordered_bits(spill_h);
            let hop_v = hop[u].saturating_add(1);
            let via = u as u32;
            let better = spill < cost[v]
                || (spill == cost[v] && (hop_v < hop[v] || (hop_v == hop[v] && via < parent[v])));
            if !better {
                continue;
            }
            cost[v] = spill;
            cost_h[v] = spill_h;
            hop[v] = hop_v;
            parent[v] = via;
            heap.push(Reverse(FloodKey {
                spill,
                hop: hop_v,
                via,
                node: next,
            }));
        }
    }

    let mut fallback = 0u32;
    for i in 0..n {
        if reach[i] || nodes[i].is_outlet() || down[i] != STREAM_NONE {
            continue;
        }
        let mut cursor = i;
        let mut seen = vec![false; n];
        for _ in 0..n {
            if seen[cursor] {
                break;
            }
            seen[cursor] = true;
            let next = parent[cursor];
            if next == STREAM_NONE {
                if let Some(outlet) = nearest_outlet(nodes, cursor) {
                    down[cursor] = outlet;
                }
                fallback += 1;
                break;
            }
            down[cursor] = next;
            let next_i = next as usize;
            if reach[next_i] || nodes[next_i].is_outlet() {
                break;
            }
            cursor = next_i;
        }
    }
    (down, fallback)
}

fn reaches_outlet(nodes: &[StreamNode], down: &[u32]) -> Vec<bool> {
    let n = nodes.len();
    // 0 unknown, 1 on the current walk, 2 reaches an outlet, 3 does not.
    let mut color = vec![0u8; n];
    for start in 0..n {
        if color[start] != 0 {
            continue;
        }
        let mut chain = Vec::new();
        let mut cursor = start;
        let reaches = loop {
            if nodes[cursor].is_outlet() || color[cursor] == 2 {
                break true;
            }
            if color[cursor] == 1 || color[cursor] == 3 {
                break false;
            }
            color[cursor] = 1;
            chain.push(cursor);
            let next = down[cursor];
            if next == STREAM_NONE || next as usize >= n {
                break false;
            }
            cursor = next as usize;
        };
        let paint = if reaches { 2 } else { 3 };
        for index in chain {
            color[index] = paint;
        }
    }
    color.into_iter().map(|color| color == 2).collect()
}

fn nearest_outlet(nodes: &[StreamNode], index: usize) -> Option<u32> {
    let mut best: Option<(u64, u32)> = None;
    for (j, node) in nodes.iter().enumerate() {
        if index == j || !node.is_outlet() {
            continue;
        }
        let key = (ordered_bits(chord_d2(nodes[index].dir, node.dir)), j as u32);
        if best.is_none_or(|kept| key < kept) {
            best = Some(key);
        }
    }
    best.map(|(_, outlet)| outlet)
}

/// Steepest step for each pure inflow. Grid pointers stay as they are.
/// A pit inflow breaches to a neighbour that already reaches an outlet,
/// then to the nearest outlet, then to the lowest other node.
fn route_inflows(nodes: &[StreamNode], down: &mut [u32]) -> u32 {
    let mut fallback = 0u32;
    let pure: Vec<usize> = (0..nodes.len())
        .filter(|&index| is_pure_inflow(&nodes[index]))
        .collect();
    for &index in &pure {
        let self_key = (ordered_bits(nodes[index].height), index as u32);
        let mut best: Option<(u64, u32)> = None;
        for next in eight_non_inflows(nodes, index) {
            let key = (ordered_bits(nodes[next as usize].height), next);
            if key < self_key && best.is_none_or(|kept| key < kept) {
                best = Some(key);
            }
        }
        if let Some((_, next)) = best {
            down[index] = next;
        }
        if flows_to_outlet(nodes, down, index) {
            continue;
        }
        fallback = fallback.saturating_add(1);
        if let Some(target) = best_breach(nodes, down, index) {
            down[index] = target;
        }
    }
    fallback
}

fn eight_non_inflows(nodes: &[StreamNode], index: usize) -> Vec<u32> {
    let mut best = [(u64::MAX, STREAM_NONE); 8];
    for (j, node) in nodes.iter().enumerate() {
        if j == index || is_pure_inflow(node) {
            continue;
        }
        let key = (ordered_bits(chord_d2(nodes[index].dir, node.dir)), j as u32);
        if key >= best[7] {
            continue;
        }
        best[7] = key;
        best.sort_unstable();
    }
    best.iter()
        .filter_map(|(_, j)| (*j != STREAM_NONE).then_some(*j))
        .collect()
}

fn flows_to_outlet(nodes: &[StreamNode], down: &[u32], start: usize) -> bool {
    let mut cursor = start;
    let mut guard = 0usize;
    while guard <= nodes.len() {
        if nodes[cursor].is_outlet() {
            return true;
        }
        let next = down[cursor];
        if next == STREAM_NONE || next as usize >= nodes.len() {
            return false;
        }
        cursor = next as usize;
        guard += 1;
    }
    false
}

fn best_breach(nodes: &[StreamNode], down: &[u32], index: usize) -> Option<u32> {
    let mut best: Option<(u64, u32, u32)> = None;
    for next in eight_non_inflows(nodes, index) {
        if !nodes[next as usize].is_outlet() && !flows_to_outlet(nodes, down, next as usize) {
            continue;
        }
        let spill = ordered_bits(higher(nodes[index].height, nodes[next as usize].height));
        let key = (spill, 1u32, next);
        if best.is_none_or(|kept| key < kept) {
            best = Some(key);
        }
    }
    if let Some((_, _, next)) = best {
        return Some(next);
    }
    if let Some(outlet) = nearest_outlet(nodes, index) {
        return Some(outlet);
    }
    let mut lowest: Option<(u64, u32)> = None;
    for (j, node) in nodes.iter().enumerate() {
        if j == index || is_pure_inflow(node) {
            continue;
        }
        let key = (ordered_bits(node.height), j as u32);
        if lowest.is_none_or(|kept| key < kept) {
            lowest = Some(key);
        }
    }
    lowest.map(|(_, index)| index)
}

fn count_r0(nodes: &[StreamNode], down: &[u32], receiver: Option<usize>) -> u32 {
    let Some(receiver) = receiver else {
        return 0;
    };
    let mut count = 0u32;
    for (start, node) in nodes.iter().enumerate() {
        if node.grid.is_none() {
            continue;
        }
        let mut cursor = start;
        let mut seen = vec![false; nodes.len()];
        loop {
            if cursor == receiver {
                count = count.saturating_add(1);
                break;
            }
            if seen[cursor] {
                break;
            }
            seen[cursor] = true;
            let next = down[cursor];
            if next == STREAM_NONE || next as usize >= nodes.len() {
                break;
            }
            cursor = next as usize;
        }
    }
    count
}

fn boundary_seed(node: &StreamNode) -> u32 {
    if node.flags & FLAG_RECEIVER != 0 {
        0
    } else if node.is_inflow() {
        node.r0_source
    } else {
        1
    }
}

fn draw_seed(node: &StreamNode) -> u32 {
    if node.flags & FLAG_RECEIVER != 0 {
        0
    } else if node.is_inflow() {
        node.weight
    } else {
        1
    }
}

fn accumulate(nodes: &[StreamNode], down: &[u32], seed: fn(&StreamNode) -> u32) -> Vec<u32> {
    let n = nodes.len();
    let mut incoming = vec![Vec::new(); n];
    for (index, &next) in down.iter().enumerate() {
        if next != STREAM_NONE && (next as usize) < n {
            incoming[next as usize].push(index as u32);
        }
    }
    let mut acc = vec![0u32; n];
    let mut seen = vec![false; n];
    fn walk(
        u: usize,
        nodes: &[StreamNode],
        incoming: &[Vec<u32>],
        acc: &mut [u32],
        seen: &mut [bool],
        seed: fn(&StreamNode) -> u32,
    ) -> u32 {
        if seen[u] {
            return acc[u];
        }
        seen[u] = true;
        let mut sum = seed(&nodes[u]);
        for &child in &incoming[u] {
            sum = sum.saturating_add(walk(child as usize, nodes, incoming, acc, seen, seed));
        }
        acc[u] = sum;
        sum
    }
    for index in 0..n {
        walk(index, nodes, &incoming, &mut acc, &mut seen, seed);
    }
    acc
}

fn emitted_at(tree: &StreamTree, index: usize, z: u32) -> bool {
    let next = tree.down[index];
    if next == STREAM_NONE || tree.nodes[index].is_outlet() {
        return false;
    }
    let next_node = &tree.nodes[next as usize];
    // The edge into the receiver uses the boundary sum and the stricter
    // of the two cells, including when that node is itself an inflow.
    if next_node.flags & FLAG_RECEIVER != 0 {
        let other = tree.receiver_arid.unwrap_or(tree.arid);
        let Some(threshold) = agreed(tree.k, tree.arid, other, z) else {
            return false;
        };
        return tree.acc[index] >= threshold;
    }
    if tree.nodes[index].is_inflow() {
        let Some(threshold) = agreed(tree.k, tree.arid, tree.nodes[index].source_arid, z) else {
            return false;
        };
        return tree.nodes[index].weight >= threshold;
    }
    let Some(threshold) = stream_threshold(z, tree.k, tree.arid) else {
        return false;
    };
    tree.draw[index] >= threshold
}

impl StreamTree {
    pub(super) fn lines(&self, cell: u32, z: u32) -> Vec<StreamLine> {
        let n = self.nodes.len();
        let mut emitted = vec![false; n];
        for (index, slot) in emitted.iter_mut().enumerate() {
            *slot = emitted_at(self, index, z);
        }
        let mut incoming = vec![Vec::new(); n];
        let mut indeg = vec![0u32; n];
        for index in 0..n {
            if !emitted[index] {
                continue;
            }
            let next = self.down[index] as usize;
            incoming[next].push(index as u32);
            indeg[next] += 1;
        }
        let order = strahler(self, &emitted, &incoming);
        let mut used = vec![false; n];
        let mut lines = Vec::new();
        for start in 0..n {
            if !emitted[start] || used[start] || !is_start(self, start, &emitted, &indeg, &order) {
                continue;
            }
            let mut path = vec![start];
            let mut cursor = start;
            loop {
                if used[cursor] {
                    break;
                }
                used[cursor] = true;
                let next = self.down[cursor] as usize;
                path.push(next);
                if !emitted[next] || indeg[next] != 1 || order[next] != order[cursor] {
                    break;
                }
                cursor = next;
            }
            let mut marked = Vec::with_capacity(path.len() + 1);
            for &index in &path {
                let node = &self.nodes[index];
                marked.push((node.dir, node.pinned()));
            }
            if let Some(node) = path.last().map(|index| &self.nodes[*index]) {
                if node.flags & FLAG_RIVER != 0 {
                    if let Some(at) = node.river_at {
                        marked.push((at, true));
                    }
                }
            }
            let smoothed = smooth_pinned(&marked);
            let mut lonlat = Vec::with_capacity(smoothed.len());
            for dir in smoothed {
                let next = lonlat_of(dir);
                if lonlat.last().is_none_or(|prev: &[f64; 2]| *prev != next) {
                    lonlat.push(next);
                }
            }
            if lonlat.len() < 2 {
                continue;
            }
            lines.push(StreamLine {
                id: format!("stream:c{cell}:{start}"),
                order: order[start],
                accumulation: self.draw[start],
                lonlat,
            });
        }
        lines
    }
}

fn is_start(
    tree: &StreamTree,
    index: usize,
    emitted: &[bool],
    indeg: &[u32],
    order: &[u16],
) -> bool {
    if !emitted[index] {
        return false;
    }
    if indeg[index] != 1 {
        return true;
    }
    let pred = (0..tree.nodes.len())
        .find(|candidate| emitted[*candidate] && tree.down[*candidate] == index as u32);
    match pred {
        Some(pred) => order[pred] != order[index],
        None => true,
    }
}

fn strahler(tree: &StreamTree, emitted: &[bool], incoming: &[Vec<u32>]) -> Vec<u16> {
    let n = tree.nodes.len();
    let mut memo = vec![None; n];
    fn walk(
        u: usize,
        tree: &StreamTree,
        emitted: &[bool],
        incoming: &[Vec<u32>],
        memo: &mut [Option<u16>],
    ) -> u16 {
        if let Some(order) = memo[u] {
            return order;
        }
        let mut orders = Vec::new();
        for &child in &incoming[u] {
            if emitted[child as usize] {
                orders.push(walk(child as usize, tree, emitted, incoming, memo));
            }
        }
        if tree.nodes[u].is_inflow() {
            let weight = tree.nodes[u].weight;
            orders.push(if weight < tree.k { 1 } else { 2 });
        }
        let value = if orders.is_empty() {
            1
        } else {
            let max = orders.iter().copied().max().unwrap_or(1);
            let count = orders.iter().filter(|order| **order == max).count();
            if count >= 2 {
                max.saturating_add(1)
            } else {
                max
            }
        };
        memo[u] = Some(value);
        value
    }
    let mut order = vec![0u16; n];
    for index in 0..n {
        if emitted[index] {
            order[index] = walk(index, tree, emitted, incoming, &mut memo);
        }
    }
    order
}

fn smooth_pinned(points: &[([f64; 3], bool)]) -> Vec<[f64; 3]> {
    if points.len() < 2 {
        return points.iter().map(|(point, _)| *point).collect();
    }
    let mut cuts = vec![0usize];
    for (index, (_, pin)) in points.iter().enumerate().skip(1) {
        if *pin || index + 1 == points.len() {
            cuts.push(index);
        }
    }
    let mut out = Vec::new();
    for pair in cuts.windows(2) {
        let sub: Vec<[f64; 3]> = points[pair[0]..=pair[1]]
            .iter()
            .map(|(point, _)| *point)
            .collect();
        let smoothed = chaikin_once(&sub);
        if out.is_empty() {
            out = smoothed;
        } else if smoothed.len() > 1 {
            out.extend_from_slice(&smoothed[1..]);
        }
    }
    out
}

fn chaikin_once(points: &[[f64; 3]]) -> Vec<[f64; 3]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len() * 2);
    out.push(points[0]);
    for pair in points.windows(2) {
        out.push(normalize(mix(pair[0], pair[1], 0.25)));
        out.push(normalize(mix(pair[0], pair[1], 0.75)));
    }
    out.push(*points.last().expect("polyline has an end"));
    out
}

fn tangent_basis(centre: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut east = cross([0.0, 0.0, 1.0], centre);
    if norm(east) < 1e-8 {
        east = cross([0.0, 1.0, 0.0], centre);
    }
    east = normalize(east);
    let north = normalize(cross(centre, east));
    (east, north)
}

fn offset_dir(
    centre: [f64; 3],
    east: [f64; 3],
    north: [f64; 3],
    east_m: f64,
    north_m: f64,
    radius: f64,
) -> [f64; 3] {
    normalize(add(
        centre,
        add(scale(east, east_m / radius), scale(north, north_m / radius)),
    ))
}

fn hash01(domain: u64, cell: u32, a: u32, b: u32) -> f64 {
    let mixed = splitmix64(
        domain
            .wrapping_add(u64::from(cell).wrapping_mul(0x9E37_79B9_7F4A_7C15))
            .wrapping_add(u64::from(a).wrapping_mul(0xBF58_476D_1CE4_E5B9))
            .wrapping_add(u64::from(b).wrapping_mul(0x94D0_49BB_1331_11EB)),
    );
    (mixed >> 11) as f64 / ((1u64 << 53) as f64)
}

fn lonlat_of(dir: [f64; 3]) -> [f64; 2] {
    let lon = dir[1].atan2(dir[0]).to_degrees();
    let lat = dir[2].clamp(-1.0, 1.0).asin().to_degrees();
    [lon, lat]
}

fn ordered_bits(value: f64) -> u64 {
    let bits = value.to_bits();
    if bits & (1 << 63) == 0 {
        bits | (1 << 63)
    } else {
        !bits
    }
}

fn higher(a: f64, b: f64) -> f64 {
    if ordered_bits(a) >= ordered_bits(b) {
        a
    } else {
        b
    }
}

fn chord_d2(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(a, b).clamp(-1.0, 1.0).acos()
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
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

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let len = norm(a);
    if len < 1e-18 {
        [1.0, 0.0, 0.0]
    } else {
        scale(a, 1.0 / len)
    }
}

fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    add(scale(a, 1.0 - t), scale(b, t))
}

#[cfg(test)]
pub(super) fn same_tree(left: &StreamTree, right: &StreamTree) -> bool {
    if left.k != right.k
        || left.arid != right.arid
        || left.fallback != right.fallback
        || left.r0 != right.r0
        || left.exit_w != right.exit_w
        || left.receiver_arid != right.receiver_arid
        || left.nodes.len() != right.nodes.len()
    {
        return false;
    }
    for (a, b) in left.nodes.iter().zip(&right.nodes) {
        if a.flags != b.flags
            || a.river_at.is_some() != b.river_at.is_some()
            || a.source != b.source
            || a.weight != b.weight
            || a.r0_source != b.r0_source
            || a.source_arid != b.source_arid
        {
            return false;
        }
        if !same_dir(a.dir, b.dir) || a.height.to_bits() != b.height.to_bits() {
            return false;
        }
        if let (Some(left_at), Some(right_at)) = (a.river_at, b.river_at) {
            if !same_dir(left_at, right_at) {
                return false;
            }
        }
    }
    left.down == right.down && left.acc == right.acc && left.draw == right.draw
}

#[cfg(test)]
fn same_dir(left: [f64; 3], right: [f64; 3]) -> bool {
    left.iter()
        .zip(right)
        .all(|(a, b)| a.to_bits() == b.to_bits())
}

/// Rising downstream steps and the number of emitted steps at zoom `z`.
#[cfg(test)]
pub(super) fn rising_steps(tree: &StreamTree, z: u32) -> (usize, usize) {
    let mut rise = 0usize;
    let mut steps = 0usize;
    for (index, node) in tree.nodes.iter().enumerate() {
        if !emitted_at(tree, index, z) {
            continue;
        }
        let next = tree.down[index] as usize;
        steps += 1;
        if tree.nodes[next].height > node.height {
            rise += 1;
        }
    }
    (rise, steps)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rayon::ThreadPoolBuilder;

    use super::super::{
        lon_lat_to_tile, render_terrain_vector_tile, TerrainVectorLayer, TerrainVectors,
    };
    use super::*;

    fn terrain() -> &'static AtlasTerrain {
        super::super::terrain_fixture()
    }

    fn land_receiver(terrain: &AtlasTerrain) -> Option<(u32, u32)> {
        let surface = terrain.surface();
        (0..surface.cell_count() as u32).find_map(|cell| {
            if !is_stream_cell(terrain, cell) {
                return None;
            }
            let receiver = surface.receiver(cell);
            if receiver == TOOL_SURFACE_NONE || receiver == cell {
                return None;
            }
            if !is_stream_cell(terrain, receiver) {
                return None;
            }
            Some((cell, receiver))
        })
    }

    #[test]
    fn tree_is_identical_across_threads_cache_and_tiles() {
        let terrain = terrain();
        let (cell, _) = land_receiver(terrain).expect("land cell with a land receiver");
        clear_cache(terrain);
        let first = tree(terrain, cell).expect("tree");
        let cached = tree(terrain, cell).expect("cached tree");
        assert!(same_tree(&first, &cached));
        clear_cache(terrain);
        let rebuilt = tree(terrain, cell).expect("rebuilt");
        assert!(same_tree(&first, &rebuilt));

        clear_cache(terrain);
        let pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .expect("pool"),
        );
        let raced: Vec<Arc<StreamTree>> = std::thread::scope(|scope| {
            let mut joins = Vec::new();
            for _ in 0..4 {
                let pool = Arc::clone(&pool);
                joins.push(
                    scope.spawn(move || pool.install(|| tree(terrain, cell).expect("raced"))),
                );
            }
            joins
                .into_iter()
                .map(|join| join.join().expect("join"))
                .collect()
        });
        for other in &raced {
            assert!(same_tree(&first, other));
        }

        let serial = ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("serial")
            .install(|| {
                clear_cache(terrain);
                tree(terrain, cell).expect("serial")
            });
        assert!(same_tree(&first, &serial));

        let (lat, lon) = terrain.surface().cell_lat_lon_degrees(cell);
        let vectors = TerrainVectors::new(terrain);
        for zoom in [8u32, 11] {
            let (x, y) = lon_lat_to_tile(lon, lat, zoom);
            clear_cache(terrain);
            let _ = render_terrain_vector_tile(
                terrain,
                &vectors,
                TerrainVectorLayer::Streams,
                zoom,
                x,
                y,
            )
            .expect("tile");
            let from_tile = tree(terrain, cell).expect("tile tree");
            assert!(
                same_tree(&first, &from_tile),
                "z{zoom} tile changed the tree"
            );
        }
    }

    fn keeps_point(lines: &[StreamLine], expected: [f64; 3]) -> bool {
        lines.iter().any(|line| {
            line.lonlat.iter().any(|point| {
                let dir = super::super::render::direction_from_lon_lat(
                    point[0].to_radians(),
                    point[1].to_radians(),
                );
                angle(dir, expected) < 1e-6
            })
        })
    }

    #[test]
    fn inflow_matches_receiver_outlet_and_leaves() {
        let terrain = terrain();
        let surface = terrain.surface();
        let mut saw_join = false;
        let mut saw_line = false;
        for cell in 0..surface.cell_count() as u32 {
            if !is_stream_cell(terrain, cell) {
                continue;
            }
            let receiver = surface.receiver(cell);
            if !is_stream_cell(terrain, receiver) {
                continue;
            }
            let upstream = tree(terrain, cell).expect("upstream");
            let downstream = tree(terrain, receiver).expect("downstream");
            let expected = shared_edge_direction(terrain, cell, receiver);
            let outlet = upstream
                .nodes
                .iter()
                .find(|node| node.flags & FLAG_RECEIVER != 0)
                .expect("receiver outlet");
            assert!(
                angle(outlet.dir, expected) < 1e-9,
                "outlet drifted by {} rad",
                angle(outlet.dir, expected)
            );
            let Some((index, inflow)) = downstream
                .nodes
                .iter()
                .enumerate()
                .find(|(_, node)| node.source == Some(cell))
            else {
                assert_eq!(upstream.exit_w, 0, "cell {cell} exit without an inflow");
                continue;
            };
            assert!(
                angle(inflow.dir, expected) < 1e-9,
                "join drifted by {} rad",
                angle(inflow.dir, expected)
            );
            assert_eq!(inflow.weight, upstream.exit_w);
            saw_join = true;
            if inflow.flags & FLAG_RECEIVER != 0 {
                continue;
            }
            assert_ne!(
                downstream.down[index], STREAM_NONE,
                "inflow has no downstream"
            );
            let threshold = agreed(downstream.k, downstream.arid, upstream.arid, 8).expect("z8");
            if inflow.weight >= threshold {
                let lines = downstream.lines(receiver, 8);
                assert!(
                    keeps_point(&lines, expected),
                    "no z8 stream keeps the inflow point"
                );
                saw_line = true;
                break;
            }
        }
        assert!(saw_join, "no land pair carried flow across the edge");
        assert!(saw_line, "no inflow reached the z8 threshold");
    }

    /// `P` is the shared edge point. Several sub-threshold links may meet
    /// exactly there; that is the only case where the downstream cell emits
    /// and the upstream cell does not. A 2-cycle point is a terminal outlet
    /// on both cells, so neither side starts a stream from it.
    #[test]
    fn edge_continuity_weights_and_grid_routing() {
        let terrain = terrain();
        let surface = terrain.surface();
        let mut pairs = 0u32;
        let mut cycles = 0u32;
        let mut merges = [0u32; 3];
        for cell in 0..surface.cell_count() as u32 {
            if !is_stream_cell(terrain, cell) {
                continue;
            }
            let built = tree(terrain, cell).expect("tree");
            assert_grid_route(&built, cell);
            let receiver = surface.receiver(cell);
            if !is_stream_cell(terrain, receiver) {
                continue;
            }
            let downstream = tree(terrain, receiver).expect("downstream");
            let outlet = receiver_index(&built.nodes).expect("receiver");
            assert_eq!(built.acc[outlet], built.exit_w, "cell {cell} exit");
            assert_eq!(built.receiver_arid, Some(downstream.arid));
            let owned: Vec<_> = downstream
                .nodes
                .iter()
                .filter(|node| node.source == Some(cell))
                .collect();
            if built.exit_w == 0 {
                assert!(owned.is_empty(), "cell {cell} added a zero inflow");
            } else {
                assert_eq!(owned.len(), 1, "cell {cell} inflow count");
                assert_eq!(owned[0].weight, built.exit_w);
            }
            pairs += 1;
            let terminal = owned.first().is_some_and(|node| node.is_outlet());
            if terminal {
                cycles += 1;
                continue;
            }
            for (slot, z) in [8u32, 10, 12].into_iter().enumerate() {
                let threshold = agreed(built.k, built.arid, downstream.arid, z).expect("zoom");
                let mut preds = 0u32;
                let mut links = 0u32;
                for (index, next) in built.down.iter().enumerate() {
                    if *next != outlet as u32 {
                        continue;
                    }
                    preds += 1;
                    if emitted_at(&built, index, z) {
                        links += 1;
                    }
                }
                let emit_down = downstream.nodes.iter().enumerate().any(|(index, node)| {
                    node.source == Some(cell) && emitted_at(&downstream, index, z)
                });
                let emit_up = links > 0;
                let merge = emit_down && !emit_up && preds >= 2 && built.exit_w >= threshold;
                assert!(
                    emit_down == emit_up || merge,
                    "cell {cell} -> {receiver} z{z}: down {emit_down} up {emit_up} exit {} preds {preds} threshold {threshold}",
                    built.exit_w
                );
                if merge {
                    merges[slot] += 1;
                }
            }
        }
        let line = format!(
            "pairs {pairs} terminal_cycles {cycles} merges z8 {} z10 {} z12 {} {}",
            merges[0],
            merges[1],
            merges[2],
            cache_payload_line()
        );
        std::fs::write("/tmp/atlas-p2b-round2-continuity.txt", &line).expect("stats");
        println!("{line}");
        assert!(pairs > 0, "fixture has no land receiver pair");
    }

    fn assert_grid_route(tree: &StreamTree, cell: u32) {
        let split = tree
            .nodes
            .iter()
            .position(is_pure_inflow)
            .unwrap_or(tree.nodes.len());
        assert!(
            tree.nodes[split..].iter().all(is_pure_inflow),
            "cell {cell} mixed inflow nodes into the grid"
        );
        for (index, next) in tree.down.iter().enumerate().skip(split) {
            assert_ne!(
                *next, STREAM_NONE,
                "cell {cell} inflow {index} has no downstream"
            );
            assert!(
                (*next as usize) < split,
                "cell {cell} inflow drains to an inflow"
            );
        }
        let prefix = &tree.nodes[..split];
        let (without, _) = route(prefix, &adjacency(prefix, tree.k));
        let (with, _) = route(&tree.nodes, &adjacency(&tree.nodes, tree.k));
        for index in 0..split {
            assert_eq!(
                without[index], with[index],
                "cell {cell} inflows changed grid routing at {index}"
            );
            assert_eq!(
                tree.down[index], without[index],
                "cell {cell} stored grid route differs at {index}"
            );
            let next = tree.down[index];
            if next != STREAM_NONE {
                assert!(
                    (next as usize) < split,
                    "cell {cell} grid node drains to an inflow"
                );
            }
        }
        let counted = count_r0(&tree.nodes, &tree.down, receiver_index(&tree.nodes));
        assert_eq!(counted, tree.r0, "cell {cell} R0");
    }

    fn cache_payload_line() -> String {
        let node = std::mem::size_of::<StreamNode>();
        let count = 16 * 16 + 8;
        let tree = count * node + count * 4 * 3;
        format!(
            "node {node}B tree~{tree}B full~{}B cores~{}B flow~{}B caps {CACHE_CAP}/{CORE_CAP}/{FLOW_CAP}",
            tree * CACHE_CAP,
            count * node * CORE_CAP,
            FLOW_CAP * 8
        )
    }

    /// A pit outlet has to climb, so at z12 the threshold `k` leaves that
    /// climb as a large share of a short trunk. The gate is every routed
    /// step. An uphill spanning tree still fails it. Drawn steps are counted
    /// beside it.
    #[test]
    fn emitted_steps_rise_on_less_than_five_percent() {
        let terrain = terrain();
        let surface = terrain.surface();
        clear_cache(terrain);
        let mut rise = 0usize;
        let mut steps = 0usize;
        let mut all_rise = 0usize;
        let mut all_steps = 0usize;
        let mut cells = 0usize;
        for cell in (0..surface.cell_count() as u32).step_by(500) {
            if !is_stream_cell(terrain, cell) {
                continue;
            }
            let built = tree(terrain, cell).expect("tree");
            let (cell_rise, cell_steps) = rising_steps(&built, 12);
            rise += cell_rise;
            steps += cell_steps;
            for (index, node) in built.nodes.iter().enumerate() {
                let next = built.down[index];
                if next == STREAM_NONE || node.is_outlet() {
                    continue;
                }
                all_steps += 1;
                if built.nodes[next as usize].height > node.height {
                    all_rise += 1;
                }
            }
            cells += 1;
        }
        assert!(steps > 0 && all_steps > 0, "no routed steps");
        let share = rise as f64 / steps as f64;
        let all_share = all_rise as f64 / all_steps as f64;
        let line = format!(
            "rising emitted {rise}/{steps} ({share:.4}) routed {all_rise}/{all_steps} ({all_share:.4}) over {cells} cells"
        );
        std::fs::write("/tmp/atlas-p2b-round2-rising.txt", &line).expect("stats");
        println!("{line}");
        assert!(
            all_share < 0.05,
            "routed rising steps {all_rise}/{all_steps} ({all_share:.4}) over {cells} cells"
        );
    }

    #[test]
    fn ocean_ice_and_wet_lake_have_no_tree() {
        let terrain = terrain();
        let surface = terrain.surface();
        let mut ocean = None;
        let mut ice = None;
        let mut wet = None;
        for cell in 0..surface.cell_count() as u32 {
            if ocean.is_none() && surface.ocean(cell) {
                ocean = Some(cell);
            }
            if ice.is_none() && surface.ice(cell) {
                ice = Some(cell);
            }
            if wet.is_none() && surface.wet_lake(cell) {
                wet = Some(cell);
            }
            if ocean.is_some() && ice.is_some() && wet.is_some() {
                break;
            }
        }
        let ocean = ocean.expect("fixture has an ocean cell");
        let ice = ice.expect("fixture has an ice cell");
        assert!(tree(terrain, ocean).is_none());
        assert!(tree(terrain, ice).is_none());
        if let Some(wet) = wet {
            assert!(tree(terrain, wet).is_none());
        }
    }
}
