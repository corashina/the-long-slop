//! Source road progression in a fixed generation frame.
use super::{SourceRandom, TerrainEvaluator, WorldBounds, source_perlin};
use crate::reference::{ReferenceError, WorldReference};
use std::sync::Arc;

pub const ROAD_SEGMENTS: u32 = 50_000;
pub const ROAD_CONTROLS: usize = 10;
pub const ROAD_CHECKPOINT_INTERVAL: u32 = 256;
const BLOCKS: usize = ROAD_SEGMENTS.div_ceil(ROAD_CHECKPOINT_INTERVAL) as usize;

#[derive(Clone, Debug)]
struct CurveOctave {
    denominator: f64,
    ordinate: f64,
    amplitude: f32,
}

/// The source OneValue curve, before multiplication by road width.
#[derive(Clone, Debug)]
pub struct RoadCurve {
    noises: Vec<Vec<CurveOctave>>,
    seed: f32,
    offset: f32,
}
impl RoadCurve {
    pub fn new(reference: &WorldReference, seed: i32) -> Self {
        let seed = if reference.terrain.random_seed != 0 {
            seed
        } else {
            reference.terrain.seed
        };
        let seed =
            (SourceRandom::new(seed).next_range(-100000, 100000) as f64 * 1E-05_f32 as f64) as f32;
        let road = &reference.road;
        let noises = road
            .noises
            .iter()
            .map(|noise| {
                let mut amplitude = noise.amplitude;
                let mut frequency = noise.frequency;
                (1..=noise.octaves + 1)
                    .map(|i| {
                        let octave = CurveOctave {
                            denominator: road.length as f64 * road.scale as f64 * frequency as f64,
                            ordinate: seed as f64
                                + (i as f32 as f64 / noise.octaves as f32 as f64 + 1.0),
                            amplitude,
                        };
                        amplitude = (amplitude as f64 * noise.persistence as f64) as f32;
                        frequency = (frequency as f64 * noise.lacunarity as f64) as f32;
                        octave
                    })
                    .collect()
            })
            .collect();
        let mut curve = Self {
            noises,
            seed,
            offset: 0.0,
        };
        // Source first-call recursion evaluates zero before the requested distance.
        curve.offset = -curve.sum(0.0);
        curve
    }

    fn sum(&self, distance: f32) -> f32 {
        let mut total = 0.0_f32;
        for noise in &self.noises {
            let mut value = 0.0_f32;
            for octave in noise {
                let x = (self.seed as f64 + distance as f64 / octave.denominator) as f32;
                let y = ((octave.ordinate + distance as f64) / octave.denominator) as f32;
                let sample = source_perlin(x, y) as f64 * octave.amplitude as f64;
                value = (value as f64 + sample) as f32;
            }
            total = (total as f64 + value as f64) as f32;
        }
        total
    }

    pub fn value(&self, distance: f32) -> f32 {
        (self.sum(distance) as f64 + self.offset as f64) as f32
    }
}

/// Carried source values at the boundary before the next segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadState {
    pub next_segment: u32,
    pub source_position: [f64; 3],
    pub source_rotation: [f32; 3],
    pub distance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadCheckpoint {
    state: RoadState,
}
impl RoadCheckpoint {
    pub fn state(&self) -> RoadState {
        self.state
    }
}

/// Source controls remain unconverted for source skinning and terrain alignment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadSegment {
    pub index: u32,
    pub biome: u8,
    pub source_position: [f64; 3],
    pub source_origin: [f64; 3],
    pub source_rotations: [[f32; 3]; ROAD_CONTROLS],
    pub source_coords: [[f32; 3]; ROAD_CONTROLS],
    pub bounds: WorldBounds,
}
impl RoadSegment {
    /// Absolute runtime origin; apply the source Z reflection once.
    pub fn position(&self) -> [f64; 3] {
        runtime_point(std::array::from_fn(|a| {
            self.source_origin[a] + self.source_position[a]
        }))
    }
}

struct RoadInputs {
    curve: RoadCurve,
    origin: [f64; 3],
    bone_length: f32,
    segment_length: f32,
    above_terrain: f32,
    angle_limits: [f32; 2],
    width: f32,
    start_straight: u32,
    biome_override: Option<u8>,
    straight_modifier: f32,
    edge: bool,
    bounds_radius: f64,
}

#[derive(Clone)]
pub struct RoadSequence {
    inputs: Arc<RoadInputs>,
    state: RoadState,
}
impl RoadSequence {
    pub fn new(reference: &WorldReference, seed: i32) -> Result<Self, ReferenceError> {
        let road = &reference.road;
        let frame = &reference.generation_frame;
        if road.road_bone_num != ROAD_CONTROLS as u32
            || road.length != 5_000_000.0
            || road.road_segment_length != 100.0
            || road.road_bone_length != 10.0
            || road.aliign_road != 0
            || road.align_buildings != 0
            || frame.translation != [0.0; 3]
            || frame.rotation != [0.0, 0.0, 0.0, 1.0]
            || frame.scale != [1.0; 3]
        {
            return Err(ReferenceError(format!(
                "Invalid reference field: {}:{} frozen road frame/extent",
                road.source.file, road.source.path_id
            )));
        }
        // FStart samples terrain before StartObjs receives its shared garage offset.
        // The road parent translation is not the generation origin.
        let terrain = TerrainEvaluator::new(reference, seed);
        let origin = [
            0.0,
            terrain.base_height([0.0; 2], road.align_edge_of_the_world != 0) as f64,
            0.0,
        ];
        Ok(Self {
            inputs: Arc::new(RoadInputs {
                curve: RoadCurve::new(reference, seed),
                origin,
                bone_length: road.road_bone_length,
                segment_length: road.road_segment_length,
                above_terrain: road.road_above_terrain,
                angle_limits: [road.xmax_angle, road.ymax_angle],
                width: road.width,
                start_straight: road.start_straight,
                biome_override: (reference.terrain.game_settings.biome != 0)
                    .then_some((reference.terrain.game_settings.biome - 1) as u8),
                straight_modifier: road.straight_modifier,
                edge: road.align_edge_of_the_world != 0,
                bounds_radius: bounds_radius(reference)?,
            }),
            state: RoadState {
                next_segment: 0,
                source_position: [0.0; 3],
                source_rotation: [0.0; 3],
                distance: 0.0,
            },
        })
    }

    pub fn checkpoint(&self) -> RoadCheckpoint {
        RoadCheckpoint { state: self.state }
    }

    /// Restore a checkpoint from this sequence's checked inputs and seed.
    pub fn restore(&mut self, checkpoint: &RoadCheckpoint) {
        self.state = checkpoint.state;
    }

    pub fn advance(&mut self, terrain: &TerrainEvaluator) -> Option<RoadSegment> {
        if self.state.next_segment == ROAD_SEGMENTS {
            return None;
        }
        let input = &self.inputs;
        let mut segment = RoadSegment {
            index: self.state.next_segment,
            biome: super::biome(
                terrain.color(
                    [
                        (input.origin[0] + self.state.source_position[0]) as f32,
                        (input.origin[2] + self.state.source_position[2]) as f32,
                    ],
                    (input.origin[1] + self.state.source_position[1]) as f32,
                ),
                input.biome_override,
            ),
            source_position: self.state.source_position,
            source_origin: input.origin,
            source_rotations: [[0.0; 3]; ROAD_CONTROLS],
            source_coords: [[0.0; 3]; ROAD_CONTROLS],
            bounds: empty_bounds(),
        };
        segment.source_rotations[0] = self.state.source_rotation;
        let mut last = forward_step(self.state.source_rotation, input.bone_length);
        self.state.source_position =
            std::array::from_fn(|a| self.state.source_position[a] + last[a] as f64);
        segment.source_coords[0] = [
            last[0],
            (last[1] as f64 - input.above_terrain as f64) as f32,
            last[2],
        ];
        for bone in 1..ROAD_CONTROLS {
            let step = forward_step(self.state.source_rotation, input.bone_length);
            let look: [f32; 3] = std::array::from_fn(|a| {
                let absolute =
                    (input.origin[a] + segment.source_position[a] + last[a] as f64) as f32;
                (absolute as f64 + step[a] as f64) as f32
            });
            let mut height = (input.above_terrain as f64
                + terrain.base_height([look[0], look[2]], input.edge) as f64)
                as f32;
            for offset in [1.0, 2.0, -1.0, -2.0] {
                let xz = [0, 2].map(|a| {
                    let first =
                        look[a] as f64 + step[a] as f64 * if offset > 0.0 { 1.0 } else { -1.0 };
                    (if offset == 2.0 {
                        first + step[a] as f64
                    } else if offset == -2.0 {
                        first - step[a] as f64
                    } else {
                        first
                    }) as f32
                });
                height = ((height as f64
                    + input.above_terrain as f64
                    + terrain.base_height(xz, input.edge) as f64)
                    * 0.5) as f32;
            }
            // OneCoord projects onto the checked identity world's forward axis.
            let distance = (look[2] as f64 - input.origin[2]) as f32;
            let target_x = (input.curve.value(distance.abs()) as f64 * input.width as f64) as f32;
            let error_x = (target_x as f64 - look[0] as f64) as f32;
            let mut rotation = [
                turn_angle(
                    (look[1] as f64 - height as f64) as f32,
                    input.bone_length,
                    input.angle_limits[0],
                ),
                turn_angle(error_x, input.bone_length, input.angle_limits[1]),
                0.0,
            ];
            if segment.index < input.start_straight {
                rotation[1] = (rotation[1] as f64 * input.straight_modifier as f64) as f32;
            }
            segment.source_rotations[bone] = rotation;
            self.state.source_rotation = std::array::from_fn(|a| {
                ((self.state.source_rotation[a] as f64 + rotation[a] as f64) as f32)
                    .clamp(-90.0, 90.0)
            });
            let step = forward_step(self.state.source_rotation, input.bone_length);
            last = std::array::from_fn(|a| (last[a] as f64 + step[a] as f64) as f32);
            self.state.source_position =
                std::array::from_fn(|a| self.state.source_position[a] + step[a] as f64);
            segment.source_coords[bone] = [
                last[0],
                (last[1] as f64 - input.above_terrain as f64) as f32,
                last[2],
            ];
        }
        self.state.next_segment += 1;
        self.state.distance += input.segment_length as f64;
        let position = segment.position();
        segment.bounds = WorldBounds {
            min: position.map(|x| x - input.bounds_radius),
            max: position.map(|x| x + input.bounds_radius),
        };
        Some(segment)
    }
}

#[derive(Clone, Copy)]
struct RoadBlock {
    first: u32,
    end: u32,
    bounds: WorldBounds,
}

const _: () = assert!(
    (BLOCKS + 1) * std::mem::size_of::<RoadCheckpoint>()
        + BLOCKS * std::mem::size_of::<RoadBlock>()
        + ROAD_CHECKPOINT_INTERVAL as usize * std::mem::size_of::<RoadSegment>()
        < 16 * 1024 * 1024
);

/// Sparse source states and conservative spatial bounds. Inputs remain private and frozen.
pub struct RoadIndex {
    sequence: RoadSequence,
    replay: RoadSequence,
    terrain: TerrainEvaluator,
    checkpoints: Vec<RoadCheckpoint>,
    blocks: Vec<RoadBlock>,
    buffer: Vec<RoadSegment>,
    buffer_first: u32,
    replay_end: u32,
    replay_work: usize,
    replay_ms: f64,
    peak_replay_ms: f64,
}
impl RoadIndex {
    pub fn new(sequence: RoadSequence, terrain: TerrainEvaluator) -> Self {
        assert_eq!(
            sequence.state.next_segment, 0,
            "road index requires a fresh sequence"
        );
        let mut checkpoints = Vec::with_capacity(BLOCKS + 1);
        checkpoints.push(sequence.checkpoint());
        Self {
            replay: sequence.clone(),
            sequence,
            terrain,
            checkpoints,
            blocks: Vec::with_capacity(BLOCKS),
            buffer: Vec::with_capacity(ROAD_CHECKPOINT_INTERVAL as usize),
            buffer_first: 0,
            replay_end: 0,
            replay_work: 0,
            replay_ms: 0.,
            peak_replay_ms: 0.,
        }
    }

    /// Advance at most max_segments source segments. Return true at the finite source end.
    pub fn prepare(&mut self, max_segments: usize) -> bool {
        for _ in 0..max_segments {
            let Some(segment) = self.sequence.advance(&self.terrain) else {
                break;
            };
            if segment.index % ROAD_CHECKPOINT_INTERVAL == 0 {
                self.blocks.push(RoadBlock {
                    first: segment.index,
                    end: segment.index,
                    bounds: empty_bounds(),
                });
            }
            let block = self.blocks.last_mut().unwrap();
            block.end = segment.index + 1;
            for a in 0..3 {
                block.bounds.min[a] = block.bounds.min[a].min(segment.bounds.min[a]);
                block.bounds.max[a] = block.bounds.max[a].max(segment.bounds.max[a]);
            }
            let next = self.sequence.state.next_segment;
            if next % ROAD_CHECKPOINT_INTERVAL == 0 || next == ROAD_SEGMENTS {
                self.checkpoints.push(self.sequence.checkpoint());
            }
        }
        self.sequence.state.next_segment == ROAD_SEGMENTS
    }

    pub fn candidate_count(&self, bounds: WorldBounds) -> usize {
        self.blocks
            .iter()
            .filter(|block| {
                (0..3).all(|a| {
                    block.bounds.min[a] <= bounds.max[a] && block.bounds.max[a] >= bounds.min[a]
                })
            })
            .map(|block| (block.end - block.first) as usize)
            .sum()
    }

    /// Return candidate segment identities from intersecting prepared blocks.
    /// Call segment to refine candidates against their segment bounds.
    pub fn segments_in(&self, bounds: WorldBounds, output: &mut Vec<u32>) {
        output.clear();
        for block in &self.blocks {
            if (0..3).all(|a| {
                block.bounds.min[a] <= bounds.max[a] && block.bounds.max[a] >= bounds.min[a]
            }) {
                output.extend(block.first..block.end);
            }
        }
    }

    /// Advance replay by a bounded number of source segments.
    pub fn prepare_segment(&mut self, index: u32, limit: usize) -> Option<RoadSegment> {
        self.replay_work = 0;
        self.replay_ms = 0.;
        if index >= self.sequence.state.next_segment {
            return None;
        }
        if (self.buffer_first..self.buffer_first + self.buffer.len() as u32).contains(&index) {
            return Some(self.buffer[(index - self.buffer_first) as usize]);
        }
        #[cfg(target_arch = "wasm32")]
        let replay_started = crate::browser::game_now();
        #[cfg(not(target_arch = "wasm32"))]
        let replay_started = std::time::Instant::now();
        let block = self.blocks[(index / ROAD_CHECKPOINT_INTERVAL) as usize];
        if self.buffer_first != block.first || self.replay_end == 0 {
            self.replay
                .restore(&self.checkpoints[(index / ROAD_CHECKPOINT_INTERVAL) as usize]);
            self.buffer.clear();
            self.buffer_first = block.first;
            self.replay_end = block.first;
        }
        for _ in 0..limit {
            if self.replay_end > index {
                break;
            }
            self.buffer
                .push(self.replay.advance(&self.terrain).unwrap());
            self.replay_end += 1;
            self.replay_work += 1;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.replay_ms = crate::browser::game_now() - replay_started;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.replay_ms = replay_started.elapsed().as_secs_f64() * 1000.;
        }
        self.peak_replay_ms = self.peak_replay_ms.max(self.replay_ms);
        (self.replay_end > index).then(|| self.buffer[(index - self.buffer_first) as usize])
    }

    /// Synchronous access for acceptance callers. Runtime jobs use prepare_segment.
    pub fn segment(&mut self, index: u32) -> RoadSegment {
        assert!(
            index < self.prepared_segments(),
            "road segment is not prepared"
        );
        self.prepare_segment(index, ROAD_CHECKPOINT_INTERVAL as usize)
            .unwrap()
    }

    pub fn replay_ms(&self) -> f64 {
        self.replay_ms
    }
    pub fn peak_replay_ms(&self) -> f64 {
        self.peak_replay_ms
    }

    pub fn replay_work(&self) -> usize {
        self.replay_work
    }

    pub fn prepared_segments(&self) -> u32 {
        self.sequence.state.next_segment
    }
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }
    pub fn retained_index_bytes(&self) -> usize {
        self.checkpoints.capacity() * std::mem::size_of::<RoadCheckpoint>()
            + self.blocks.capacity() * std::mem::size_of::<RoadBlock>()
            + self.buffer.capacity() * std::mem::size_of::<RoadSegment>()
    }
}

fn runtime_point([x, y, z]: [f64; 3]) -> [f64; 3] {
    [x, y, -z]
}
fn empty_bounds() -> WorldBounds {
    WorldBounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    }
}

fn bounds_radius(reference: &WorldReference) -> Result<f64, ReferenceError> {
    let mut radius = 0.0_f64;
    for identity in &reference.road.prototypes {
        let prototype = reference
            .prototypes
            .iter()
            .find(|p| &p.source == identity)
            .ok_or_else(|| {
                ReferenceError(format!(
                    "Missing road prototype {}:{}",
                    identity.file, identity.path_id
                ))
            })?;
        for skin in &prototype.skins {
            for (bone, bind) in skin.bones.iter().zip(&skin.bind_poses) {
                for vertex in &skin.positions {
                    let local: [f64; 3] = std::array::from_fn(|a| {
                        bind[a] as f64 * vertex[0] as f64
                            + bind[4 + a] as f64 * vertex[1] as f64
                            + bind[8 + a] as f64 * vertex[2] as f64
                            + bind[12 + a] as f64
                    });
                    let mut reach = local.into_iter().map(|v| v * v).sum::<f64>().sqrt();
                    let mut node = Some(*bone);
                    while let Some(index) = node {
                        let frame = &prototype.nodes[index as usize];
                        let scale = frame
                            .transform
                            .scale
                            .into_iter()
                            .map(|v| (v as f64).abs())
                            .fold(0.0, f64::max);
                        let translation = frame
                            .transform
                            .translation
                            .into_iter()
                            .map(|v| (v as f64).powi(2))
                            .sum::<f64>()
                            .sqrt();
                        // Cover float quaternion length and transform rounding at each node.
                        reach = translation + reach * scale * 1.00001;
                        node = frame.parent;
                    }
                    radius = radius.max(reach);
                }
            }
        }
    }
    // Include the source control path even if a skin has no influence at its final control.
    radius = radius.max(reference.road.road_segment_length as f64);
    let mut placement = 0.0_f64;
    let side_extent = |side: &crate::reference::WorldBuildingSide| {
        (side.min_xpos.abs().max(side.max_xpos.abs())) as f64
    };
    for side in &reference.buildings.default_sides {
        placement = placement.max(side_extent(side));
    }
    for group in &reference.buildings.groups {
        for selection in &group.types {
            let sides = if selection.dont_use_default == 0 {
                &reference.buildings.default_sides
            } else {
                &selection.sides
            };
            for side in sides {
                let modifier = if selection.dont_use_default == 0 && selection.use_modifier != 0 {
                    selection.x_dist_modifier.abs() as f64
                } else {
                    1.0
                };
                placement = placement.max(side_extent(side) * modifier);
            }
        }
    }
    Ok(radius + placement + reference.road.road_above_terrain.abs() as f64)
}

fn turn_angle(error: f32, length: f32, limit: f32) -> f32 {
    let ratio = ((error as f64 / length as f64) as f32).clamp(-1.0, 1.0);
    let radians = (ratio as f64).asin() as f32;
    let degrees = (radians as f64 * 57.29578_f32 as f64) as f32;
    degrees.clamp(-limit, limit)
}

// UnityPlayer's source mode-one finite-angle polynomials. Road angles stay in [-90, 90].
fn sin_poly(x: f64) -> f64 {
    let square = x * x;
    let p = 2.7557319223985893e-6_f64.mul_add(square, -0.0001984126984126984);
    let p = p
        .mul_add(square, 0.008333333333333333)
        .mul_add(square, -0.16666666666666666);
    p.mul_add(x * square, x)
}
fn cos_poly(x: f64, reduced_sine: bool) -> f64 {
    let square = x * x;
    let base = if reduced_sine {
        (-0.5_f64).mul_add(square, 1.0)
    } else {
        1.0 - square * 0.5
    };
    let p = (-2.755731922398589e-7_f64).mul_add(square, 2.4801587301587298e-5);
    let p = p
        .mul_add(square, -0.0013888888888888887)
        .mul_add(square, 0.041666666666666664);
    p.mul_add(square * square, base)
}
fn half_angle(degrees: f32) -> (f32, f32) {
    let radians = (degrees as f64 * 0.0174532924_f32 as f64) as f32;
    let angle = radians * 0.5;
    let x = angle as f64;
    let abs = x.abs();
    if abs > std::f64::consts::FRAC_PI_4 {
        // Only the first quadrant reduction can occur in the checked road domain.
        let reduced = (-1.0_f64).mul_add(1.5707963267341256, abs) - 6.077100506506192e-11;
        return (
            (cos_poly(reduced, true).copysign(x)) as f32,
            (-sin_poly(reduced)) as f32,
        );
    }
    if abs < 0.0001220703125 {
        return (angle, 1.0);
    }
    if abs < 0.0078125 {
        return (
            (-(x * x * x)).mul_add(0.16666666666666666, x) as f32,
            (-(x * 0.5)).mul_add(x, 1.0) as f32,
        );
    }
    (sin_poly(x) as f32, cos_poly(x, false) as f32)
}
fn forward_step(rotation: [f32; 3], length: f32) -> [f32; 3] {
    let (sx, cx) = half_angle(rotation[0]);
    let (sy, cy) = half_angle(rotation[1]);
    // Source Euler order is Y * X * Z. The source road holds Z at zero.
    // Keep native f32 products before the managed vector overload's f64 locals.
    let x = (sx * cy) as f64;
    let y = (cx * sy) as f64;
    let z = (0.0_f32 - sx * sy) as f64;
    let w = (cx * cy) as f64;
    let x2 = x * 2.0;
    let y2 = y * 2.0;
    let z2 = z * 2.0;
    [
        ((x * z2 + w * y2) * length as f64) as f32,
        ((y * z2 - w * x2) * length as f64) as f32,
        ((1.0 - (x * x2 + y * y2)) * length as f64) as f32,
    ]
}

pub(crate) fn road_surface_bound(reference: &WorldReference, biome: u8) -> usize {
    let prototype = reference
        .prototypes
        .iter()
        .find(|p| p.source == reference.road.prototypes[biome as usize])
        .expect("road prototype");
    prototype
        .skins
        .iter()
        .map(|s| {
            s.positions.len() * 24
                + s.tangents.len() * 16
                + s.uv.len() * 8
                + s.triangles.iter().map(|t| t.len() * 12).sum::<usize>()
                + s.triangles.len() * std::mem::size_of::<std::ops::Range<usize>>()
        })
        .sum::<usize>()
        + prototype.nodes.len() * std::mem::size_of::<RoadFrame>()
}
/// Deform the admitted road skin in its authored bone and influence order.
/// Source mesh inputs already contain the Z reflection; do not reflect them again.
pub fn deform_road(
    reference: &WorldReference,
    segment: &RoadSegment,
    output: &mut super::WorldSurface,
) {
    use rapier3d::math::Vector;
    let identity = &reference.road.prototypes[segment.biome as usize];
    let prototype = reference
        .prototypes
        .iter()
        .find(|p| &p.source == identity)
        .expect("validated road prototype");
    let frames = road_frames(prototype, segment);
    output.origin = segment.position();
    output.vertices.clear();
    output.normals.clear();
    output.tangents.clear();
    output.uv.clear();
    output.colors.clear();
    output.triangles.clear();
    output.submeshes.clear();
    output
        .vertices
        .reserve_exact(prototype.skins.iter().map(|s| s.positions.len()).sum());
    output
        .normals
        .reserve_exact(prototype.skins.iter().map(|s| s.normals.len()).sum());
    output
        .tangents
        .reserve_exact(prototype.skins.iter().map(|s| s.tangents.len()).sum());
    output
        .uv
        .reserve_exact(prototype.skins.iter().map(|s| s.uv.len()).sum());
    output.triangles.reserve_exact(
        prototype
            .skins
            .iter()
            .flat_map(|s| &s.triangles)
            .map(Vec::len)
            .sum(),
    );
    output
        .submeshes
        .reserve_exact(prototype.skins.iter().map(|s| s.triangles.len()).sum());
    for skin in &prototype.skins {
        let base = output.vertices.len() as u32;
        output.vertices.reserve(skin.positions.len());
        output.normals.reserve(skin.positions.len());
        for index in 0..skin.positions.len() {
            let p = skin.positions[index];
            let normal = skin.normals[index];
            let tangent = skin.tangents.get(index);
            let mut point = Vector::ZERO;
            let mut n = Vector::ZERO;
            let mut t = Vector::ZERO;
            for influence in 0..4 {
                let weight = skin.weights[index][influence];
                if weight == 0.0 {
                    continue;
                }
                let bone = skin.joints[index][influence] as usize;
                let bind = skin.bind_poses[bone];
                let frame = frames[skin.bones[bone] as usize];
                let transform = |v: [f32; 3], w: f32| {
                    Vector::from_array(std::array::from_fn(|a| {
                        bind[a] * v[0] + bind[4 + a] * v[1] + bind[8 + a] * v[2] + bind[12 + a] * w
                    }))
                };
                point +=
                    (frame.position + frame.rotation * (frame.scale * transform(p, 1.0))) * weight;
                n += (frame.rotation * (frame.scale * transform(normal, 0.0))) * weight;
                if let Some(tangent) = tangent {
                    t += (frame.rotation
                        * (frame.scale * transform([tangent[0], tangent[1], tangent[2]], 0.0)))
                        * weight;
                }
            }
            output.vertices.push(point.to_array());
            output.normals.push(n.normalize_or_zero().to_array());
            if let Some(tangent) = tangent {
                let t = t.normalize_or_zero();
                output.tangents.push([t.x, t.y, t.z, tangent[3]]);
            }
        }
        output.uv.extend_from_slice(&skin.uv);
        for group in &skin.triangles {
            let first = output.triangles.len();
            output
                .triangles
                .extend(group.iter().map(|triangle| triangle.map(|i| i + base)));
            output.submeshes.push(first..output.triangles.len());
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct RoadFrame {
    pub position: rapier3d::math::Vector,
    pub rotation: rapier3d::math::Rotation,
    pub scale: rapier3d::math::Vector,
}

pub(crate) fn road_frames(
    prototype: &crate::reference::WorldPrototypeReference,
    segment: &RoadSegment,
) -> Vec<RoadFrame> {
    use rapier3d::math::{Rotation, Vector};
    let mut frames = Vec::with_capacity(prototype.nodes.len());
    for (index, node) in prototype.nodes.iter().enumerate() {
        let parent = node
            .parent
            .map(|i| frames[i as usize])
            .unwrap_or(RoadFrame {
                position: Vector::ZERO,
                rotation: Rotation::IDENTITY,
                scale: Vector::ONE,
            });
        let mut frame = if index == 0 {
            parent
        } else {
            RoadFrame {
                position: parent.position
                    + parent.rotation
                        * (Vector::from_array(node.transform.translation) * parent.scale),
                rotation: parent.rotation * Rotation::from_array(node.transform.rotation),
                scale: parent.scale * Vector::from_array(node.transform.scale),
            }
        };
        if let Some(control) = prototype.controls.iter().position(|&i| i as usize == index) {
            let angles = segment.source_rotations[control];
            let axes = [
                -(parent.rotation * Vector::X),
                -Vector::Y,
                -(parent.rotation * Vector::Y),
            ];
            for (angle, axis) in angles.into_iter().zip(axes) {
                let (sin, cos) = half_angle(angle);
                let axis = axis.normalize() * sin;
                frame.rotation = Rotation::from_xyzw(axis.x, axis.y, axis.z, cos) * frame.rotation;
            }
        }
        frames.push(frame);
    }
    frames
}
