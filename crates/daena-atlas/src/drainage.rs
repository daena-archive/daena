//! Topology-affecting atlas-only minor tributaries.

use std::collections::BTreeSet;

use daena_physical::hydrology::HydrologyField;
use daena_physical::Grid;

use crate::detail::{domain_key, lattice_sample, nearest_cell};
use crate::request::DetailLevel;
use crate::{AtlasError, ATLAS_DERIVED_DRAINAGE_VERSION};

pub const MINOR_TRIBUTARY_DOMAIN: &str = "minor-tributaries";
pub const MAX_DERIVED_TRIBUTARIES: usize = 512;
const MAX_TRACE_STEPS: usize = 32;
const OCEAN: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedTributary {
    pub id: String,
    pub source_cell: usize,
    pub join_cell: usize,
    pub parent_river_id: u32,
    pub watershed_id: u32,
    pub path: Vec<[i32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DerivedDrainage {
    pub version: u32,
    pub tributaries: Vec<DerivedTributary>,
}

impl DerivedDrainage {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&(self.tributaries.len() as u32).to_le_bytes());
        for tributary in &self.tributaries {
            bytes.extend_from_slice(&(tributary.source_cell as u32).to_le_bytes());
            bytes.extend_from_slice(&(tributary.join_cell as u32).to_le_bytes());
            bytes.extend_from_slice(&tributary.parent_river_id.to_le_bytes());
            bytes.extend_from_slice(&tributary.watershed_id.to_le_bytes());
            bytes.extend_from_slice(&(tributary.path.len() as u32).to_le_bytes());
            for point in &tributary.path {
                bytes.extend_from_slice(&point[0].to_le_bytes());
                bytes.extend_from_slice(&point[1].to_le_bytes());
            }
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, AtlasError> {
        let mut offset = 0;
        let version = read_u32(bytes, &mut offset)?;
        if version != ATLAS_DERIVED_DRAINAGE_VERSION {
            return Err(AtlasError::invalid(
                "derived drainage cache version mismatch",
            ));
        }
        let count = read_u32(bytes, &mut offset)? as usize;
        if count > MAX_DERIVED_TRIBUTARIES {
            return Err(AtlasError::limit("derived drainage cache is over budget"));
        }
        let mut tributaries = Vec::with_capacity(count);
        for _ in 0..count {
            let source_cell = read_u32(bytes, &mut offset)? as usize;
            let join_cell = read_u32(bytes, &mut offset)? as usize;
            let parent_river_id = read_u32(bytes, &mut offset)?;
            let watershed_id = read_u32(bytes, &mut offset)?;
            let path_len = read_u32(bytes, &mut offset)? as usize;
            if path_len > MAX_TRACE_STEPS + 1 {
                return Err(AtlasError::limit("derived tributary path is over budget"));
            }
            let mut path = Vec::with_capacity(path_len);
            for _ in 0..path_len {
                let lon = read_i32(bytes, &mut offset)?;
                let lat = read_i32(bytes, &mut offset)?;
                path.push([lon, lat]);
            }
            tributaries.push(DerivedTributary {
                id: tributary_id(source_cell),
                source_cell,
                join_cell,
                parent_river_id,
                watershed_id,
                path,
            });
        }
        if offset != bytes.len() {
            return Err(AtlasError::invalid(
                "derived drainage cache has trailing bytes",
            ));
        }
        Ok(Self {
            version,
            tributaries,
        })
    }
}

pub fn tributary_id(source_cell: usize) -> String {
    format!("atlas:tributary:v{ATLAS_DERIVED_DRAINAGE_VERSION}:{source_cell}")
}

fn keep_threshold_ppm(level: DetailLevel) -> u32 {
    match level {
        DetailLevel::Standard => 400,
        DetailLevel::Detailed => 900,
        DetailLevel::Print => 1_600,
    }
}

fn cell_distance(grid: Grid, from: usize, to: usize) -> u32 {
    let (from_row, from_col) = grid.row_col(from);
    let (to_row, to_col) = grid.row_col(to);
    let drow = from_row.abs_diff(to_row);
    let dcol = from_col.abs_diff(to_col);
    let wrapped = dcol.min(grid.width.saturating_sub(dcol));
    drow.max(wrapped)
}

fn coordinate_for_cell(grid: Grid, cell: usize) -> [i32; 2] {
    let (row, col) = grid.row_col(cell);
    [
        (-180_000_000i64 + 360_000_000i64 * (i64::from(col) * 2 + 1) / i64::from(grid.width * 2))
            as i32,
        (-90_000_000i64 + 180_000_000i64 * (i64::from(row) * 2 + 1) / i64::from(grid.height * 2))
            as i32,
    ]
}

fn river_cells(hydrology: &HydrologyField) -> (BTreeSet<usize>, Vec<Vec<usize>>) {
    let mut cells = BTreeSet::new();
    let mut per_river = Vec::with_capacity(hydrology.rivers.len());
    for path in &hydrology.river_coordinates {
        let mut river = Vec::new();
        for point in path {
            let cell = nearest_cell(hydrology.grid, point[0], point[1]);
            cells.insert(cell);
            if river.last().copied() != Some(cell) {
                river.push(cell);
            }
        }
        per_river.push(river);
    }
    (cells, per_river)
}

fn same_partition(hydrology: &HydrologyField, cell: usize, join: usize) -> bool {
    let watershed = hydrology.watershed_id.get(cell).copied().unwrap_or(OCEAN);
    let join_watershed = hydrology.watershed_id.get(join).copied().unwrap_or(OCEAN);
    if watershed == OCEAN || watershed != join_watershed {
        return false;
    }
    let basin = hydrology.basin_by_cell.get(cell).copied().unwrap_or(OCEAN);
    let join_basin = hydrology.basin_by_cell.get(join).copied().unwrap_or(OCEAN);
    if join_basin != OCEAN && basin != join_basin {
        return false;
    }
    true
}

fn land_ok(
    hydrology: &HydrologyField,
    elevations_mm: &[i32],
    cell: usize,
    allow_river: bool,
) -> bool {
    elevations_mm.get(cell).copied().unwrap_or(i32::MIN) > hydrology.sea_level_mm
        && hydrology.ice_cells.get(cell).copied() != Some(true)
        && (allow_river || hydrology.lake_cells.get(cell).copied() != Some(true))
}

fn nearest_river_cell(
    hydrology: &HydrologyField,
    river_cells: &BTreeSet<usize>,
    cell: usize,
) -> Option<usize> {
    river_cells
        .iter()
        .copied()
        .filter(|river| same_partition(hydrology, *river, cell))
        .min_by_key(|river| (cell_distance(hydrology.grid, cell, *river), *river))
}

fn trace_to_nearest(
    hydrology: &HydrologyField,
    elevations_mm: &[i32],
    river_cells: &BTreeSet<usize>,
    start: usize,
) -> Option<(usize, Vec<usize>)> {
    let grid = hydrology.grid;
    let target = nearest_river_cell(hydrology, river_cells, start)?;
    let mut path = vec![start];
    let mut seen = BTreeSet::from([start]);
    let mut current = start;
    for _ in 0..MAX_TRACE_STEPS {
        if river_cells.contains(&current) && current != start {
            return Some((current, path));
        }
        let mut best: Option<(i32, u32, usize)> = None;
        for neighbor in grid.neighbors(current) {
            if seen.contains(&neighbor) || !same_partition(hydrology, neighbor, start) {
                continue;
            }
            let on_river = river_cells.contains(&neighbor);
            if !land_ok(hydrology, elevations_mm, neighbor, on_river) {
                continue;
            }
            let elevation = elevations_mm[neighbor];
            if elevation > elevations_mm[current] {
                continue;
            }
            let distance = cell_distance(grid, neighbor, target);
            if distance > cell_distance(grid, current, target) {
                continue;
            }
            let score = (elevation, distance, neighbor);
            if best
                .map(|current_best| score < current_best)
                .unwrap_or(true)
            {
                best = Some(score);
            }
        }
        let Some((_, _, next)) = best else {
            return None;
        };
        seen.insert(next);
        path.push(next);
        current = next;
        if river_cells.contains(&next) {
            return Some((next, path));
        }
    }
    None
}

pub fn derive_minor_tributaries(
    hydrology: &HydrologyField,
    elevations_mm: &[i32],
    identity: &[u8],
    variant: u32,
    level: DetailLevel,
    check_cancelled: &mut dyn FnMut() -> Result<(), AtlasError>,
) -> Result<DerivedDrainage, AtlasError> {
    if elevations_mm.len() != hydrology.grid.sample_count() {
        return Err(AtlasError::invalid(
            "elevation sample count does not match hydrology",
        ));
    }
    let key = domain_key(
        identity,
        ATLAS_DERIVED_DRAINAGE_VERSION,
        variant,
        MINOR_TRIBUTARY_DOMAIN,
    );
    let (river_cell_set, per_river) = river_cells(hydrology);
    if river_cell_set.is_empty() {
        return Ok(DerivedDrainage {
            version: ATLAS_DERIVED_DRAINAGE_VERSION,
            tributaries: Vec::new(),
        });
    }
    let grid = hydrology.grid;
    let threshold = keep_threshold_ppm(level);
    let mut tributaries = Vec::new();
    for cell in 0..grid.sample_count() {
        if cell % 2_048 == 0 {
            check_cancelled()?;
        }
        if river_cell_set.contains(&cell)
            || !land_ok(hydrology, elevations_mm, cell, false)
            || hydrology.watershed_id.get(cell).copied().unwrap_or(OCEAN) == OCEAN
        {
            continue;
        }
        let adjacent_river = grid
            .neighbors(cell)
            .iter()
            .any(|neighbor| river_cell_set.contains(neighbor));
        if !adjacent_river {
            continue;
        }
        let (row, col) = grid.row_col(cell);
        let sample = lattice_sample(&key, col, row, 0);
        if (sample % 10_000) as u32 >= threshold {
            continue;
        }
        let Some((join, cells)) = trace_to_nearest(hydrology, elevations_mm, &river_cell_set, cell)
        else {
            continue;
        };
        if cells.len() < 2 {
            continue;
        }
        if cells
            .iter()
            .any(|step| !same_partition(hydrology, *step, join))
        {
            continue;
        }
        let Some(parent_river_id) = per_river.iter().enumerate().find_map(|(index, cells)| {
            cells.contains(&join).then(|| {
                hydrology
                    .rivers
                    .get(index)
                    .map(|river| river.id)
                    .unwrap_or(index as u32)
            })
        }) else {
            continue;
        };
        tributaries.push(DerivedTributary {
            id: tributary_id(cell),
            source_cell: cell,
            join_cell: join,
            parent_river_id,
            watershed_id: hydrology.watershed_id[cell],
            path: cells
                .into_iter()
                .map(|step| coordinate_for_cell(grid, step))
                .collect(),
        });
        if tributaries.len() >= MAX_DERIVED_TRIBUTARIES {
            break;
        }
    }
    tributaries.sort_by(|left, right| left.source_cell.cmp(&right.source_cell));
    Ok(DerivedDrainage {
        version: ATLAS_DERIVED_DRAINAGE_VERSION,
        tributaries,
    })
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, AtlasError> {
    let slice = bytes
        .get(*offset..*offset + 4)
        .ok_or_else(|| AtlasError::invalid("derived drainage cache is truncated"))?;
    *offset += 4;
    Ok(u32::from_le_bytes(slice.try_into().expect("u32")))
}

fn read_i32(bytes: &[u8], offset: &mut usize) -> Result<i32, AtlasError> {
    Ok(read_u32(bytes, offset)? as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden_world;
    use crate::request::DetailLevel;
    use crate::spike_identity_from_source;
    use daena_physical::history::{derive_historical_world, HistoricalForcingParameters};
    use daena_physical::NoopProgress as PhysicalNoop;

    fn epoch(offset: i64) -> (HydrologyField, Vec<i32>, Vec<u8>) {
        let world = golden_world();
        let field = world.field.clone();
        let historical = derive_historical_world(
            &field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            HistoricalForcingParameters::default_for(field.seed, field.retry_index),
            offset,
            &mut PhysicalNoop,
        )
        .unwrap();
        (
            historical.hydrology,
            field.elevations_mm.clone(),
            spike_identity_from_source(&world.source),
        )
    }

    #[test]
    fn tributaries_stay_inside_parent_watershed_and_round_trip() {
        let (hydrology, elevations, identity) = epoch(0);
        let mut cancel = || Ok(());
        let drainage = derive_minor_tributaries(
            &hydrology,
            &elevations,
            &identity,
            0,
            DetailLevel::Detailed,
            &mut cancel,
        )
        .unwrap();
        assert!(!drainage.tributaries.is_empty());
        for tributary in &drainage.tributaries {
            assert!(tributary.id.starts_with("atlas:tributary:v1:"));
            assert!(tributary.path.len() >= 2);
            let join_watershed = hydrology.watershed_id[tributary.join_cell];
            assert_eq!(tributary.watershed_id, join_watershed);
            assert_ne!(tributary.watershed_id, OCEAN);
            for point in &tributary.path {
                let cell = nearest_cell(hydrology.grid, point[0], point[1]);
                assert_eq!(hydrology.watershed_id[cell], tributary.watershed_id);
                let join_basin = hydrology.basin_by_cell[tributary.join_cell];
                if join_basin != OCEAN {
                    assert_eq!(hydrology.basin_by_cell[cell], join_basin);
                }
            }
        }
        let decoded = DerivedDrainage::decode(&drainage.encode()).unwrap();
        assert_eq!(decoded, drainage);
    }

    #[test]
    fn tributary_ids_are_stable_across_resolution_style_inputs() {
        let (hydrology, elevations, identity) = epoch(0);
        let mut cancel = || Ok(());
        let first = derive_minor_tributaries(
            &hydrology,
            &elevations,
            &identity,
            0,
            DetailLevel::Detailed,
            &mut cancel,
        )
        .unwrap();
        let second = derive_minor_tributaries(
            &hydrology,
            &elevations,
            &identity,
            0,
            DetailLevel::Detailed,
            &mut cancel,
        )
        .unwrap();
        assert_eq!(first, second);
        let (past, past_elevations, _) = epoch(-8000);
        let past_drainage = derive_minor_tributaries(
            &past,
            &past_elevations,
            &identity,
            0,
            DetailLevel::Detailed,
            &mut cancel,
        )
        .unwrap();
        let present_ids = first
            .tributaries
            .iter()
            .map(|tributary| (tributary.id.clone(), tributary.source_cell))
            .collect::<BTreeSet<_>>();
        for tributary in &past_drainage.tributaries {
            if let Some((_, source)) = present_ids.iter().find(|(id, _)| id == &tributary.id) {
                assert_eq!(*source, tributary.source_cell);
            }
        }
    }
}
