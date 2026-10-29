//! X05: world part conditions repair and paint live, like garage surfaces.
use super::drive_out::frame;
use super::*;
use crate::world::ObjectKey;

#[test]
#[ignore = "requires the installed source export"]
fn lada_part_paint_and_repair_change_its_appearance() {
    let reference = reference();
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    let mut lada = None;
    for _ in 0..2000 {
        lada = run.vehicles.keys().find_map(|key| match key {
            ObjectKey::Generated(id) if id.layer == WorldLayer::Startup => Some(id.clone()),
            _ => None,
        });
        if lada.is_some() {
            break;
        }
        frame(&mut run, &reference);
    }
    let lada = lada.expect("start car spawns");
    let content = run
        .stream
        .contents
        .active
        .iter()
        .position(|c| c.content.placement.id == lada)
        .expect("start car content");
    let entries = run.stream.contents.active[content].conditions.clone();
    assert!(!entries.is_empty(), "start car has part conditions");
    let conditions = &mut run.area.interaction.conditions;
    // Paint the first entry that spray paint can paint and that shows its color (state below 3).
    let &(painted, index, _) = entries
        .iter()
        .find(|(id, ..)| conditions.can_repair(*id, 0) && conditions.get(*id).state < 3)
        .expect("a paintable start car part");
    let color = [0.1, 0.6, 0.3, 1.];
    assert!(conditions.paint(painted, color));
    // Repair the first entry that the sponge or the wire brush can improve.
    let repaired = entries.iter().find_map(|&(id, ..)| {
        let before = conditions.get(id).state;
        (before > 0 && (conditions.can_repair(id, 2) || conditions.can_repair(id, 1)))
            .then_some((id, before))
    });
    if let Some((id, before)) = repaired.filter(|(id, _)| *id != painted) {
        let tool = if conditions.can_repair(id, 2) { 2 } else { 1 };
        assert_eq!(
            conditions.repair(id, tool, 1.),
            crate::conditions::RepairResult::Improved
        );
        assert_eq!(conditions.get(id).state, before - 1);
    }
    frame(&mut run, &reference);
    let content = &run.stream.contents.active[content];
    let source = &content.content.inputs.conditions[index].source;
    let renderers = run
        .stream
        .contents
        .rules
        .get(&run.stream.reference, source)
        .unwrap()
        .fields;
    let renderers = crate::world::spawn_rules::array(renderers, "renderers").unwrap();
    if !renderers.is_empty() {
        assert!(
            content
                .content
                .appearance
                .iter()
                .any(|a| a.color == Some(color)),
            "the painted color reaches the appearance"
        );
    }
}
