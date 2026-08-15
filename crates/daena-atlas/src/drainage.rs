//! Atlas-only derived tributaries from refined drainage version 2.

use crate::{AtlasError, ATLAS_DERIVED_DRAINAGE_VERSION};

pub const MAX_DERIVED_TRIBUTARIES: usize = 512;
const MAX_TRACE_STEPS: usize = 64;

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

    pub fn encode_product(&self, width: u32, height: u32, worked_mm: &[i32]) -> Vec<u8> {
        let inner = self.encode();
        let residual = crate::cache::encode_residual(width, height, worked_mm);
        let mut bytes = Vec::with_capacity(8 + inner.len() + residual.len());
        bytes.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(residual.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&inner);
        bytes.extend_from_slice(&residual);
        bytes
    }

    pub fn decode_product(bytes: &[u8]) -> Result<(Self, u32, u32, Vec<i32>), AtlasError> {
        if bytes.len() < 8 {
            return Err(AtlasError::invalid("refined drainage cache is truncated"));
        }
        let inner_len = u32::from_le_bytes(bytes[0..4].try_into().expect("u32")) as usize;
        let residual_len = u32::from_le_bytes(bytes[4..8].try_into().expect("u32")) as usize;
        let inner_end = 8usize.saturating_add(inner_len);
        let residual_end = inner_end.saturating_add(residual_len);
        let inner = bytes
            .get(8..inner_end)
            .ok_or_else(|| AtlasError::invalid("refined drainage cache is truncated"))?;
        let residual = bytes
            .get(inner_end..residual_end)
            .ok_or_else(|| AtlasError::invalid("refined drainage cache is truncated"))?;
        if bytes.len() != residual_end {
            return Err(AtlasError::invalid(
                "refined drainage cache has trailing bytes",
            ));
        }
        let drainage = Self::decode(inner)?;
        let (width, height, worked_mm) = crate::cache::decode_residual(residual)?;
        Ok((drainage, width, height, worked_mm))
    }
}

pub fn tributary_id(source_cell: usize) -> String {
    format!("atlas:tributary:v{ATLAS_DERIVED_DRAINAGE_VERSION}:{source_cell}")
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
    use crate::detail::nearest_cell;
    use crate::golden_world;
    use crate::prepare_from_source;
    use crate::request::AtlasRenderRequest;
    use crate::spike_identity_from_source;
    use crate::NoopProgress;

    #[test]
    fn tributaries_use_version_two_ids_and_round_trip() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let scene = prepare_from_source(
            &world.source,
            &identity,
            &AtlasRenderRequest::spike_png(64, 32).unwrap(),
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        for tributary in &scene.drainage.tributaries {
            assert!(tributary.id.starts_with("atlas:tributary:v5:"));
            assert!(tributary.path.len() >= 2);
            for point in &tributary.path {
                let cell = nearest_cell(scene.hydrology.grid, point[0], point[1]);
                assert_eq!(scene.hydrology.watershed_id[cell], tributary.watershed_id);
            }
        }
        let decoded = DerivedDrainage::decode(&scene.drainage.encode()).unwrap();
        assert_eq!(decoded, scene.drainage);
        let product = scene.drainage.encode_product(
            scene.model.lattice_width,
            scene.model.lattice_height,
            &scene.model.residual_mm,
        );
        let (again, width, height, _) = DerivedDrainage::decode_product(&product).unwrap();
        assert_eq!(again, scene.drainage);
        assert_eq!(width, scene.model.lattice_width);
        assert_eq!(height, scene.model.lattice_height);
    }
}
