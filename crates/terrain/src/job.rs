//! Byte boundary for one terrain tile job. Values cross it without change.
use super::{
    TerrainContext, TerrainEvaluator, TerrainFoundation, TerrainMesh, TerrainRoadPoint,
    TerrainSettings, TerrainTile, surface,
};

/// One tile job. Tile jobs use every road point without a radius filter.
#[derive(Clone, Copy)]
pub struct TerrainJob<'a> {
    pub settings: TerrainSettings,
    pub unit: [i64; 2],
    pub tile: TerrainTile,
    pub biome_override: Option<u8>,
    pub roads: &'a [TerrainRoadPoint],
    pub foundations: &'a [TerrainFoundation],
    pub holes: &'a [[[f32; 3]; 4]],
}

impl TerrainJob<'_> {
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.clear();
        out.extend(self.settings.seed.to_le_bytes());
        f32s(out, &[self.settings.edge, self.settings.edge_gradient]);
        out.extend(self.unit[0].to_le_bytes());
        out.extend(self.unit[1].to_le_bytes());
        for value in [
            self.tile.tier as u32,
            self.tile.size,
            self.tile.resolution,
            self.tile.draw_distance,
            self.biome_override.map_or(u32::MAX, u32::from),
        ] {
            out.extend(value.to_le_bytes());
        }
        f32s(out, &[self.tile.y, self.tile.material_scale]);
        out.extend((self.roads.len() as u32).to_le_bytes());
        for road in self.roads {
            f32s(out, &road.position);
            f32s(out, &road.segment_origin);
            f32s(out, &[road.range]);
        }
        out.extend((self.foundations.len() as u32).to_le_bytes());
        for foundation in self.foundations {
            f32s(out, &foundation.position);
            f32s(out, &foundation.range_squared);
        }
        out.extend((self.holes.len() as u32).to_le_bytes());
        for hole in self.holes {
            f32s(out, hole.as_flattened());
        }
    }
}

fn f32s(out: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        out.extend(value.to_le_bytes());
    }
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let (head, tail) = self
            .0
            .split_first_chunk::<N>()
            .ok_or("Terrain job input is truncated")?;
        self.0 = tail;
        Ok(*head)
    }
    fn u32(&mut self) -> Result<u32, String> {
        self.bytes().map(u32::from_le_bytes)
    }
    fn f32(&mut self) -> Result<f32, String> {
        self.bytes().map(f32::from_le_bytes)
    }
    fn f32s<const N: usize>(&mut self) -> Result<[f32; N], String> {
        let mut values = [0.0; N];
        for value in &mut values {
            *value = self.f32()?;
        }
        Ok(values)
    }
    fn count(&mut self, item: usize) -> Result<usize, String> {
        let count = self.u32()? as usize;
        if count.saturating_mul(item) > self.0.len() {
            return Err("Terrain job count exceeds its input".into());
        }
        Ok(count)
    }
}

/// Decode one job and generate its tile.
pub fn evaluate(input: &[u8]) -> Result<TerrainMesh, String> {
    let mut reader = Reader(input);
    let settings = TerrainSettings {
        seed: reader.bytes().map(i32::from_le_bytes)?,
        edge: reader.f32()?,
        edge_gradient: reader.f32()?,
    };
    let unit = [
        reader.bytes().map(i64::from_le_bytes)?,
        reader.bytes().map(i64::from_le_bytes)?,
    ];
    let tier = reader.u32()?;
    let size = reader.u32()?;
    let resolution = reader.u32()?;
    let draw_distance = reader.u32()?;
    let biome = reader.u32()?;
    if tier > 2 || size == 0 || !(4..=1024).contains(&resolution) {
        return Err(format!(
            "Invalid terrain job tile {tier} {size} {resolution}"
        ));
    }
    let tile = TerrainTile {
        tier: tier as usize,
        size,
        resolution,
        draw_distance,
        y: reader.f32()?,
        material_scale: reader.f32()?,
    };
    let biome_override = match biome {
        u32::MAX => None,
        0..=2 => Some(biome as u8),
        _ => return Err(format!("Invalid terrain job biome {biome}")),
    };
    let count = reader.count(24)?;
    let mut roads = Vec::with_capacity(count);
    for _ in 0..count {
        roads.push(TerrainRoadPoint {
            position: reader.f32s()?,
            segment_origin: reader.f32s()?,
            range: reader.f32()?,
        });
    }
    let count = reader.count(20)?;
    let mut foundations = Vec::with_capacity(count);
    for _ in 0..count {
        foundations.push(TerrainFoundation {
            position: reader.f32s()?,
            range_squared: reader.f32s()?,
        });
    }
    let count = reader.count(48)?;
    let mut holes = Vec::with_capacity(count);
    for _ in 0..count {
        let corners: [f32; 12] = reader.f32s()?;
        holes.push(std::array::from_fn(|i| {
            [corners[i * 3], corners[i * 3 + 1], corners[i * 3 + 2]]
        }));
    }
    if !reader.0.is_empty() {
        return Err("Terrain job input has trailing bytes".into());
    }
    let evaluator = TerrainEvaluator::new(settings);
    Ok(surface(
        unit,
        TerrainContext {
            evaluator: &evaluator,
            tile,
            roads: &roads,
            road_radius_squared: None,
            foundations: &foundations,
            holes: &holes,
            biome_override,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worker boundary must return the same tile as a direct call.
    #[test]
    fn encoded_job_matches_direct_surface() {
        let settings = TerrainSettings {
            seed: -1705217151,
            edge: 100000.0,
            edge_gradient: 5000.0,
        };
        let tile = TerrainTile {
            tier: 0,
            size: 100,
            resolution: 21,
            draw_distance: 3,
            y: 0.0,
            material_scale: 1.0,
        };
        let roads = [
            TerrainRoadPoint {
                position: [-60.0, 12.0, 3.0],
                segment_origin: [-60.0, 0.0],
                range: 30.0,
            },
            TerrainRoadPoint {
                position: [60.0, 14.0, -3.0],
                segment_origin: [60.0, 0.0],
                range: 30.0,
            },
        ];
        let foundations = [TerrainFoundation {
            position: [20.0, 9.0, 20.0],
            range_squared: [64.0, 400.0],
        }];
        let holes = [[
            [10.0, 5.0, -10.0],
            [14.0, 5.0, -10.0],
            [10.0, 5.0, -14.0],
            [14.0, 5.0, -14.0],
        ]];
        let job = TerrainJob {
            settings,
            unit: [0, 0],
            tile,
            biome_override: None,
            roads: &roads,
            foundations: &foundations,
            holes: &holes,
        };
        let mut bytes = Vec::new();
        job.encode(&mut bytes);
        let evaluator = TerrainEvaluator::new(settings);
        let direct = surface(
            job.unit,
            TerrainContext {
                evaluator: &evaluator,
                tile,
                roads: &roads,
                road_radius_squared: None,
                foundations: &foundations,
                holes: &holes,
                biome_override: None,
            },
        );
        let encoded = evaluate(&bytes).unwrap();
        assert_eq!(encoded.vertices.len(), 21 * 21);
        assert!(encoded.triangles.len() < 20 * 20 * 2);
        let bits = |mesh: &TerrainMesh| {
            [
                mesh.vertices.as_flattened(),
                mesh.normals.as_flattened(),
                mesh.tangents.as_flattened(),
                mesh.uv.as_flattened(),
                mesh.colors.as_flattened(),
            ]
            .map(|values| values.iter().map(|v| v.to_bits()).collect::<Vec<_>>())
        };
        assert_eq!(bits(&encoded), bits(&direct));
        assert_eq!(encoded.triangles, direct.triangles);
        assert_eq!(encoded.origin, direct.origin);
        assert!(evaluate(&bytes[..bytes.len() - 1]).is_err());
    }
}
