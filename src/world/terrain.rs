use super::ChunkData;
use crate::reference::Bounds;

type Polygon = Vec<[f32; 3]>;

// Chunk setup clips the terrain to room edges. A centroid cut leaves gaps beside the source floor.
pub(super) fn carve_rooms(data: &mut ChunkData, rooms: &[Bounds]) {
    let origin = data.origin().map(|value| value as f32);
    let triangles = std::mem::take(&mut data.indices);
    data.indices.reserve(triangles.len());
    for triangle in triangles {
        let points = triangle.map(|index| data.vertices[index as usize]);
        if !rooms.iter().any(|room| overlaps(&points, room, origin)) {
            data.indices.push(triangle);
            continue;
        }
        let mut polygons = vec![points.to_vec()];
        for room in rooms {
            let mut exterior = Vec::new();
            for polygon in polygons {
                if !overlaps(&polygon, room, origin) {
                    exterior.push(polygon);
                    continue;
                }
                let mut interior = polygon;
                for (axis, bound, greater) in [
                    (0, room.min[0], true),
                    (0, room.max[0], false),
                    (2, room.min[2], true),
                    (2, room.max[2], false),
                ] {
                    let (next, outside) = split(&interior, axis, bound - origin[axis], greater);
                    if outside.len() >= 3 {
                        exterior.push(outside);
                    }
                    interior = next;
                    if interior.len() < 3 {
                        break;
                    }
                }
            }
            polygons = exterior;
        }
        for polygon in polygons {
            let first = data.vertices.len() as u32;
            data.vertices.extend_from_slice(&polygon);
            for index in 1..polygon.len() - 1 {
                let a = rapier3d::math::Vector::from_array(polygon[index])
                    - rapier3d::math::Vector::from_array(polygon[0]);
                let b = rapier3d::math::Vector::from_array(polygon[index + 1])
                    - rapier3d::math::Vector::from_array(polygon[0]);
                if a.cross(b).length_squared() > 1e-12 {
                    data.indices
                        .push([first, first + index as u32, first + index as u32 + 1]);
                }
            }
        }
    }
}

fn overlaps(points: &[[f32; 3]], room: &Bounds, origin: [f32; 3]) -> bool {
    [0, 2].into_iter().all(|axis| {
        points
            .iter()
            .any(|point| point[axis] + origin[axis] > room.min[axis])
            && points
                .iter()
                .any(|point| point[axis] + origin[axis] < room.max[axis])
    })
}

fn split(polygon: &[[f32; 3]], axis: usize, bound: f32, greater: bool) -> (Polygon, Polygon) {
    let mut inside = Vec::with_capacity(polygon.len() + 1);
    let mut outside = Vec::with_capacity(polygon.len() + 1);
    let sign = if greater { 1.0 } else { -1.0 };
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let da = (a[axis] - bound) * sign;
        let db = (b[axis] - bound) * sign;
        if da >= 0.0 {
            inside.push(a);
        } else {
            outside.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            let mut point = std::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * t);
            point[axis] = bound;
            inside.push(point);
            outside.push(point);
        }
    }
    (inside, outside)
}

use super::{TerrainEvaluator, WorldSurface};
use crate::reference::{ReferenceError, WorldReference, WorldValue};
use rapier3d::math::Vector;

/// Strict source thresholds. A supplied index is already zero based.
pub fn biome(color: [f32; 4], override_index: Option<u8>) -> u8 {
    override_index.unwrap_or(if color[2] > 0.25 {
        1
    } else if color[0] > 0.5 {
        0
    } else {
        2
    })
}

#[derive(Clone, Copy, Debug)]
pub struct TerrainTile {
    pub tier: usize,
    pub size: u32,
    pub resolution: u32,
    pub draw_distance: u32,
    pub y: f32,
    pub material_scale: f32,
}

impl TerrainTile {
    pub fn new(reference: &WorldReference, tier: usize) -> Result<Self, ReferenceError> {
        let tile = reference
            .terrain
            .tiles
            .get(tier)
            .ok_or_else(|| ReferenceError(format!("Terrain tier {tier}")))?;
        let settings = reference
            .records
            .iter()
            .find(|r| r.source == reference.terrain.source)
            .ok_or_else(|| ReferenceError("Missing TerrainGenerationSettings".into()))?;
        let number = |field: &str| -> Result<f32, ReferenceError> {
            match settings.fields.get(field) {
                Some(WorldValue::Number(v)) => Ok(*v as f32),
                Some(WorldValue::Integer(v)) => Ok(*v as f32),
                _ => Err(ReferenceError(format!("TerrainGenerationSettings.{field}"))),
            }
        };
        let distance = |field: &str| {
            reference
                .distances
                .get(field)
                .copied()
                .ok_or_else(|| ReferenceError(format!("World distances.{field}")))
        };
        let (scale, draw) = match tier {
            0 => (1.0, distance("TerrainGenDist")?),
            1 => (
                ((distance("drawDistance")? as f64 / 3.0 / 3.0 * 4.0)
                    / number("defDistantTerrainSize")? as f64) as f32,
                distance("DistantTerrainGenDist")?,
            ),
            2 => (
                ((distance("Cam2FarPlane")? as f64 / 3.0 / 3.0 * 4.0)
                    / number("defDistantTerrain2Size")? as f64) as f32,
                distance("DistantTerrain2GenDist")?,
            ),
            _ => return Err(ReferenceError(format!("Terrain tier {tier}"))),
        };
        let size = (tile.tile_size as f64 * scale as f64) as u32;
        if size == 0 || !scale.is_finite() || scale <= 0.0 {
            return Err(ReferenceError(format!("Terrain tier {tier} scale")));
        }
        Ok(Self {
            tier,
            size,
            resolution: ((tile.resolution as f64 * scale as f64) as u32).max(4),
            draw_distance: ((draw as f64 / size as f64) as u32).max(1),
            y: tile.y_coord,
            material_scale: scale,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerrainRoadPoint {
    /// Frozen source coordinates. These are never rebased.
    pub position: [f32; 3],
    pub segment_origin: [f32; 2],
    pub range: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TerrainFoundation {
    pub position: [f32; 3],
    pub range_squared: [f32; 2],
}

#[derive(Clone, Copy)]
pub struct TerrainContext<'a> {
    pub evaluator: &'a TerrainEvaluator,
    pub tile: TerrainTile,
    /// Source caller order controls ties and foundation precedence.
    pub roads: &'a [TerrainRoadPoint],
    pub road_radius_squared: Option<f32>,
    pub foundations: &'a [TerrainFoundation],
    /// Source corner order, in frozen source coordinates.
    pub holes: &'a [[[f32; 3]; 4]],
    pub biome_override: Option<u8>,
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        ((value as f64 - a as f64) / (b as f64 - a as f64)).clamp(0.0, 1.0) as f32
    }
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (a as f64 + (b as f64 - a as f64) * t.clamp(0.0, 1.0) as f64) as f32
}
fn distance_squared(a: [f32; 2], b: [f32; 2]) -> f32 {
    let x = (a[0] as f64 - b[0] as f64) as f32;
    let y = (a[1] as f64 - b[1] as f64) as f32;
    (x as f64 * x as f64 + y as f64 * y as f64) as f32
}
fn projection(a: [f32; 2], b: [f32; 2]) -> f32 {
    let magnitude = (b[0] as f64 * b[0] as f64 + b[1] as f64 * b[1] as f64).sqrt() as f32;
    if magnitude <= 0.00001 {
        return 0.0;
    }
    let b = b.map(|v| (v as f64 / magnitude as f64) as f32);
    let dot = (a[0] as f64 * b[0] as f64 + a[1] as f64 * b[1] as f64) as f32;
    let p = b.map(|v| (v as f64 * dot as f64) as f32);
    (p[0] as f64 * p[0] as f64 + p[1] as f64 * p[1] as f64).sqrt() as f32
}

impl TerrainContext<'_> {
    pub fn height(&self, point: [f32; 2]) -> f32 {
        let mut value = self.evaluator.base_height(point, true);
        if self.tile.tier != 0 {
            return value;
        }
        let mut close = None;
        for helper in self.foundations {
            let dist = distance_squared(point, [helper.position[0], helper.position[2]]);
            if dist < helper.range_squared[0] {
                close = Some(helper.position[1]);
                value = helper.position[1];
                break;
            }
            if dist < helper.range_squared[1] {
                value = lerp(
                    helper.position[1],
                    value,
                    inverse_lerp(helper.range_squared[0], helper.range_squared[1], dist),
                );
            }
        }
        let mut nearest = [(usize::MAX, f32::MAX); 2];
        for (index, road) in self.roads.iter().enumerate() {
            if self
                .road_radius_squared
                .is_some_and(|radius| distance_squared(point, road.segment_origin) >= radius)
            {
                continue;
            }
            let dist = distance_squared(point, [road.position[0], road.position[2]]);
            if dist < nearest[0].1 {
                nearest[1] = nearest[0];
                nearest[0] = (index, dist);
            } else if dist < nearest[1].1 {
                nearest[1] = (index, dist);
            }
        }
        if nearest[1].0 != usize::MAX {
            let a = self.roads[nearest[0].0];
            let b = self.roads[nearest[1].0];
            if nearest[0].1 < a.range * a.range {
                let delta = [
                    (b.position[0] as f64 - a.position[0] as f64) as f32,
                    (b.position[2] as f64 - a.position[2] as f64) as f32,
                ];
                let offset = [
                    (point[0] as f64 - a.position[0] as f64) as f32,
                    (point[1] as f64 - a.position[2] as f64) as f32,
                ];
                let across = projection(offset, [-delta[1], delta[0]]);
                let length = distance_squared(
                    [a.position[0], a.position[2]],
                    [b.position[0], b.position[2]],
                )
                .sqrt();
                let along = inverse_lerp(0.0, length, projection(offset, delta));
                let road_height = lerp(a.position[1], b.position[1], along);
                let fade = inverse_lerp(a.range * 0.5, a.range, across);
                value = lerp(road_height, value, fade);
                if across < 4.5 {
                    value = (value as f64 - 0.1_f32 as f64) as f32;
                    close = Some(value);
                }
            }
        }
        close.unwrap_or(value)
    }
}

/// Reuses border storage; one update samples at most 4096 vertices.
#[derive(Default)]
pub struct TerrainSurfaceJob {
    border: Vec<[f32; 3]>,
    border_normals: Vec<[f32; 3]>,
    cursor: usize,
    complete: bool,
}

impl TerrainSurfaceJob {
    pub fn prepared_vertices(&self) -> usize {
        self.cursor
    }

    pub fn retained_bytes(&self) -> usize {
        (self.border.capacity() + self.border_normals.capacity()) * 12
    }

    pub fn begin(
        &mut self,
        unit: [i64; 2],
        context: TerrainContext<'_>,
        output: &mut WorldSurface,
    ) {
        let tile = context.tile;
        output.origin = [
            unit[0] as f64 * tile.size as f64,
            tile.y as f64,
            -(unit[1] as f64 * tile.size as f64),
        ];
        output.vertices.clear();
        output.normals.clear();
        output.tangents.clear();
        output.colors.clear();
        output.uv.clear();
        output.triangles.clear();
        output.submeshes.clear();
        let side = tile.resolution as usize + 2;
        self.border.resize(side * side, [0.0; 3]);
        self.border_normals.resize(side * side, [0.0; 3]);
        self.border_normals.fill([0.0; 3]);
        self.cursor = 0;
        self.complete = false;
    }

    pub fn advance(&mut self, context: TerrainContext<'_>, output: &mut WorldSurface) -> bool {
        if self.complete {
            return true;
        }
        let tile = context.tile;
        let n = tile.resolution as usize;
        let side = n + 2;
        let spacing = (tile.size as f64 / (n - 1) as f64) as f32;
        let offset = -(tile.size as f32) * 0.5;
        let end = (self.cursor + 4096).min(self.border.len());
        for index in self.cursor..end {
            let x = (((index % side) as f64 - 1.0) * spacing as f64 + offset as f64) as f32;
            let z = (((index / side) as f64 - 1.0) * spacing as f64 + offset as f64) as f32;
            let point = [
                (x as f64 + output.origin[0] as f32 as f64) as f32,
                (z as f64 + (-output.origin[2]) as f32 as f64) as f32,
            ];
            self.border[index] = [x, context.height(point), z];
        }
        self.cursor = end;
        if end != self.border.len() {
            return false;
        }
        for z in 0..side - 1 {
            for x in 0..side - 1 {
                let a = z * side + x;
                let b = a + side;
                for triangle in [[a, b, b + 1], [b + 1, a + 1, a]] {
                    let [a, b, c] = triangle.map(|i| Vector::from_array(self.border[i]));
                    let cross = (b - a).cross(c - a);
                    let normal = if cross.length() > 0.00001 {
                        cross.normalize()
                    } else {
                        Vector::ZERO
                    };
                    for i in triangle {
                        self.border_normals[i] =
                            (Vector::from_array(self.border_normals[i]) + normal).to_array();
                    }
                }
            }
        }
        output.vertices.reserve(n * n);
        output.normals.reserve(n * n);
        output.tangents.reserve(n * n);
        output.uv.reserve(n * n);
        output.colors.reserve(n * n);
        for z in 0..n {
            for x in 0..n {
                let i = (z + 1) * side + x + 1;
                output.vertices.push(self.border[i]);
                let normal = Vector::from_array(self.border_normals[i]);
                output.normals.push(if normal.length() > 0.00001 {
                    normal.normalize().to_array()
                } else {
                    [0.0; 3]
                });
                if x < n - 1 && z < n - 1 {
                    let a = (z * n + x) as u32;
                    let b = a + n as u32;
                    output.triangles.extend([[a, b, b + 1], [b + 1, a + 1, a]]);
                }
            }
        }
        if tile.tier == 0 {
            for hole in context.holes {
                let local = hole.map(|p| {
                    [
                        (p[0] as f64 - output.origin[0]) as f32,
                        p[1],
                        (p[2] as f64 + output.origin[2]) as f32,
                    ]
                });
                carve_source_hole(output, &local, n, spacing);
            }
        }
        let reciprocal = (1.0 / tile.size as f64) as f32;
        for index in 0..output.vertices.len() {
            let p = output.vertices[index];
            output.uv.push([
                (reciprocal as f64 * p[0] as f64) as f32,
                (reciprocal as f64 * p[2] as f64) as f32,
            ]);
            // Source colors retain the original grid XZ and the final hole-adjusted height.
            let grid = self.border[(index / n + 1) * side + index % n + 1];
            let mut color = context.evaluator.color(
                [
                    (grid[0] as f64 + output.origin[0] as f32 as f64) as f32,
                    (grid[2] as f64 + (-output.origin[2]) as f32 as f64) as f32,
                ],
                p[1],
            );
            if let Some(biome) = context.biome_override {
                let alpha = color[3];
                color = match biome {
                    0 => [1.0, 0.0, 0.0, alpha],
                    1 => [0.0, 0.0, 1.0, alpha],
                    2 => [0.0, 1.0, 0.0, alpha],
                    _ => color,
                };
            }
            output.colors.push(color);
            output.vertices[index][2] = -p[2];
            output.normals[index][2] = -output.normals[index][2];
            // Unity D3D11 missing TANGENT0 is RGBA8_UNORM FF0000FF.
            // Reflect the source (1,0,0,1) tangent frame once.
            output.tangents.push([1.0, 0.0, 0.0, -1.0]);
        }
        for triangle in &mut output.triangles {
            triangle.swap(1, 2);
        }
        output.submeshes.push(0..output.triangles.len());
        self.complete = true;
        true
    }
}

pub fn terrain_surface(
    _reference: &WorldReference,
    unit: [i64; 2],
    context: TerrainContext<'_>,
    output: &mut WorldSurface,
) {
    let mut job = TerrainSurfaceJob::default();
    job.begin(unit, context, output);
    while !job.advance(context, output) {}
}

fn horizontal_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    distance_squared([a[0], a[2]], [b[0], b[2]])
}
fn triangle_area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = horizontal_squared(a, b).sqrt();
    let bc = horizontal_squared(b, c).sqrt();
    let ca = horizontal_squared(c, a).sqrt();
    let half = ((ab as f64 + bc as f64 + ca as f64) * 0.5) as f32;
    ((half as f64
        * (half as f64 - ab as f64)
        * (half as f64 - bc as f64)
        * (half as f64 - ca as f64)) as f32)
        .sqrt()
}
fn inside_source_hole(point: [f32; 3], hole: &[[f32; 3]; 4]) -> bool {
    let area = triangle_area(hole[0], hole[1], hole[3]) as f64
        + triangle_area(hole[0], hole[2], hole[3]) as f64;
    let pieces = triangle_area(hole[0], hole[1], point) as f64
        + triangle_area(hole[1], hole[3], point) as f64
        + triangle_area(hole[3], hole[2], point) as f64
        + triangle_area(hole[2], hole[0], point) as f64;
    ((area - pieces) as f32).abs() < 0.2
}
fn carve_source_hole(output: &mut WorldSurface, hole: &[[f32; 3]; 4], n: usize, spacing: f32) {
    let mut nearest = (usize::MAX, f32::MAX);
    for z in 0..n - 1 {
        for x in 0..n - 1 {
            let index = z * n + x;
            let distance = horizontal_squared(output.vertices[index], hole[2]);
            if distance < nearest.1 {
                nearest = (index, distance);
            }
        }
    }
    if nearest.1 < (spacing as f64 * 3.0) as f32 {
        let index = nearest.0;
        output.vertices[index] = hole[2];
        output.vertices[index + 1] = hole[3];
        output.vertices[index + n] = hole[0];
        output.vertices[index + n + 1] = hole[1];
    }
    let squared = (spacing as f64 * spacing as f64) as f32;
    for point in &mut output.vertices {
        let inside = inside_source_hole(*point, hole);
        let mut selected = None;
        let mut closest = f32::MAX;
        for (index, corner) in hole.iter().enumerate() {
            let distance = horizontal_squared(*point, *corner);
            if distance < closest && (inside || distance < squared) {
                selected = Some(index);
                closest = distance;
            }
        }
        if let Some(index) = selected {
            *point = hole[index];
        }
    }
    output.triangles.retain(|triangle| {
        let center = std::array::from_fn(|axis| {
            let ab = (output.vertices[triangle[0] as usize][axis] as f64
                + output.vertices[triangle[1] as usize][axis] as f64) as f32;
            let sum = (ab as f64 + output.vertices[triangle[2] as usize][axis] as f64) as f32;
            (sum as f64 / 3.0) as f32
        });
        !inside_source_hole(center, hole)
    });
}
