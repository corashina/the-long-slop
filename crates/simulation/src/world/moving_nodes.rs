//! Prototype nodes that runtime code writes after spawn.
use super::WorldStream;
use crate::reference::{ReferenceError, ReferenceManifest, WorldPrototypeReference};
use crate::world::spawn_rules::*;
impl WorldStream {
    /// Prototype nodes whose transform or visibility runtime code writes after spawn.
    /// Presentation spawns entities only for these nodes. Every node writer must use a node from this list.
    /// Renderers carry node activity (food parts, appearance) themselves, so it is not in this list.
    pub fn moving_nodes(
        &self,
        reference: &ReferenceManifest,
        prototype: &WorldPrototypeReference,
    ) -> Result<Vec<bool>, ReferenceError> {
        let rules = &self.contents.rules;
        let world = &*self.reference;
        let mut moving = vec![false; prototype.nodes.len()];
        // Item scripts, tank caps, spirit levels and mechanisms move or show many nodes.
        // Radio needle, music box, metal detector, dial, cap and valve nodes, level bubbles, mechanism poses.
        let mut scripts = Vec::new();
        for class in [
            "radioscript",
            "musicboxscript",
            "metalDetectorScript",
            "compassscript",
            "clockscript",
            "tankcapscript",
        ] {
            rules.children(world, &prototype.source, class, true, &mut scripts)?;
        }
        // Cars move wheels, doors, needles, pedals and show or hide parts.
        if !scripts.is_empty()
            || prototype.vehicle.is_some()
            || prototype.source == reference.small_items.spirit_level.prototype
            || world
                .mechanisms
                .iter()
                .any(|m| m.prototype == prototype.source)
        {
            moving.fill(true);
            return Ok(moving);
        }
        for body in &prototype.bodies {
            moving[body.node as usize] = true;
        }
        // Spray bottle trigger.
        let trigger = &reference.small_items.spray_bottle.trigger;
        // Lamp buttons: flashlightscript GButton.
        let mut lamps = Vec::new();
        rules.children(
            world,
            &prototype.source,
            "flashlightscript",
            true,
            &mut lamps,
        )?;
        let mut buttons = Vec::with_capacity(lamps.len());
        for lamp in lamps {
            if let Some(button) = optional(rules.get(world, lamp)?.fields, "GButton")? {
                buttons.push(button);
            }
        }
        for (node, flag) in prototype.nodes.iter().zip(&mut moving) {
            *flag |= node.source == *trigger || buttons.contains(&&node.source);
        }
        Ok(moving)
    }
}
