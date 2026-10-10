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
