//! Source lean, independent look, zoom, and head tracking.
use crate::{
    player::PlayerStepContext,
    reference::{PlayerPostureReference, PlayerViewReference},
    simulation::{FrameInput, keys, player_keys},
    vehicle::VehiclePose,
};
use rapier3d::math::{Pose, Rotation, Vector};

#[derive(Clone, Copy, Debug)]
pub struct PlayerView {
    pub head_pose: VehiclePose,
    pub camera_pose: VehiclePose,
    pub fov_degrees: f32,
    pub ray_actions_blocked: bool,
}

pub(super) struct ViewState {
    reference: PlayerViewReference,
    standing: Vector,
    prone: Vector,
    scroll_sensitivity: f32,
    charge: Option<f32>,
    independent: [f32; 2],
    camera_rotation: Rotation,
    seat_entry_rotation: Option<Rotation>,
    return_remaining: f32,
    standing_lean: bool,
    seated_lean: bool,
    looking: bool,
    lean: Vector,
    head: Vector,
    height: f32,
    zoom: f32,
}
impl ViewState {
    pub(super) fn new(
        posture: &PlayerPostureReference,
        reference: &PlayerViewReference,
        scroll_sensitivity: f32,
    ) -> Self {
        let standing = Vector::from_array(posture.standing_head.translation);
        Self {
            reference: reference.clone(),
            standing,
            prone: Vector::from_array(posture.prone_head.translation),
            scroll_sensitivity,
            charge: None,
            independent: [0.; 2],
            camera_rotation: Rotation::IDENTITY,
            seat_entry_rotation: None,
            return_remaining: 0.,
            standing_lean: false,
            seated_lean: false,
            looking: false,
            lean: standing,
            head: standing,
            height: 0.,
            zoom: 1.,
        }
    }
    fn neutral_head(&self, posture: f32) -> Vector {
        self.prone.lerp(self.standing, posture)
    }
    pub(super) fn source_anchor_midpoint(&self, posture: f32, seated: bool) -> Vector {
        let direction_anchor = if seated {
            self.lean
        } else {
            self.neutral_head(posture)
        };
        (direction_anchor + self.prone) * 0.5
    }
    pub(super) fn cancel(&mut self, posture: f32) {
        self.charge = None;
        self.independent = [0.; 2];
        self.camera_rotation = Rotation::IDENTITY;
        self.seat_entry_rotation = None;
        self.return_remaining = 0.;
        self.standing_lean = false;
        self.seated_lean = false;
        self.looking = false;
        self.height = 0.;
        self.zoom = 1.;
        self.lean = self.neutral_head(posture);
        self.head = self.lean;
    }
    /// Returns the ordinary look multiplier when this mouse sample is available.
    pub(super) fn step(
        &mut self,
        input: FrameInput,
        context: PlayerStepContext,
        posture: f32,
        seated: bool,
        head_rotation: Rotation,
        dt: f32,
    ) -> Option<f32> {
        let zooming = input.player_held & player_keys::ZOOM != 0
            || (input.held & keys::TRANSLATE != 0 && !context.picked_up);
        let (target, rate) = if zooming {
            (self.reference.zoom_multiplier, self.reference.zoom_in_rate)
        } else {
            (1., self.reference.zoom_out_rate)
        };
        self.zoom += (target - self.zoom) * (dt * rate).clamp(0., 1.);
        let head_rotation = self.seat_entry_rotation.take().unwrap_or(head_rotation);
        let held = input.player_held & player_keys::LEAN != 0;
        if input.player_pressed & player_keys::LEAN != 0 {
            self.charge = Some(0.);
        }
        if input.player_released & player_keys::LEAN != 0 {
            if !seated
                && self.charge.is_some_and(|charge| {
                    super::source_short_hold(charge, self.reference.tap_seconds)
                })
            {
                self.standing_lean = !self.standing_lean;
            }
            self.charge = None;
        }
        if held && let Some(charge) = &mut self.charge {
            *charge += dt;
        }
        let mouse = input.mouse.map(|v| if v.is_finite() { v } else { 0. });
        let scroll = if input.wheel.is_finite() {
            input.wheel * self.scroll_sensitivity
        } else {
            0.
        };
        self.seated_lean = seated && held;
        self.looking = !seated && (held || self.standing_lean);
        if seated {
            self.independent = [0.; 2];
            self.camera_rotation = Rotation::IDENTITY;
            self.height = 0.;
        } else {
            if context.duck_allowed && scroll > 0. && posture > 0.95 {
                self.height = (self.height + scroll * self.reference.height_scroll_step).clamp(
                    self.reference.height_limits[0],
                    self.reference.height_limits[1],
                );
            }
            if held {
                for (angle, delta) in self.independent.iter_mut().zip(if context.mouse_consumed {
                    [0.; 2]
                } else {
                    mouse
                }) {
                    *angle = (*angle + delta).clamp(
                        -self.reference.free_look_limit_degrees,
                        self.reference.free_look_limit_degrees,
                    );
                }
                self.camera_rotation = Rotation::from_rotation_y(-self.independent[0].to_radians())
                    * Rotation::from_rotation_x(-self.independent[1].to_radians());
                self.return_remaining = self.reference.return_window_seconds;
            } else {
                self.independent = [0.; 2];
                self.return_remaining = (self.return_remaining - dt).max(0.);
                self.camera_rotation = if self.return_remaining > 0. {
                    self.camera_rotation.lerp(
                        Rotation::IDENTITY,
                        (dt * self.reference.return_rate).min(1.),
                    )
                } else {
                    Rotation::IDENTITY
                };
                self.height = 0.;
            }
        }
        let leaning = self.seated_lean || (!seated && self.standing_lean && !held);
        if leaning {
            let forward = if input.held & keys::RUN != 0 {
                Vector::Y
            } else {
                let forward = head_rotation * -Vector::Z;
                Vector::new(forward.x, 0., forward.z).normalize_or_zero()
            };
            // Browser +Y is down. Source LOOKY is positive up.
            self.lean += (forward * -mouse[1]
                + head_rotation * Vector::X * mouse[0]
                + Vector::Y * scroll * self.reference.lean_scroll_multiplier)
                * dt
                * self.reference.lean_sensitivity;
        } else if !seated && !self.standing_lean {
            self.lean = self.lean.lerp(
                self.neutral_head(posture)
                    + if held {
                        Vector::Y * self.height
                    } else {
                        Vector::ZERO
                    },
                (dt * self.reference.return_rate).min(1.),
            );
        }
        if self.standing_lean {
            let center = (self.standing + self.prone + self.neutral_head(posture)) * 0.3333;
            self.lean =
                center + (self.lean - center).clamp_length_max(self.reference.lean_distance);
        } else if seated {
            self.lean = self.standing
                + (self.lean - self.standing).clamp_length_max(self.reference.lean_distance);
        }
        self.head = self
            .head
            .lerp(self.lean, (dt * self.reference.head_lerp).min(1.));
        (!held && !(self.standing_lean && !seated) && !context.mouse_consumed).then_some(
            if zooming {
                self.reference.zoom_multiplier
            } else {
                1.
            },
        )
    }
    pub(super) fn enter(&mut self, head_in_seat: Pose) {
        self.cancel(1.);
        self.head = head_in_seat.translation;
        self.seat_entry_rotation = Some(head_in_seat.rotation);
    }
    pub(super) fn looking(&self) -> bool {
        self.looking
    }
    pub(super) fn recenter(&mut self) {
        self.lean = self.standing;
    }
    pub(super) fn resolve(
        &self,
        body: Pose,
        head_rotation: Rotation,
        seated: bool,
        neutral_posture: Option<f32>,
    ) -> PlayerView {
        let head = Pose::from_parts(
            body.translation
                + body.rotation * neutral_posture.map_or(self.head, |p| self.neutral_head(p)),
            body.rotation
                * if neutral_posture.is_none() {
                    self.seat_entry_rotation.unwrap_or(head_rotation)
                } else {
                    head_rotation
                },
        );
        let camera = Pose::from_parts(
            head.translation,
            head.rotation
                * if neutral_posture.is_some() {
                    Rotation::IDENTITY
                } else {
                    self.camera_rotation
                },
        );
        PlayerView {
            head_pose: super::from_rapier(head),
            camera_pose: super::from_rapier(camera),
            fov_degrees: (if seated {
                self.reference.car_fov_degrees
            } else {
                self.reference.foot_fov_degrees
            }) * if neutral_posture.is_some() {
                1.
            } else {
                self.zoom
            },
            ray_actions_blocked: neutral_posture.is_none() && self.seated_lean,
        }
    }
}
pub(super) fn exit_permission(camera: Pose, seat: Pose, minimum_angle: f32) -> bool {
    let angle = (camera.rotation * -Vector::Z)
        .dot(seat.rotation * -Vector::Z)
        .clamp(-1., 1.)
        .acos()
        .to_degrees();
    angle + 1.0e-5 >= minimum_angle
}

/// Permission uses the camera. This source placement search uses only the head.
pub(super) fn search_exit(
    head: Pose,
    limits: &super::ExitReference,
    padding: f32,
    capsule: &dyn rapier3d::parry::shape::Shape,
    rays: &rapier3d::prelude::QueryPipeline<'_>,
    clearance: &rapier3d::prelude::QueryPipeline<'_>,
) -> Option<Pose> {
    use rapier3d::prelude::Ray;
    let forward = head.rotation * -Vector::Z;
    let max_distance = rays
        .cast_ray_and_get_normal(&Ray::new(head.translation, forward), limits.distance, true)
        .map_or(limits.distance, |(_, hit)| {
            let displacement = forward * hit.time_of_impact + hit.normal * padding;
            Vector::new(displacement.x, 0., displacement.z)
                .length()
                .min(limits.distance)
        });
    let mut vertical = 0.;
    for index in 0..limits.horizontal_steps {
        let distance = index as f32 * limits.horizontal_step;
        if distance >= max_distance {
            break;
        }
        for _ in 0..limits.vertical_steps {
            let point = head.translation + forward * distance + Vector::Y * vertical;
            if clearance
                .intersect_shape(Pose::from_translation(point), capsule)
                .next()
                .is_none()
            {
                let mut candidate = Pose::from_translation(point);
                candidate.rotation = Rotation::from_rotation_y(
                    -super::yaw_from_rotation(head.rotation).to_radians(),
                );
                return Some(candidate);
            }
            vertical += limits.vertical_step;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        player::PlayerStepContext,
        reference::PlayerViewReference,
        simulation::{FrameInput, keys, player_keys},
    };
    use rapier3d::math::Rotation;
    fn reference() -> PlayerViewReference {
        PlayerViewReference {
            head_lerp: 15.,
            lean_sensitivity: 0.15,
            lean_distance: 0.6,
            height_limits: [-0.1, 0.375],
            foot_fov_degrees: 65.,
            car_fov_degrees: 65.,
            tap_seconds: 0.2,
            return_window_seconds: 0.5,
            return_rate: 10.,
            free_look_limit_degrees: 90.,
            zoom_multiplier: 0.4,
            zoom_in_rate: 10.,
            zoom_out_rate: 20.,
            lean_scroll_multiplier: 5.,
            height_scroll_step: 0.05,
        }
    }
    fn state() -> ViewState {
        ViewState::new(&crate::player::posture::test_reference(), &reference(), 1.)
    }
    fn step(state: &mut ViewState, input: FrameInput, seated: bool, dt: f32) -> bool {
        state
            .step(
                input,
                PlayerStepContext {
                    duck_allowed: true,
                    wheel_posture_allowed: true,
                    ..Default::default()
                },
                1.,
                seated,
                Rotation::IDENTITY,
                dt,
            )
            .is_some()
    }
    #[test]
    fn source_anchor_midpoint_uses_posture_and_unsmoothed_seated_lean() {
        let mut view = state();
        for posture in [1., 0.17, 0.] {
            let expected = Vector::Y * (-0.685 + 1.335 * posture * 0.5);
            assert!((view.source_anchor_midpoint(posture, false) - expected).length() < 1e-6);
        }
        step(
            &mut view,
            FrameInput {
                player_held: player_keys::LEAN,
                player_pressed: player_keys::LEAN,
                wheel: 1.,
                ..Default::default()
            },
            true,
            0.02,
        );
        let expected = (view.lean + view.prone) * 0.5;
        assert!((view.source_anchor_midpoint(1., true) - expected).length() < 1e-6);
        assert!((expected - (view.head + view.prone) * 0.5).length() > 0.001);
    }
    #[test]
    fn zoom_reduces_normal_look_but_keeps_q_source_sensitivity() {
        let mut state = state();
        let input = FrameInput {
            player_held: player_keys::ZOOM,
            mouse: [10., 20.],
            ..Default::default()
        };
        assert_eq!(
            state.step(
                input,
                Default::default(),
                1.,
                false,
                Rotation::IDENTITY,
                0.02
            ),
            Some(0.4)
        );
        assert_eq!(
            state.step(
                FrameInput {
                    player_held: player_keys::ZOOM | player_keys::LEAN,
                    ..input
                },
                Default::default(),
                1.,
                false,
                Rotation::IDENTITY,
                0.02
            ),
            None
        );
        assert_eq!(state.independent, [10., 20.]);
    }
    #[test]
    fn dedicated_zoom_while_carrying_and_release_use_source_rates() {
        let mut state = state();
        state.step(
            FrameInput {
                player_held: player_keys::ZOOM,
                ..Default::default()
            },
            PlayerStepContext {
                picked_up: true,
                ..Default::default()
            },
            1.,
            false,
            Rotation::IDENTITY,
            0.02,
        );
        let view = state.resolve(Pose::IDENTITY, Rotation::IDENTITY, false, None);
        assert!((view.fov_degrees - 65. * 0.88).abs() < 1e-5);
        step(&mut state, FrameInput::default(), false, 0.02);
        let view = state.resolve(Pose::IDENTITY, Rotation::IDENTITY, false, None);
        assert!((view.fov_degrees - 65. * 0.928).abs() < 1e-5);
    }
    #[test]
    fn rmb_zoom_requires_empty_picked_up_slot() {
        for (picked_up, multiplier) in [(false, 0.88), (true, 1.)] {
            let mut state = state();
            state.step(
                FrameInput {
                    held: keys::TRANSLATE,
                    ..Default::default()
                },
                PlayerStepContext {
                    picked_up,
                    ..Default::default()
                },
                1.,
                false,
                Rotation::IDENTITY,
                0.02,
            );
            assert!(
                (state
                    .resolve(Pose::IDENTITY, Rotation::IDENTITY, false, None)
                    .fov_degrees
                    - 65. * multiplier)
                    .abs()
                    < 1e-5
            );
        }
    }
    #[test]
    fn neutral_candidate_and_cancel_reset_zoom_but_keep_base_selection() {
        let mut state = state();
        state.reference.car_fov_degrees = 70.;
        step(
            &mut state,
            FrameInput {
                player_held: player_keys::ZOOM,
                ..Default::default()
            },
            true,
            1.,
        );
        assert!(
            (state
                .resolve(Pose::IDENTITY, Rotation::IDENTITY, true, None)
                .fov_degrees
                - 28.)
                .abs()
                < 1e-5
        );
        assert_eq!(
            state
                .resolve(Pose::IDENTITY, Rotation::IDENTITY, true, Some(1.))
                .fov_degrees,
            70.
        );
        assert_eq!(
            state
                .resolve(Pose::IDENTITY, Rotation::IDENTITY, false, Some(0.17))
                .fov_degrees,
            65.
        );
        state.cancel(0.17);
        assert_eq!(
            state
                .resolve(Pose::IDENTITY, Rotation::IDENTITY, false, None)
                .fov_degrees,
            65.
        );
    }
    #[test]
    fn q_tap_boundary_and_cancel() {
        for (duration, lean) in [(0.19, true), (0.2, false)] {
            let mut state = state();
            step(
                &mut state,
                FrameInput {
                    player_pressed: player_keys::LEAN,
                    player_held: player_keys::LEAN,
                    ..Default::default()
                },
                false,
                duration,
            );
            step(
                &mut state,
                FrameInput {
                    player_released: player_keys::LEAN,
                    ..Default::default()
                },
                false,
                0.02,
            );
            assert_eq!(state.standing_lean, lean);
        }
        let mut boundary = state();
        for index in 0..10 {
            step(
                &mut boundary,
                FrameInput {
                    player_pressed: if index == 0 { player_keys::LEAN } else { 0 },
                    player_held: player_keys::LEAN,
                    ..Default::default()
                },
                false,
                crate::simulation::FIXED_DT,
            );
        }
        step(
            &mut boundary,
            FrameInput {
                player_released: player_keys::LEAN,
                ..Default::default()
            },
            false,
            crate::simulation::FIXED_DT,
        );
        assert!(
            !boundary.standing_lean,
            "Ten gameplay steps reach the strict tap boundary"
        );
        let mut state = state();
        step(
            &mut state,
            FrameInput {
                player_pressed: player_keys::LEAN,
                player_held: player_keys::LEAN,
                ..Default::default()
            },
            false,
            0.1,
        );
        state.cancel(1.);
        step(
            &mut state,
            FrameInput {
                player_released: player_keys::LEAN,
                ..Default::default()
            },
            false,
            0.02,
        );
        assert!(!state.standing_lean);
    }
    #[test]
    fn free_look_preserves_body_and_head_anchors() {
        let mut state = state();
        let head_before = state.head;
        assert!(!step(
            &mut state,
            FrameInput {
                player_pressed: player_keys::LEAN,
                player_held: player_keys::LEAN,
                mouse: [200., -200.],
                ..Default::default()
            },
            false,
            0.02
        ));
        assert_eq!(state.independent, [90., -90.]);
        assert_eq!(state.head, head_before);
        assert!(!state.seated_lean);
        step(
            &mut state,
            FrameInput {
                player_held: player_keys::LEAN,
                mouse: [-400., 400.],
                ..Default::default()
            },
            false,
            0.02,
        );
        assert_eq!(state.independent, [-90., 90.]);
        step(
            &mut state,
            FrameInput {
                player_released: player_keys::LEAN,
                ..Default::default()
            },
            false,
            0.02,
        );
        assert_eq!(state.independent, [0.; 2]);
        assert_ne!(state.camera_rotation, Rotation::IDENTITY);
        step(&mut state, FrameInput::default(), false, 0.5);
        assert_eq!(state.camera_rotation, Rotation::IDENTITY);
    }
    #[test]
    fn seated_lean_blocks_ray_actions_and_clamps_source_radius() {
        let mut state = state();
        assert!(!step(
            &mut state,
            FrameInput {
                player_held: player_keys::LEAN,
                mouse: [10000., 10000.],
                ..Default::default()
            },
            true,
            0.02
        ));
        assert!(state.seated_lean);
        assert!((state.lean - state.standing).length() <= 0.600001);
        assert!(state.lean.z > 0., "Browser down is source negative LOOKY");
        step(&mut state, FrameInput::default(), true, 0.02);
        assert!(!state.seated_lean);
    }
    #[test]
    fn seat_root_switch_preserves_head_pose_then_tracks_anchor() {
        let mut state = state();
        let old_body = Pose::from_parts(Vector::new(10., 2., 3.), Rotation::from_rotation_y(0.4));
        let head_rotation = Rotation::from_rotation_x(0.2);
        let old_head = super::super::to_rapier(
            state
                .resolve(old_body, head_rotation, false, None)
                .head_pose,
        );
        let seat = Pose::from_parts(
            Vector::new(11., 2.5, 3.),
            Rotation::from_rotation_z(0.3) * Rotation::from_rotation_y(0.7),
        );
        state.enter(seat.inverse() * old_head);
        let entered = super::super::to_rapier(
            state
                .resolve(seat, Rotation::IDENTITY, true, None)
                .head_pose,
        );
        assert!((entered.translation - old_head.translation).length() < 1e-5);
        assert!(entered.rotation.dot(old_head.rotation).abs() > 0.99999);
        let before = state.head;
        state.step(
            FrameInput::default(),
            Default::default(),
            1.,
            true,
            Rotation::IDENTITY,
            0.02,
        );
        assert!((state.head - before.lerp(state.standing, 0.3)).length() < 1e-6);
        assert!(state.seat_entry_rotation.is_none());
    }
    #[test]
    fn neutral_preview_and_cancel_preserve_posture_and_local_offsets() {
        let mut state = state();
        step(
            &mut state,
            FrameInput {
                player_held: player_keys::LEAN,
                player_pressed: player_keys::LEAN,
                mouse: [40., 20.],
                wheel: 1.,
                ..Default::default()
            },
            false,
            0.1,
        );
        let body = Pose::from_translation(Vector::new(12., 3., -4.));
        let normal = state.resolve(body, Rotation::IDENTITY, false, None);
        assert_ne!(normal.camera_pose.rotation, normal.head_pose.rotation);
        assert!(!normal.ray_actions_blocked);
        let neutral = state.resolve(body, Rotation::IDENTITY, false, Some(0.17));
        state.cancel(0.17);
        state.step(
            FrameInput {
                player_released: player_keys::LEAN,
                ..Default::default()
            },
            PlayerStepContext::default(),
            0.17,
            false,
            Rotation::IDENTITY,
            0.02,
        );
        assert!(!state.standing_lean);
        assert_eq!(
            state
                .resolve(body, Rotation::IDENTITY, false, None)
                .camera_pose,
            neutral.camera_pose
        );
        let shifted = Pose::from_translation(body.translation - Vector::new(100., 0., -20.));
        let shifted_view = state.resolve(shifted, Rotation::IDENTITY, false, None);
        assert_eq!(
            state
                .resolve(body, Rotation::IDENTITY, false, None)
                .camera_pose
                .rotation,
            shifted_view.camera_pose.rotation
        );
        assert!((state.head - state.neutral_head(0.17)).length() < 1e-6);
        state.reference.car_fov_degrees = 70.;
        assert_eq!(
            state
                .resolve(body, Rotation::IDENTITY, false, None)
                .fov_degrees,
            65.
        );
        assert_eq!(
            state
                .resolve(body, Rotation::IDENTITY, true, None)
                .fov_degrees,
            70.
        );
    }

    #[test]
    fn source_wheel_predicates_and_shared_effects() {
        for tuning in [false, true] {
            for picked_up in [false, true] {
                for q in [false, true] {
                    for rmb in [false, true] {
                        for slideable in [false, true] {
                            let input = FrameInput {
                                player_held: if q { player_keys::LEAN } else { 0 },
                                held: if rmb { keys::TRANSLATE } else { 0 },
                                ..Default::default()
                            };
                            let allowed = PlayerStepContext::can_duck(input, tuning, picked_up);
                            assert_eq!(allowed, !tuning && (!picked_up || (q && !rmb)));
                            assert_eq!(
                                allowed && !slideable,
                                !tuning && !slideable && (!picked_up || (q && !rmb))
                            );
                        }
                    }
                }
            }
        }
        let mut shared_mouse = state();
        shared_mouse.standing_lean = true;
        let before = shared_mouse.lean;
        shared_mouse.step(
            FrameInput {
                mouse: [10., 0.],
                ..Default::default()
            },
            PlayerStepContext {
                mouse_consumed: true,
                ..Default::default()
            },
            1.,
            false,
            Rotation::IDENTITY,
            0.02,
        );
        assert!(shared_mouse.lean.x > before.x);
        let mut view = state();
        view.standing_lean = true;
        let mut posture = crate::player::posture::PostureState::new(0.8);
        let input = FrameInput {
            wheel: -1.,
            ..Default::default()
        };
        let before = view.lean;
        view.step(
            input,
            PlayerStepContext {
                duck_allowed: true,
                wheel_posture_allowed: true,
                ..Default::default()
            },
            0.8,
            false,
            Rotation::IDENTITY,
            0.02,
        );
        posture.step(
            input,
            true,
            true,
            &crate::player::posture::test_reference(),
            0.02,
            1.25,
            0.5,
        );
        assert_ne!(view.lean, before);
        assert_eq!(posture.value(), 0.75);
        let mut view = state();
        step(
            &mut view,
            FrameInput {
                wheel: 100.,
                player_held: player_keys::LEAN,
                ..Default::default()
            },
            false,
            0.02,
        );
        assert_eq!(view.height, 0.375);
    }
}

#[cfg(test)]
mod exit_tests {
    use super::*;
    use rapier3d::prelude::{ColliderBuilder, QueryFilter, RigidBodyBuilder};
    #[test]
    fn permission_uses_camera_at_sixty_five_and_search_uses_head() {
        let seat = Pose::IDENTITY;
        let head = Pose::from_translation(Vector::Y);
        let camera = Pose::from_parts(
            head.translation,
            Rotation::from_rotation_y(-65_f32.to_radians()),
        );
        assert!(exit_permission(camera, seat, 65.));
        assert!(!exit_permission(
            Pose::from_parts(
                head.translation,
                Rotation::from_rotation_y(-64.99_f32.to_radians())
            ),
            seat,
            65.
        ));
        assert!(!exit_permission(head, seat, 65.));
        let mut world = rapier3d::prelude::PhysicsWorld::new();
        world.gravity = Vector::ZERO;
        world.insert(
            RigidBodyBuilder::fixed(),
            ColliderBuilder::cuboid(1., 3., 0.25).translation(head.translation),
        );
        world.insert(
            RigidBodyBuilder::fixed(),
            ColliderBuilder::cuboid(0.25, 3., 0.6).translation(Vector::new(0., 1., -0.8)),
        );
        world.step();
        let outside_origin = |_: rapier3d::prelude::ColliderHandle,
                              c: &rapier3d::prelude::Collider| {
            !c.shape().contains_point(c.position(), head.translation)
        };
        let rays =
            world.query_pipeline_with_filter(QueryFilter::default().predicate(&outside_origin));
        let clearance = world.query_pipeline();
        let limits = crate::player::ExitReference {
            distance: 2.,
            minimum_look_angle_degrees: 65.,
            horizontal_step: 0.1,
            vertical_step: 0.1,
            horizontal_steps: 20,
            vertical_steps: 1,
        };
        let capsule = rapier3d::parry::shape::Capsule::new_y(0.1, 0.1);
        assert!(search_exit(head, &limits, 0.35, &capsule, &rays, &clearance).is_none());
        assert!(search_exit(camera, &limits, 0.35, &capsule, &rays, &clearance).is_some());
    }
}
