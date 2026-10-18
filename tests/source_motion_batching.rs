#[path = "../src/presentation/motion_batch.rs"]
mod motion_batch;

use motion_batch::{MotionInstance, MotionRun, MotionRunPlanner};

fn plan(
    entries: impl IntoIterator<Item = Option<(u32, u32, MotionInstance)>>,
) -> Vec<MotionRun<u32, u32>> {
    let mut planner = MotionRunPlanner::new();
    let mut runs = Vec::new();
    for entry in entries {
        runs.extend(planner.push(entry));
    }
    runs.extend(planner.finish());
    assert!(planner.finish().is_none());
    runs
}

fn entry(
    mesh: u32,
    entity: u32,
    index: u32,
    dynamic_offset: Option<u32>,
) -> Option<(u32, u32, MotionInstance)> {
    Some((
        mesh,
        entity,
        MotionInstance {
            index,
            dynamic_offset,
        },
    ))
}

#[test]
fn mixed_eligibility_preserves_packed_model_order_and_holes() {
    // Stable history slots are deliberately unrelated to the prepared bin order.
    let packed_models = [91, 7, 43, 12];
    let runs = plan([
        None,
        entry(3, 91, 0, Some(0)),
        entry(3, 7, 1, Some(0)),
        None,
        entry(3, 43, 2, Some(0)),
        None,
        entry(3, 12, 3, Some(0)),
        None,
    ]);
    assert_eq!(
        runs.iter().map(|r| r.first).collect::<Vec<_>>(),
        [91, 43, 12]
    );
    assert_eq!(
        runs.iter().map(|r| r.instances.clone()).collect::<Vec<_>>(),
        [0..2, 2..3, 3..4]
    );
    let drawn = runs
        .iter()
        .flat_map(|r| r.instances.clone().map(|i| packed_models[i as usize]))
        .collect::<Vec<_>>();
    assert_eq!(drawn, packed_models);
    assert!(plan([None, None]).is_empty());
}

#[test]
fn uniform_chunk_boundaries_and_authored_mesh_changes_split_draws() {
    let runs = plan([
        entry(3, 91, 26, Some(0)),
        entry(3, 7, 27, Some(0)),
        entry(3, 43, 0, Some(4096)),
        entry(3, 12, 1, Some(4096)),
        entry(4, 5, 2, Some(4096)),
        entry(4, 6, 3, Some(4096)),
        entry(4, 8, 5, Some(4096)),
    ]);
    assert_eq!(
        runs.iter()
            .map(|r| (r.first, r.instances.clone(), r.dynamic_offset))
            .collect::<Vec<_>>(),
        [
            (91, 26..28, Some(0)),
            (43, 0..2, Some(4096)),
            (5, 2..4, Some(4096)),
            (8, 5..6, Some(4096)),
        ]
    );
}

#[test]
fn storage_indices_merge_without_dynamic_offsets_and_finish_between_batches() {
    let mut planner = MotionRunPlanner::new();
    assert!(planner.push(entry(3, 91, 20, None)).is_none());
    assert!(planner.push(entry(3, 7, 21, None)).is_none());
    assert_eq!(planner.finish().unwrap().instances, 20..22);
    assert!(planner.push(entry(3, 43, 22, None)).is_none());
    let run = planner.finish().unwrap();
    assert_eq!(run.first, 43);
    assert_eq!(run.instances, 22..23);
    assert_eq!(run.dynamic_offset, None);
}
