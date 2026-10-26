//! Checked computer (computerscript) and TV (displayscript) inputs.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerRecord {
    pub computer: ComputerReference,
    pub display: DisplayReference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerReference {
    pub item: u32,
    pub prototype: SourceId,
    pub source: SourceId,
    pub x_size: u16,
    pub y_size: u16,
    /// CursorPosM: screen pixels per cursor column and row.
    pub cursor_step: [f32; 2],
    pub volume: f32,
    pub pitch_difference_chance: f32,
    pub pitch_difference: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayReference {
    pub item: u32,
    pub prototype: SourceId,
    pub source: SourceId,
    /// The node of displayscript.R.
    pub screen: SourceId,
    pub on_material: SourceId,
    pub off_material: SourceId,
    pub startup_on: bool,
    pub switch: DisplaySwitchReference,
}

/// The usablescript whose turnedEvent calls displayscript.Turned.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplaySwitchReference {
    pub source: SourceId,
    pub node: SourceId,
    pub clip: SourceId,
    pub volume: f32,
}

const COMPUTER_ITEM: u32 = 73;
const DISPLAY_ITEM: u32 = 152;

fn checked(ok: bool, field: &str) -> Result<(), ReferenceError> {
    if ok {
        Ok(())
    } else {
        Err(ReferenceError(format!("Invalid computer field {field}")))
    }
}

impl ComputerRecord {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        let prototype = |item: u32, name: &str, source: &SourceId| {
            m.world
                .catalog
                .items
                .get(item as usize)
                .filter(|i| i.index == item && i.name == name)
                .and_then(|i| i.prototype.as_ref())
                .filter(|p| *p == source)
                .and_then(|p| m.world.prototypes.iter().find(|w| &w.source == p))
                .ok_or_else(|| ReferenceError(format!("Invalid computer owner {name}")))
        };
        let inside =
            |p: &WorldPrototypeReference, node: &SourceId| p.nodes.iter().any(|n| &n.source == node);
        let c = &self.computer;
        prototype(COMPUTER_ITEM, "gep01", &c.prototype)?;
        checked(
            c.item == COMPUTER_ITEM
                && c.x_size > 0
                && c.y_size > 0
                && finite(&c.cursor_step)
                && finite(&[c.volume, c.pitch_difference_chance, c.pitch_difference])
                && (0. ..=1.).contains(&c.volume)
                && (0. ..=1.).contains(&c.pitch_difference_chance)
                && c.pitch_difference >= 0.,
            "computer",
        )?;
        let d = &self.display;
        let p = prototype(DISPLAY_ITEM, "teve01", &d.prototype)?;
        let s = &d.switch;
        checked(
            d.item == DISPLAY_ITEM
                && inside(p, &d.screen)
                && inside(p, &s.node)
                && d.on_material != d.off_material
                && item_prototypes::clip_exists(m, &s.clip)
                && s.volume.is_finite()
                && (0. ..=1.).contains(&s.volume),
            "display",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_reject_unknown_fields() {
        let switch = r#"{"source":{"file":"sharedassets1.assets","path_id":143226},
            "node":{"file":"sharedassets1.assets","path_id":77821},
            "clip":{"file":"sharedassets1.assets","path_id":4115},"volume":0.5}"#;
        let parsed: DisplaySwitchReference = serde_json::from_str(switch).unwrap();
        assert_eq!(parsed.volume, 0.5);
        let extra = switch.replacen("\"volume\"", "\"extra\":1,\"volume\"", 1);
        assert!(serde_json::from_str::<DisplaySwitchReference>(&extra).is_err());
    }
}
