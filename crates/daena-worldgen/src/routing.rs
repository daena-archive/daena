//! Steepest-descent receivers, donor stacks and depression carving.
//!
//! Depression routing follows Cordonnier, Bovy & Braun (2019): pit basins are
//! joined by a minimum spanning tree of passes and carved by reversing the
//! receiver path. A [`BasinBalance`] may keep a pit closed. `None` carves
//! every pit, which is the phase A behaviour.

use std::collections::VecDeque;
use std::ops::Range;

use rayon::prelude::*;

use crate::grid::{dot3, CubeSphere, NONE};

/// Scratch buffers reused across iterations of one level.
pub struct RoutingScratch {
    /// Downstream neighbour, or the cell itself at a root.
    pub receiver: Vec<u32>,
    /// Distance to the receiver, in metres. Zero at a root.
    pub receiver_dist: Vec<f32>,
    /// CSR offsets into [`Self::donors`], length `cell_count + 1`.
    pub donor_offset: Vec<u32>,
    /// Donor cell ids, grouped by receiver.
    pub donors: Vec<u32>,
    /// Topological order: each cell appears after its receiver.
    pub stack: Vec<u32>,
    /// Root cell of the basin, before the post-carve rebuild.
    /// Depression routing reuses this buffer as the basin-node map.
    pub basin: Vec<u32>,
    /// Pass elevation for lake cells, otherwise NaN.
    pub lake_level: Vec<f32>,
    /// Drainage area in square metres, accumulated in `f64`.
    pub area: Vec<f64>,
    /// Stack position of each cell, used by the basin-parallel passes.
    pub(crate) pos: Vec<u32>,
    /// Contiguous stack ranges, each whole basins and about `chunk_cells`
    /// cells unless one basin is larger.
    pub(crate) chunks: Vec<(u32, u32)>,
    /// Stack-order work buffer for `f64` basin passes.
    pub(crate) stack_f64: Vec<f64>,
    /// Below this cell count the basin passes run in cell order, in place.
    pub(crate) parallel_min_cells: usize,
    /// Target cell count of one basin chunk.
    pub(crate) chunk_cells: usize,
    /// Empty means every cell has weight 1.
    weight: Vec<f32>,
    segs: Vec<BasinSeg>,
    cursor: Vec<u32>,
    dfs: Vec<u32>,
    edges: Vec<PassEdge>,
}

#[derive(Clone, Copy)]
struct BasinSeg {
    root: u32,
    start: u32,
    end: u32,
}

/// A pass between basin nodes `a < b` through adjacent cells `c` (in `a`) and
/// `k` (in `b`), at height `max(h_c, h_k)`.
pub(crate) struct PassEdge {
    pub(crate) a: u32,
    pub(crate) b: u32,
    pub(crate) pass: f32,
    pub(crate) c: u32,
    pub(crate) k: u32,
}

/// Per-cell water balance for depression closure. Length matches the grid.
///
/// Runoff and evaporation are metres per year. `None` passed to [`route`]
/// leaves every pit open.
pub struct BasinBalance<'a> {
    /// Runoff depth, metres per year.
    pub runoff_m_yr: &'a [f32],
    /// Net evaporation, metres per year. Lake loss is `max(0, PET − P)`.
    pub evap_m_yr: &'a [f32],
}

/// Pit count before carving, and how many basins were carved.
pub struct RouteStats {
    /// Domain roots before carving, including closed pits.
    pub pits: u32,
    /// Basins whose receiver path was reversed.
    pub carved: u32,
    /// Pass edges stored before the sort and dedup. Zero when nothing was carved.
    pub edges: u32,
    /// Pit basins whose evaporation swallowed their inflow.
    pub closed_basins: u32,
    /// Pit cell of each closed basin. Empty outside tests.
    #[cfg(test)]
    pub closed_pits: Vec<u32>,
}

impl RoutingScratch {
    /// Allocate buffers for `cell_count` cells.
    pub fn new(cell_count: usize) -> Self {
        Self {
            receiver: vec![0; cell_count],
            receiver_dist: vec![0.0; cell_count],
            donor_offset: vec![0; cell_count + 1],
            donors: Vec::new(),
            stack: Vec::with_capacity(cell_count),
            basin: vec![0; cell_count],
            lake_level: vec![f32::NAN; cell_count],
            area: vec![0.0; cell_count],
            pos: vec![0; cell_count],
            chunks: Vec::new(),
            stack_f64: vec![0.0; cell_count],
            parallel_min_cells: PARALLEL_BASIN_MIN_CELLS,
            chunk_cells: BASIN_CHUNK_CELLS,
            weight: Vec::new(),
            segs: Vec::new(),
            cursor: vec![0; cell_count],
            dfs: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Rebuild the chunk ranges from the current basins. Production calls
    /// this from `build_donors_stack` with `chunk_cells`; tests may pass a
    /// smaller target so a small grid gets several chunks.
    pub(crate) fn rechunk(&mut self, target_cells: usize) {
        self.chunks.clear();
        let mut start = 0u32;
        let mut count = 0usize;
        for seg in &self.segs {
            let len = (seg.end - seg.start) as usize;
            if count > 0 && count + len > target_cells {
                self.chunks.push((start, seg.start));
                start = seg.start;
                count = 0;
            }
            count += len;
        }
        if count > 0 {
            self.chunks.push((start, self.stack.len() as u32));
        }
    }

    /// True when the basin passes use stack-order chunks in parallel.
    pub(crate) fn parallel_basins(&self) -> bool {
        self.receiver.len() >= self.parallel_min_cells
    }

    fn resize(&mut self, n: usize) {
        if self.receiver.len() != n {
            self.receiver.resize(n, 0);
            self.receiver_dist.resize(n, 0.0);
            self.basin.resize(n, 0);
            self.lake_level.resize(n, f32::NAN);
            self.area.resize(n, 0.0);
            self.pos.resize(n, 0);
            self.chunks.clear();
            self.stack_f64.resize(n, 0.0);
            self.weight.clear();
            self.weight.shrink_to_fit();
            self.cursor.resize(n, 0);
            self.donor_offset.resize(n + 1, 0);
        }
    }
}

/// Production chunk target for the basin passes.
pub(crate) const BASIN_CHUNK_CELLS: usize = 16_384;
/// Below this cell count the basin passes run in cell order, in place.
pub(crate) const PARALLEL_BASIN_MIN_CELLS: usize = 131_072;

/// Cell-to-slot mapping for a basin kernel.
///
/// [`CellIndex`] is the identity used when the kernel runs on the field in
/// cell order. [`Slots`] offsets the stack position by the chunk start.
pub(crate) trait SlotIndex: Copy {
    /// Slot of an arbitrary cell: the receiver or a donor.
    fn at(self, cell: usize) -> usize;
    /// Slot of the cell at `idx` in the kernel's stack range.
    fn current(self, idx: usize, cell: usize) -> usize;
}

/// Identity mapping: the field is indexed by cell.
#[derive(Clone, Copy)]
pub(crate) struct CellIndex;

impl SlotIndex for CellIndex {
    #[inline(always)]
    fn at(self, cell: usize) -> usize {
        cell
    }

    #[inline(always)]
    fn current(self, _idx: usize, cell: usize) -> usize {
        cell
    }
}

/// Chunk mapping: stack position minus the chunk start.
#[derive(Clone, Copy)]
pub(crate) struct Slots<'a> {
    pos: &'a [u32],
    base: usize,
}

impl SlotIndex for Slots<'_> {
    #[inline(always)]
    fn at(self, cell: usize) -> usize {
        self.pos[cell] as usize - self.base
    }

    #[inline(always)]
    fn current(self, idx: usize, _cell: usize) -> usize {
        idx - self.base
    }
}

/// Run `body` over every cell, sequentially below the parallel threshold and
/// in parallel above it. Results are combined with `combine`.
pub(crate) fn fold_cells_mut<T, U, F, C>(
    field: &mut [T],
    parallel: bool,
    init: U,
    body: F,
    combine: C,
) -> U
where
    T: Send,
    U: Copy + Send + Sync,
    F: Fn(usize, &mut T) -> U + Sync + Send,
    C: Fn(U, U) -> U + Sync + Send,
{
    if parallel {
        field
            .par_iter_mut()
            .enumerate()
            .map(|(i, slot)| body(i, slot))
            .reduce(|| init, &combine)
    } else {
        let mut acc = init;
        for (i, slot) in field.iter_mut().enumerate() {
            acc = combine(acc, body(i, slot));
        }
        acc
    }
}

/// Stack, slot and chunk metadata shared by one basin pass.
#[derive(Clone, Copy)]
pub(crate) struct BasinView<'a> {
    pub stack: &'a [u32],
    pub pos: &'a [u32],
    pub chunks: &'a [(u32, u32)],
}

/// Like [`fold_cells_mut`], for two equal-length fields updated together.
pub(crate) fn fold_cells_zip_mut<A, B, U, F, C>(
    first: &mut [A],
    second: &mut [B],
    parallel: bool,
    init: U,
    body: F,
    combine: C,
) -> U
where
    A: Send,
    B: Send,
    U: Copy + Send + Sync,
    F: Fn(usize, &mut A, &mut B) -> U + Sync + Send,
    C: Fn(U, U) -> U + Sync + Send,
{
    if parallel {
        first
            .par_iter_mut()
            .zip(second.par_iter_mut())
            .enumerate()
            .map(|(i, (a, b))| body(i, a, b))
            .reduce(|| init, &combine)
    } else {
        let mut acc = init;
        for i in 0..first.len() {
            acc = combine(acc, body(i, &mut first[i], &mut second[i]));
        }
        acc
    }
}

fn run_range<T, F>(work: &mut [T], view: BasinView<'_>, lo: usize, hi: usize, kernel: &F) -> u32
where
    T: Copy + Send + Sync,
    F: Fn(Range<usize>, Slots<'_>, &mut [T]) -> u32 + Sync + Send,
{
    if hi - lo == 1 {
        let (start, end) = view.chunks[lo];
        let base = start as usize;
        let range = base..end as usize;
        return kernel(
            range,
            Slots {
                pos: view.pos,
                base,
            },
            work,
        );
    }
    let mid = (lo + hi) / 2;
    let split = (view.chunks[mid].0 - view.chunks[lo].0) as usize;
    let (left, right) = work.split_at_mut(split);
    let (a, b) = rayon::join(
        || run_range(left, view, lo, mid, kernel),
        || run_range(right, view, mid, hi, kernel),
    );
    a.saturating_add(b)
}

/// Slot initialiser shared by the three runner entry points.
struct Init<I> {
    fill: I,
    /// Whether cell mode also applies `fill` before the kernel.
    cell_mode: bool,
}

/// Run one basin kernel over `field`, filling each slot with `init` first.
///
/// Below `parallel_min_cells` the kernel runs in cell order on the field
/// itself, with no gather or scatter. Above it, `work` holds a stack-order
/// copy, the kernel runs over the precomputed chunks, and the result is
/// scattered back. `cell_kernel` receives the field itself; `chunk_kernel`
/// maps each cell through the chunk slots. Both call the same kernel body.
pub(crate) fn run_basin_kernel<T, I, C, F>(
    field: &mut [T],
    work: &mut [T],
    view: BasinView<'_>,
    parallel: bool,
    init: I,
    cell_kernel: C,
    chunk_kernel: F,
) -> u32
where
    T: Copy + Send + Sync,
    I: Fn(usize, T) -> T + Sync + Send,
    C: FnOnce(Range<usize>, &mut [T]) -> u32,
    F: Fn(Range<usize>, Slots<'_>, &mut [T]) -> u32 + Sync + Send,
{
    run_basin_kernel_inner(
        field,
        work,
        view,
        parallel,
        Init {
            fill: init,
            cell_mode: true,
        },
        cell_kernel,
        chunk_kernel,
    )
}

/// Like [`run_basin_kernel`], but `init` only fills the chunk-order copy.
/// Cell mode leaves the field untouched for the kernel to read in place.
pub(crate) fn run_basin_kernel_gather<T, I, C, F>(
    field: &mut [T],
    work: &mut [T],
    view: BasinView<'_>,
    parallel: bool,
    init: I,
    cell_kernel: C,
    chunk_kernel: F,
) -> u32
where
    T: Copy + Send + Sync,
    I: Fn(usize, T) -> T + Sync + Send,
    C: FnOnce(Range<usize>, &mut [T]) -> u32,
    F: Fn(Range<usize>, Slots<'_>, &mut [T]) -> u32 + Sync + Send,
{
    run_basin_kernel_inner(
        field,
        work,
        view,
        parallel,
        Init {
            fill: init,
            cell_mode: false,
        },
        cell_kernel,
        chunk_kernel,
    )
}

/// Run one basin kernel over `field` without touching slots before the
/// kernel. Cell mode reads the field in place; chunk mode copies it to `work`.
pub(crate) fn run_basin_kernel_keep<T, C, F>(
    field: &mut [T],
    work: &mut [T],
    view: BasinView<'_>,
    parallel: bool,
    cell_kernel: C,
    chunk_kernel: F,
) -> u32
where
    T: Copy + Send + Sync,
    C: FnOnce(Range<usize>, &mut [T]) -> u32,
    F: Fn(Range<usize>, Slots<'_>, &mut [T]) -> u32 + Sync + Send,
{
    run_basin_kernel_inner::<T, fn(usize, T) -> T, C, F>(
        field,
        work,
        view,
        parallel,
        Init {
            fill: |_: usize, value: T| value,
            cell_mode: false,
        },
        cell_kernel,
        chunk_kernel,
    )
}

fn run_basin_kernel_inner<T, I, C, F>(
    field: &mut [T],
    work: &mut [T],
    view: BasinView<'_>,
    parallel: bool,
    init: Init<I>,
    cell_kernel: C,
    chunk_kernel: F,
) -> u32
where
    T: Copy + Send + Sync,
    I: Fn(usize, T) -> T + Sync + Send,
    C: FnOnce(Range<usize>, &mut [T]) -> u32,
    F: Fn(Range<usize>, Slots<'_>, &mut [T]) -> u32 + Sync + Send,
{
    if !parallel {
        if init.cell_mode {
            for (cell, slot) in field.iter_mut().enumerate() {
                *slot = (init.fill)(cell, *slot);
            }
        }
        return cell_kernel(0..view.stack.len(), field);
    }
    if view.chunks.is_empty() {
        return 0;
    }
    {
        let current: &[T] = field;
        work[..view.stack.len()]
            .par_iter_mut()
            .enumerate()
            .for_each(|(idx, slot)| {
                let cell = view.stack[idx] as usize;
                *slot = (init.fill)(cell, current[cell]);
            });
    }
    let stats = run_range(
        &mut work[..view.stack.len()],
        view,
        0,
        view.chunks.len(),
        &chunk_kernel,
    );
    let work: &[T] = work;
    field.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        *slot = work[view.pos[cell] as usize];
    });
    stats
}

/// Route `elev` over `domain` (`1` = erodible). `weight` defaults to 1.
///
/// `balance: None` carves every pit. A balance builds the same tree, then
/// closes a pit when evaporation over the area below its pass meets the inflow.
pub fn route(
    grid: &CubeSphere,
    elev: &[f32],
    domain: &[u8],
    weight: Option<&[f32]>,
    scratch: &mut RoutingScratch,
    balance: Option<&BasinBalance<'_>>,
) -> RouteStats {
    let n = elev.len();
    scratch.resize(n);
    if let Some(w) = weight {
        scratch.weight.clear();
        scratch.weight.extend_from_slice(w);
    } else if !scratch.weight.is_empty() {
        scratch.weight.clear();
    }
    receivers(grid, elev, domain, scratch);
    build_donors_stack(scratch);
    let stats = depress(grid, elev, domain, scratch, balance);
    // Depression carving clears the donor graph. Rebuild it from the receivers
    // that carving left, including the case where nothing was carved.
    build_donors_stack(scratch);
    drainage(grid, scratch);
    if cfg!(debug_assertions) {
        if let Err(err) = validate_routing(&scratch.receiver, &scratch.stack) {
            panic!("routing validation failed: {err}");
        }
    }
    stats
}

fn receivers(grid: &CubeSphere, elev: &[f32], domain: &[u8], scratch: &mut RoutingScratch) {
    let receiver = &mut scratch.receiver;
    let receiver_dist = &mut scratch.receiver_dist;
    receiver
        .par_iter_mut()
        .zip(receiver_dist.par_iter_mut())
        .enumerate()
        .for_each(|(i, (slot, dist_slot))| {
            if domain[i] == 0 {
                *slot = i as u32;
                *dist_slot = 0.0;
                return;
            }
            let h = elev[i];
            let nbs = grid.neighbors(i as u32);
            let mut best_slope = 0.0f32;
            let mut best = i as u32;
            let mut best_d = 0.0f32;
            for (k, &nb) in nbs.iter().enumerate() {
                if nb == NONE {
                    continue;
                }
                let d = grid.neighbor_distance(i as u32, k);
                if d <= 0.0 {
                    continue;
                }
                let slope = (h - elev[nb as usize]) / d;
                if slope > best_slope {
                    best_slope = slope;
                    best = nb;
                    best_d = d;
                }
            }
            *slot = best;
            *dist_slot = best_d;
        });
}

fn build_donors_stack(scratch: &mut RoutingScratch) {
    let n = scratch.receiver.len();
    // Cell mode never reads `pos` or the chunk list, so skip both writes.
    let parallel = scratch.parallel_basins();
    scratch.cursor.fill(0);
    for c in 0..n {
        let r = scratch.receiver[c] as usize;
        if r != c {
            scratch.cursor[r] += 1;
        }
    }
    scratch.donor_offset[0] = 0;
    for c in 0..n {
        scratch.donor_offset[c + 1] = scratch.donor_offset[c] + scratch.cursor[c];
    }
    for c in 0..n {
        scratch.cursor[c] = scratch.donor_offset[c];
    }
    let total = scratch.donor_offset[n] as usize;
    scratch.donors.clear();
    scratch.donors.resize(total, 0);
    for c in 0..n {
        let r = scratch.receiver[c] as usize;
        if r != c {
            let slot = scratch.cursor[r] as usize;
            scratch.donors[slot] = c as u32;
            scratch.cursor[r] += 1;
        }
    }
    scratch.stack.clear();
    scratch.segs.clear();
    scratch.dfs.clear();
    for root in 0..n {
        if scratch.receiver[root] as usize != root {
            continue;
        }
        let start = scratch.stack.len() as u32;
        let root_s = scratch.donor_offset[root] as usize;
        let root_e = scratch.donor_offset[root + 1] as usize;
        if root_s == root_e {
            // Isolated roots are most of the ocean; skip the DFS stack.
            if parallel {
                scratch.pos[root] = start;
            }
            scratch.stack.push(root as u32);
            scratch.basin[root] = root as u32;
            scratch.segs.push(BasinSeg {
                root: root as u32,
                start,
                end: start + 1,
            });
            continue;
        }
        scratch.dfs.clear();
        scratch.dfs.push(root as u32);
        while let Some(node) = scratch.dfs.pop() {
            if parallel {
                scratch.pos[node as usize] = scratch.stack.len() as u32;
            }
            scratch.stack.push(node);
            scratch.basin[node as usize] = root as u32;
            let s = scratch.donor_offset[node as usize] as usize;
            let e = scratch.donor_offset[node as usize + 1] as usize;
            for &donor in scratch.donors[s..e].iter().rev() {
                scratch.dfs.push(donor);
            }
        }
        let end = scratch.stack.len() as u32;
        scratch.segs.push(BasinSeg {
            root: root as u32,
            start,
            end,
        });
    }
    if parallel {
        scratch.rechunk(scratch.chunk_cells);
    }
}

/// Pass edges from every cell in `cells`, reduced to the lowest pass per basin
/// pair. `cells` are the members of the open pit basins; `basin` maps cells to
/// nodes (`NONE` for cells outside the graph). Tests compare this with a
/// brute-force scan.
#[cfg(test)]
pub(crate) fn collect_pass_edges(
    grid: &CubeSphere,
    elev: &[f32],
    basin: &[u32],
    ocean_node: u32,
    cells: &[u32],
) -> Vec<PassEdge> {
    let mut edges = Vec::new();
    for &c in cells {
        push_lowest_passes(grid, elev, basin, ocean_node, &mut edges, c);
    }
    keep_lowest_pass_per_pair(&mut edges);
    edges
}

/// Sort by basin pair and keep the lowest pass (ties: lowest cell ids).
fn keep_lowest_pass_per_pair(edges: &mut Vec<PassEdge>) {
    edges.sort_unstable_by(|e, f| {
        e.a.cmp(&f.a)
            .then(e.b.cmp(&f.b))
            .then(f32_key(e.pass).cmp(&f32_key(f.pass)))
            .then(e.c.cmp(&f.c))
            .then(e.k.cmp(&f.k))
    });
    edges.dedup_by(|a, b| a.a == b.a && a.b == b.b);
}

/// One lowest pass from `c` to each neighbouring basin.
///
/// The pass `max(h_c, h_nb)` is the same from either end. Both cells of a
/// pit–pit pair are visited, so that pair is emitted once, from the lower cell
/// id. Cells of the ocean node are never visited, so links to it are always
/// emitted here. Later dedup keeps the lowest pass between two basins.
fn push_lowest_passes(
    grid: &CubeSphere,
    elev: &[f32],
    basin: &[u32],
    ocean_node: u32,
    edges: &mut Vec<PassEdge>,
    c: u32,
) {
    let na = basin[c as usize];
    if na == NONE {
        return;
    }
    let hc = elev[c as usize];
    let mut node = [NONE; 8];
    let mut pass = [0.0f32; 8];
    let mut nb_id = [NONE; 8];
    let mut used = 0usize;
    for nb in grid.neighbors(c) {
        if nb == NONE {
            continue;
        }
        let nb_node = basin[nb as usize];
        if nb_node == NONE || nb_node == na {
            continue;
        }
        if nb < c && nb_node != ocean_node {
            continue;
        }
        let height = hc.max(elev[nb as usize]);
        let mut found = None;
        for (slot, &seen) in node.iter().enumerate().take(used) {
            if seen == nb_node {
                found = Some(slot);
                break;
            }
        }
        if let Some(slot) = found {
            let better = match height.total_cmp(&pass[slot]) {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Equal => nb < nb_id[slot],
                std::cmp::Ordering::Greater => false,
            };
            if better {
                pass[slot] = height;
                nb_id[slot] = nb;
            }
        } else {
            node[used] = nb_node;
            pass[used] = height;
            nb_id[used] = nb;
            used += 1;
        }
    }
    for slot in 0..used {
        let nb = nb_id[slot];
        let nb_node = node[slot];
        let (a, b, ca, cb) = if na < nb_node {
            (na, nb_node, c, nb)
        } else {
            (nb_node, na, nb, c)
        };
        edges.push(PassEdge {
            a,
            b,
            pass: pass[slot],
            c: ca,
            k: cb,
        });
    }
}

fn depress(
    grid: &CubeSphere,
    elev: &[f32],
    domain: &[u8],
    scratch: &mut RoutingScratch,
    balance: Option<&BasinBalance<'_>>,
) -> RouteStats {
    scratch.lake_level.fill(f32::NAN);
    scratch.donors.clear();
    let n = elev.len();
    let pits = scratch
        .segs
        .iter()
        .filter(|seg| domain[seg.root as usize] == 1)
        .count() as u32;
    let has_ocean = scratch
        .segs
        .iter()
        .any(|seg| domain[seg.root as usize] == 0);
    let mut open = Vec::new();
    for seg in &scratch.segs {
        if domain[seg.root as usize] == 1 {
            open.push(*seg);
        }
    }
    if open.is_empty() {
        release_edges(scratch);
        return route_stats(pits, 0, 0, 0, Vec::new());
    }
    let ocean_nodes = usize::from(has_ocean);
    let node_count = open.len() + ocean_nodes;
    scratch.basin.fill(NONE);
    if has_ocean {
        for seg in &scratch.segs {
            if domain[seg.root as usize] == 0 {
                for idx in seg.start..seg.end {
                    scratch.basin[scratch.stack[idx as usize] as usize] = 0;
                }
            }
        }
    }
    let mut node_pit = vec![NONE; node_count];
    let mut node_start = vec![0u32; node_count];
    let mut node_end = vec![0u32; node_count];
    for (i, seg) in open.iter().enumerate() {
        let node = (i + ocean_nodes) as u32;
        node_pit[node as usize] = seg.root;
        node_start[node as usize] = seg.start;
        node_end[node as usize] = seg.end;
        for idx in seg.start..seg.end {
            scratch.basin[scratch.stack[idx as usize] as usize] = node;
        }
    }
    let ocean_node = if has_ocean { 0 } else { NONE };
    let parts: Vec<Vec<PassEdge>> = open
        .par_chunks(64)
        .map(|chunk| {
            let mut local = Vec::new();
            for seg in chunk {
                for idx in seg.start..seg.end {
                    push_lowest_passes(
                        grid,
                        elev,
                        &scratch.basin,
                        ocean_node,
                        &mut local,
                        scratch.stack[idx as usize],
                    );
                }
            }
            local
        })
        .collect();
    scratch.edges.clear();
    for part in parts {
        scratch.edges.extend(part);
    }
    let edges = u32::try_from(scratch.edges.len()).unwrap_or(u32::MAX);
    keep_lowest_pass_per_pair(&mut scratch.edges);
    scratch.edges.sort_unstable_by(|e, f| {
        f32_key(e.pass)
            .cmp(&f32_key(f.pass))
            .then(e.a.cmp(&f.a))
            .then(e.b.cmp(&f.b))
            .then(e.c.cmp(&f.c))
            .then(e.k.cmp(&f.k))
    });
    let mut uf = UnionFind::new(node_count);
    let mut mst = Vec::new();
    for (ei, edge) in scratch.edges.iter().enumerate() {
        let ra = uf.find(edge.a as usize);
        let rb = uf.find(edge.b as usize);
        if ra != rb {
            uf.union(ra, rb);
            mst.push(ei);
        }
    }
    let mut degree = vec![0u32; node_count];
    for &ei in &mst {
        let edge = &scratch.edges[ei];
        degree[edge.a as usize] += 1;
        degree[edge.b as usize] += 1;
    }
    let mut off = vec![0u32; node_count + 1];
    for i in 0..node_count {
        off[i + 1] = off[i] + degree[i];
    }
    scratch.cursor[..node_count].copy_from_slice(&off[..node_count]);
    let mut adj = vec![0u32; off[node_count] as usize];
    for &ei in &mst {
        let (ia, ib) = {
            let edge = &scratch.edges[ei];
            (edge.a as usize, edge.b as usize)
        };
        let slot_a = scratch.cursor[ia] as usize;
        scratch.cursor[ia] += 1;
        adj[slot_a] = ei as u32;
        let slot_b = scratch.cursor[ib] as usize;
        scratch.cursor[ib] += 1;
        adj[slot_b] = ei as u32;
    }
    let mut parent_edge = vec![NONE; node_count];
    let mut seen = vec![0u8; node_count];
    let mut queue = VecDeque::new();
    let root0 = if has_ocean {
        Some(0u32)
    } else {
        lowest_unseen(&node_pit, elev, &seen)
    };
    if let Some(root) = root0 {
        orient(
            root,
            &scratch.edges,
            &adj,
            &off,
            &mut parent_edge,
            &mut seen,
            &mut queue,
        );
    }
    while let Some(root) = lowest_unseen(&node_pit, elev, &seen) {
        orient(
            root,
            &scratch.edges,
            &adj,
            &off,
            &mut parent_edge,
            &mut seen,
            &mut queue,
        );
    }
    let (closed, closed_basins, closed_pits) = match balance {
        Some(balance) => close_basins(&ClosureIn {
            grid,
            elev,
            edges: &scratch.edges,
            stack: &scratch.stack,
            pit: &node_pit,
            start: &node_start,
            end: &node_end,
            parent_edge: &parent_edge,
            balance,
        }),
        None => (vec![0u8; node_count], 0, Vec::new()),
    };
    let mut carved = 0u32;
    for node in 0..node_count {
        if closed[node] == 1 {
            continue;
        }
        let ei = parent_edge[node];
        if ei == NONE {
            continue;
        }
        let edge = &scratch.edges[ei as usize];
        let node_u = node as u32;
        let (c_in, c_out, pass) = if edge.a == node_u {
            (edge.c, edge.k, edge.pass)
        } else {
            (edge.k, edge.c, edge.pass)
        };
        let pit = node_pit[node];
        let mut prev = c_out;
        let mut r = c_in;
        for _guard in 0..n {
            let next = scratch.receiver[r as usize];
            scratch.receiver[r as usize] = prev;
            scratch.receiver_dist[r as usize] = link_distance(grid, r, prev);
            if r == pit {
                break;
            }
            prev = r;
            r = next;
        }
        let start = node_start[node] as usize;
        let end = node_end[node] as usize;
        for idx in start..end {
            let cell = scratch.stack[idx] as usize;
            if elev[cell] < pass {
                scratch.lake_level[cell] = pass;
            }
        }
        carved += 1;
    }
    release_edges(scratch);
    route_stats(pits, carved, edges, closed_basins, closed_pits)
}

fn route_stats(
    pits: u32,
    carved: u32,
    edges: u32,
    closed_basins: u32,
    closed_pits: Vec<u32>,
) -> RouteStats {
    #[cfg(not(test))]
    let _ = closed_pits;
    RouteStats {
        pits,
        carved,
        edges,
        closed_basins,
        #[cfg(test)]
        closed_pits,
    }
}

struct ClosureIn<'a> {
    grid: &'a CubeSphere,
    elev: &'a [f32],
    edges: &'a [PassEdge],
    stack: &'a [u32],
    pit: &'a [u32],
    start: &'a [u32],
    end: &'a [u32],
    parent_edge: &'a [u32],
    balance: &'a BasinBalance<'a>,
}

/// Leaf-to-root water balance. A basin is closed when `e · A_lake >= inflow`.
fn close_basins(input: &ClosureIn<'_>) -> (Vec<u8>, u32, Vec<u32>) {
    let nodes = input.pit.len();
    let mut child_n = vec![0u32; nodes];
    let mut parent = vec![NONE; nodes];
    for (node, &ei) in input.parent_edge.iter().enumerate() {
        if ei == NONE {
            continue;
        }
        let edge = &input.edges[ei as usize];
        let up = if edge.a == node as u32 {
            edge.b
        } else {
            edge.a
        };
        parent[node] = up;
        child_n[up as usize] += 1;
    }
    let mut off = vec![0u32; nodes + 1];
    for i in 0..nodes {
        off[i + 1] = off[i] + child_n[i];
    }
    let mut cursor = off.clone();
    let mut kids = vec![0u32; off[nodes] as usize];
    for (node, &up) in parent.iter().enumerate() {
        if up == NONE {
            continue;
        }
        let slot = cursor[up as usize] as usize;
        cursor[up as usize] += 1;
        kids[slot] = node as u32;
    }
    let mut order = Vec::with_capacity(nodes);
    let mut walk = Vec::new();
    for (node, &up) in parent.iter().enumerate() {
        if up == NONE {
            walk.push((node as u32, 0u8));
        }
    }
    while let Some((node, seen)) = walk.pop() {
        if seen == 1 {
            order.push(node);
            continue;
        }
        walk.push((node, 1));
        let s = off[node as usize] as usize;
        let e = off[node as usize + 1] as usize;
        for &kid in kids[s..e].iter().rev() {
            walk.push((kid, 0));
        }
    }
    let mut overflow = vec![0.0f64; nodes];
    let mut open = vec![0u8; nodes];
    let mut closed = vec![0u8; nodes];
    let mut closed_pits = Vec::new();
    let mut closed_basins = 0u32;
    for &node_u in &order {
        let node = node_u as usize;
        let pit = input.pit[node];
        if pit == NONE {
            continue;
        }
        let start = input.start[node] as usize;
        let end = input.end[node] as usize;
        let mut q_local = 0.0f64;
        for idx in start..end {
            let cell = input.stack[idx];
            let runoff = f64::from(
                input
                    .balance
                    .runoff_m_yr
                    .get(cell as usize)
                    .copied()
                    .unwrap_or(0.0),
            )
            .max(0.0);
            q_local += runoff * input.grid.area_m2(cell);
        }
        let mut a_lake = 0.0f64;
        let ei = input.parent_edge[node];
        if ei != NONE {
            let pass = input.edges[ei as usize].pass;
            for idx in start..end {
                let cell = input.stack[idx];
                if input.elev[cell as usize] < pass {
                    a_lake += input.grid.area_m2(cell);
                }
            }
        }
        let evap = f64::from(
            input
                .balance
                .evap_m_yr
                .get(pit as usize)
                .copied()
                .unwrap_or(0.0),
        )
        .max(0.0);
        let mut inflow = q_local;
        let s = off[node] as usize;
        let e = off[node + 1] as usize;
        for &kid in &kids[s..e] {
            if open[kid as usize] == 1 {
                inflow += overflow[kid as usize];
            }
        }
        if evap * a_lake >= inflow {
            closed[node] = 1;
            overflow[node] = 0.0;
            closed_basins = closed_basins.saturating_add(1);
            closed_pits.push(pit);
        } else {
            open[node] = 1;
            overflow[node] = inflow - evap * a_lake;
        }
    }
    (closed, closed_basins, closed_pits)
}

/// Replace drainage area using `weight`. Used after glacier weights are known.
pub(crate) fn accumulate_with_weight(
    grid: &CubeSphere,
    scratch: &mut RoutingScratch,
    weight: &[f32],
) {
    scratch.weight.clear();
    scratch.weight.extend_from_slice(weight);
    drainage(grid, scratch);
}

fn release_edges(scratch: &mut RoutingScratch) {
    scratch.edges.clear();
}

fn lowest_unseen(node_pit: &[u32], elev: &[f32], seen: &[u8]) -> Option<u32> {
    let mut best_node: Option<u32> = None;
    let mut best_h = 0.0f32;
    let mut best_pit = 0u32;
    for (node, &pit) in node_pit.iter().enumerate() {
        if pit == NONE || seen[node] != 0 {
            continue;
        }
        let h = elev[pit as usize];
        let replace = match best_node {
            None => true,
            Some(_) => match h.total_cmp(&best_h) {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Equal => pit < best_pit,
                std::cmp::Ordering::Greater => false,
            },
        };
        if replace {
            best_node = Some(node as u32);
            best_h = h;
            best_pit = pit;
        }
    }
    best_node
}

fn orient(
    root: u32,
    edges: &[PassEdge],
    adj: &[u32],
    off: &[u32],
    parent_edge: &mut [u32],
    seen: &mut [u8],
    queue: &mut VecDeque<u32>,
) {
    let r = root as usize;
    if r >= seen.len() || seen[r] != 0 {
        return;
    }
    seen[r] = 1;
    queue.clear();
    queue.push_back(root);
    while let Some(node) = queue.pop_front() {
        let s = off[node as usize] as usize;
        let e = off[node as usize + 1] as usize;
        for &ei in &adj[s..e] {
            let edge = &edges[ei as usize];
            let other = if edge.a == node { edge.b } else { edge.a };
            if seen[other as usize] == 0 {
                seen[other as usize] = 1;
                parent_edge[other as usize] = ei;
                queue.push_back(other);
            }
        }
    }
}

fn link_distance(grid: &CubeSphere, a: u32, b: u32) -> f32 {
    let nbs = grid.neighbors(a);
    for (k, &nb) in nbs.iter().enumerate() {
        if nb == b {
            return grid.neighbor_distance(a, k);
        }
    }
    let pa = grid.position(a);
    let pb = grid.position(b);
    let c = crate::grid::cross(pa, pb);
    let s = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    let d = dot3(pa, pb).clamp(-1.0, 1.0);
    (s.atan2(d) * grid.radius_m) as f32
}

fn drainage(grid: &CubeSphere, scratch: &mut RoutingScratch) {
    let n = scratch.receiver.len();
    let parallel = scratch.parallel_basins();
    let RoutingScratch {
        area,
        stack_f64,
        stack,
        pos,
        chunks,
        receiver,
        weight,
        ..
    } = scratch;
    let weight = if weight.len() == n {
        Some(weight.as_slice())
    } else {
        None
    };
    run_basin_kernel(
        area,
        stack_f64,
        BasinView { stack, pos, chunks },
        parallel,
        |cell, _| {
            let weight = weight.map_or(1.0, |weight| f64::from(weight[cell]));
            grid.area_m2(cell as u32) * weight
        },
        |range, values| drainage_kernel(range, CellIndex, values, stack, receiver),
        |range, slots, values| drainage_kernel(range, slots, values, stack, receiver),
    );
}

fn drainage_kernel<S: SlotIndex>(
    range: Range<usize>,
    slots: S,
    values: &mut [f64],
    stack: &[u32],
    receiver: &[u32],
) -> u32 {
    for idx in range.rev() {
        let cell = stack[idx] as usize;
        let r = receiver[cell] as usize;
        if r != cell {
            let add = values[slots.current(idx, cell)];
            values[slots.at(r)] += add;
        }
    }
    0
}

/// Every cell reaches a root, `stack` is a permutation, and receivers come first.
pub fn validate_routing(receiver: &[u32], stack: &[u32]) -> Result<(), String> {
    let n = receiver.len();
    if stack.len() != n {
        return Err(format!("stack len {} != {n}", stack.len()));
    }
    let mut seen = vec![0u8; n];
    let mut pos = vec![0u32; n];
    for (i, &c) in stack.iter().enumerate() {
        if c as usize >= n || seen[c as usize] != 0 {
            return Err("stack is not a permutation".to_string());
        }
        seen[c as usize] = 1;
        pos[c as usize] = i as u32;
    }
    for (c, &r) in receiver.iter().enumerate() {
        if r as usize >= n {
            return Err(format!("receiver of {c} is out of range"));
        }
        if c != r as usize && pos[c] <= pos[r as usize] {
            return Err(format!("cell {c} appears before receiver {r}"));
        }
    }
    let mut color = vec![0u8; n];
    for s in 0..n {
        if color[s] != 0 {
            continue;
        }
        let mut path = Vec::new();
        let mut x = s;
        loop {
            if color[x] == 2 {
                break;
            }
            if color[x] == 1 {
                return Err(format!("cycle at {x}"));
            }
            color[x] = 1;
            path.push(x);
            let r = receiver[x] as usize;
            if r == x {
                break;
            }
            x = r;
            if path.len() > n {
                return Err("receiver path exceeded cell count".to_string());
            }
        }
        for p in path {
            color[p] = 2;
        }
    }
    Ok(())
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        while self.parent[x] != root {
            let next = self.parent[x];
            self.parent[x] = root;
            x = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let a = self.find(a);
        let b = self.find(b);
        if a == b {
            return;
        }
        if self.rank[a] < self.rank[b] {
            self.parent[a] = b;
        } else if self.rank[a] > self.rank[b] {
            self.parent[b] = a;
        } else {
            self.parent[b] = a;
            self.rank[a] = self.rank[a].saturating_add(1);
        }
    }
}

fn f32_key(x: f32) -> u32 {
    let bits = x.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits ^ 0x8000_0000
    }
}
