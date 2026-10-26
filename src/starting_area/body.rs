//! Player body inputs from the frame state. All poses go to the Player frame.
use super::GarageRuntime;
use crate::player::body::{BodyInputs, CarriedItem, InHandItem, SeatFrames};
use crate::reference::ReferenceManifest;
use crate::simulation::FIXED_DT;
use crate::sleep::SleepPhase;
use crate::vehicle::{VehiclePose, from_rapier, to_rapier};
use rapier3d::math::Pose;

impl GarageRuntime {
    /// Run after the view, carry, and equipment frames update.
    pub(super) fn step_body(&mut self, reference: &ReferenceManifest) {
        let area = &mut self.area;
        let player = &area.player;
        let body = to_rapier(player.body_pose(&self.physics)).inverse();
        let local = |pose: Pose| from_rapier(&(body * pose));
        let view = player.view(&self.physics);
        let [forward, backward] = player.walk_keys();
        let hand = reference.clothing.pose.anchors["RH"].node;
        // Source carscript moves `LLDrivePos` and `RLDrivePos` to the pedals.
        let pedals = player
            .seat_vehicle()
            .and_then(|key| self.vehicles.get(key))
            .map(|v| {
                let prototype = &reference.world.prototypes[v.prototype];
                v.pedals
                    .legs
                    .map(|n| n.map(|n| crate::world::node_frame(prototype, 0, n as usize)))
            })
            .unwrap_or_default();
        // Source `doubleHand` is not exported yet, so it is false.
        let input = BodyInputs {
            camera: local(to_rapier(view.camera_pose)),
            head: local(to_rapier(view.head_pose)),
            duck: player.posture(),
            sleeping: self.sleep.output().phase == SleepPhase::Sleeping,
            walk_add: player.walk_add(),
            forward,
            backward,
            carried: area
                .interaction
                .carry
                .hand_frames(player, &self.physics)
                .map(|(grab, anchor, mass)| CarriedItem {
                    grab: local(grab),
                    anchor: local(anchor),
                    mass,
                    double_hand: false,
                }),
            in_hand: area
                .interaction
                .equipment
                .item()
                .and_then(|_| area.pose.anchor(hand))
                .map(|hand| InHandItem {
                    hand: local(to_rapier(hand)),
                    double_hand: false,
                }),
            seat: player
                .seat_frames()
                .zip(player.seat_body())
                .map(|(f, owner)| {
                    let seat = body * *self.physics.rapier().bodies[owner].position();
                    let frame =
                        |p: Option<VehiclePose>| p.map(|p| from_rapier(&(seat * to_rapier(p))));
                    SeatFrames {
                        hands: f.hands.map(frame),
                        legs: f.legs.map(frame),
                        driving_hands: f.driving_hands.map(frame),
                        driving_legs: [0, 1]
                            .map(|i| frame(f.driving_legs[i].and(pedals[i]).or(f.driving_legs[i]))),
                        ..*f
                    }
                }),
            point: Some((from_rapier(&player.point().at), player.point().value)),
        };
        area.body.step(&input, FIXED_DT);
    }
}
