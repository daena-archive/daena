//! Steepest-descent receivers, donor stacks and depression carving.
//!
//! Depression routing follows Cordonnier, Bovy & Braun (2019): pit basins are
//! joined by a minimum spanning tree of passes and carved by reversing the
//! receiver path. A [`BasinBalance`] may keep a pit closed. `None` carves
//! every pit, which is the phase A behaviour.

use std::collections::VecDeque;
use std::ops::Range;
use std::sync::atomic::{AtomicU32, Ordering};

use rayon::prelude::*;

use crate::grid::{dot3, CubeSphere, NONE};

/// Cells that take part in routing and erosion, in ascending cell id.
///
/// The full index covers every cell and keeps the pre-compaction behaviour.
/// The active index holds the domain plus the ring of non-domain roots the
/// domain drains into. Slots are positions in `cells`; ascending slots keep
/// every tie-break the full-grid order had.
#[derive(Clone)]
pub(crate) struct DomainIndex {
    cells: Vec<u32>,
    /// Full-grid cell to slot, or [`NONE`].
    slot_of: Vec<u32>,
}

impl DomainIndex {
    /// Identity index over every cell.
    pub(crate) fn full(cell_count: usize) -> Self {
        let cells: Vec<u32> = (0..cell_count as u32).collect();
        let slot_of = cells.clone();
        Self { cells, slot_of }
    }

    /// Domain cells plus every non-domain neighbour they can drain into.
    pub(crate) fn active(grid: &CubeSphere, domain: &[u8]) -> Self {
        let mut include = vec![0u8; domain.len()];
        for (cell, &flag) in domain.iter().enumerate() {
            if flag == 0 {
                continue;
            }
            include[cell] = 1;
            for nb in grid.neighbors(cell as u32) {
                if nb != NONE {
                    include[nb as usize] = 1;
                }
            }
        }
        let mut cells = Vec::new();
        let mut slot_of = vec![NONE; domain.len()];
        for (cell, &flag) in include.iter().enumerate() {
            if flag == 1 {
                slot_of[cell] = cells.len() as u32;
                cells.push(cell as u32);
            }
        }
        Self { cells, slot_of }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }

    #[inline]
    pub(crate) fn cell(&self, slot: usize) -> u32 {
        self.cells[slot]
    }

    #[inline]
    pub(crate) fn slot(&self, cell: usize) -> usize {
        self.slot_of[cell] as usize
    }

    #[inline]
    pub(crate) fn is_active(&self, cell: usize) -> bool {
        self.slot_of[cell] != NONE
    }

    pub(crate) fn cells(&self) -> &[u32] {
        &self.cells
    }
}

/// Scratch buffers reused across iterations of one level.
pub struct RoutingScratch {
    /// Compact `1` = erodible domain, `0` = root. Rewritten by [`route`].
    domain: Vec<u8>,
    /// True when the full grid has at least one non-domain cell.
    has_outside: bool,
    /// Downstream neighbour slot, or the cell's own slot at a root.
    pub receiver: Vec<u32>,
    /// Distance to the receiver, in metres. Zero at a root.
    pub receiver_dist: Vec<f32>,
    /// CSR offsets into [`Self::donors`], length `cell_count + 1`.
    pub donor_offset: Vec<u32>,
    /// Donor cell ids, grouped by receiver.
    pub donors: Vec<u32>,
    /// Topological order: each cell appears after its receiver.
    pub stack: Vec<u32>,
    /// Scratch map from cell to depression-graph node, rewritten by each
    /// `depress` call.
    pub basin: Vec<u32>,
    /// Pass elevation for lake cells, otherwise NaN.
    pub lake_level: Vec<f32>,
    /// Drainage area in square metres, accumulated in `f64`.
    pub area: Vec<f64>,
    /// Stack position of each cell, used by the basin-parallel passes.
    pub(crate) pos: Vec<AtomicU32>,
    /// Contiguous stack ranges, each whole basins and about `chunk_cells`
    /// cells unless one basin is larger.
    pub(crate) chunks: Vec<(u32, u32)>,
    /// Stack-order work buffer for `f64` basin passes.
    pub(crate) stack_f64: Vec<f64>,
    /// Below this cell count the basin passes run in cell order, in place.
    pub(crate) parallel_min_cells: usize,
    /// Target cell count of one basin chunk.
    pub(crate) chunk_cells: usize,
    segs: Vec<BasinSeg>,
    cursor: Vec<u8>,
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

/// Outcome of the final-level outlet cut, one count per rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CarveStats {
    /// Overflowing basins whose outlet was cut.
    pub carved: u32,
    /// Basins left because at least a fifth of their cells subside.
    pub kept_subsiding: u32,
    /// Basins left because the last level's ice covers them.
    pub kept_glacial: u32,
    /// Basins left because evaporation beats inflow.
    pub kept_arid: u32,
}

/// Basin cells that must be glaciated for the glacial exemption. Percent.
const GLACIAL_KEEP_PERCENT: usize = 10;

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
    /// Allocate buffers for a full-index scratch over `cell_count` cells.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(cell_count: usize) -> Self {
        Self::with_index(&DomainIndex::full(cell_count))
    }

    /// Allocate buffers on a compact domain index.
    pub(crate) fn with_index(cell_index: &DomainIndex) -> Self {
        let len = cell_index.len();
        Self {
            domain: vec![0; len],
            has_outside: false,
            receiver: vec![0; len],
            receiver_dist: vec![0.0; len],
            donor_offset: vec![0; len + 1],
            donors: Vec::new(),
            stack: Vec::with_capacity(len),
            basin: vec![0; len],
            lake_level: vec![f32::NAN; len],
            area: vec![0.0; len],
            pos: (0..len).map(|_| AtomicU32::new(0)).collect(),
            chunks: Vec::new(),
            stack_f64: vec![0.0; len],
            parallel_min_cells: PARALLEL_BASIN_MIN_CELLS,
            chunk_cells: BASIN_CHUNK_CELLS,
            segs: Vec::new(),
            cursor: vec![0; len],
            dfs: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Copy the full-grid domain mask into slot order.
    pub(crate) fn set_domain(&mut self, index: &DomainIndex, domain: &[u8]) {
        debug_assert_eq!(domain.len(), index.slot_of.len());
        self.domain.resize(index.len(), 0);
        for (slot, &cell) in index.cells.iter().enumerate() {
            self.domain[slot] = domain[cell as usize];
        }
        self.has_outside = domain.contains(&0);
    }

    /// Carve a through-outlet for every overflowing closed basin left by the
    /// last routing pass. `h` is the full-grid elevation; `runoff` and `evap`
    /// are metres per year in slot order; `glaciated` is the last level's
    /// glacier mask in slot order.
    ///
    /// This is the geological breach: the outlet is lowered through the rim to
    /// below the basin floor, so the basin drains and the channel is visible.
    /// Basins kept open by active subsidence or glacial overdeepening stay.
    pub(crate) fn carve_overflow_outlets(
        &self,
        index: &DomainIndex,
        h: &mut [f32],
        uplift: &[f32],
        water: Option<(&[f32], &[f32])>,
        glaciated: Option<&[u8]>,
    ) -> CarveStats {
        let mut stats = CarveStats::default();
        for seg in &self.segs {
            let start = seg.start as usize;
            let end = seg.end as usize;
            if start >= end {
                continue;
            }
            let cells = &self.stack[start..end];
            let mut floor = f32::INFINITY;
            let mut spill = f32::NEG_INFINITY;
            for &slot in cells {
                let cell = index.cell(slot as usize);
                let height = h[cell as usize];
                floor = floor.min(height);
                let level = self.lake_level[slot as usize];
                if level.is_finite() {
                    spill = spill.max(level);
                }
            }
            if !spill.is_finite() {
                continue;
            }
            // An actively subsiding rift basin outpaces its outlet; leave it.
            let mut subsiding = 0usize;
            let mut glacial = 0usize;
            for &slot in cells {
                let cell = index.cell(slot as usize);
                if uplift[cell as usize] < 0.0 {
                    subsiding += 1;
                }
                if glaciated.is_some_and(|mask| mask[slot as usize] != 0) {
                    glacial += 1;
                }
            }
            if subsiding * 5 >= cells.len() {
                stats.kept_subsiding += 1;
                continue;
            }
            // A basin under the last level's ice is glacially overdeepened.
            if glacial * 100 >= cells.len() * GLACIAL_KEEP_PERCENT {
                stats.kept_glacial += 1;
                continue;
            }
            if let Some((runoff, evap)) = water {
                let mut net = 0.0f64;
                for &slot in cells {
                    net += f64::from(runoff[slot as usize]) - f64::from(evap[slot as usize]);
                }
                if net <= 0.0 {
                    stats.kept_arid += 1;
                    continue;
                }
            }
            // Follow the reversed receiver path from the pit through the spill
            // and one cell beyond it, cutting a monotonic channel.
            let mut slot = seg.root as usize;
            let mut step = 0usize;
            loop {
                let cell = index.cell(slot);
                let target = floor - 0.5 - 0.02 * step as f32;
                if h[cell as usize] > target {
                    h[cell as usize] = target;
                }
                let next = self.receiver[slot] as usize;
                if next == slot || step >= 4096 {
                    break;
                }
                let next_cell = index.cell(next);
                if h[next_cell as usize] < floor {
                    let target = floor - 0.5 - 0.02 * (step + 1) as f32;
                    if h[next_cell as usize] > target {
                        h[next_cell as usize] = target;
                    }
                    break;
                }
                slot = next;
                step += 1;
            }
            stats.carved += 1;
        }
        stats
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
            self.pos.resize_with(n, || AtomicU32::new(0));
            self.chunks.clear();
            self.stack_f64.resize(n, 0.0);
            self.cursor.resize(n, 0);
            self.donor_offset.resize(n + 1, 0);
        }
    }

    /// Resize a recycled scratch for a new slot count.
    pub(crate) fn rebase(&mut self, slot_count: usize) {
        self.resize(slot_count);
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
    pos: &'a [AtomicU32],
    base: usize,
}

impl SlotIndex for Slots<'_> {
    #[inline(always)]
    fn at(self, cell: usize) -> usize {
        self.pos[cell].load(Ordering::Relaxed) as usize - self.base
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
    pub pos: &'a [AtomicU32],
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
        *slot = work[view.pos[cell].load(Ordering::Relaxed) as usize];
    });
    stats
}

/// Route `elev` over `domain` (`1` = erodible). `weight` defaults to 1.
///
/// `balance: None` carves every pit. A balance builds the same tree, then
/// closes a pit when evaporation over the area below its pass meets the inflow.
#[cfg_attr(not(test), allow(dead_code))]
pub fn route(
    grid: &CubeSphere,
    elev: &[f32],
    domain: &[u8],
    weight: Option<&[f32]>,
    scratch: &mut RoutingScratch,
    balance: Option<&BasinBalance<'_>>,
) -> RouteStats {
    let index = DomainIndex::full(elev.len());
    route_with_cancel(
        grid,
        elev,
        domain,
        &index,
        weight,
        scratch,
        balance,
        &|| false,
    )
    .expect("routing without a cancellation callback cannot cancel")
}

/// A cancellation callback that rayon tasks may call.
pub(crate) type Cancel<'a> = &'a (dyn Fn() -> bool + Sync);

/// Cells between cancellation checks in sequential graph loops.
const CANCEL_STRIDE: usize = 65_536;

#[inline]
fn check_stride(iteration: usize, cancelled: Cancel<'_>) -> Result<(), crate::WorldError> {
    if iteration.is_multiple_of(CANCEL_STRIDE) && cancelled() {
        Err(crate::WorldError::Cancelled)
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn route_with_cancel(
    grid: &CubeSphere,
    elev: &[f32],
    domain: &[u8],
    index: &DomainIndex,
    weight: Option<&[f32]>,
    scratch: &mut RoutingScratch,
    balance: Option<&BasinBalance<'_>>,
    cancelled: Cancel<'_>,
) -> Result<RouteStats, crate::WorldError> {
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    let n = elev.len();
    if n != index.len() {
        return Err(crate::WorldError::Internal(format!(
            "routing elevation has {n} slots, index has {}",
            index.len()
        )));
    }
    if weight.is_some_and(|weight| weight.len() != n) {
        return Err(crate::WorldError::Internal(
            "routing weight does not match the slot count".to_string(),
        ));
    }
    scratch.resize(n);
    scratch.set_domain(index, domain);
    receivers(grid, index, elev, scratch);
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    build_donors(grid, index, scratch, cancelled)?;
    build_pit_stack(scratch, cancelled)?;
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    let stats = depress(grid, index, elev, scratch, balance, cancelled)?;
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    // Depression carving clears the donor graph. Rebuild it from the receivers
    // that carving left, including the case where nothing was carved.
    if stats.carved > 0 {
        build_donors(grid, index, scratch, cancelled)?;
    }
    build_stack(scratch, cancelled)?;
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    drainage(grid, index, scratch, weight);
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    #[cfg(debug_assertions)]
    {
        debug_assert_receivers_are_neighbors(grid, index, &scratch.receiver);
        if let Err(err) = validate_routing(&scratch.receiver, &scratch.stack) {
            panic!("routing validation failed: {err}");
        }
    }
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    Ok(stats)
}

#[cfg(debug_assertions)]
fn debug_assert_receivers_are_neighbors(grid: &CubeSphere, index: &DomainIndex, receiver: &[u32]) {
    for (slot, &r) in receiver.iter().enumerate() {
        if r as usize != slot {
            let cell = index.cell(slot);
            let target = index.cell(r as usize);
            debug_assert!(
                grid.neighbors(cell).contains(&target),
                "receiver of slot {slot} (cell {cell}) is cell {target}, not a neighbour"
            );
        }
    }
}

fn receivers(grid: &CubeSphere, index: &DomainIndex, elev: &[f32], scratch: &mut RoutingScratch) {
    let domain = &scratch.domain;
    let receiver = &mut scratch.receiver;
    let receiver_dist = &mut scratch.receiver_dist;
    receiver
        .par_iter_mut()
        .zip(receiver_dist.par_iter_mut())
        .enumerate()
        .for_each(|(slot, (slot_receiver, dist_slot))| {
            if domain[slot] == 0 {
                *slot_receiver = slot as u32;
                *dist_slot = 0.0;
                return;
            }
            let cell = index.cell(slot);
            let h = elev[slot];
            let nbs = grid.neighbors(cell);
            let mut best_slope = 0.0f32;
            let mut best = slot as u32;
            let mut best_d = 0.0f32;
            for (k, &nb) in nbs.iter().enumerate() {
                if nb == NONE {
                    continue;
                }
                let d = grid.neighbor_distance(cell, k);
                if d <= 0.0 {
                    continue;
                }
                let nb_slot = index.slot(nb as usize);
                debug_assert!(index.is_active(nb as usize));
                let slope = (h - elev[nb_slot]) / d;
                if slope > best_slope {
                    best_slope = slope;
                    best = nb_slot as u32;
                    best_d = d;
                }
            }
            *slot_receiver = best;
            *dist_slot = best_d;
        });
}

/// Cells per task in the donor fill and the stack build.
const GRAPH_GROUP_CELLS: usize = 16_384;

fn donor_count(grid: &CubeSphere, index: &DomainIndex, receiver: &[u32], slot: usize) -> u8 {
    let mut count = 0u8;
    for nb in grid.neighbors(index.cell(slot)) {
        if nb != NONE && index.is_active(nb as usize) {
            let nb_slot = index.slot(nb as usize);
            if receiver[nb_slot] == slot as u32 {
                count += 1;
            }
        }
    }
    count
}

fn fill_donor_group(
    grid: &CubeSphere,
    index: &DomainIndex,
    receiver: &[u32],
    offsets: &[u32],
    out: &mut [u32],
    start: usize,
    end: usize,
) {
    let mut used = 0usize;
    for slot in start..end {
        let count = (offsets[slot + 1] - offsets[slot]) as usize;
        if count == 0 {
            continue;
        }
        let slice = &mut out[used..used + count];
        used += count;
        let mut position = 0usize;
        for nb in grid.neighbors(index.cell(slot)) {
            if nb != NONE && index.is_active(nb as usize) {
                let nb_slot = index.slot(nb as usize);
                if receiver[nb_slot] == slot as u32 {
                    slice[position] = nb_slot as u32;
                    position += 1;
                }
            }
        }
        slice.sort_unstable();
    }
}

/// CSR donor lists. A donor is always one of the eight neighbours and
/// neighbour links are symmetric, so each cell finds its own donors and the
/// lists are written in ascending cell id.
fn build_donors(
    grid: &CubeSphere,
    index: &DomainIndex,
    scratch: &mut RoutingScratch,
    cancelled: Cancel<'_>,
) -> Result<(), crate::WorldError> {
    let n = scratch.receiver.len();
    let parallel = scratch.parallel_basins();
    let receiver = scratch.receiver.as_slice();
    if parallel {
        scratch
            .cursor
            .par_iter_mut()
            .enumerate()
            .for_each(|(slot, count_slot)| {
                *count_slot = donor_count(grid, index, receiver, slot);
            });
    } else {
        for (slot, count_slot) in scratch.cursor.iter_mut().enumerate() {
            check_stride(slot, cancelled)?;
            *count_slot = donor_count(grid, index, receiver, slot);
        }
    }
    scratch.donor_offset[0] = 0;
    for c in 0..n {
        check_stride(c, cancelled)?;
        scratch.donor_offset[c + 1] = scratch.donor_offset[c] + u32::from(scratch.cursor[c]);
    }
    let total = scratch.donor_offset[n] as usize;
    scratch.donors.clear();
    scratch.donors.resize(total, 0);
    let offsets = scratch.donor_offset.as_slice();
    let mut rest = scratch.donors.as_mut_slice();
    let mut jobs = Vec::with_capacity(n / GRAPH_GROUP_CELLS + 1);
    for start in (0..n).step_by(GRAPH_GROUP_CELLS) {
        let end = (start + GRAPH_GROUP_CELLS).min(n);
        let len = (offsets[end] - offsets[start]) as usize;
        let (head, tail) = std::mem::take(&mut rest).split_at_mut(len);
        rest = tail;
        jobs.push((start, end, head));
    }
    if parallel {
        jobs.into_par_iter().for_each(|(start, end, out)| {
            fill_donor_group(grid, index, receiver, offsets, out, start, end);
        });
    } else {
        for (start, end, out) in jobs {
            if cancelled() {
                return Err(crate::WorldError::Cancelled);
            }
            fill_donor_group(grid, index, receiver, offsets, out, start, end);
        }
    }
    Ok(())
}

/// Depth-first order of one basin, appended to `out`.
#[inline]
fn push_basin(
    root: u32,
    donor_offset: &[u32],
    donors: &[u32],
    dfs: &mut Vec<u32>,
    out: &mut Vec<u32>,
    cancelled: Cancel<'_>,
) -> Result<(), crate::WorldError> {
    let root_s = donor_offset[root as usize] as usize;
    let root_e = donor_offset[root as usize + 1] as usize;
    if root_s == root_e {
        out.push(root);
        return Ok(());
    }
    dfs.clear();
    dfs.push(root);
    let mut steps = 0usize;
    while let Some(node) = dfs.pop() {
        check_stride(steps, cancelled)?;
        steps += 1;
        out.push(node);
        let s = donor_offset[node as usize] as usize;
        let e = donor_offset[node as usize + 1] as usize;
        for &donor in donors[s..e].iter().rev() {
            dfs.push(donor);
        }
    }
    Ok(())
}

/// Members of every pit basin, in the same depth-first order as the full
/// stack. Depression routing needs nothing else from the pre-carve graph.
fn build_pit_stack(
    scratch: &mut RoutingScratch,
    cancelled: Cancel<'_>,
) -> Result<(), crate::WorldError> {
    let n = scratch.receiver.len();
    let parallel = scratch.parallel_basins();
    let receiver = scratch.receiver.as_slice();
    let domain = scratch.domain.as_slice();
    let is_root = |slot: &usize| receiver[*slot] as usize == *slot && domain[*slot] == 1;
    let roots: Vec<u32> = if parallel {
        (0..n)
            .into_par_iter()
            .filter(is_root)
            .map(|slot| slot as u32)
            .collect()
    } else {
        let mut roots = Vec::new();
        for slot in 0..n {
            check_stride(slot, cancelled)?;
            if is_root(&slot) {
                roots.push(slot as u32);
            }
        }
        roots
    };
    scratch.stack.clear();
    scratch.segs.clear();
    for (index, root) in roots.into_iter().enumerate() {
        check_stride(index, cancelled)?;
        let start = scratch.stack.len() as u32;
        push_basin(
            root,
            &scratch.donor_offset,
            &scratch.donors,
            &mut scratch.dfs,
            &mut scratch.stack,
            cancelled,
        )?;
        scratch.segs.push(BasinSeg {
            root,
            start,
            end: scratch.stack.len() as u32,
        });
    }
    Ok(())
}

/// Topological order of every basin, roots in ascending cell id. Basins are
/// independent, so groups of roots are walked in parallel and joined in order.
fn build_stack(
    scratch: &mut RoutingScratch,
    cancelled: Cancel<'_>,
) -> Result<(), crate::WorldError> {
    let n = scratch.receiver.len();
    let target = scratch.chunk_cells;
    let parallel = scratch.parallel_basins();
    let receiver = scratch.receiver.as_slice();
    let donor_offset = scratch.donor_offset.as_slice();
    let donors = scratch.donors.as_slice();
    let groups = n.div_ceil(GRAPH_GROUP_CELLS);
    let build_group = |group: usize| -> Result<(Vec<u32>, Vec<u32>), crate::WorldError> {
        let start = group * GRAPH_GROUP_CELLS;
        let end = (start + GRAPH_GROUP_CELLS).min(n);
        let mut local = Vec::with_capacity(end - start);
        let mut cuts = Vec::new();
        let mut dfs = Vec::new();
        let mut count = 0usize;
        for (index, root) in (start..end).enumerate() {
            check_stride(index, cancelled)?;
            if receiver[root] as usize != root {
                continue;
            }
            let seg_start = local.len();
            push_basin(
                root as u32,
                donor_offset,
                donors,
                &mut dfs,
                &mut local,
                cancelled,
            )?;
            let len = local.len() - seg_start;
            if count > 0 && count + len > target {
                cuts.push(seg_start as u32);
                count = 0;
            }
            count += len;
        }
        Ok((local, cuts))
    };
    let parts: Vec<(Vec<u32>, Vec<u32>)> = if parallel {
        (0..groups)
            .into_par_iter()
            .map(build_group)
            .collect::<Result<_, _>>()?
    } else {
        (0..groups).map(build_group).collect::<Result<_, _>>()?
    };
    scratch.stack.clear();
    scratch.stack.resize(n, 0);
    scratch.chunks.clear();
    let mut rest = scratch.stack.as_mut_slice();
    let mut jobs = Vec::with_capacity(parts.len());
    let mut base = 0u32;
    for (index, (local, cuts)) in parts.iter().enumerate() {
        check_stride(index, cancelled)?;
        let (head, tail) = std::mem::take(&mut rest).split_at_mut(local.len());
        rest = tail;
        if !local.is_empty() {
            let mut from = base;
            for cut in cuts {
                scratch.chunks.push((from, base + cut));
                from = base + cut;
            }
            scratch.chunks.push((from, base + local.len() as u32));
        }
        jobs.push((base, local, head));
        base += local.len() as u32;
    }
    let pos = scratch.pos.as_slice();
    let copy_group = |(base, local, out): (u32, &Vec<u32>, &mut [u32])| {
        out.copy_from_slice(local);
        for (k, &cell) in local.iter().enumerate() {
            pos[cell as usize].store(base + k as u32, Ordering::Relaxed);
        }
    };
    if parallel {
        jobs.into_par_iter().for_each(copy_group);
    } else {
        for job in jobs {
            copy_group(job);
        }
    }
    Ok(())
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
    let index = DomainIndex::full(elev.len());
    let mut edges = Vec::new();
    for &c in cells {
        push_lowest_passes(grid, elev, &index, basin, ocean_node, &mut edges, c);
    }
    keep_lowest_pass_per_pair(&mut edges);
    edges
}

/// Sort by basin pair and keep the lowest pass (ties: lowest cell ids).
fn keep_lowest_pass_per_pair(edges: &mut Vec<PassEdge>) {
    edges.par_sort_unstable_by(|e, f| {
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
    index: &DomainIndex,
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
    let cell = index.cell(c as usize);
    for nb_cell in grid.neighbors(cell) {
        if nb_cell == NONE {
            continue;
        }
        let nb = index.slot(nb_cell as usize) as u32;
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
    index: &DomainIndex,
    elev: &[f32],
    scratch: &mut RoutingScratch,
    balance: Option<&BasinBalance<'_>>,
    cancelled: Cancel<'_>,
) -> Result<RouteStats, crate::WorldError> {
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    scratch.lake_level.fill(f32::NAN);
    let n = elev.len();
    // Every stored segment is a pit basin; the rest of the sphere drains to
    // a root outside the domain and is the one ocean node.
    let pits = scratch.segs.len() as u32;
    let has_ocean = scratch.has_outside;
    let open = scratch.segs.clone();
    if open.is_empty() {
        release_edges(scratch);
        return Ok(route_stats(pits, 0, 0, 0, Vec::new()));
    }
    let ocean_nodes = usize::from(has_ocean);
    let node_count = open.len() + ocean_nodes;
    scratch
        .basin
        .par_iter_mut()
        .for_each(|slot| *slot = if has_ocean { 0 } else { NONE });
    let mut node_pit = vec![NONE; node_count];
    let mut node_start = vec![0u32; node_count];
    let mut node_end = vec![0u32; node_count];
    for (i, seg) in open.iter().enumerate() {
        check_stride(i, cancelled)?;
        let node = (i + ocean_nodes) as u32;
        node_pit[node as usize] = seg.root;
        node_start[node as usize] = seg.start;
        node_end[node as usize] = seg.end;
        for idx in seg.start..seg.end {
            scratch.basin[scratch.stack[idx as usize] as usize] = node;
        }
    }
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    let ocean_node = if has_ocean { 0 } else { NONE };
    let parallel = scratch.parallel_basins();
    let collect_part = |chunk: &[BasinSeg]| -> Result<Vec<PassEdge>, crate::WorldError> {
        let mut local = Vec::new();
        let mut steps = 0usize;
        for seg in chunk {
            for idx in seg.start..seg.end {
                check_stride(steps, cancelled)?;
                steps += 1;
                push_lowest_passes(
                    grid,
                    elev,
                    index,
                    &scratch.basin,
                    ocean_node,
                    &mut local,
                    scratch.stack[idx as usize],
                );
            }
        }
        Ok(local)
    };
    let parts: Vec<Vec<PassEdge>> = if parallel {
        open.par_chunks(64)
            .map(collect_part)
            .collect::<Result<_, _>>()?
    } else {
        open.chunks(64)
            .map(collect_part)
            .collect::<Result<_, _>>()?
    };
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    scratch.edges.clear();
    for part in parts {
        scratch.edges.extend(part);
    }
    let edges = u32::try_from(scratch.edges.len()).unwrap_or(u32::MAX);
    keep_lowest_pass_per_pair(&mut scratch.edges);
    scratch.edges.par_sort_unstable_by(|e, f| {
        f32_key(e.pass)
            .cmp(&f32_key(f.pass))
            .then(e.a.cmp(&f.a))
            .then(e.b.cmp(&f.b))
            .then(e.c.cmp(&f.c))
            .then(e.k.cmp(&f.k))
    });
    if cancelled() {
        return Err(crate::WorldError::Cancelled);
    }
    let mut uf = UnionFind::new(node_count);
    let mut mst = Vec::new();
    for (ei, edge) in scratch.edges.iter().enumerate() {
        check_stride(ei, cancelled)?;
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
    let mut cursor = off.clone();
    let mut adj = vec![0u32; off[node_count] as usize];
    for &ei in &mst {
        let (ia, ib) = {
            let edge = &scratch.edges[ei];
            (edge.a as usize, edge.b as usize)
        };
        let slot_a = cursor[ia] as usize;
        cursor[ia] += 1;
        adj[slot_a] = ei as u32;
        let slot_b = cursor[ib] as usize;
        cursor[ib] += 1;
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
            cancelled,
        )?;
    }
    let mut rounds = 0usize;
    while let Some(root) = lowest_unseen(&node_pit, elev, &seen) {
        check_stride(rounds, cancelled)?;
        rounds += 1;
        orient(
            root,
            &scratch.edges,
            &adj,
            &off,
            &mut parent_edge,
            &mut seen,
            &mut queue,
            cancelled,
        )?;
    }
    let (closed, closed_basins, closed_pits) = match balance {
        Some(balance) => close_basins(
            &ClosureIn {
                grid,
                index,
                elev,
                edges: &scratch.edges,
                stack: &scratch.stack,
                pit: &node_pit,
                start: &node_start,
                end: &node_end,
                parent_edge: &parent_edge,
                balance,
            },
            cancelled,
        )?,
        None => (vec![0u8; node_count], 0, Vec::new()),
    };
    let mut carved = 0u32;
    for node in 0..node_count {
        check_stride(node, cancelled)?;
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
        for guard in 0..n {
            check_stride(guard, cancelled)?;
            let next = scratch.receiver[r as usize];
            scratch.receiver[r as usize] = prev;
            scratch.receiver_dist[r as usize] = link_distance(grid, index, r, prev);
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
    #[cfg(test)]
    let closed_pits: Vec<u32> = closed_pits
        .into_iter()
        .map(|slot| index.cell(slot as usize))
        .collect();
    Ok(route_stats(pits, carved, edges, closed_basins, closed_pits))
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
    index: &'a DomainIndex,
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
fn close_basins(
    input: &ClosureIn<'_>,
    cancelled: Cancel<'_>,
) -> Result<(Vec<u8>, u32, Vec<u32>), crate::WorldError> {
    let nodes = input.pit.len();
    let mut child_n = vec![0u32; nodes];
    let mut parent = vec![NONE; nodes];
    for (node, &ei) in input.parent_edge.iter().enumerate() {
        check_stride(node, cancelled)?;
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
        check_stride(order.len(), cancelled)?;
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
    for (index, &node_u) in order.iter().enumerate() {
        check_stride(index, cancelled)?;
        let node = node_u as usize;
        let pit = input.pit[node];
        if pit == NONE {
            continue;
        }
        let start = input.start[node] as usize;
        let end = input.end[node] as usize;
        let mut q_local = 0.0f64;
        for idx in start..end {
            let slot = input.stack[idx] as usize;
            let runoff =
                f64::from(input.balance.runoff_m_yr.get(slot).copied().unwrap_or(0.0)).max(0.0);
            q_local += runoff * input.grid.area_m2(input.index.cell(slot));
        }
        let mut a_lake = 0.0f64;
        let ei = input.parent_edge[node];
        if ei != NONE {
            let pass = input.edges[ei as usize].pass;
            for idx in start..end {
                let slot = input.stack[idx] as usize;
                if input.elev[slot] < pass {
                    a_lake += input.grid.area_m2(input.index.cell(slot));
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
    Ok((closed, closed_basins, closed_pits))
}

/// Replace drainage area using `weight`. Used after glacier weights are known.
pub(crate) fn accumulate_with_weight(
    grid: &CubeSphere,
    index: &DomainIndex,
    scratch: &mut RoutingScratch,
    weight: &[f32],
) {
    drainage(grid, index, scratch, Some(weight));
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

#[allow(clippy::too_many_arguments)]
fn orient(
    root: u32,
    edges: &[PassEdge],
    adj: &[u32],
    off: &[u32],
    parent_edge: &mut [u32],
    seen: &mut [u8],
    queue: &mut VecDeque<u32>,
    cancelled: Cancel<'_>,
) -> Result<(), crate::WorldError> {
    let r = root as usize;
    if r >= seen.len() || seen[r] != 0 {
        return Ok(());
    }
    seen[r] = 1;
    queue.clear();
    queue.push_back(root);
    let mut steps = 0usize;
    while let Some(node) = queue.pop_front() {
        check_stride(steps, cancelled)?;
        steps += 1;
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
    Ok(())
}

fn link_distance(grid: &CubeSphere, index: &DomainIndex, a: u32, b: u32) -> f32 {
    let a_cell = index.cell(a as usize);
    let b_cell = index.cell(b as usize);
    let nbs = grid.neighbors(a_cell);
    for (k, &nb) in nbs.iter().enumerate() {
        if nb == b_cell {
            return grid.neighbor_distance(a_cell, k);
        }
    }
    let pa = grid.position(a_cell);
    let pb = grid.position(b_cell);
    let c = crate::grid::cross(pa, pb);
    let s = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    let d = dot3(pa, pb).clamp(-1.0, 1.0);
    (s.atan2(d) * grid.radius_m) as f32
}

fn drainage(
    grid: &CubeSphere,
    index: &DomainIndex,
    scratch: &mut RoutingScratch,
    weight: Option<&[f32]>,
) {
    let n = scratch.receiver.len();
    let parallel = scratch.parallel_basins();
    let RoutingScratch {
        area,
        stack_f64,
        stack,
        pos,
        chunks,
        receiver,
        ..
    } = scratch;
    let weight = weight.filter(|weight| weight.len() == n);
    run_basin_kernel(
        area,
        stack_f64,
        BasinView { stack, pos, chunks },
        parallel,
        |slot, _| {
            let weight = weight.map_or(1.0, |weight| f64::from(weight[slot]));
            grid.area_m2(index.cell(slot)) * weight
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
#[cfg_attr(not(debug_assertions), allow(dead_code))]
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
