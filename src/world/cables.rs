//! Source power cable math: rope aim, pole selection and cable recipes.
use super::RoadSegment;
use super::source_lights::{RoadPole, object_node, record, road_controller, transform_node};
use super::spawn_rules::{array, link, optional, value_link};
use crate::reference::{
    ReferenceError, SourceId, WorldPrototypeReference, WorldReference, WorldValue,
};
use rapier3d::math::{Rotation, Vector};

/// Source `roadGenScript.CheckCableConnections` range in meters.
pub(crate) const CABLE_RANGE: f64 = 200.;

/// Local pose of one rope node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodePose {
    pub node: SourceId,
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// Source `ropeScript.Connect`: LookAt the target, localScale z = distance / 100.
/// Port forward is -Z and up is +Y. Returns the local rotation and scale z.
pub(crate) fn aim(rope: [f64; 3], parent_rotation: Rotation, target: [f64; 3]) -> ([f32; 4], f32) {
    let delta = [0, 1, 2].map(|i| target[i] - rope[i]);
    let distance = delta.iter().map(|d| d * d).sum::<f64>().sqrt();
    if distance == 0. {
        return (Rotation::IDENTITY.to_array(), 0.);
    }
    let forward = Vector::from_array(delta.map(|d| (d / distance) as f32));
    let right = forward.cross(Vector::Y);
    // Straight up or down has no right axis. Use the shortest arc.
    let look = if right.length_squared() < 1e-12 {
        Rotation::from_rotation_arc(Vector::NEG_Z, forward)
    } else {
        let right = right.normalize();
        Rotation::from_rotation_axes(right, right.cross(forward), -forward)
    };
    let local = (parent_rotation.inverse() * look).normalize();
    (local.to_array(), (distance / 100.) as f32)
}

/// Source `roadGenScript.CheckCableConnections`.
/// Keep the pole with the nearest target 0 under 200 m. Then pick its nearest target.
pub(crate) fn nearest_target<'a>(
    rope0: [f64; 3],
    poles: impl Iterator<Item = (u32, &'a [[f64; 3]])>,
) -> Option<(u32, usize)> {
    let distance = |target: &[f64; 3]| {
        (0..3)
            .map(|i| (target[i] - rope0[i]).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let mut best: Option<(u32, &[[f64; 3]], f64)> = None;
    for (pole, targets) in poles {
        let Some(first) = targets.first() else {
            continue;
        };
        let d = distance(first);
        if d < best.map_or(CABLE_RANGE, |(_, _, best)| best) {
            best = Some((pole, targets, d));
        }
    }
    let (pole, targets, _) = best?;
    let mut index = 0;
    for (i, target) in targets.iter().enumerate().skip(1) {
        if distance(target) < distance(&targets[index]) {
            index = i;
        }
    }
    Some((pole, index))
}

/// One road rope in its pole frame.
struct Rope {
    node: usize,
    position: Vector,
    parent: Rotation,
    scale: [f32; 2],
}
/// Source `ropeScript` of one road prototype, in its pole frame.
struct RoadCable {
    pole: RoadPole,
    ropes: Vec<Rope>,
    targets: Vec<Vector>,
}
impl RoadCable {
    fn new(
        prototype: &WorldPrototypeReference,
        pole: RoadPole,
        ropes: &[usize],
        targets: &[usize],
    ) -> Result<Self, ReferenceError> {
        let ropes = ropes
            .iter()
            .map(|&node| {
                let parent = prototype.nodes[node]
                    .parent
                    .ok_or_else(|| ReferenceError("Source rope is outside its pole".into()))?;
                let scale = prototype.nodes[node].transform.scale;
                Ok(Rope {
                    node,
                    position: pole.local(prototype, node)?.translation,
                    parent: pole.local(prototype, parent as usize)?.rotation,
                    scale: [scale[0], scale[1]],
                })
            })
            .collect::<Result<_, ReferenceError>>()?;
        let targets = targets
            .iter()
            .map(|&node| Ok(pole.local(prototype, node)?.translation))
            .collect::<Result<_, ReferenceError>>()?;
        Ok(Self {
            pole,
            ropes,
            targets,
        })
    }
}
/// Source `buildingscript.connectToPole`: the inactive rope root and its ropes.
pub(crate) struct BuildingCable {
    pub prototype: SourceId,
    pub root: usize,
    pub ropes: Vec<usize>,
}
/// Road and building cables, resolved once per stream.
pub(crate) struct CableRecipes {
    roads: Vec<RoadCable>,
    buildings: Vec<BuildingCable>,
    seed: i32,
}
fn nodes(
    prototype: &WorldPrototypeReference,
    links: &[WorldValue],
) -> Result<Vec<usize>, ReferenceError> {
    links
        .iter()
        .map(|v| transform_node(prototype, value_link(v)?))
        .collect()
}
fn prototype<'a>(
    world: &'a WorldReference,
    source: &SourceId,
) -> Result<&'a WorldPrototypeReference, ReferenceError> {
    world
        .prototypes
        .iter()
        .find(|p| &p.source == source)
        .ok_or_else(|| ReferenceError("Missing cable prototype".into()))
}
impl CableRecipes {
    pub fn new(world: &WorldReference, seed: i32) -> Result<Self, ReferenceError> {
        let mut roads = Vec::with_capacity(world.road.prototypes.len());
        for source in &world.road.prototypes {
            let prototype = prototype(world, source)?;
            let controller = road_controller(world, source)?;
            let rope = &record(world, link(&controller.fields, "rope")?)?.fields;
            roads.push(RoadCable::new(
                prototype,
                RoadPole::new(prototype, controller)?,
                &nodes(prototype, array(rope, "ropes")?)?,
                &nodes(prototype, array(rope, "ropeTargets")?)?,
            )?);
        }
        let mut buildings: Vec<BuildingCable> = Vec::new();
        for kind in &world.buildings.types {
            if buildings.iter().any(|b| b.prototype == kind.prototype) {
                continue;
            }
            let prototype = prototype(world, &kind.prototype)?;
            let owner = link(&record(world, &prototype.source)?.fields, "m_GameObject")?;
            let Some(script) = world.records.iter().find(|r| {
                r.class_name == "buildingscript"
                    && matches!(r.fields.get("m_GameObject"), Some(WorldValue::Link(go)) if go == owner)
            }) else {
                continue;
            };
            let Some(rope) = optional(&script.fields, "connectToPole")? else {
                continue;
            };
            let rope = &record(world, rope)?.fields;
            buildings.push(BuildingCable {
                prototype: prototype.source.clone(),
                root: object_node(world, prototype, link(rope, "m_GameObject")?)?,
                ropes: nodes(prototype, array(rope, "ropes")?)?,
            });
        }
        Ok(Self {
            roads,
            buildings,
            seed,
        })
    }
    pub fn building(&self, prototype: &SourceId) -> Option<&BuildingCable> {
        self.buildings.iter().find(|b| &b.prototype == prototype)
    }
    /// Source `roadGenScript.PlaceOneRoad`: rope k of `segment` aims at target k of `next`.
    pub fn road(
        &self,
        world: &WorldReference,
        segment: &RoadSegment,
        next: &RoadSegment,
        output: &mut Vec<NodePose>,
    ) {
        let find = |s: &RoadSegment| {
            let source = &world.road.prototypes[s.biome as usize];
            world
                .prototypes
                .iter()
                .find(|p| &p.source == source)
                .unwrap()
        };
        self.road_between([find(segment), find(next)], segment, next, output);
    }
    fn road_between(
        &self,
        prototypes: [&WorldPrototypeReference; 2],
        segment: &RoadSegment,
        next: &RoadSegment,
        output: &mut Vec<NodePose>,
    ) {
        output.clear();
        let cable = &self.roads[segment.biome as usize];
        let target = &self.roads[next.biome as usize];
        let from = cable.pole.frame(prototypes[0], segment, self.seed);
        let to = target.pole.frame(prototypes[1], next, self.seed);
        for (rope, point) in cable.ropes.iter().zip(&target.targets) {
            let (rotation, scale) = aim(
                from.point(rope.position),
                from.rotation * rope.parent,
                to.point(*point),
            );
            output.push(NodePose {
                node: prototypes[0].nodes[rope.node].source.clone(),
                rotation,
                scale: [rope.scale[0], rope.scale[1], scale],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_forward(parent: Rotation, local: [f32; 4]) -> Vector {
        (parent * Rotation::from_array(local) * Vector::NEG_Z).normalize()
    }

    #[test]
    fn aim_points_forward_at_target() {
        let (rotation, scale) = aim([0.; 3], Rotation::IDENTITY, [3., 4., -12.]);
        assert!((scale - 0.13).abs() < 1e-6);
        let expected = Vector::new(3., 4., -12.) / 13.;
        assert!((world_forward(Rotation::IDENTITY, rotation) - expected).length() < 1e-5);
    }

    #[test]
    fn aim_removes_parent_rotation() {
        let parent = Rotation::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let (rotation, _) = aim([1., 2., 3.], parent, [4., 6., -9.]);
        let expected = Vector::new(3., 4., -12.) / 13.;
        assert!((world_forward(parent, rotation) - expected).length() < 1e-5);
    }

    #[test]
    fn nearest_target_follows_source_rule() {
        let far = [[201., 0., 0.]];
        let near = [[199., 0., 0.]];
        assert_eq!(nearest_target([0.; 3], [(1, &far[..])].into_iter()), None);
        assert_eq!(
            nearest_target([0.; 3], [(1, &near[..])].into_iter()),
            Some((1, 0))
        );
        let first = [[50., 0., 0.]];
        let second = [[20., 0., 0.], [10., 0., 0.], [15., 0., 0.]];
        let poles = [(7, &first[..]), (8, &second[..])];
        assert_eq!(nearest_target([0.; 3], poles.into_iter()), Some((8, 1)));
    }

    #[test]
    fn road_ropes_point_at_next_targets_on_tilted_poles() {
        use crate::reference::{AssetId, Transform, WorldPrototypeNode, WorldPrototypeRole};
        use bevy::math::{Affine3A, Quat, Vec3};
        let id = |path_id| SourceId {
            file: "level1".into(),
            path_id,
        };
        let yaw = |degrees: f32| Rotation::from_rotation_y(degrees.to_radians()).to_array();
        let node = |path_id, parent, translation, rotation, scale| WorldPrototypeNode {
            source: id(path_id),
            name: String::new(),
            parent,
            active: true,
            renderer_enabled: true,
            layer: 0,
            transform: Transform {
                translation,
                rotation,
                scale,
            },
            gltf_node: 0,
        };
        let identity = [0., 0., 0., 1.];
        let prototype = WorldPrototypeReference {
            source: id(1),
            role: WorldPrototypeRole::Road,
            asset: AssetId(0),
            nodes: vec![
                node(1, None, [0.; 3], identity, [1.; 3]),
                node(2, Some(0), [2., 0., -1.], yaw(20.), [1.2; 3]),
                node(3, Some(1), [0., 5., 0.], yaw(30.), [1.; 3]),
                node(4, Some(2), [-1., 0., 0.], identity, [1.; 3]),
                node(5, Some(2), [1., 0., 0.], identity, [1.; 3]),
                node(6, Some(2), [-1., -0.2, 0.3], identity, [1.; 3]),
                node(7, Some(2), [1., -0.2, 0.3], identity, [1.; 3]),
            ],
            controls: Vec::new(),
            excluded_actions: Vec::new(),
            deferred_materials: Vec::new(),
            skins: Vec::new(),
            bodies: Vec::new(),
            colliders: Vec::new(),
            prefab_pose: Transform {
                translation: [0.; 3],
                rotation: identity,
                scale: [1.; 3],
            },
            interiors: Vec::new(),
        };
        let pole = RoadPole {
            node: 1,
            scale: Vector::splat(1.2),
            chances: [1., 0.],
            angles: [[15, 10, 15]; 2],
        };
        let seed = 7;
        let segment = |index, source_position| RoadSegment {
            index,
            biome: 0,
            source_position,
            source_origin: [0.; 3],
            source_rotations: [[0.; 3]; super::super::road::ROAD_CONTROLS],
            source_coords: [[0.; 3]; super::super::road::ROAD_CONTROLS],
            bounds: super::super::WorldBounds {
                min: [0.; 3],
                max: [0.; 3],
            },
        };
        let (first, second) = (segment(4, [0.; 3]), segment(5, [3., 1., 20.]));
        for s in [&first, &second] {
            let angles = super::super::lamps::source_angles(
                seed as u64,
                super::super::lamps::RoadLampId(s.index as i64),
                pole.chances,
                pole.angles,
            );
            assert_ne!(angles, [0; 3], "the test pole must tilt");
        }
        let recipes = CableRecipes {
            roads: vec![RoadCable::new(&prototype, pole.clone(), &[3, 4], &[5, 6]).unwrap()],
            buildings: Vec::new(),
            seed,
        };
        let mut poses = Vec::new();
        recipes.road_between([&prototype; 2], &first, &second, &mut poses);
        assert_eq!(poses.len(), 2);
        // Independent Unity TRS chain from the tilted pole.
        let affine = |t: &Transform| {
            Affine3A::from_scale_rotation_translation(
                Vec3::from_array(t.scale),
                Quat::from_array(t.rotation),
                Vec3::from_array(t.translation),
            )
        };
        let tilted = |s: &RoadSegment| {
            let frame = pole.frame(&prototype, s, seed);
            let p = frame.position.map(|v| v as f32);
            Affine3A::from_scale_rotation_translation(
                Vec3::splat(1.2),
                Quat::from_array(frame.rotation.to_array()),
                Vec3::from_array(p),
            ) * affine(&prototype.nodes[2].transform)
        };
        let (root, next_root) = (tilted(&first), tilted(&second));
        let (_, parent, _) = root.to_scale_rotation_translation();
        for (k, pose) in poses.iter().enumerate() {
            assert_eq!(pose.node, id(4 + k as i64));
            let rope = root.transform_point3(Vec3::from_array(
                prototype.nodes[3 + k].transform.translation,
            ));
            let target = next_root.transform_point3(Vec3::from_array(
                prototype.nodes[5 + k].transform.translation,
            ));
            let delta = target - rope;
            let forward = parent * Quat::from_array(pose.rotation) * Vec3::NEG_Z;
            assert!((forward - delta.normalize()).length() < 1e-4);
            assert!((pose.scale[2] - delta.length() / 100.).abs() < 1e-4);
            assert_eq!(&pose.scale[..2], &[1., 1.]);
        }
    }
}
