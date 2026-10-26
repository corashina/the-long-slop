//! Flat prop entities. An instance spawns entities only for dynamic glTF nodes.
//! Renderers of static nodes attach to the nearest dynamic ancestor with a baked matrix.
use bevy::math::{DMat4, Mat4, Vec3};
use bevy::transform::components::Transform;

/// Per prototype glTF node plan. Built once per loaded prototype.
pub struct NodePlan {
    /// glTF parent of each glTF node.
    pub parents: Vec<Option<u32>>,
    /// Prototype node of each glTF node.
    pub sources: Vec<Option<u32>>,
    /// glTF nodes, each parent before its children.
    pub order: Vec<u32>,
    has_children: Vec<bool>,
    /// Runtime code writes the transform or visibility of the node.
    dynamic: Vec<bool>,
}
/// The flat layout of one instance.
pub struct FlatNodes {
    /// The instance spawns an entity for the node.
    pub spawned: Vec<bool>,
    /// Nearest spawned ancestor of each node. `None` is the instance root.
    pub anchors: Vec<Option<u32>>,
    /// Static matrix from the anchor to the node.
    pub rest: Vec<Mat4>,
    /// Matrix from the instance root to the node.
    pub chain: Vec<Mat4>,
}
impl NodePlan {
    /// `children` holds the glTF children of each node. `dynamic` holds the dynamic glTF nodes.
    pub fn new(children: &[&[u32]], sources: Vec<Option<u32>>, dynamic: Vec<bool>) -> Self {
        let count = children.len();
        let mut parents = vec![None; count];
        for (parent, list) in children.iter().enumerate() {
            for &child in *list {
                parents[child as usize] = Some(parent as u32);
            }
        }
        let mut order = Vec::with_capacity(count);
        order.extend((0..count as u32).filter(|&n| parents[n as usize].is_none()));
        let mut next = 0;
        while next < order.len() {
            order.extend_from_slice(children[order[next] as usize]);
            next += 1;
        }
        Self {
            parents,
            sources,
            order,
            has_children: children.iter().map(|c| !c.is_empty()).collect(),
            dynamic,
        }
    }
    /// Places the nodes of one instance from its local matrices.
    /// A node with children and a non-uniform scale also spawns an entity.
    /// So each baked matrix stays a pure scale, rotation and translation.
    pub fn flatten(&self, locals: &[Mat4]) -> FlatNodes {
        let count = locals.len();
        let mut flat = FlatNodes {
            spawned: vec![false; count],
            anchors: vec![None; count],
            rest: vec![Mat4::IDENTITY; count],
            chain: vec![Mat4::IDENTITY; count],
        };
        for &node in &self.order {
            let node = node as usize;
            let local = locals[node];
            flat.spawned[node] = self.dynamic[node] || (self.has_children[node] && !uniform(local));
            match self.parents[node] {
                None => {
                    flat.rest[node] = local;
                    flat.chain[node] = local;
                }
                Some(parent) => {
                    let parent = parent as usize;
                    flat.chain[node] = flat.chain[parent] * local;
                    if flat.spawned[parent] {
                        flat.anchors[node] = Some(parent as u32);
                        flat.rest[node] = local;
                    } else {
                        flat.anchors[node] = flat.anchors[parent];
                        flat.rest[node] = flat.rest[parent] * local;
                    }
                }
            }
        }
        flat
    }
}
impl FlatNodes {
    /// A static node under the instance root. It never moves after spawn.
    pub fn on_root(&self, node: usize) -> bool {
        !self.spawned[node] && self.anchors[node].is_none()
    }
}
fn uniform(local: Mat4) -> bool {
    let scale = local.to_scale_rotation_translation().0;
    (scale - Vec3::splat(scale.x)).abs().max_element() <= 1e-5 * scale.abs().max_element()
}
/// Transform of a spawned node from its pose relative to its glTF parent.
/// `parent` holds the anchor and glTF parent world matrices. It is `None` when the glTF parent is the anchor.
pub fn anchor_local(parent: Option<(DMat4, DMat4)>, local: Transform) -> Transform {
    match parent {
        Some((anchor, parent)) => {
            Transform::from_matrix((anchor.inverse() * parent).as_mat4() * local.to_matrix())
        }
        None => local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::{Quat, Vec3};

    // World matrix of every node through the spawned entities, as transform propagation gives it.
    fn propagated(flat: &FlatNodes, locals: &[Mat4], root: Mat4) -> Vec<Mat4> {
        let count = locals.len();
        let mut entity = vec![None; count];
        let mut done = 0;
        while done < count {
            done = 0;
            for node in 0..count {
                let anchor = match flat.anchors[node] {
                    None => Some(root),
                    Some(a) => entity[a as usize],
                };
                if let (Some(anchor), true) = (anchor, flat.spawned[node]) {
                    entity[node] = Some(anchor * Transform::from_matrix(locals[node]).to_matrix());
                }
                done += usize::from(!flat.spawned[node] || entity[node].is_some());
            }
        }
        (0..count)
            .map(|node| {
                if flat.spawned[node] {
                    entity[node].unwrap()
                } else {
                    let anchor = flat.anchors[node].map_or(root, |a| entity[a as usize].unwrap());
                    anchor * Transform::from_matrix(flat.rest[node]).to_matrix()
                }
            })
            .collect()
    }
    fn close(a: Mat4, b: Mat4) -> bool {
        a.abs_diff_eq(b, 1e-4)
    }

    #[test]
    fn renderer_worlds_match_gltf_composition() {
        // 0 root, 1 body, 2 cap under body, 3 lid under cap, 4 needle, 5 static leaf,
        // 6 static parent under the needle, 7 leaf under 6, 8 non-uniform parent, 9 its leaf.
        let children: [&[u32]; 10] = [
            &[1, 4, 5, 8],
            &[2],
            &[3],
            &[],
            &[6],
            &[],
            &[7],
            &[],
            &[9],
            &[],
        ];
        let mut dynamic = vec![false; 10];
        for node in [1, 2, 4] {
            dynamic[node] = true;
        }
        let plan = NodePlan::new(&children, vec![None; 10], dynamic);
        let trs = |s: [f32; 3], r: Quat, t: [f32; 3]| {
            Mat4::from_scale_rotation_translation(Vec3::from_array(s), r, Vec3::from_array(t))
        };
        let mut locals = vec![
            trs([2.0; 3], Quat::from_rotation_y(0.3), [1.0, 2.0, 3.0]),
            trs([1.0, 1.0, 1.0], Quat::from_rotation_x(0.7), [0.0, 1.0, 0.0]),
            trs([0.5; 3], Quat::from_rotation_z(-0.4), [0.2, 0.0, 0.1]),
            trs([1.0, 3.0, 0.5], Quat::from_rotation_y(1.1), [0.0, 0.3, 0.0]),
            trs([1.0; 3], Quat::from_rotation_z(0.2), [0.5, 0.5, 0.0]),
            trs(
                [4.0, 1.0, 2.0],
                Quat::from_rotation_x(-0.9),
                [0.0, 0.0, 2.0],
            ),
            trs([1.5; 3], Quat::from_rotation_y(0.5), [0.1, 0.0, 0.0]),
            trs([1.0, 2.0, 1.0], Quat::from_rotation_z(0.8), [0.0, 0.2, 0.0]),
            trs([1.0, 2.0, 3.0], Quat::from_rotation_x(0.4), [3.0, 0.0, 0.0]),
            trs([1.0; 3], Quat::from_rotation_y(0.6), [0.0, 1.0, 0.0]),
        ];
        let flat = plan.flatten(&locals);
        assert_eq!(
            flat.spawned,
            [
                false, true, true, false, true, false, false, false, true, false
            ]
        );
        assert_eq!(flat.anchors[7], Some(4));
        let root = trs(
            [1.0, 1.5, 0.8],
            Quat::from_rotation_y(-0.2),
            [10.0, 0.0, -5.0],
        );
        let worlds = propagated(&flat, &flat.rest, root);
        for node in 0..10 {
            assert!(close(worlds[node], root * flat.chain[node]), "node {node}");
        }
        // Move the body and the needle as the sync writers do.
        let moved = [
            (
                1,
                trs([1.0; 3], Quat::from_rotation_z(1.3), [0.4, 1.2, -0.3]),
            ),
            (
                4,
                trs([1.0; 3], Quat::from_rotation_z(-0.6), [0.5, 0.5, 0.0]),
            ),
        ];
        let mut spawned_locals = flat.rest.clone();
        for (node, local) in moved {
            locals[node] = local;
            let anchor = root.as_dmat4()
                * flat.anchors[node]
                    .map_or(Mat4::IDENTITY, |a| flat.chain[a as usize])
                    .as_dmat4();
            let parent = plan.parents[node]
                .filter(|&p| Some(p) != flat.anchors[node])
                .map(|p| (anchor, root.as_dmat4() * flat.chain[p as usize].as_dmat4()));
            spawned_locals[node] = anchor_local(parent, Transform::from_matrix(local)).to_matrix();
        }
        let after = plan.flatten(&locals);
        let worlds = propagated(&flat, &spawned_locals, root);
        for node in 0..10 {
            assert!(
                close(worlds[node], root * after.chain[node]),
                "moved node {node}"
            );
        }
    }
}
