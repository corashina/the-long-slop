//! V04: a world item seat holds the player on the item body.
use super::*;
use crate::player::SeatOwner;
use crate::vehicle::to_rapier;
use rapier3d::math::Vector;

#[test]
#[ignore = "requires the installed source export"]
fn item_seat_follows_item_body() {
    let reference = reference();
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    let prototype = reference
        .world
        .prototypes
        .iter()
        .find(|p| p.nodes[0].name == "Szanko")
        .expect("sled prototype");
    let first = super::super::content_seats::prototype_seats(
        &reference.world,
        &run.stream.contents.rules,
        prototype,
    )
    .unwrap()
    .remove(0);
    let mut placement = replay_placement(&prototype.source, 800, [0., 10., 0.]);
    placement.scale = prototype.prefab_pose.scale;
    publish_decoration_fixture(&mut run, placement.clone());
    sync_decoration_fixture(&mut run, &reference);
    sync_decoration_fixture(&mut run, &reference);
    let owner = crate::world::ObjectKey::Generated(placement.id.clone());
    let item = run.area.items.resolve_key(&owner).expect("sled item");
    let body = run.area.items.get(item).unwrap().body.unwrap();
    let seat = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find_map(|(handle, _)| {
            run.area
                .interaction
                .item_seat(handle)
                .filter(|seat| seat.seat.source == first.source)
        })
        .expect("bound first sled seat");
    assert_eq!((seat.item, seat.body, &seat.owner), (item, body, &owner));
    let (record, pose) = (seat.seat.clone(), seat.pose);
    assert!(
        run.area
            .player
            .enter_item_seat(owner.clone(), body, &record, pose, &mut run.physics)
    );
    assert_eq!(run.area.player.seat_owner(), Some(&SeatOwner::Item(owner)));
    assert_eq!(run.area.player.seat(), None);
    let start = run.physics.rapier().bodies[body].translation();
    run.physics.edit_body(body, |b| {
        b.set_linvel(Vector::new(1., 0., 0.), true);
    });
    for _ in 0..30 {
        run.physics.step_physics();
        let expected = *run.physics.rapier().bodies[body].position() * to_rapier(pose);
        let seated = to_rapier(run.area.player.body_pose(&run.physics));
        assert!((seated.translation - expected.translation).length() < 1e-4);
        assert!(seated.rotation.angle_between(expected.rotation) < 1e-4);
    }
    let moved = (run.physics.rapier().bodies[body].translation() - start).length();
    assert!(moved > 0.01, "sled moved {moved} m");
    let point = run.physics.rapier().bodies[body].translation().to_array();
    assert!(
        run.area
            .interaction
            .pickup_item(
                item,
                point,
                &run.area.player,
                &mut run.area.items,
                &mut run.physics
            )
            .is_err(),
        "occupied item refuses pickup"
    );
    assert!(run.area.player.try_exit(&mut run.physics));
    assert!(!run.area.player.seated());
}
