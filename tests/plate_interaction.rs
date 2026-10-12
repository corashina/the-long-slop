mod common;
use my_summer_slop::interaction::{InteractionBlock, items::ItemLocation};
use my_summer_slop::reference::{AttachmentTargetId, ReferenceManifest};
use my_summer_slop::starting_area::GarageRuntime;
fn fixture() -> ReferenceManifest {
    let mut r = common::reference();
    let rule = r.items.iter().find_map(|i| i.pickup.clone()).unwrap();
    for p in &r.plates.plates {
        let item = r.items.iter_mut().find(|i| i.id == p.item).unwrap();
        let mut rule = rule.clone();
        rule.source = p.pickup_source.clone();
        rule.cant_pickup = false;
        rule.cant_pickup_while_mounted = false;
        rule.cant_pickup_while_seated = false;
        rule.no_rotation = false;
        item.pickup = Some(rule);
    }
    for c in &mut r.colliders {
        if r.plates.plates.iter().any(|p| p.colliders.contains(&c.id))
            || r.plates.targets.iter().any(|t| t.colliders.contains(&c.id))
        {
            c.enabled = true;
        }
    }
    r
}
fn ownership(
    g: &GarageRuntime,
    r: &ReferenceManifest,
) -> Vec<(
    u32,
    Option<rapier3d::prelude::RigidBodyHandle>,
    rapier3d::math::Pose,
    bool,
)> {
    g.physics
        .rapier()
        .colliders
        .iter()
        .filter(|(_, c)| {
            r.plates.plates.iter().any(|p| {
                p.colliders
                    .iter()
                    .any(|id| id.0 == (c.user_data >> 64) as u32)
            })
        })
        .map(|(_, c)| {
            (
                (c.user_data >> 64) as u32,
                c.parent(),
                *c.position(),
                c.is_enabled(),
            )
        })
        .collect()
}
#[test]
fn plate_remove_carry_drop_remount_keeps_identity_and_text() {
    let r = fixture();
    let mut g = GarageRuntime::new(&r).unwrap();
    let p = &r.plates.plates[0];
    let text = g.area.plates.text(p.item).unwrap().to_owned();
    let revision = g.area.plates.revision(p.item);
    g.area
        .interaction
        .remove_plate(
            p.item,
            g.area.items.get(p.item).unwrap().pose.position,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    assert_eq!(g.area.interaction.held(), Some(p.item));
    g.area
        .interaction
        .drop_held(&mut g.area.items, &mut g.physics);
    assert_eq!(
        g.area.items.get(p.item).unwrap().location,
        ItemLocation::World
    );
    let point = g.area.items.get(p.item).unwrap().pose.position;
    g.area
        .interaction
        .pickup_item(
            p.item,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.area
        .interaction
        .mount_plate(p.item, p.startup_target, &mut g.area.items, &mut g.physics)
        .unwrap();
    assert_eq!(g.area.interaction.held(), None);
    assert_eq!(g.area.plates.text(p.item), Some(text.as_str()));
    assert_eq!(g.area.plates.revision(p.item), revision);
}
#[test]
fn failed_plate_acquisition_keeps_attachment() {
    let r = fixture();
    let mut g = GarageRuntime::new(&r).unwrap();
    let p = &r.plates.plates[0];
    let before = g.area.items.get(p.item).unwrap().clone();
    let colliders = ownership(&g, &r);
    let text = g.area.plates.text(p.item).unwrap().to_owned();
    let revision = g.area.plates.revision(p.item);
    assert!(
        g.area
            .interaction
            .remove_plate(
                p.item,
                [f32::NAN; 3],
                &g.area.player,
                &mut g.area.items,
                &mut g.physics
            )
            .is_err()
    );
    assert_eq!(ownership(&g, &r), colliders);
    assert_eq!(g.area.plates.text(p.item), Some(text.as_str()));
    assert_eq!(g.area.plates.revision(p.item), revision);
    let after = g.area.items.get(p.item).unwrap();
    assert_eq!(after.location, before.location);
    assert_eq!(after.body, before.body);
    assert_eq!(after.pose, before.pose);
    assert_eq!(
        g.area.interaction.attachments.target(p.item),
        Some(p.startup_target)
    );
}
#[test]
fn invalid_plate_mount_keeps_carry() {
    let r = fixture();
    let mut g = GarageRuntime::new(&r).unwrap();
    let p = &r.plates.plates[0];
    let point = g.area.items.get(p.item).unwrap().pose.position;
    g.area
        .interaction
        .remove_plate(
            p.item,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    let body = g.area.items.get(p.item).unwrap().body;
    let joints = g.physics.rapier().impulse_joints.len();
    let colliders = ownership(&g, &r);
    let text = g.area.plates.text(p.item).unwrap().to_owned();
    let revision = g.area.plates.revision(p.item);
    assert_eq!(
        g.area.interaction.attachments.can_mount(
            p.item,
            AttachmentTargetId(u32::MAX),
            &g.area.items
        ),
        Err(InteractionBlock::Unavailable)
    );
    assert!(
        g.area
            .interaction
            .mount_plate(
                p.item,
                AttachmentTargetId(u32::MAX),
                &mut g.area.items,
                &mut g.physics
            )
            .is_err()
    );
    assert_eq!(ownership(&g, &r), colliders);
    assert_eq!(g.area.plates.text(p.item), Some(text.as_str()));
    assert_eq!(g.area.plates.revision(p.item), revision);
    assert_eq!(g.area.interaction.held(), Some(p.item));
    assert_eq!(g.area.items.get(p.item).unwrap().body, body);
    assert_eq!(g.physics.rapier().impulse_joints.len(), joints);
}
#[test]
fn plate_target_allows_multiple_source_members() {
    let r = fixture();
    let mut g = GarageRuntime::new(&r).unwrap();
    let p = &r.plates.plates[1];
    let target = r.plates.plates[0].startup_target;
    let point = g.area.items.get(p.item).unwrap().pose.position;
    g.area
        .interaction
        .remove_plate(
            p.item,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.area
        .interaction
        .mount_plate(p.item, target, &mut g.area.items, &mut g.physics)
        .unwrap();
    assert_eq!(g.area.interaction.attachments.members(target).count(), 2);
}
#[test]
#[ignore = "requires installed bumper source records"]
fn plate_follows_removed_bumper_and_rebase() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut g = GarageRuntime::new(&r).unwrap();
    let p = &r.plates.plates[0];
    let owner = r
        .plates
        .targets
        .iter()
        .find(|t| t.id == p.startup_target)
        .unwrap()
        .owner;
    let text = g.area.plates.text(p.item).unwrap().to_owned();
    common::detach(&mut g, owner);
    assert_eq!(
        g.area.items.get(p.item).unwrap().body,
        g.area.items.get(owner).unwrap().body
    );
    let bumper_mass_before =
        g.physics.rapier().bodies[g.area.items.get(owner).unwrap().body.unwrap()].mass();
    let before = my_summer_slop::vehicle::to_rapier(g.area.items.get(p.item).unwrap().pose);
    let point = g.area.items.get(p.item).unwrap().pose.position;
    g.area
        .interaction
        .remove_plate(
            p.item,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    let after = my_summer_slop::vehicle::to_rapier(g.area.items.get(p.item).unwrap().pose);
    assert!(
        (after.translation
            - before.translation
            - before.rotation * rapier3d::math::Vector::from_array(p.remove_target_pos))
        .length()
            < 1e-5
    );
    let plate_body = g.area.items.get(p.item).unwrap().body.unwrap();
    let bumper_body = g.area.items.get(owner).unwrap().body.unwrap();
    assert_ne!(plate_body, bumper_body);
    assert!(
        (bumper_mass_before - g.physics.rapier().bodies[bumper_body].mass() - p.mass_kg).abs()
            < 1e-4
    );
    g.area
        .interaction
        .drop_held(&mut g.area.items, &mut g.physics);
    g.physics.rapier_mut().bodies[bumper_body]
        .set_translation(rapier3d::math::Vector::new(30., 2., 0.), true);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.items.sync_poses(&g.physics);
    assert_eq!(g.area.items.get(p.item).unwrap().body, Some(plate_body));
    assert!(
        (my_summer_slop::vehicle::to_rapier(g.area.items.get(p.item).unwrap().pose).translation
            - after.translation)
            .length()
            < 1e-5
    );
    assert_eq!(g.area.plates.text(p.item), Some(text.as_str()));
    for (_, c) in g.physics.rapier().colliders.iter().filter(|(_, c)| {
        p.colliders
            .iter()
            .any(|id| id.0 == (c.user_data >> 64) as u32)
    }) {
        assert_eq!(c.parent(), Some(plate_body));
    }
}
#[test]
#[ignore = "requires fresh installed source export"]
fn installed_plate_transactions() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut g = GarageRuntime::new(&r).unwrap();
    for p in &r.plates.plates {
        let text = g.area.plates.text(p.item).unwrap().to_owned();
        let point = g.area.items.get(p.item).unwrap().pose.position;
        g.area
            .interaction
            .remove_plate(
                p.item,
                point,
                &g.area.player,
                &mut g.area.items,
                &mut g.physics,
            )
            .unwrap();
        let b = g.area.items.get(p.item).unwrap().body.unwrap();
        assert!((g.physics.rapier().bodies[b].mass() - 1.).abs() < 1e-5);
        assert_eq!(g.physics.rapier().bodies[b].gravity_scale(), 0.);
        g.area
            .interaction
            .drop_held(&mut g.area.items, &mut g.physics);
        assert_eq!(g.physics.rapier().bodies[b].gravity_scale(), 1.);
        let point = g.area.items.get(p.item).unwrap().pose.position;
        g.area
            .interaction
            .pickup_item(
                p.item,
                point,
                &g.area.player,
                &mut g.area.items,
                &mut g.physics,
            )
            .unwrap();
        g.area
            .interaction
            .mount_plate(p.item, p.startup_target, &mut g.area.items, &mut g.physics)
            .unwrap();
        assert_eq!(g.area.plates.text(p.item), Some(text.as_str()));
    }
}
#[test]
fn plate_rebase_preserves_mounted_and_loose_pose() {
    let r = fixture();
    let mut g = GarageRuntime::new(&r).unwrap();
    let loose = r.plates.plates[1].item;
    let point = g.area.items.get(loose).unwrap().pose.position;
    g.area
        .interaction
        .remove_plate(
            loose,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.area
        .interaction
        .drop_held(&mut g.area.items, &mut g.physics);
    let before: Vec<_> = r
        .plates
        .plates
        .iter()
        .map(|p| g.area.items.get(p.item).unwrap().pose)
        .collect();
    let shift = [128., 0., -96.];
    g.physics.rebase(shift);
    g.area.sync_origin(shift.map(f64::from));
    g.area.items.sync_poses(&g.physics);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    for (p, b) in r.plates.plates.iter().zip(before) {
        let a = g.area.items.get(p.item).unwrap().pose;
        for i in 0..3 {
            assert!((a.position[i] + shift[i] - b.position[i]).abs() < 1e-4);
        }
        assert_eq!(a.rotation, b.rotation);
    }
}
#[test]
#[ignore = "requires installed source geometry"]
fn installed_plate_aim_and_release_controls() {
    use my_summer_slop::interaction::{InteractionAction, SlotTarget};
    use my_summer_slop::simulation::{FrameInput, keys};
    use my_summer_slop::vehicle::{from_rapier, to_rapier};
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut g = GarageRuntime::new(&r).unwrap();
    for p in &r.plates.plates {
        let mut view = to_rapier(g.area.items.get(p.item).unwrap().pose);
        view.translation += view.rotation * rapier3d::math::Vector::Z * 0.5;
        let previous_view = g.area.player.view_pose(&g.physics);
        g.area.player.rebase(std::array::from_fn(|i| {
            previous_view.position[i] - view.translation.to_array()[i]
        }));
        let player_body = g.area.player.body_handle();
        g.physics.rapier_mut().bodies[player_body].set_translation(
            view.translation - rapier3d::math::Vector::Y * r.player.standing_height * 0.5,
            true,
        );
        let aim = g
            .area
            .interaction
            .resolve(&g.physics, from_rapier(&view), &g.area.items);
        assert_eq!(aim.grab_target.map(|t| t.item), Some(p.item));
        assert_eq!(aim.grab_target.and_then(|t| t.slot), None);
        let action = g.area.interaction.step(
            FrameInput {
                pressed: keys::GRAB,
                held: keys::GRAB,
                ..Default::default()
            },
            aim,
            &mut g.area.player,
            &mut g.area.items,
            &mut g.physics,
            0.02,
        );
        assert_eq!(action, Some(InteractionAction::PlateRemove(p.item)));
        g.area.interaction.step(
            FrameInput {
                released: keys::GRAB,
                ..Default::default()
            },
            Default::default(),
            &mut g.area.player,
            &mut g.area.items,
            &mut g.physics,
            0.02,
        );
        let aim = g
            .area
            .interaction
            .resolve(&g.physics, from_rapier(&view), &g.area.items);
        assert_eq!(
            aim.installation_target.and_then(|t| t.slot),
            Some(SlotTarget::Attachment(p.startup_target))
        );
        g.area.interaction.step(
            FrameInput {
                pressed: keys::GRAB,
                held: keys::GRAB,
                ..Default::default()
            },
            aim,
            &mut g.area.player,
            &mut g.area.items,
            &mut g.physics,
            0.02,
        );
        let action = g.area.interaction.step(
            FrameInput {
                released: keys::GRAB,
                ..Default::default()
            },
            aim,
            &mut g.area.player,
            &mut g.area.items,
            &mut g.physics,
            0.02,
        );
        assert_eq!(action, Some(InteractionAction::PlateMount(p.item)));
    }
}
#[test]
#[ignore = "requires installed bumper target geometry"]
fn installed_empty_target_keeps_loose_bumper_pickup() {
    use my_summer_slop::vehicle::{from_rapier, to_rapier};
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut g = GarageRuntime::new(&r).unwrap();
    let p = &r.plates.plates[0];
    let owner = r
        .plates
        .targets
        .iter()
        .find(|t| t.id == p.startup_target)
        .unwrap()
        .owner;
    common::detach(&mut g, owner);
    let target_pose = to_rapier(g.area.items.get(p.item).unwrap().pose);
    let point = g.area.items.get(p.item).unwrap().pose.position;
    g.area
        .interaction
        .remove_plate(
            p.item,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.area
        .interaction
        .drop_held(&mut g.area.items, &mut g.physics);
    let b = g.area.items.get(p.item).unwrap().body.unwrap();
    g.physics.rapier_mut().bodies[b].set_translation(
        target_pose.translation + rapier3d::math::Vector::X * 5.,
        true,
    );
    g.physics.refresh_queries();
    let mut view = target_pose;
    view.translation += view.rotation * rapier3d::math::Vector::Z * 0.3;
    let aim = g
        .area
        .interaction
        .resolve(&g.physics, from_rapier(&view), &g.area.items);
    assert_eq!(aim.grab_target.map(|t| t.item), Some(owner));
    assert_eq!(aim.grab_target.and_then(|t| t.slot), None);
    let actions =
        g.area
            .interaction
            .available_actions(&aim, &g.area.player, &g.area.items, &g.physics);
    assert_ne!(
        actions.actions.0 & my_summer_slop::interaction::feedback::ActionMask::PICKUP,
        0
    );
}
