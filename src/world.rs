use crate::reference::{Bounds, ColliderGeometry, ReferenceManifest, StartingAreaReference};
use crate::vehicle::{ColliderHandle, PhysicsWorld, VehiclePose};

mod lamps;
mod terrain;
pub use lamps::{RoadLampId, RoadLampPlacement, road_lamp_placement, road_lamp_range};

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

pub(crate) fn hash(mut value: u64) -> u64 {
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

fn inside(point: [f32; 3], bounds: &Bounds, margin: f32) -> bool {
    [0, 2].iter().all(|&axis| {
        point[axis] >= bounds.min[axis] - margin && point[axis] <= bounds.max[axis] + margin
    })
}

fn prepare_terrain(
    data: &mut ChunkData,
    seed: u64,
    area: &StartingAreaReference,
    interiors: &[Bounds],
) {
    let origin = data.origin();
    let start = -origin[2];
    let home = &area.home_bounds;
    if start > -home.min[2] as f64 + 8.0 || start + CHUNK_LENGTH < -home.max[2] as f64 - 8.0 {
        return;
    }
    let road_x = road_sample(seed, -(home.min[2] + home.max[2]) as f64 * 0.5).center[0];
    let mut rows: Vec<f64> = (0..=16).map(|row| start + row as f64 * 8.0).collect();
    for distance in ((-home.max[2] - 4.0).floor() as i32)..=((-home.min[2] + 4.0).ceil() as i32) {
        if (start..=start + CHUNK_LENGTH).contains(&(distance as f64)) {
            rows.push(distance as f64);
        }
    }
    rows.sort_by(f64::total_cmp);
    rows.dedup();
    for x in ((home.min[0] - road_x as f32 - 4.0).floor() as i32)
        ..=((home.max[0] - road_x as f32 + 4.0).ceil() as i32)
    {
        if (origin[0] - TERRAIN_HALF_WIDTH..=origin[0] + TERRAIN_HALF_WIDTH).contains(&(x as f64)) {
            data.lateral_samples.push(x as f64);
        }
    }
    data.lateral_samples.sort_by(f64::total_cmp);
    data.lateral_samples.dedup();
    let columns = data.lateral_samples.len();
    data.vertices.clear();
    data.indices.clear();
    for &distance in &rows {
        let road = road_sample(seed, distance);
        for &lateral in &data.lateral_samples {
            let x = road.center[0] + lateral;
            let mut y = height(seed, distance, lateral);
            let driveway = &area.driveway_bounds;
            let z_fade = ((distance + driveway.max[2] as f64 + 4.0) / 4.0).clamp(0.0, 1.0)
                * ((-driveway.min[2] as f64 + 4.0 - distance) / 4.0).clamp(0.0, 1.0);
            let x_fade = ((x - road.center[0] - 4.0)
                / (driveway.min[0] as f64 - road.center[0] - 4.0).max(1.0))
            .clamp(0.0, 1.0)
                * ((home.max[0] as f64 + 4.0 - x) / 4.0).clamp(0.0, 1.0);
            y += (area.driveway_height as f64 - y) * z_fade * x_fade;
            let yard_x = ((x - home.min[0] as f64 + 4.0) / 4.0).clamp(0.0, 1.0)
                * ((home.max[0] as f64 + 4.0 - x) / 4.0).clamp(0.0, 1.0);
            let yard_z = ((distance + home.max[2] as f64 + 4.0) / 4.0).clamp(0.0, 1.0)
                * ((-home.min[2] as f64 + 4.0 - distance) / 4.0).clamp(0.0, 1.0);
            y += (area.driveway_height as f64 - y) * yard_x * yard_z;
            data.vertices.push([
                (x - origin[0]) as f32,
                y as f32,
                (-distance - origin[2]) as f32,
            ]);
        }
    }
    for row in 0..rows.len() - 1 {
        for col in 0..columns - 1 {
            let a = (row * columns + col) as u32;
            let b = a + 1;
            let c = a + columns as u32;
            let d = c + 1;
            for triangle in [[a, b, c], [b, d, c]] {
                data.indices.push(triangle);
            }
        }
    }
    terrain::carve_rooms(data, interiors);
    data.props
        .retain(|prop| !inside(prop.absolute_point.map(|x| x as f32), home, 4.0));
    data.stations
        .retain(|station| !inside(station.absolute_point.map(|x| x as f32), home, 10.0));
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
    pumps: Vec<(i64, ColliderHandle)>,
}

pub struct WorldStream {
    pub seed: u64,
    pub origin: WorldOrigin,
    chunks: Vec<LoadedChunk>,
    lateral_band: i64,
    starting_area: Option<StartingAreaReference>,
    interiors: Vec<Bounds>,
    station_items: Vec<(i64, crate::reference::ItemId)>,
    vehicle_support: Vec<(ChunkKey, i64, ColliderHandle)>,
    /// Reused list of the chunk cells that the vehicles need.
    support_cells: Vec<(ChunkKey, i64)>,
}

impl WorldStream {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            origin: WorldOrigin::default(),
            chunks: Vec::with_capacity(11),
            lateral_band: 0,
            starting_area: None,
            interiors: Vec::new(),
            station_items: Vec::new(),
            vehicle_support: Vec::with_capacity(6),
            support_cells: Vec::with_capacity(6),
        }
    }

    pub fn configure_starting_area(&mut self, reference: &ReferenceManifest) {
        assert!(
            self.chunks.is_empty(),
            "Configure terrain before loading chunks"
        );
        self.starting_area = Some(reference.area.clone());
        for collider in &reference.colliders {
            let Some(node) = reference.nodes.iter().find(|node| node.id == collider.node) else {
                continue;
            };
            if !node.name.starts_with("interior")
                || node.asset
                    != reference
                        .nodes
                        .iter()
                        .find(|node| node.id == reference.area.home_root)
                        .unwrap()
                        .asset
            {
                continue;
            }
            let ColliderGeometry::Convex { points } = &collider.geometry else {
                continue;
            };
            let body = reference
                .bodies
                .iter()
                .find(|body| body.id == collider.body)
                .unwrap();
            let pose = crate::vehicle::to_rapier(VehiclePose {
                position: body.pose.translation,
                rotation: body.pose.rotation,
            });
            let mut bounds = Bounds {
                min: [f32::INFINITY; 3],
                max: [f32::NEG_INFINITY; 3],
            };
            for point in points {
                let point = (pose * rapier3d::math::Vector::from_array(*point)).to_array();
                for axis in 0..3 {
                    bounds.min[axis] = bounds.min[axis].min(point[axis]);
                    bounds.max[axis] = bounds.max[axis].max(point[axis]);
                }
            }
            self.interiors.push(bounds);
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
            let mut data = generate_chunk_in_band(self.seed, key, self.lateral_band);
            if let Some(area) = &self.starting_area {
                prepare_terrain(&mut data, self.seed, area, &self.interiors);
            }
            let mut colliders = Vec::with_capacity(7);
            let mut pumps = Vec::new();
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
                let handle = physics.insert_box(self.origin.local(pump), [0.35, 0.7, 0.35]);
                colliders.push(handle);
                pumps.push((station.id, handle));
            }
            self.chunks.push(LoadedChunk {
                data,
                colliders,
                pumps,
            });
        }
        self.sync_vehicle_support(physics);
    }

    /// Keeps terrain under every vehicle, also far from the streamed focus.
    fn sync_vehicle_support(&mut self, physics: &mut PhysicsWorld) {
        self.support_cells.clear();
        for vehicle in 0..physics.vehicle_count() {
            let car = self
                .origin
                .absolute(physics.snapshot(vehicle).pose.position);
            let distance = -car[2];
            let lateral = car[0] - road_sample(self.seed, distance).center[0];
            let band = (lateral / LATERAL_BAND_WIDTH).round() as i64;
            let center = (distance / CHUNK_LENGTH).floor() as i64;
            for key in [center - 1, center, center + 1] {
                // One terrain cell covers the fixed car's full footprint.
                let covered = self.chunks.iter().any(|chunk| {
                    chunk.data.key == ChunkKey(key)
                        && (lateral - chunk.data.lateral_band as f64 * LATERAL_BAND_WIDTH).abs()
                            < TERRAIN_HALF_WIDTH - 8.0
                });
                if !covered && !self.support_cells.contains(&(ChunkKey(key), band)) {
                    self.support_cells.push((ChunkKey(key), band));
                }
            }
        }
        let cells = &self.support_cells;
        self.vehicle_support.retain(|&(key, band, handle)| {
            if cells.contains(&(key, band)) {
                return true;
            }
            physics.remove_collider(handle);
            false
        });
        for index in 0..self.support_cells.len() {
            let (key, band) = self.support_cells[index];
            if self
                .vehicle_support
                .iter()
                .any(|&(old_key, old_band, _)| old_key == key && old_band == band)
            {
                continue;
            }
            let mut data = generate_chunk_in_band(self.seed, key, band);
            if let Some(area) = &self.starting_area {
                prepare_terrain(&mut data, self.seed, area, &self.interiors);
            }
            let handle = physics.insert_mesh_at(
                &data.vertices,
                &data.indices,
                self.origin.local(data.origin()),
            );
            self.vehicle_support.push((key, band, handle));
        }
    }

    /// Call between fixed steps. Stored metadata remains absolute.
    pub fn rebase(&mut self, position: [f32; 3], physics: &mut PhysicsWorld) -> Option<[f32; 3]> {
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

pub fn sync_station_supplies(
    stream: &mut WorldStream,
    items: &mut crate::interaction::items::ItemStore,
    interaction: &mut crate::interaction::InteractionState,
    physics: &mut PhysicsWorld,
) {
    for chunk in &stream.chunks {
        for &(station, handle) in &chunk.pumps {
            let item = if let Some((_, item)) =
                stream.station_items.iter().find(|(id, _)| *id == station)
            {
                *item
            } else {
                let item =
                    crate::reference::ItemId(0x8000_0000 + stream.station_items.len() as u32);
                let pose =
                    crate::vehicle::from_rapier(physics.rapier().colliders[handle].position());
                items.insert_supply(item, pose);
                // Retain the road stop's original four-liter-per-second rate.
                interaction
                    .register_pump(item, 4.0)
                    .expect("valid station pump");
                stream.station_items.push((station, item));
                item
            };
            physics.rapier_mut().colliders[handle].user_data = item.0 as u128 + 1;
            interaction.bind_surface(
                handle,
                crate::reference::ActionBinding {
                    collider: crate::reference::ColliderId(0),
                    use_owner: Some(item),
                    pickup_owner: None,
                    removal_owner: None,
                    push_owner: None,
                },
            );
            let state = items.get_mut(item).expect("registered station supply");
            state.pose = crate::vehicle::from_rapier(physics.rapier().colliders[handle].position());
            state
                .fluid
                .as_mut()
                .expect("pump reservoir")
                .mixture
                .quantities_liters = [1000.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        }
    }
}
