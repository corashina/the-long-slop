//! `TerrainGenerationJob` tile grid. Tiles use game space relative to their center.

use crate::reference::Bounds;

type Polygon = Vec<[f32; 3]>;

/// One near terrain tile. `key` is the Unity grid cell, scaled by the tile size.
pub struct Tile {
    pub key: [i32; 2],
    pub center: [f64; 3],
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<[u32; 3]>,
}

/// Builds the source grid. `height` takes absolute Unity X and Z and returns game height.
pub fn build(
    key: [i32; 2],
    size: f32,
    resolution: usize,
    height: impl Fn([f32; 2]) -> f32,
    rooms: &[Bounds],
) -> Tile {
    let step = size / (resolution - 1) as f32;
    let corner = -size * 0.5;
    let center = [key[0] as f32 * size, key[1] as f32 * size];
    // One ring outside the tile gives seamless normals.
    let side = resolution + 2;
    let mut heights = Vec::with_capacity(side * side);
    for i in 0..side {
        for j in 0..side {
            let local = [
                corner + (j as f32 - 1.0) * step,
                corner + (i as f32 - 1.0) * step,
            ];
            heights.push(height([center[0] + local[0], center[1] + local[1]]));
        }
    }
    let h = |i: usize, j: usize| heights[i * side + j];
    let mut positions = Vec::with_capacity(resolution * resolution);
    let mut normals = Vec::with_capacity(resolution * resolution);
    for i in 1..=resolution {
        for j in 1..=resolution {
            let local = [
                corner + (j - 1) as f32 * step,
                corner + (i - 1) as f32 * step,
            ];
            positions.push([local[0], h(i, j), -local[1]]);
            let normal = [
                h(i, j - 1) - h(i, j + 1),
                2.0 * step,
                h(i + 1, j) - h(i - 1, j),
            ];
            let length = normal.iter().map(|v| v * v).sum::<f32>().sqrt();
            normals.push(normal.map(|v| v / length));
        }
    }
    let mut indices = Vec::with_capacity((resolution - 1) * (resolution - 1) * 2);
    for i in 0..resolution - 1 {
        for j in 0..resolution - 1 {
            let a = (i * resolution + j) as u32;
            let row = resolution as u32;
            // Source diagonal. Game space mirrors Z, so the winding reverses.
            indices.extend([[a, a + row + 1, a + row], [a, a + 1, a + row + 1]]);
        }
    }
    let mut tile = Tile {
        key,
        center: [center[0] as f64, 0.0, -center[1] as f64],
        positions,
        normals,
        uvs: Vec::new(),
        indices,
    };
    carve_rooms(&mut tile, rooms);
    tile.uvs = tile
        .positions
        .iter()
        .map(|p| [p[0] / size, -p[2] / size])
        .collect();
    tile
}

// Clip the terrain to room edges. A centroid cut leaves gaps beside the source floor.
fn carve_rooms(tile: &mut Tile, rooms: &[Bounds]) {
    let origin = tile.center.map(|value| value as f32);
    let triangles = std::mem::take(&mut tile.indices);
    tile.indices.reserve(triangles.len());
    for triangle in triangles {
        let points = triangle.map(|index| tile.positions[index as usize]);
        if !rooms.iter().any(|room| overlaps(&points, room, origin)) {
            tile.indices.push(triangle);
            continue;
        }
        let normal = tile.normals[triangle[0] as usize];
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
            let first = tile.positions.len() as u32;
            tile.positions.extend_from_slice(&polygon);
            tile.normals.resize(tile.positions.len(), normal);
            for index in 1..polygon.len() - 1 {
                let a = rapier3d::math::Vector::from_array(polygon[index])
                    - rapier3d::math::Vector::from_array(polygon[0]);
                let b = rapier3d::math::Vector::from_array(polygon[index + 1])
                    - rapier3d::math::Vector::from_array(polygon[0]);
                if a.cross(b).length_squared() > 1e-12 {
                    tile.indices
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
