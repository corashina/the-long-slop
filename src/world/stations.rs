//! Source fuel station placement on the current procedural road.
use super::{CHUNK_LENGTH, ChunkData, STATION_SPACING, hash, height, inside, road_sample};
use crate::reference::{Bounds, StationsReference};
use rapier3d::math::{Rotation, Vector};

/// TerrainGenerator keeps road height within 4.5 meters of the road line.
const ROAD_HALF_WIDTH: f64 = 4.5;
/// Pump distance from the road line. Every station model stays at least 7.6 meters away.
const PUMP_LATERAL: f32 = 6.5;

/// One station per spacing interval. Its root sits mid-chunk, so its ground stays in that chunk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub id: i64,
    /// Index in `StationsReference::types`.
    pub kind: usize,
    pub absolute_point: [f64; 3],
    pub rotation: [f32; 4],
    /// Ground point of the refuel pump on the road shoulder.
    pub pump_point: [f64; 3],
}

impl Station {
    pub fn distance(id: i64) -> f64 {
        id as f64 * STATION_SPACING + CHUNK_LENGTH * 0.5
    }

    /// Root-local point in absolute coordinates.
    pub fn absolute(&self, local: [f32; 3]) -> [f64; 3] {
        let offset = Rotation::from_array(self.rotation) * Vector::from_array(local);
        std::array::from_fn(|axis| self.absolute_point[axis] + offset[axis] as f64)
    }
}

fn unit(bits: u64) -> f32 {
    (bits & 0xffff) as f32 / 65535.
}

fn lerp([a, b]: [f32; 2], t: f32) -> f32 {
    a + (b - a) * t
}

/// The source draws type, side, offset, and angle from roadBuildingScript. This road keeps its own spacing.
pub fn station(seed: u64, id: i64, reference: &StationsReference) -> Station {
    let bits = hash(seed ^ (id as u64).wrapping_mul(0x2545_f491_4f6c_dd1d));
    let kind = (bits % reference.types.len() as u64) as usize;
    let sides = &reference.types[kind].sides;
    let side = &sides[((bits >> 8) % sides.len() as u64) as usize];
    let lateral = lerp(side.lateral_meters, unit(bits >> 16));
    let angle = lerp(side.yaw_degrees, unit(bits >> 32)).to_radians();
    let road = road_sample(seed, Station::distance(id));
    let yaw = (-road.tangent[0]).atan2(-road.tangent[2]);
    // Converted source handedness keeps X and turns a source Y angle into a negative turn.
    let offset = Rotation::from_rotation_y(yaw) * Vector::new(lateral, 0., 0.);
    let x = road.center[0] + offset.x as f64;
    let z = road.center[2] + offset.z as f64;
    let distance = -z;
    let pump = Rotation::from_rotation_y(yaw) * Vector::new(PUMP_LATERAL.copysign(lateral), 0., 0.);
    let pump_x = road.center[0] + pump.x as f64;
    let pump_z = road.center[2] + pump.z as f64;
    let mut station = Station {
        id,
        kind,
        absolute_point: [
            x,
            height(seed, distance, x - road_sample(seed, distance).center[0]),
            z,
        ],
        rotation: Rotation::from_rotation_y(yaw - angle).to_array(),
        pump_point: [pump_x, 0., pump_z],
    };
    let ground = height(seed, -pump_z, pump_x - road_sample(seed, -pump_z).center[0]);
    station.pump_point[1] = leveled(&station, reference, pump_x, pump_z, ground);
    station
}

pub fn nearest_station(seed: u64, distance_m: f64, reference: &StationsReference) -> Station {
    let id = ((distance_m - CHUNK_LENGTH * 0.5) / STATION_SPACING).round() as i64;
    station(seed, id, reference)
}

/// Adds the station that owns this chunk and levels ground under every station that reaches it.
/// Ground follows terrainHeightAlignToBuildingScript markers through TerrainGenerator.AlignToBuilding.
/// The road surface keeps its height, as in the source road rule. No station touches the home.
pub(super) fn place(
    data: &mut ChunkData,
    seed: u64,
    reference: &StationsReference,
    home: Option<&Bounds>,
) {
    let start = data.key.0 as f64 * CHUNK_LENGTH;
    let owner = (start / STATION_SPACING).floor() as i64;
    for id in owner..=owner + 1 {
        let station = station(seed, id, reference);
        if home.is_some_and(|home| {
            reference.types[station.kind].ground.iter().any(|ground| {
                let center = station.absolute(ground.center).map(|x| x as f32);
                inside(center, home, ground.blend_radius)
            })
        }) {
            continue;
        }
        level_ground(data, &station, reference);
        let lateral =
            station.absolute_point[0] - road_sample(seed, Station::distance(id)).center[0];
        if (start..start + CHUNK_LENGTH).contains(&Station::distance(id))
            && (lateral - data.lateral_band as f64 * super::LATERAL_BAND_WIDTH).abs()
                < super::TERRAIN_HALF_WIDTH - 8.0
        {
            data.stations.push(station);
        }
    }
}

fn level_ground(data: &mut ChunkData, station: &Station, reference: &StationsReference) {
    let origin = data.origin();
    let columns = data.lateral_samples.len();
    for (index, vertex) in data.vertices.iter_mut().enumerate() {
        if data.lateral_samples[index % columns].abs() > ROAD_HALF_WIDTH {
            let x = vertex[0] as f64 + origin[0];
            let z = vertex[2] as f64 + origin[2];
            vertex[1] = leveled(station, reference, x, z, vertex[1] as f64) as f32;
        }
    }
}

/// Terrain height after the station ground markers.
fn leveled(station: &Station, reference: &StationsReference, x: f64, z: f64, mut y: f64) -> f64 {
    for ground in &reference.types[station.kind].ground {
        let center = station.absolute(ground.center);
        let inner = (ground.radius as f64).powi(2);
        let outer = (ground.blend_radius as f64).powi(2);
        let distance = (x - center[0]).powi(2) + (z - center[2]).powi(2);
        if distance < inner {
            y = center[1];
        } else if distance < outer {
            y = center[1] + (y - center[1]) * (distance - inner) / (outer - inner);
        }
    }
    y
}
