//! Port of the source terrain and road generators.
//! Values use Unity space: left-handed, +Z is the road's start direction.

use crate::reference::world::{RoadNoise, WorldReference};

const MBIG: i32 = i32::MAX;

/// .NET Framework `System.Random`. Mono uses the same seeded sequence. Arithmetic wraps as in C#.
pub struct DotNetRandom {
    seeds: [i32; 56],
    next: usize,
    next_p: usize,
}

impl DotNetRandom {
    pub fn new(seed: i32) -> Self {
        let mut seeds = [0; 56];
        let mut mj = 161_803_398_i32.wrapping_sub(if seed == i32::MIN { MBIG } else { seed.abs() });
        seeds[55] = mj;
        let mut mk = 1;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            seeds[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk += MBIG;
            }
            mj = seeds[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                seeds[i] = seeds[i].wrapping_sub(seeds[1 + (i + 30) % 55]);
                if seeds[i] < 0 {
                    seeds[i] += MBIG;
                }
            }
        }
        Self {
            seeds,
            next: 0,
            next_p: 21,
        }
    }

    fn sample(&mut self) -> i32 {
        self.next = if self.next >= 55 { 1 } else { self.next + 1 };
        self.next_p = if self.next_p >= 55 {
            1
        } else {
            self.next_p + 1
        };
        let mut value = self.seeds[self.next].wrapping_sub(self.seeds[self.next_p]);
        if value == MBIG {
            value -= 1;
        }
        if value < 0 {
            value += MBIG;
        }
        self.seeds[self.next] = value;
        value
    }

    pub fn next_double(&mut self) -> f64 {
        self.sample() as f64 * (1.0 / MBIG as f64)
    }

    /// `Next(min, max)` for ranges below `int.MaxValue`.
    pub fn range(&mut self, min: i32, max: i32) -> i32 {
        (self.next_double() * (max as i64 - min as i64) as f64) as i32 + min
    }

    /// Source helper `Rnv`.
    fn vector(&mut self) -> [f32; 2] {
        [
            self.range(-100_000, 100_000) as f32 * 1e-5,
            self.range(-100_000, 100_000) as f32 * 1e-5,
        ]
    }
}

const PERMUTATION: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

fn gradient(hash: u8, x: f32, y: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        0.0
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

/// Unity `Mathf.PerlinNoise`: improved noise on the Z = 0 plane, scaled as the engine does.
/// The engine source is closed. This form returns the engine value 0.4652731 at lattice points.
pub fn perlin(x: f32, y: f32) -> f32 {
    let p = |i: usize| PERMUTATION[i & 255] as usize;
    let (xf, yf) = (x.floor(), y.floor());
    let (xi, yi) = (xf as i32 as usize & 255, yf as i32 as usize & 255);
    let (x, y) = (x - xf, y - yf);
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v) = (fade(x), fade(y));
    let a = p(xi) + yi;
    let b = p(xi + 1) + yi;
    let lerp = |t: f32, a: f32, b: f32| a + t * (b - a);
    let value = lerp(
        v,
        lerp(
            u,
            gradient(p(p(a)) as u8, x, y),
            gradient(p(p(b)) as u8, x - 1.0, y),
        ),
        lerp(
            u,
            gradient(p(p(a + 1)) as u8, x, y - 1.0),
            gradient(p(p(b + 1)) as u8, x - 1.0, y - 1.0),
        ),
    );
    (value + 0.69) / 1.483
}

/// Unity `Mathf.InverseLerp`.
pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}

/// Flat zone from `terrainHeightAlignToBuildingScript`. Ranges are squared meters.
#[derive(Clone, Copy, Debug)]
pub struct FlatZone {
    pub center: [f32; 3],
    pub inner_sqr: f32,
    pub outer_sqr: f32,
}

/// Road bone points within reach of one terrain query. Absolute Unity meters.
pub struct RoadPoints<'a> {
    pub points: &'a [[f32; 3]],
    pub range: f32,
}

/// `TerrainGenerationJob` height and vertex color.
pub struct TerrainNoise {
    vectors: [[f32; 2]; 8],
    edge: f32,
    edge_gradient: f32,
}

impl TerrainNoise {
    pub fn new(seed: i32, edge: f32, edge_gradient: f32) -> Self {
        let mut random = DotNetRandom::new(seed);
        Self {
            vectors: std::array::from_fn(|_| random.vector()),
            edge,
            edge_gradient,
        }
    }

    /// Octave offsets come from a fresh `Random(seed)`, so they equal the first vectors.
    fn noise(
        &self,
        c: [f32; 2],
        position: [f32; 2],
        frequency: f32,
        amplitude: f32,
        octaves: usize,
        persistence: f32,
        lacunarity: f32,
    ) -> f32 {
        let (mut value, mut f, mut a) = (0.0, 1.0, 1.0);
        for offset in &self.vectors[..octaves] {
            let frequency = frequency * f;
            value += perlin(
                offset[0] + position[0] + c[0] / frequency,
                offset[1] + position[1] + c[1] / frequency,
            ) * amplitude
                * a;
            f *= lacunarity;
            a *= persistence;
        }
        value
    }

    fn landscape(&self, c: [f32; 2], position: [f32; 2], min: f32, max: f32) -> f32 {
        inverse_lerp(
            min,
            max,
            perlin(
                position[0] + c[0] / 100_000.0,
                position[1] + c[1] / 100_000.0,
            ),
        )
    }

    /// Source `randomize` without alignment. `c` is absolute Unity X and Z.
    pub fn base_height(&self, c: [f32; 2], edge: bool) -> f32 {
        let v = &self.vectors;
        let mut height = self.noise(c, v[1], 1000.0, 100.0, 2, 0.05, 0.1)
            * self.landscape(c, v[0], 0.85, 0.9)
            + self.noise(c, v[3], 10_000.0, 875.0, 3, 0.05, 0.1)
                * self.landscape(c, v[2], 0.3, 0.5)
            + self.noise(c, v[5], 3000.0, 250.0, 3, 0.05, 0.1)
                * self.landscape(c, v[4], 0.45, 0.55)
            + self.noise(c, v[7], 10_000.0, 2500.0, 4, 0.05, 0.1)
                * self.landscape(c, v[6], 0.5, 0.7);
        if edge {
            let start = self.edge - self.edge_gradient;
            height *= inverse_lerp(self.edge, start, c[0].abs())
                * inverse_lerp(self.edge, start, c[1].abs());
        }
        height
    }

    /// Source `randomize` with building and road alignment.
    pub fn height(&self, c: [f32; 2], zones: &[FlatZone], road: &RoadPoints) -> f32 {
        let mut height = self.base_height(c, true);
        let mut forced = None;
        for zone in zones {
            let d = (c[0] - zone.center[0]).powi(2) + (c[1] - zone.center[2]).powi(2);
            if d < zone.inner_sqr {
                forced = Some(zone.center[1]);
                height = zone.center[1];
                break;
            }
            if d < zone.outer_sqr {
                height = zone.center[1]
                    + (height - zone.center[1]) * inverse_lerp(zone.inner_sqr, zone.outer_sqr, d);
            }
        }
        if let Some((value, close)) = align_to_road(c, height, road) {
            height = value;
            if close {
                forced = Some(value);
            }
        }
        forced.unwrap_or(height)
    }

    /// Channels: R sand, G grass, B snow by altitude, A detail.
    pub fn vertex_color(&self, c: [f32; 2], height: f32) -> [f32; 4] {
        let v = &self.vectors;
        [
            inverse_lerp(0.55, 0.65, self.noise(c, v[0], 50_000.0, 1.0, 2, 0.3, 0.25)),
            inverse_lerp(0.3, 0.4, self.noise(c, v[1], 50_000.0, 1.0, 2, 0.3, 0.25)),
            inverse_lerp(1600.0, 2500.0, height),
            inverse_lerp(0.6, 0.8, self.noise(c, v[2], 20_000.0, 1.0, 2, 0.3, 0.25)),
        ]
    }

    /// `TerrainGenerator.GetBiome`: 0 desert, 1 winter, 2 grass.
    pub fn biome(&self, c: [f32; 2], height: f32) -> usize {
        let color = self.vertex_color(c, height);
        if color[2] > 0.25 {
            1
        } else if color[0] > 0.5 {
            0
        } else {
            2
        }
    }
}

/// Returns the aligned height, or `None` outside the road range.
/// The flag marks the 4.5 meter band where the road height replaces building alignment.
fn align_to_road(c: [f32; 2], value: f32, road: &RoadPoints) -> Option<(f32, bool)> {
    let distance = |p: &[f32; 3]| (c[0] - p[0]).powi(2) + (c[1] - p[2]).powi(2);
    let mut first = None;
    let mut best = f32::MAX;
    for (i, point) in road.points.iter().enumerate() {
        let d = distance(point);
        if d < best {
            best = d;
            first = Some(i);
        }
    }
    let first = first?;
    let mut second = None;
    let mut next = f32::MAX;
    for (i, point) in road.points.iter().enumerate() {
        let d = distance(point);
        if i != first && d < next {
            next = d;
            second = Some(i);
        }
    }
    let second = second?;
    if best >= road.range * road.range {
        return None;
    }
    let (p1, p2) = (road.points[first], road.points[second]);
    let along = [p2[0] - p1[0], p2[2] - p1[2]];
    let offset = [c[0] - p1[0], c[1] - p1[2]];
    let length = (along[0] * along[0] + along[1] * along[1]).sqrt();
    let project = |axis: [f32; 2]| {
        let norm = (axis[0] * axis[0] + axis[1] * axis[1]).sqrt();
        if norm == 0.0 {
            0.0
        } else {
            ((offset[0] * axis[0] + offset[1] * axis[1]) / norm).abs()
        }
    };
    let lateral = project([-along[1], along[0]]);
    let fade = inverse_lerp(road.range * 0.5, road.range, lateral);
    let road_height = p1[1] + (p2[1] - p1[1]) * inverse_lerp(0.0, length, project(along));
    let result = road_height + (value - road_height) * fade;
    if lateral < 4.5 {
        Some((result - 0.1, true))
    } else {
        Some((result, false))
    }
}

/// Source `mainscript.TurnAngle` in degrees.
fn turn_angle(a: f32, c: f32, max: f32) -> f32 {
    ((a / c).clamp(-1.0, 1.0).asin().to_degrees()).clamp(-max, max)
}

/// Unity `Quaternion.Euler(x, y, 0) * Vector3.forward * length`.
fn heading(rotation: [f32; 2], length: f32) -> [f32; 3] {
    let (x, y) = (rotation[0].to_radians(), rotation[1].to_radians());
    [
        x.cos() * y.sin() * length,
        -x.sin() * length,
        x.cos() * y.cos() * length,
    ]
}

/// One `roadclass`: start relative to the road origin, bone rotations, and bone ends.
/// Rotations are pitch and yaw degrees. The first holds the start heading.
#[derive(Clone, Debug)]
pub struct RoadSegment {
    pub position: [f64; 3],
    pub rotations: Vec<[f32; 2]>,
    pub points: Vec<[f32; 3]>,
}

/// `roadGenScript` path state. The road starts at the origin and runs toward +Z.
pub struct RoadPath {
    pub origin: [f64; 3],
    pub segments: Vec<RoadSegment>,
    seed: f32,
    offset: f32,
    rotation: [f32; 2],
    position: [f64; 3],
}

impl RoadPath {
    pub fn new(world: &WorldReference, noise: &TerrainNoise) -> Self {
        let mut random = DotNetRandom::new(world.seed);
        let mut path = Self {
            origin: [0.0, noise.base_height([0.0, 0.0], true) as f64, 0.0],
            segments: Vec::new(),
            seed: random.range(-100_000, 100_000) as f32 * 1e-5,
            offset: 0.0,
            rotation: [0.0; 2],
            position: [0.0; 3],
        };
        path.offset = -path.value(world, 0.0);
        path
    }

    fn noise_value(noise: &RoadNoise, x: f32, length: f32, seed: f32) -> f32 {
        let (mut value, mut a, mut f) = (0.0, noise.amplitude, noise.frequency);
        for i in 1..=noise.octaves + 1 {
            let scale = length * f;
            value += perlin(
                seed + x / scale,
                (seed + (i as f32 / noise.octaves as f32 + 1.0) + x) / scale,
            ) * a;
            a *= noise.persistence;
            f *= noise.lacunarity;
        }
        value
    }

    /// Source `OneValue`: lateral target in road widths at a forward distance.
    fn value(&self, world: &WorldReference, distance: f32) -> f32 {
        let length = world.road.length * world.road.scale;
        world
            .road
            .noises
            .iter()
            .map(|noise| Self::noise_value(noise, distance, length, self.seed))
            .sum::<f32>()
            + self.offset
    }

    /// Source `CalcOneRoad`. Returns false at the source road length.
    pub fn extend(&mut self, world: &WorldReference, noise: &TerrainNoise) -> bool {
        if self.segments.len() >= (world.road.length / world.road.segment_length) as usize {
            return false;
        }
        let road = &world.road;
        let bone = road.bone_length;
        let start = self.position;
        let mut rotations = Vec::with_capacity(road.bone_count);
        let mut points = Vec::with_capacity(road.bone_count);
        rotations.push(self.rotation);
        let mut last = heading(self.rotation, bone);
        for i in 0..3 {
            self.position[i] += last[i] as f64;
        }
        points.push([last[0], last[1] - road.above_terrain, last[2]]);
        let ground = |x: f32, z: f32| road.above_terrain + noise.base_height([x, z], true);
        for _ in 1..road.bone_count {
            let step = heading(self.rotation, bone);
            let probe: [f32; 3] = std::array::from_fn(|i| {
                (self.origin[i] + start[i] + last[i] as f64) as f32 + step[i]
            });
            let mut height = ground(probe[0], probe[2]);
            for k in [1.0, 2.0, -1.0, -2.0] {
                height = (height + ground(probe[0] + step[0] * k, probe[2] + step[2] * k)) * 0.5;
            }
            let lateral = self.value(world, probe[2].abs()) * road.width - probe[0];
            let mut turn = [
                turn_angle(probe[1] - height, bone, road.max_pitch_degrees),
                turn_angle(lateral, bone, road.max_yaw_degrees),
            ];
            if self.segments.len() < road.start_straight {
                turn[1] *= road.straight_modifier;
            }
            rotations.push(turn);
            for i in 0..2 {
                self.rotation[i] = (self.rotation[i] + turn[i]).clamp(-90.0, 90.0);
            }
            let step = heading(self.rotation, bone);
            for i in 0..3 {
                last[i] += step[i];
                self.position[i] += step[i] as f64;
            }
            points.push([last[0], last[1] - road.above_terrain, last[2]]);
        }
        self.segments.push(RoadSegment {
            position: start,
            rotations,
            points,
        });
        true
    }

    /// Absolute Unity Z where the next segment starts.
    pub fn end_z(&self) -> f64 {
        self.origin[2] + self.position[2]
    }

    /// Absolute Unity point of one bone end.
    pub fn point(&self, segment: usize, bone: usize) -> [f32; 3] {
        let s = &self.segments[segment];
        std::array::from_fn(|i| (self.origin[i] + s.position[i] + s.points[bone][i] as f64) as f32)
    }

    /// Absolute Unity start of one segment.
    pub fn start(&self, segment: usize) -> [f64; 3] {
        std::array::from_fn(|i| self.origin[i] + self.segments[segment].position[i])
    }
}
