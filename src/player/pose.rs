//! Retained source frames; full body animation belongs to the later pose task.
use crate::{
    interaction::{
        InteractionState,
        items::{ItemStore, node_pose},
    },
    player::PlayerState,
    reference::{ClothingReference, NodeId, ReferenceError, ReferenceManifest},
    vehicle::{VehiclePose, from_rapier, to_rapier},
};
use std::collections::HashMap;
pub struct PlayerPoseState {
    frames: HashMap<NodeId, (VehiclePose, VehiclePose, bool)>,
    view: VehiclePose,
    body: VehiclePose,
    pub inventory: NodeId,
}
impl PlayerPoseState {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let anchors = &r.clothing.pose.anchors;
        let head = r
            .nodes
            .iter()
            .find(|n| n.id == anchors["RH"].node)
            .and_then(|n| n.parent)
            .ok_or_else(|| ReferenceError("Missing source hand parent".into()))?;
        let camera = to_rapier(node_pose(r, anchors["CamParent"].node)?).inverse();
        let body = to_rapier(node_pose(r, r.clothing.pose.root)?).inverse();
        // fpscontroller level1:80125.TInv points to level1:31175.
        let inventory = r
            .nodes
            .iter()
            .find(|n| n.source.file == "level1" && n.source.path_id == 31175)
            .ok_or_else(|| ReferenceError("Missing source inventory frame".into()))?
            .id;
        let mut frames = HashMap::new();
        for id in std::iter::once(anchors["RH"].node)
            .chain(r.clothing.pose.wearable_anchors.iter().copied())
            .chain([inventory])
        {
            let mut parent = Some(id);
            let mut camera_relative = false;
            while let Some(node) = parent {
                if node == head {
                    camera_relative = true;
                    break;
                }
                parent = r.nodes.iter().find(|n| n.id == node).and_then(|n| n.parent);
            }
            let local = from_rapier(
                &((if camera_relative { camera } else { body }) * to_rapier(node_pose(r, id)?)),
            );
            frames.insert(id, (local, local, camera_relative));
        }
        let identity = VehiclePose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        };
        Ok(Self {
            frames,
            view: identity,
            body: identity,
            inventory,
        })
    }
    pub fn set_frames(&mut self, view: VehiclePose, body: VehiclePose) {
        self.view = view;
        self.body = body;
    }
    pub fn update(
        &mut self,
        _player: &PlayerState,
        _interaction: &InteractionState,
        _items: &ItemStore,
        _reference: &ClothingReference,
        _dt: f32,
    ) {
        let view = to_rapier(self.view);
        let body = to_rapier(self.body);
        for (local, world, camera) in self.frames.values_mut() {
            *world = from_rapier(&((if *camera { view } else { body }) * to_rapier(*local)));
        }
    }
    pub fn inventory_pose(&self, body: VehiclePose) -> VehiclePose {
        from_rapier(&(to_rapier(body) * to_rapier(self.frames[&self.inventory].0)))
    }
    pub fn anchor(&self, node: NodeId) -> Option<VehiclePose> {
        self.frames.get(&node).map(|(_, world, _)| *world)
    }
}
