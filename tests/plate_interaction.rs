mod common;
use my_summer_slop::reference::ReferenceManifest;
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
