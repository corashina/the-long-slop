//! Source seatscript seats on world prototypes, decoded from the checked world records.
use crate::reference::{ReferenceError, SourceId, WorldPrototypeReference, WorldReference};
use crate::world::spawn_rules::*;

/// Player view pitch clamp of the source seat look (`FyRot = Mathf.Clamp(FyRot, -89f, 89f)`).
const SEAT_PITCH_LIMITS_DEGREES: [f32; 2] = [-89.0, 89.0];

/// One main seatscript of a world prototype. Node values are prototype node indices.
/// The exit distance is `PlayerReference::exit_distance`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorldSeatReference {
    /// The main seatscript. It is the save key of the seat.
    pub source: SourceId,
    /// `sitPos`, or the seatscript node.
    pub sit_node: u32,
    /// Nodes of the main seatscript and of all seatscripts whose `mainseat` is this seat.
    pub hit_nodes: Vec<u32>,
    pub yaw_limits_degrees: [f32; 2],
    pub pitch_limits_degrees: [f32; 2],
    pub default_pitch_degrees: f32,
    pub min_exit_angle_degrees: f32,
    pub exit_height: f32,
    pub show_sleep_text: bool,
}

/// Active seatscript seats of a prototype, in source hierarchy order.
#[expect(dead_code, reason = "item seat binding (v04 Task 2.3) reads it")]
pub(crate) fn prototype_seats(
    reference: &WorldReference,
    rules: &SourceRules,
    prototype: &WorldPrototypeReference,
) -> Result<Vec<WorldSeatReference>, ReferenceError> {
    let mut scripts = Vec::new();
    rules.children(
        reference,
        &prototype.source,
        "seatscript",
        false,
        &mut scripts,
    )?;
    if scripts.is_empty() {
        return Ok(Vec::new());
    }
    let node_of = |object: &SourceId| -> Result<u32, ReferenceError> {
        let transform = rules.root(reference, object)?;
        prototype
            .nodes
            .iter()
            .position(|n| &n.source == transform)
            .map(|i| i as u32)
            .ok_or_else(|| ReferenceError("node outside its prototype".into()))
    };
    let mut seats = Vec::new();
    let mut secondary = Vec::new();
    for &script in &scripts {
        let seat = |e: ReferenceError| {
            ReferenceError(format!(
                "Unreadable seat {}:{} of prototype {}:{}: {}",
                script.file, script.path_id, prototype.source.file, prototype.source.path_id, e.0
            ))
        };
        let f = rules.get(reference, script).map_err(seat)?.fields;
        let node = node_of(script).map_err(seat)?;
        if let Some(main) = optional(f, "mainseat").map_err(seat)?
            && main != script
        {
            secondary.push((main, node));
            continue;
        }
        seats.push(main_seat(script, f, node, &node_of).map_err(seat)?);
    }
    for (main, node) in secondary {
        let seat = seats
            .iter_mut()
            .find(|s| &s.source == main)
            .ok_or_else(|| {
                ReferenceError(format!(
                    "Seat main seat {}:{} is not an active main seat of prototype {}:{}",
                    main.file, main.path_id, prototype.source.file, prototype.source.path_id
                ))
            })?;
        seat.hit_nodes.push(node);
    }
    Ok(seats)
}

fn main_seat(
    script: &SourceId,
    f: Fields<'_>,
    node: u32,
    node_of: &impl Fn(&SourceId) -> Result<u32, ReferenceError>,
) -> Result<WorldSeatReference, ReferenceError> {
    // The port has only the clamped yaw lean look of the selected car seats.
    for unsupported in ["lookInsteadOfLean", "BcantSitAngle", "limitLessRot"] {
        if flag(f, unsupported)? {
            return Err(ReferenceError(format!("unsupported {unsupported}")));
        }
    }
    let sit_node = match optional(f, "sitPos")? {
        Some(sit) => node_of(sit)?,
        None => node,
    };
    let rot_limit = number(f, "rotLimit")?;
    let seat = WorldSeatReference {
        source: script.clone(),
        sit_node,
        hit_nodes: vec![node],
        yaw_limits_degrees: [-rot_limit, rot_limit],
        pitch_limits_degrees: SEAT_PITCH_LIMITS_DEGREES,
        default_pitch_degrees: number(f, "defYrot")?,
        min_exit_angle_degrees: number(f, "minGetUpAnlge")?,
        exit_height: number(f, "getOutHeight")?,
        show_sleep_text: flag(f, "showSleepText")?,
    };
    let values = [
        rot_limit,
        seat.default_pitch_degrees,
        seat.min_exit_angle_degrees,
        seat.exit_height,
    ];
    if values.iter().any(|v| !v.is_finite())
        || rot_limit < 0.
        || !(SEAT_PITCH_LIMITS_DEGREES[0]..=SEAT_PITCH_LIMITS_DEGREES[1])
            .contains(&seat.default_pitch_degrees)
    {
        return Err(ReferenceError("invalid seat limits".into()));
    }
    Ok(seat)
}
