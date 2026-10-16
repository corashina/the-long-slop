//! Source coordinate-seeded building children. These do not use Unity random state.
use super::spawn_rules::*;
use super::{AbsolutePose, Placement, SourceRandom, WorldBounds};
use crate::reference::{ReferenceError, SourceId, WorldReference};
use std::collections::{HashMap, HashSet};

pub(crate) fn append_ramps(
    reference: &WorldReference,
    rules: &SourceRules,
    seed: i32,
    placements: &mut Vec<Placement>,
    maximum: usize,
) -> Result<(), ReferenceError> {
    // One pass finds existing ramps. Buildings share a few prototypes, so their
    // components resolve once per call.
    let mut ramps: HashSet<_> = placements
        .iter()
        .filter_map(|p| {
            let (source, ordinal) = p.id.children.last()?;
            (*ordinal == u32::MAX).then(|| (p.id.layer, p.id.unit, p.id.ordinal, source.clone()))
        })
        .collect();
    let mut buildings: HashMap<SourceId, Option<SourceRecord>> = HashMap::new();
    let parents = placements.len();
    for index in 0..parents {
        let parent = &placements[index];
        if !parent.id.children.is_empty() {
            continue;
        }
        let building = match buildings.get(&parent.prototype) {
            Some(building) => *building,
            None => {
                let building = rules.component(reference, &parent.prototype, "buildingscript")?;
                buildings.insert(parent.prototype.clone(), building);
                building
            }
        };
        let Some(building) = building else {
            continue;
        };
        let key = (
            parent.id.layer,
            parent.id.unit,
            parent.id.ordinal,
            building.source.clone(),
        );
        if ramps.contains(&key) {
            continue;
        }
        let f = building.fields;
        let position = [
            parent.pose.position[0],
            parent.pose.position[1],
            -parent.pose.position[2],
        ];
        // savedatascript.buildingRounding is the checked constant 20.0.
        let local_seed = position.into_iter().fold(seed, |seed, value| {
            seed.wrapping_add(source_i32((value / 20.).round_ties_even() * 20.))
        });
        let mut random = SourceRandom::new(local_seed);
        let chance = random.sample() as f32;
        let target = optional(f, "rampPrefab")?;
        let threshold = number(f, "rampChance")?;
        let Some(target) = target.filter(|_| chance < threshold) else {
            continue;
        };
        let angle = random.sample() as f32 * 360.;
        let radius = number(f, "rampMinRadius")?;
        // The source passes rampMinRadius twice and still consumes this draw.
        let fraction = random.sample() as f32;
        let radius = radius + (radius - radius) * fraction;
        let low = number(f, "rampMinOffsety")?;
        let high = number(f, "rampMaxOffsety")?;
        let height = low + (high - low) * (random.sample() as f32);
        let rotation = angle
            + number(f, "rampOffsetRY")?
            + if random.next_range(0, 2) == 0 {
                -90.
            } else {
                90.
            };
        let root = rules.root(reference, target)?;
        let prototype = reference
            .prototypes
            .iter()
            .find(|p| &p.source == root)
            .ok_or_else(|| ReferenceError("Missing checked source ramp prototype".into()))?;
        let offset = super::placement::rotate(
            super::spawn_building::source_euler([0., angle, 0.]),
            [0., 0., -radius],
        );
        let position = [
            parent.pose.position[0] + f64::from(offset[0]),
            parent.pose.position[1] + f64::from(height),
            parent.pose.position[2] + f64::from(offset[2]),
        ];
        let mut id = parent.id.clone();
        id.children.push((building.source.clone(), u32::MAX));
        if placements.len() == maximum {
            return Err(ReferenceError(
                "Source ramp object admission exceeds capacity".into(),
            ));
        }
        let radius = rules
            .radius(root)
            .ok_or_else(|| ReferenceError("Missing source ramp bounds".into()))?;
        placements.push(Placement {
            id,
            prototype: root.clone(),
            pose: AbsolutePose {
                position,
                rotation: super::spawn_building::source_euler([0., rotation, 0.]),
            },
            scale: prototype.prefab_pose.scale,
            bounds: WorldBounds {
                min: position.map(|v| v - radius),
                max: position.map(|v| v + radius),
            },
            object_seed: local_seed,
        });
        ramps.insert(key);
    }
    Ok(())
}

fn source_i32(value: f64) -> i32 {
    if !value.is_finite() || value < -2147483648. || value >= 2147483648. {
        i32::MIN
    } else {
        value as i32
    }
}
