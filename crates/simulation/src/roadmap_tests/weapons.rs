//! ROADMAP test pass: Weapons and damage (WD rows).
use crate::audio::AudioKind;
use crate::damage::BreakKind;
use crate::interaction::guns::Strike;
use crate::interaction::items::ItemLocation;
use crate::reference::{GunKind, ItemId, ReferenceManifest};
use crate::simulation::{FIXED_DT, FrameInput, keys};
use crate::starting_area::GarageRuntime;
use crate::vehicle::to_rapier;
use crate::world::stream::tests::kit::*;
use rapier3d::math::{Pose, Vector};

/// Frames from one press to the next.
pub(super) const PRESS_FRAMES: usize = 50;

/// Spawns item `name` (a catalog or world prototype root name) 1.5 m in front of the player and
/// returns its item. A world prototype waits for the stream.
pub(super) fn spawn(run: &mut GarageRuntime, reference: &ReferenceManifest, name: &str) -> ItemId {
    let before: Vec<ItemId> = run.area.items.iter().map(|s| s.id).collect();
    let data = act(
        run,
        reference,
        &format!(r#"{{"action":"spawn","name":"{name}","forward":1.5}}"#),
    );
    if let Some(id) = data[0].as_u64() {
        return ItemId(id as u32);
    }
    let at = run
        .stream
        .origin
        .local(std::array::from_fn(|i| data[i].as_f64().unwrap()));
    load_placements(run, reference);
    let at = Vector::from_array(at);
    run.area
        .items
        .iter()
        .filter(|s| !before.contains(&s.id) && !s.removed && s.body.is_some())
        .min_by(|a, b| {
            let d = |s: &crate::interaction::items::ItemState| {
                (Vector::from_array(s.pose.position) - at).length()
            };
            d(a).total_cmp(&d(b))
        })
        .map(|s| s.id)
        .unwrap_or_else(|| panic!("{name} spawns no item"))
}

/// Runs a sandbox `hand` or `equip` of `name`. Waits while the stream loads it.
pub(super) fn take(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    action: &str,
    name: &str,
) -> ItemId {
    let json = format!(r#"{{"action":"{action}","name":"{name}"}}"#);
    let mut data = act(run, reference, &json);
    for _ in 0..10 {
        if data.get(0).is_some() {
            break;
        }
        load_placements(run, reference);
        // The inventory frame binds the gun runtime.
        input(run, reference, FrameInput::default());
        data = act(run, reference, &json);
    }
    let item = ItemId(data[0].as_u64().unwrap_or_else(|| panic!("{name}: {data}")) as u32);
    seconds(run, reference, 0.5);
    item
}

/// Puts gun `name` in the hand with full ammo.
pub(super) fn equip(run: &mut GarageRuntime, reference: &ReferenceManifest, name: &str) -> ItemId {
    take(run, reference, "equip", name)
}

/// One frame with `input`: the inventory frame, as the driver runs it, then one fixed step.
pub(super) fn input(run: &mut GarageRuntime, reference: &ReferenceManifest, input: FrameInput) {
    let view = run.area.player.view(&run.physics).head_pose;
    let frame_input = run
        .advance_inventory_frame(reference, input, FIXED_DT, view)
        .fixed_input;
    frame_with(run, reference, frame_input);
}

/// One frame of repeated short presses of `key` with `held` keys down.
pub(super) fn press_frame(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    i: usize,
    key: u32,
    held: u32,
) {
    let mut keys = FrameInput {
        held,
        ..Default::default()
    };
    match i % PRESS_FRAMES {
        0 => {
            keys.held |= key;
            keys.pressed = key;
        }
        1 => keys.released = key,
        _ => {}
    }
    input(run, reference, keys);
}

/// Takes the queued one-shot sounds and counts those of `kind`.
fn sounds(run: &mut GarageRuntime, kind: AudioKind) -> usize {
    let audio = run.audio.as_mut().expect("audio state");
    let count = audio
        .survival_events
        .iter()
        .filter(|e| e.kind == kind)
        .count();
    audio.survival_events.clear();
    count
}

/// `Th` in physics space.
fn view(run: &GarageRuntime) -> Pose {
    to_rapier(run.area.player.view(&run.physics).head_pose)
}

/// Moves the body of `item` to `at` with no velocity.
fn place(run: &mut GarageRuntime, item: ItemId, at: Vector) {
    let body = run.area.items.get(item).unwrap().body.unwrap();
    run.physics.edit_body(body, |b| {
        let mut pose = *b.position();
        pose.translation = at;
        b.set_position(pose, true);
        b.set_linvel(Vector::ZERO, true);
        b.set_angvel(Vector::ZERO, true);
    });
    run.physics.refresh_queries();
}

/// Gives `item` the speed of a fall from `height` m, 0.2 m above its pose.
fn drop_from(run: &mut GarageRuntime, item: ItemId, height: f32) {
    let state = run.area.items.get(item).unwrap();
    let (body, p) = (state.body.unwrap(), state.pose.position);
    let speed = (2. * run.physics.rapier().gravity.y.abs() * height).sqrt();
    run.physics.edit_body(body, |b| {
        b.set_translation(Vector::new(p[0], p[1] + 0.2, p[2]), true);
        b.set_linvel(Vector::new(0., -speed, 0.), true);
        b.set_angvel(Vector::ZERO, true);
    });
    run.physics.refresh_queries();
}

/// The breakable target of `item`.
fn target(run: &GarageRuntime, item: ItemId) -> usize {
    run.stream
        .breakables
        .targets()
        .iter()
        .position(|t| t.kind == BreakKind::Item(item))
        .expect("the item binds its breakablescript")
}

fn gone(run: &GarageRuntime, item: ItemId) -> bool {
    run.area.items.get(item).is_none_or(|s| s.removed)
}

/// Collects the spawn serials of item prototype `prototype`. One prototype spawn has one serial.
fn serials(run: &GarageRuntime, prototype: usize, seen: &mut Vec<u32>) {
    for s in run.area.spawned.iter() {
        if s.prototype == prototype && !seen.contains(&s.serial) {
            seen.push(s.serial);
        }
    }
}

/// Spawned copies of item prototype `prototype`.
fn copies(run: &GarageRuntime, prototype: usize) -> usize {
    run.area
        .spawned
        .iter()
        .filter(|s| s.prototype == prototype)
        .count()
}

fn prototype_of(run: &GarageRuntime, item: ItemId) -> usize {
    run.area
        .spawned
        .iter()
        .find(|s| s.id == item)
        .unwrap()
        .prototype
}

/// Source `breakablescript.OnCollisionEnter` and `TryBreak(float)`: the relative speed breaks the
/// item at its health. `gibscript` spawns the gib once.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd1_bottle_vase_and_box_keep_soft_drops_break_on_hard_ones_and_stay_broken() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    let g = run.physics.rapier().gravity.y.abs();
    let mut broken = Vec::new();
    for name in ["beer", "kocsog", "box01"] {
        let item = spawn(&mut run, &reference, name);
        let key = run.area.items.object_key(item);
        seconds(&mut run, &reference, 1.5);
        let t = target(&run, item);
        let rules = &reference.breakables[run.stream.breakables.targets()[t].record];
        let health = rules.health;
        let gib = rules.gib.as_ref().filter(|_| !rules.no_gib).map(|g| {
            reference
                .item_prototypes
                .iter()
                .position(|p| &p.id == g)
                .unwrap()
        });
        // Half the break speed, then twice the break speed.
        drop_from(&mut run, item, (0.5 * health).powi(2) / (2. * g));
        seconds(&mut run, &reference, 2.);
        assert!(
            !gone(&run, item),
            "{name}: a soft drop keeps health {health}"
        );
        let mut seen = Vec::new();
        if let Some(p) = gib {
            serials(&run, p, &mut seen);
        }
        let before = seen.len();
        drop_from(&mut run, item, (2. * health).powi(2) / (2. * g));
        for _ in 0..(3. / FIXED_DT) as usize {
            frame(&mut run, &reference);
            if let Some(p) = gib {
                serials(&run, p, &mut seen);
            }
        }
        assert!(gone(&run, item), "{name}: a hard drop breaks it");
        if gib.is_some() {
            assert_eq!(seen.len(), before + 1, "{name}: one gib spawn");
        }
        broken.push((name, key));
    }
    let mut loaded = save_load(&run, &reference);
    seconds(&mut loaded, &reference, 1.);
    for (name, key) in broken {
        assert!(
            loaded
                .area
                .items
                .resolve_key(&key)
                .is_none_or(|id| gone(&loaded, id)),
            "{name} stays broken"
        );
    }
}

/// The world prototype trigger node of melee weapon `item` in physics space.
fn melee_trigger(run: &GarageRuntime, reference: &ReferenceManifest, item: ItemId) -> Vector {
    let pickup = &run.area.interaction.carry.rule(item).unwrap().source;
    let w = &reference.melee.weapons[run.area.interaction.melee.weapon_for(pickup).unwrap()];
    let prototype = reference
        .world
        .prototypes
        .iter()
        .find(|p| p.source == w.prototype.0)
        .unwrap();
    let node = prototype
        .nodes
        .iter()
        .position(|n| n.source == w.trigger)
        .unwrap();
    let state = run.area.items.get(item).unwrap();
    let body = run.physics.rapier().bodies[state.body.unwrap()].position();
    (*body * to_rapier(state.local_pose) * prototype.root_relative(node).0).translation
}

/// Source `weaponhandlingscript.StartAttack`: front, A, D, S and S with D pick their own
/// trigger. `meleeweaponscript` plays a swing clip and breaks a bottle at the trigger.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd3_each_melee_weapon_swings_with_each_key_and_breaks_a_bottle() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    let keys_and_types = [
        (0, 1),
        (keys::LEFT, 2),
        (keys::RIGHT, 3),
        (keys::BACK, 4),
        (keys::BACK | keys::RIGHT, 5),
    ];
    let names = ["katana", "crowbar", "legycsapo", "leg"];
    // The export must hold a melee weapon record for each one.
    let roots: Vec<String> = reference
        .melee
        .weapons
        .iter()
        .filter_map(|w| {
            let p = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == w.prototype.0)?;
            let root = p.nodes.iter().find(|n| n.parent.is_none())?;
            Some(root.name.to_lowercase())
        })
        .collect();
    let missing: Vec<_> = names
        .iter()
        .filter(|n| !roots.iter().any(|r| r.contains(*n)))
        .collect();
    assert!(
        missing.is_empty(),
        "the export has no melee weapon for {missing:?}. It has {roots:?}"
    );
    for name in names {
        let weapon = melee(&mut run, &reference, name);
        assert_eq!(
            run.area.interaction.equipment.item(),
            Some(weapon),
            "{name} in the hand"
        );
        for (held, attack) in keys_and_types {
            sounds(&mut run, AudioKind::MeleeHit);
            let mut types = Vec::new();
            let mut swings = 0;
            for i in 0..PRESS_FRAMES {
                press_frame(&mut run, &reference, i, keys::PRIMARY, held);
                types.push(run.area.interaction.melee.attack_type());
                swings += sounds(&mut run, AudioKind::MeleeHit);
            }
            let rule = run
                .area
                .interaction
                .carry
                .rule(weapon)
                .map(|r| r.source.clone());
            let bound = rule.and_then(|r| run.area.interaction.melee.weapon_for(&r));
            assert!(
                types.contains(&attack),
                "{name}: keys {held} pick attack {attack}: types {types:?}, melee weapon {bound:?}, hand {:?}",
                run.area.interaction.equipment.item()
            );
            assert!(swings >= 1, "{name}: attack {attack} plays a swing clip");
        }
        let bottle = spawn(&mut run, &reference, "beer");
        seconds(&mut run, &reference, 0.5);
        let t = target(&run, bottle);
        for i in 0..10 * PRESS_FRAMES {
            if run.stream.breakables.targets()[t].destroyed {
                break;
            }
            let at = melee_trigger(&run, &reference, weapon);
            place(&mut run, bottle, at);
            press_frame(&mut run, &reference, i, keys::PRIMARY, 0);
        }
        assert!(
            run.stream.breakables.targets()[t].destroyed,
            "the {name} breaks a bottle"
        );
        // The next weapon replaces it in the hand.
        drop_hand(&mut run);
        seconds(&mut run, &reference, 0.2);
    }
}

/// Drops the item in the hand.
pub(super) fn drop_hand(run: &mut GarageRuntime) {
    let area = &mut run.area;
    area.interaction
        .equipment
        .detach(&mut area.items, &mut run.physics);
    area.sync_inventory();
}

/// Spawns melee weapon `name` (its world prototype root name) at the player, as `melee.rs`
/// does, and puts its pickup item in the hand.
fn melee(run: &mut GarageRuntime, reference: &ReferenceManifest, name: &str) -> ItemId {
    let root = |w: &&crate::reference::MeleeWeaponReference| {
        reference
            .world
            .prototypes
            .iter()
            .find(|p| p.source == w.prototype.0)
            .and_then(|p| p.nodes.iter().find(|n| n.parent.is_none()))
            .map(|n| n.name.to_lowercase())
    };
    let weapons = &reference.melee.weapons;
    let w = weapons
        .iter()
        .find(|w| root(w).is_some_and(|n| n == name))
        .or_else(|| {
            weapons
                .iter()
                .find(|w| root(w).is_some_and(|n| n.contains(name)))
        })
        .unwrap_or_else(|| panic!("melee weapon {name}"));
    let prototype = reference
        .item_prototypes
        .iter()
        .position(|p| p.id == w.prototype)
        .unwrap();
    let pickup = reference.item_prototypes[prototype]
        .pickups
        .iter()
        .position(|p| p.pickup.source == w.pickup)
        .unwrap();
    let pose = run.area.player.body_pose(&run.physics);
    let weapon = crate::starting_area::spawned::spawn_prototype(
        reference,
        &mut run.area,
        &mut run.physics,
        prototype,
        pose,
    )
    .unwrap()[pickup];
    let point = run.area.items.get(weapon).unwrap().pose.position;
    let area = &mut run.area;
    area.interaction
        .pickup_item(
            weapon,
            point,
            &area.player,
            &mut area.items,
            &mut run.physics,
        )
        .unwrap();
    area.sync_inventory();
    assert!(area.equip_carried(&mut run.physics).unwrap());
    seconds(run, reference, 0.5);
    weapon
}

/// Gun root names of the export.
const GUNS: [&str; 6] = [
    "Revolver", "Geppityu", "BBGun", "Shotgun1", "Shotgun2", "Shotgun3",
];

/// Source `weaponscript.Shot`: each trigger press with a full gun plays the shot clip and uses
/// one round, also for the shotguns' pellets. A revolver shot breaks a bottle. Shotgun pellets
/// push a crowbar. The crosshair takes its empty color at 0 rounds.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd6_wd7_each_gun_fires_breaks_a_bottle_pushes_and_shows_its_crosshair() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    for name in GUNS {
        let gun = equip(&mut run, &reference, name);
        let guns = &mut run.area.interaction.guns;
        let r = guns.record(gun).unwrap().clone();
        if let GunKind::BbGun { max_power, .. } = r.kind {
            guns.runtime_mut(gun).unwrap().power = max_power;
        }
        let before = guns.runtime(gun).unwrap().ammo.count;
        assert!(before >= 1., "{name} has rounds");
        input(&mut run, &reference, FrameInput::default());
        let guns = &mut run.area.interaction.guns;
        let crosshair = guns.crosshair().expect("gun crosshair");
        assert_eq!(crosshair.color, r.crosshair.color, "{name}: ready color");
        sounds(&mut run, AudioKind::ItemHit);
        let mut shots = 0;
        for i in 0..PRESS_FRAMES {
            press_frame(&mut run, &reference, i, keys::PRIMARY, 0);
            shots += sounds(&mut run, AudioKind::ItemHit);
        }
        let guns = &run.area.interaction.guns;
        assert_eq!(
            guns.runtime(gun).unwrap().ammo.count,
            before - 1.,
            "{name}: one shot uses one round"
        );
        assert!(shots >= 1, "{name}: the shot plays its clip");
        let guns = &mut run.area.interaction.guns;
        guns.runtime_mut(gun).unwrap().ammo.count = 0.;
        assert_eq!(
            guns.crosshair().unwrap().color,
            r.crosshair.empty_color,
            "{name}: empty color"
        );
        guns.fill(gun);
        if name == "Revolver" {
            let bottle = spawn(&mut run, &reference, "beer");
            seconds(&mut run, &reference, 0.5);
            let t = target(&run, bottle);
            for i in 0..5 * PRESS_FRAMES {
                if run.stream.breakables.targets()[t].destroyed {
                    break;
                }
                let at = view(&run) * Vector::new(0., 0., -1.5);
                place(&mut run, bottle, at);
                press_frame(&mut run, &reference, i, keys::PRIMARY, 0);
            }
            assert!(
                run.stream.breakables.targets()[t].destroyed,
                "a revolver shot breaks a bottle"
            );
        }
        if name == "Shotgun1" {
            let crowbar = spawn(&mut run, &reference, "crowbar");
            let body = run.area.items.get(crowbar).unwrap().body.unwrap();
            let mut pushed = false;
            for i in 0..5 * PRESS_FRAMES {
                let view = view(&run);
                if i % PRESS_FRAMES == 0 {
                    place(&mut run, crowbar, view * Vector::new(0., 0., -1.));
                }
                press_frame(&mut run, &reference, i, keys::PRIMARY, 0);
                let velocity = run.physics.rapier().bodies[body].linvel();
                if i % PRESS_FRAMES == 0 && velocity.dot(view.rotation * -Vector::Z) > 0.1 {
                    pushed = true;
                    break;
                }
            }
            assert!(pushed, "the pellets push the crowbar");
        }
        drop_hand(&mut run);
        seconds(&mut run, &reference, 0.2);
    }
}

/// Source `weaponscript.Csoletores` and `CsoletoresSound`, and `bbgunscript.Felhuzva`: R and the
/// mouse move `FRValue`. The open end adds `powerGain` to a BB gun. `CanShoot` and `Can` follow
/// the FR rules. The saved gun keeps its FR value and power.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd9_breaking_the_barrel_opens_closes_and_powers_the_bb_gun() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    for name in ["BBGun", "Shotgun1", "Shotgun2", "Shotgun3"] {
        let gun = equip(&mut run, &reference, name);
        let r = run.area.interaction.guns.record(gun).unwrap().clone();
        assert!(r.fr.breakable, "{name} breaks open");
        let mut mouse = [0.; 2];
        // Browser +Y is down. Source LOOKY is up.
        let axis = if r.fr.use_x { 0 } else { 1 };
        let sign = if axis == 0 { 1. } else { -1. };
        let mut set_fr = |run: &mut GarageRuntime, to: f32| {
            for _ in 0..200 {
                let fr = run.area.interaction.guns.runtime(gun).unwrap().fr;
                if fr == to {
                    return;
                }
                mouse[axis] = sign * if to > fr { 1. } else { -1. } / r.fr.fr_speed;
                input(
                    run,
                    &reference,
                    FrameInput {
                        held: keys::RELOAD,
                        mouse,
                        ..Default::default()
                    },
                );
            }
            panic!("{name}: FR does not reach {to}");
        };
        set_fr(&mut run, 0.);
        let start = run.area.interaction.guns.runtime(gun).unwrap().power;
        set_fr(&mut run, 1.);
        let rt = run.area.interaction.guns.runtime(gun).unwrap();
        if let GunKind::BbGun {
            power_gain,
            max_power,
            ..
        } = r.kind
        {
            assert_eq!(
                rt.power,
                (start + power_gain).min(max_power),
                "the BB gun gains power"
            );
        }
        let open = crate::interaction::guns::fr_allows(&r.fr.shoot, 1.);
        let closed = crate::interaction::guns::fr_allows(&r.fr.shoot, 0.);
        assert!(closed && !open, "{name}: closed fires, open does not");
        assert!(
            crate::interaction::guns::fr_allows(&r.fr.reload, 1.),
            "{name}: open reloads"
        );
        let saved = (rt.fr, rt.power);
        let mut loaded = save_load(&run, &reference);
        input(&mut loaded, &reference, FrameInput::default());
        let rt = loaded.area.interaction.guns.runtime(gun);
        let rt = rt.or_else(|| {
            let id = loaded.area.interaction.equipment.item()?;
            loaded.area.interaction.guns.runtime(id)
        });
        let rt = rt.expect("the gun loads");
        assert_eq!((rt.fr, rt.power), saved, "{name}: FR and power save");
        set_fr(&mut run, 0.);
        drop_hand(&mut run);
        seconds(&mut run, &reference, 0.2);
    }
}

/// Source `revolverScript.RWheel`, `Update` and `DoRotate`: R and scroll queue turns, each plays
/// `teker`. `Shot` with an empty selected chamber fires no round. A saved cylinder loads.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd10_the_revolver_cylinder_turns_and_an_empty_chamber_clicks() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    let gun = equip(&mut run, &reference, "Revolver");
    let turns = run.area.interaction.guns.runtime(gun).unwrap().cylinder;
    sounds(&mut run, AudioKind::ItemHit);
    for wheel in [-1., -1., 1.] {
        input(
            &mut run,
            &reference,
            FrameInput {
                held: keys::RELOAD,
                wheel,
                ..Default::default()
            },
        );
    }
    let mut clips = 0;
    for _ in 0..100 {
        input(&mut run, &reference, FrameInput::default());
        clips += sounds(&mut run, AudioKind::ItemHit);
    }
    let rt = run.area.interaction.guns.runtime(gun).unwrap();
    assert_eq!(rt.cylinder, turns + 1, "two turns forward and one back");
    assert!(clips >= 1, "a turn plays teker");
    // All chambers empty: the trigger clicks and no round goes.
    let rt = run.area.interaction.guns.runtime_mut(gun).unwrap();
    rt.chambers = 0;
    let rounds = rt.ammo.count;
    for i in 0..PRESS_FRAMES {
        press_frame(&mut run, &reference, i, keys::PRIMARY, 0);
    }
    let rt = run.area.interaction.guns.runtime(gun).unwrap();
    assert_eq!(rt.ammo.count, rounds, "an empty chamber fires no round");
    let saved = rt.snapshot();
    let mut loaded = save_load(&run, &reference);
    input(&mut loaded, &reference, FrameInput::default());
    let id = loaded
        .area
        .interaction
        .equipment
        .item()
        .expect("the revolver loads in the hand");
    assert_eq!(
        loaded.area.interaction.guns.runtime(id).unwrap().snapshot(),
        saved,
        "the cylinder and chambers save"
    );
}

/// Source `geppityuscript.Reload`: without a magazine the SMG does not fire. E at loose ammo drops
/// the magazine with the gun rounds. R at it loads it back. A held magazine drops the old one.
/// Save and load keep the gun state and the dropped magazine rounds.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd11_wd12_the_smg_needs_a_magazine_and_unloads_it_with_its_rounds() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    let gun = equip(&mut run, &reference, "Geppityu");
    let rt = run.area.interaction.guns.runtime_mut(gun).unwrap();
    rt.ammo.count = 17.;
    let GunKind::Magazine { mag_prefab, .. } = &run.area.interaction.guns.record(gun).unwrap().kind
    else {
        panic!("the SMG takes magazines")
    };
    let mag = reference
        .item_prototypes
        .iter()
        .position(|p| &p.source == mag_prefab)
        .expect("magazine item prototype");
    let pose = run.area.player.body_pose(&run.physics);
    let magazine = crate::starting_area::spawned::spawn_prototype(
        &reference,
        &mut run.area,
        &mut run.physics,
        mag,
        pose,
    )
    .unwrap()[0];
    let prototype = prototype_of(&run, magazine);
    let full = run.area.spawned.ammo(magazine).unwrap().ammo.count;
    let mags = |run: &GarageRuntime| -> Vec<ItemId> {
        run.area
            .spawned
            .iter()
            .filter(|s| s.prototype == prototype)
            .map(|s| s.id)
            .collect()
    };
    // E at loose ammo.
    run.reload_gun(&reference, gun, magazine, true);
    let rt = run.area.interaction.guns.runtime(gun).unwrap();
    assert!(!rt.has_mag && rt.ammo.count == 0., "the gun is empty");
    assert_eq!(run.area.spawned.ammo(magazine).unwrap().ammo.count, full);
    let dropped = *mags(&run)
        .iter()
        .find(|id| **id != magazine)
        .expect("a dropped magazine");
    assert_eq!(run.area.spawned.ammo(dropped).unwrap().ammo.count, 17.);
    // No magazine: no fire.
    for i in 0..PRESS_FRAMES {
        press_frame(&mut run, &reference, i, keys::PRIMARY, 0);
    }
    assert!(!run.area.interaction.guns.runtime(gun).unwrap().has_mag);
    // R at the dropped magazine: it goes in and is used up.
    run.reload_gun(&reference, gun, dropped, false);
    let rt = run.area.interaction.guns.runtime(gun).unwrap();
    assert!(rt.has_mag && rt.ammo.count == 17.);
    assert!(
        !run.area.spawned.contains(dropped),
        "the used magazine is gone"
    );
    // Save and load with a dropped magazine.
    run.reload_gun(&reference, gun, magazine, true);
    let saved = run.area.interaction.guns.runtime(gun).unwrap().snapshot();
    let mut loaded = save_load(&run, &reference);
    seconds(&mut loaded, &reference, 1.);
    input(&mut loaded, &reference, FrameInput::default());
    let counts: Vec<f32> = mags(&loaded)
        .iter()
        .filter_map(|id| loaded.area.spawned.ammo(*id))
        .map(|a| a.ammo.count)
        .collect();
    assert!(
        counts.contains(&17.),
        "the dropped magazine keeps 17 rounds"
    );
    let id = loaded
        .area
        .interaction
        .equipment
        .item()
        .expect("the SMG loads in the hand");
    assert_eq!(
        loaded.area.interaction.guns.runtime(id).unwrap().snapshot(),
        saved
    );
}

/// Explosive tanks near the player that a blast sets off: (tank, its content root).
fn explosive_tanks(run: &GarageRuntime) -> Vec<(ItemId, ItemId)> {
    let interaction = &run.area.interaction;
    let fluids = interaction.explosions.explosive_fluids();
    run.area
        .items
        .iter()
        .filter(|s| !s.removed && s.body.is_some())
        .filter_map(|s| {
            let e = interaction.tank_explosion(s.id)?;
            crate::interaction::explosions::tank_factor(e, s.fluid.as_ref()?, fluids, 1e6)?;
            let root = match s.location {
                ItemLocation::Attached(owner) => owner,
                _ => s.id,
            };
            (run.area.items.get(root)?.location == ItemLocation::World).then_some((s.id, root))
        })
        .collect()
}

fn position(run: &GarageRuntime, item: ItemId) -> Vector {
    let body = run.area.items.get(item).unwrap().body.unwrap();
    run.physics.rapier().bodies[body].translation()
}

/// Source `explosionscript.Explode` from a `tankscript` shot over the explosive amount: the tank
/// goes, a near bottle breaks, a second tank goes off. Save and load: they stay gone.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd14_a_shot_fuel_can_explodes_breaks_a_bottle_sets_off_a_second_and_stays_gone() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    let tanks = explosive_tanks(&run);
    let (first, root) = tanks[0];
    let (_, second) = *tanks
        .iter()
        .find(|(_, r)| *r != root)
        .expect("two explosive tanks near the start");
    let bottle = spawn(&mut run, &reference, "beer");
    let at = position(&run, root);
    place(&mut run, bottle, at + Vector::new(0.6, 0.3, 0.));
    place(&mut run, second, at + Vector::new(-0.8, 0.3, 0.));
    run.shot_explosive(Strike {
        item: first,
        force: 1e6,
        explode: true,
        hole: None,
    });
    run.run_explosions(&reference, FIXED_DT);
    seconds(&mut run, &reference, 0.5);
    assert!(gone(&run, root), "the shot tank explodes");
    assert!(gone(&run, bottle), "the blast breaks the bottle");
    assert!(gone(&run, second), "the blast sets off the second tank");
    let keys = [root, second].map(|id| run.area.items.object_key(id));
    let loaded = save_load(&run, &reference);
    for key in keys {
        assert!(
            loaded
                .area
                .items
                .resolve_key(&key)
                .is_none_or(|id| gone(&loaded, id)),
            "the exploded tanks stay gone"
        );
    }
}

/// Source `explosionscript.Explode` on the garage 20 L can: the can and its cap go away and stay
/// gone after save and load.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd15_the_shot_garage_can_explodes_and_stays_gone() {
    let (reference, mut run) = load_garage();
    seconds(&mut run, &reference, 1.);
    let interaction = &run.area.interaction;
    let fluids = interaction.explosions.explosive_fluids();
    let can = reference
        .tanks
        .iter()
        .filter(|t| t.role == crate::reference::TankRole::Portable)
        .map(|t| t.item)
        .find(|&id| {
            let state = run.area.items.get(id).unwrap();
            interaction.tank_explosion(id).is_some_and(|e| {
                crate::interaction::explosions::tank_factor(
                    e,
                    state.fluid.as_ref().unwrap(),
                    fluids,
                    1e6,
                )
                .is_some()
            })
        })
        .expect("an explosive garage can");
    let body = run.area.items.get(can).unwrap().body;
    let on_body: Vec<ItemId> = run
        .area
        .items
        .iter()
        .filter(|s| s.body == body)
        .map(|s| s.id)
        .collect();
    run.shot_explosive(Strike {
        item: can,
        force: 1e6,
        explode: true,
        hole: None,
    });
    run.run_explosions(&reference, FIXED_DT);
    seconds(&mut run, &reference, 0.1);
    for id in &on_body {
        assert!(gone(&run, *id), "the can and its cap are gone");
    }
    let mut loaded = save_load(&run, &reference);
    seconds(&mut loaded, &reference, 1.);
    for id in &on_body {
        assert!(gone(&loaded, *id), "the can stays gone after load");
    }
}

/// Source `tankscript` hole leak: a shot hole facing down drains a non-explosive tank, also the
/// garage water tank.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd17_a_hole_facing_down_leaks_a_tank_and_the_garage_water_tank() {
    let (reference, mut run) = load_garage();
    seconds(&mut run, &reference, 1.);
    let interaction = &run.area.interaction;
    let fluids = interaction.explosions.explosive_fluids();
    let leaky: Vec<ItemId> = run
        .area
        .items
        .iter()
        .filter(|s| {
            !s.removed
                && s.body.is_some()
                && s.fluid.as_ref().is_some_and(|f| {
                    f.total() > 0.5 && !crate::interaction::explosions::is_explosive(f, fluids)
                })
                && interaction
                    .tank_explosion(s.id)
                    .is_some_and(|e| !e.no_shoot_hole && !e.always_explode)
        })
        .map(|s| s.id)
        .collect();
    // The garage tank that is not portable: the water tank.
    let water = reference
        .tanks
        .iter()
        .filter(|t| t.role != crate::reference::TankRole::Portable)
        .map(|t| t.item)
        .find(|id| leaky.contains(id))
        .unwrap_or_else(|| {
            let tanks: Vec<_> = reference
                .tanks
                .iter()
                .map(|t| (t.role, run.area.items.name(t.item).map(str::to_string)))
                .collect();
            panic!("the garage water tank takes holes: {tanks:?}")
        });
    let amount = |run: &GarageRuntime, id: ItemId| {
        run.area
            .items
            .get(id)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .total()
    };
    for tank in [leaky[0], water] {
        let before = amount(&run, tank);
        run.shot_explosive(Strike {
            item: tank,
            force: 1.,
            explode: false,
            hole: Some([position(&run, tank), -Vector::Y]),
        });
        assert!(run.area.interaction.holes.count(tank) >= 1);
        seconds(&mut run, &reference, 3.);
        assert!(
            amount(&run, tank) < before,
            "the fluid leaks through the hole"
        );
    }
}

/// Source `breakablescript.TryBreak` on the tumbleweed: a hard hit breaks it with its gib. Save
/// and load: it stays gone.
#[test]
#[ignore = "requires the installed source export"]
fn tp_wd20_a_hard_hit_breaks_a_tumbleweed_with_its_gib_and_it_stays_gone() {
    let (reference, mut run) = load_garage();
    settle(&mut run, &reference);
    let prototype = reference
        .item_prototypes
        .iter()
        .position(|p| p.item_id == Some(reference.weather.tumbleweeds.item_id))
        .expect("tumbleweed prototype");
    let mut pose = run.area.player.body_pose(&run.physics);
    pose.position[1] += 2.;
    pose.rotation = [0., 0., 0., 1.];
    let weed = crate::starting_area::spawned::spawn_prototype(
        &reference,
        &mut run.area,
        &mut run.physics,
        prototype,
        pose,
    )
    .unwrap()[0];
    seconds(&mut run, &reference, 2.);
    let rules = &reference.breakables[run.stream.breakables.targets()[target(&run, weed)].record];
    let gib = rules.gib.as_ref().map(|g| {
        reference
            .item_prototypes
            .iter()
            .position(|p| &p.id == g)
            .unwrap()
    });
    assert!(!rules.no_gib && gib.is_some(), "the tumbleweed has a gib");
    let mut seen = Vec::new();
    serials(&run, gib.unwrap(), &mut seen);
    let before = seen.len();
    drop_from(&mut run, weed, 10.);
    for _ in 0..(3. / FIXED_DT) as usize {
        frame(&mut run, &reference);
        serials(&run, gib.unwrap(), &mut seen);
    }
    let most = seen.len();
    assert!(!run.area.spawned.contains(weed), "the hit breaks it");
    assert_eq!(most, before + 1, "the break spawns its gib once");
    let loaded = save_load(&run, &reference);
    assert_eq!(copies(&loaded, prototype), 0, "it stays gone");
}
