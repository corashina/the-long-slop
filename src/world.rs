use crate::vehicle::{ColliderHandle, PhysicsWorld, VehiclePose};

pub const CHUNK_LENGTH: f64 = 128.0;
pub const STATION_SPACING: f64 = 640.0;
pub const REBASE_LIMIT: f32 = 2048.0;
pub const DEFAULT_SEED: u64 = 42;
pub const LATERAL_BAND_WIDTH: f64 = 128.0;
pub const TERRAIN_HALF_WIDTH: f64 = 160.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct WorldOrigin {
    pub offset: [f64; 3],
}

impl WorldOrigin {
    pub fn absolute(&self, local: [f32; 3]) -> [f64; 3] {
        std::array::from_fn(|i| self.offset[i] + local[i] as f64)
    }

    pub fn local(&self, absolute: [f64; 3]) -> [f32; 3] {
        std::array::from_fn(|i| (absolute[i] - self.offset[i]) as f32)
    }

    pub fn shift(&mut self, shift: [f32; 3]) {
        for (offset, shift) in self.offset.iter_mut().zip(shift) {
            *offset += shift as f64;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkKey(pub i64);

pub struct RoadSample {
    pub center: [f64; 3],
    pub tangent: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub id: i64,
    pub absolute_point: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind {
    Rock,
    Cactus,
    Post,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub kind: PropKind,
    pub absolute_point: [f64; 3],
    pub half_size: [f32; 3],
}

/// Vertex origin is [lateral_band * 128, 0, -key * 128]. Distance increases along absolute -Z.
#[derive(Debug, PartialEq)]
pub struct ChunkData {
    pub key: ChunkKey,
    pub lateral_band: i64,
    pub lateral_samples: Vec<f64>,
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
    pub stations: Vec<Station>,
    pub props: Vec<Prop>,
}

impl ChunkData {
    pub fn origin(&self) -> [f64; 3] {
        [
            self.lateral_band as f64 * LATERAL_BAND_WIDTH,
            0.0,
            -(self.key.0 as f64 * CHUNK_LENGTH),
        ]
    }
}

fn phase(seed: u64) -> f64 {
    (hash(seed) % 10_000) as f64 * std::f64::consts::TAU / 10_000.0
}

fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub fn road_sample(seed: u64, distance_m: f64) -> RoadSample {
    let p = phase(seed);
    let a = distance_m / 700.0 + p;
    let b = distance_m / 190.0 + p;
    let x = 16.0 * (a.sin() - p.sin()) + 5.0 * (b.sin() - p.sin());
    let y = 0.5 * ((distance_m / 300.0 + p).sin() - p.sin());
    let dx = 16.0 / 700.0 * a.cos() + 5.0 / 190.0 * b.cos();
    let dy = 0.5 / 300.0 * (distance_m / 300.0 + p).cos();
    let length = (dx * dx + dy * dy + 1.0).sqrt();
    RoadSample {
        center: [x, y, -distance_m],
        tangent: [
            (dx / length) as f32,
            (dy / length) as f32,
            (-1.0 / length) as f32,
        ],
    }
}

fn height(seed: u64, distance: f64, lateral: f64) -> f64 {
    let road = road_sample(seed, distance);
    let fade = ((lateral.abs() - 20.0) / 35.0).clamp(0.0, 1.0);
    let p = phase(seed);
    road.center[1]
        + fade
            * (3.5
                + 2.2 * (lateral / 32.0 + distance / 83.0 + p).sin()
                + 1.2 * (distance / 41.0 - lateral / 24.0).sin())
}

pub fn generate_chunk(seed: u64, key: ChunkKey) -> ChunkData {
    generate_chunk_in_band(seed, key, 0)
}

pub fn generate_chunk_in_band(seed: u64, key: ChunkKey, lateral_band: i64) -> ChunkData {
    let start = key.0 as f64 * CHUNK_LENGTH;
    let band_center = lateral_band as f64 * LATERAL_BAND_WIDTH;
    let mut lateral_samples: Vec<f64> = (-20..=20).map(|i| band_center + i as f64 * 8.0).collect();
    for edge in [-4.0, 4.0] {
        if (band_center - TERRAIN_HALF_WIDTH..=band_center + TERRAIN_HALF_WIDTH).contains(&edge) {
            lateral_samples.push(edge);
        }
    }
    lateral_samples.sort_by(f64::total_cmp);
    let columns = lateral_samples.len();
    let mut vertices = Vec::with_capacity(17 * columns);
    let mut indices = Vec::with_capacity(16 * (columns - 1) * 2);
    for row in 0..=16 {
        let distance = start + row as f64 * 8.0;
        let road = road_sample(seed, distance);
        for &lateral in &lateral_samples {
            vertices.push([
                (road.center[0] + lateral - band_center) as f32,
                height(seed, distance, lateral) as f32,
                -(row as f32 * 8.0),
            ]);
        }
    }
    for row in 0..16 {
        for col in 0..columns - 1 {
            let a = (row * columns + col) as u32;
            let b = a + 1;
            let c = a + columns as u32;
            let d = c + 1;
            indices.extend([[a, b, c], [b, d, c]]);
        }
    }
    let stations =
        if key.0.rem_euclid(5) == 0 && (10.0 - band_center).abs() < TERRAIN_HALF_WIDTH - 8.0 {
            vec![station(seed, key.0 / 5)]
        } else {
            Vec::new()
        };
    let mut props = Vec::with_capacity(5);
    for i in 0..2 {
        let bits = hash(seed ^ (key.0 as u64).wrapping_mul(17) ^ i);
        let distance = start + if i == 0 { 25.0 } else { 45.0 };
        let lateral = if key.0 == 0 {
            5.5
        } else {
            7.0 + (bits % 24) as f64
        } * if i == 0 { 1.0 } else { -1.0 };
        let road = road_sample(seed, distance);
        props.push(Prop {
            kind: PropKind::Rock,
            absolute_point: [
                road.center[0] + lateral,
                height(seed, distance, lateral) + 0.6,
                -distance,
            ],
            half_size: [1.0, 0.6, 1.0],
        });
    }
    for lateral in [-4.8, 4.8] {
        let distance = start + 80.0;
        let road = road_sample(seed, distance);
        props.push(Prop {
            kind: PropKind::Post,
            absolute_point: [road.center[0] + lateral, road.center[1] + 0.6, -distance],
            half_size: [0.09, 0.6, 0.09],
        });
    }
    let distance = start + 64.0;
    let lateral = if hash(seed ^ key.0 as u64) & 1 == 0 {
        46.0
    } else {
        -46.0
    };
    let road = road_sample(seed, distance);
    props.push(Prop {
        kind: PropKind::Cactus,
        absolute_point: [
            road.center[0] + lateral,
            height(seed, distance, lateral) + 1.2,
            -distance,
        ],
        half_size: [0.2, 1.2, 0.2],
    });
    props.retain(|prop| {
        let road = road_sample(seed, -prop.absolute_point[2]);
        (prop.absolute_point[0] - road.center[0] - band_center).abs() < TERRAIN_HALF_WIDTH - 8.0
    });
    ChunkData {
        key,
        lateral_band,
        lateral_samples,
        vertices,
        indices,
        stations,
        props,
    }
}

pub fn active_chunks(distance_m: f64) -> [ChunkKey; 11] {
    let current = (distance_m / CHUNK_LENGTH).floor() as i64;
    std::array::from_fn(|i| ChunkKey(current + i as i64 - 3))
}

fn station(seed: u64, id: i64) -> Station {
    let mut point = road_sample(seed, id as f64 * STATION_SPACING).center;
    point[0] += 10.0;
    Station {
        id,
        absolute_point: point,
    }
}

pub fn nearest_station(seed: u64, distance_m: f64) -> Station {
    station(seed, (distance_m / STATION_SPACING).round() as i64)
}

pub struct LoadedChunk {
    pub data: ChunkData,
    colliders: Vec<ColliderHandle>,
}

pub struct WorldStream {
    pub seed: u64,
    pub origin: WorldOrigin,
    chunks: Vec<LoadedChunk>,
    lateral_band: i64,
}

impl WorldStream {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            origin: WorldOrigin::default(),
            chunks: Vec::with_capacity(11),
            lateral_band: 0,
        }
    }

    pub fn chunks(&self) -> impl Iterator<Item = &LoadedChunk> {
        self.chunks.iter()
    }

    pub fn lateral_band(&self) -> i64 {
        self.lateral_band
    }

    pub fn sync_at(&mut self, absolute: [f64; 3], physics: &mut PhysicsWorld) {
        let distance = -absolute[2];
        let lateral = absolute[0] - road_sample(self.seed, distance).center[0];
        self.lateral_band = (lateral / LATERAL_BAND_WIDTH).round() as i64;
        let keys = active_chunks(distance);
        self.chunks.retain(|chunk| {
            if keys.contains(&chunk.data.key) && chunk.data.lateral_band == self.lateral_band {
                return true;
            }
            for handle in &chunk.colliders {
                physics.remove_collider(*handle);
            }
            false
        });
        for key in keys {
            if self.chunks.iter().any(|c| c.data.key == key) {
                continue;
            }
            let data = generate_chunk_in_band(self.seed, key, self.lateral_band);
            let mut colliders = Vec::with_capacity(7);
            colliders.push(physics.insert_mesh_at(
                &data.vertices,
                &data.indices,
                self.origin.local(data.origin()),
            ));
            for prop in &data.props {
                if prop.kind != PropKind::Cactus {
                    colliders.push(
                        physics.insert_box(self.origin.local(prop.absolute_point), prop.half_size),
                    );
                }
            }
            for station in &data.stations {
                let mut center = station.absolute_point;
                center[0] += 6.0;
                center[1] += 1.5;
                colliders.push(physics.insert_box(self.origin.local(center), [2.0, 1.5, 3.0]));
                let mut pump = station.absolute_point;
                pump[1] += 0.7;
                pump[2] -= 3.0;
                colliders.push(physics.insert_box(self.origin.local(pump), [0.35, 0.7, 0.35]));
            }
            self.chunks.push(LoadedChunk { data, colliders });
        }
    }

    /// Call between fixed steps. Stored metadata remains absolute.
    pub fn rebase(&mut self, physics: &mut PhysicsWorld) -> Option<[f32; 3]> {
        let position = physics.snapshot().pose.position;
        if position[0].abs().max(position[2].abs()) <= REBASE_LIMIT {
            return None;
        }
        let shift = [
            (position[0] / 128.0).round() * 128.0,
            0.0,
            (position[2] / 128.0).round() * 128.0,
        ];
        physics.rebase(shift);
        self.origin.shift(shift);
        Some(shift)
    }

    /// Load the destination before the caller resets the vehicle.
    pub fn reset_pose(
        &mut self,
        distance: f64,
        at_station: bool,
        physics: &mut PhysicsWorld,
    ) -> VehiclePose {
        let sample_distance = if at_station {
            nearest_station(self.seed, distance).id as f64 * STATION_SPACING
        } else {
            distance
        };
        let road = road_sample(self.seed, sample_distance);
        let mut point = if at_station {
            nearest_station(self.seed, distance).absolute_point
        } else {
            road.center
        };
        self.sync_at(point, physics);
        point[1] += 1.1;
        let yaw = (-road.tangent[0]).atan2(-road.tangent[2]);
        VehiclePose {
            position: self.origin.local(point),
            rotation: [0.0, (yaw * 0.5).sin(), 0.0, (yaw * 0.5).cos()],
        }
    }
}
