//! Binds source breakablescripts of active content to `damage::Breakables` targets and steps their hits.
//! Items bind by their world prototype. Lamp triggers bind by their structure lamp control.
use super::WorldStream;
use crate::damage::BreakKind;
use crate::interaction::items::ItemStore;
use crate::reference::{BreakableOwnerKind, BreakableReference, ReferenceManifest, SourceId};
use crate::vehicle::PhysicsWorld;

/// The node of the breakablescript in a world prototype.
fn world_node(record: &BreakableReference, prototype: &SourceId) -> Option<u32> {
    record
        .owners
        .iter()
        .find(|o| o.kind == BreakableOwnerKind::World && &o.prototype == prototype)
        .map(|o| o.node)
}

impl WorldStream {
    /// Run after the physics step. Queues the targets that break this step.
    pub fn step_breakables(
        &mut self,
        reference: &ReferenceManifest,
        items: &ItemStore,
        physics: &PhysicsWorld,
        dt: f32,
    ) {
        self.refresh_boxes();
        self.refresh_breakables(reference, items);
        self.breakables.step_hits(
            &reference.breakables,
            physics,
            &self.trigger_roles,
            |item| items.get(item).and_then(|s| s.body),
            dt,
        );
    }
    /// Rebinds targets on a content, item or lamp revision change only.
    fn refresh_breakables(&mut self, reference: &ReferenceManifest, items: &ItemStore) {
        let scan = [
            self.request.session,
            self.contents.active_revision(),
            items.membership_revision(),
            self.lamp_binding_revision,
        ];
        if !self.breakables.rescan(scan) {
            return;
        }
        let records = &reference.breakables;
        let active = &self.contents.active;
        let lamps = &self.lamp_bindings;
        let structures = &self.structures;
        let present = |item| items.get(item).is_some_and(|s| !s.removed);
        self.breakables.retain(|t| match &t.kind {
            BreakKind::Item(item) => present(*item) && active.iter().any(|c| c.item == Some(*item)),
            BreakKind::Lamp { parent, control } => lamps
                .iter()
                .any(|b| b.control == *control && &b.parent == parent),
        });
        // A content item whose world prototype holds a breakablescript without a lamp.
        for content in active {
            let Some(item) = content.item.filter(|i| present(*i)) else {
                continue;
            };
            let prototype = &content.content.placement.prototype;
            if let Some(record) = records
                .iter()
                .position(|r| r.lamp.is_none() && world_node(r, prototype).is_some())
            {
                self.breakables
                    .bind(BreakKind::Item(item), record, &records[record]);
            }
        }
        // A structure lamp control: each breakablescript in its prototype that breaks its lamps.
        for binding in lamps {
            let Some(instance) = structures.iter().find(|s| s.placement.id == binding.parent)
            else {
                continue;
            };
            for (record, rules) in records.iter().enumerate() {
                if rules
                    .lamp
                    .as_ref()
                    .is_some_and(|l| l.control == binding.source)
                    && world_node(rules, &instance.placement.prototype).is_some()
                {
                    let kind = BreakKind::Lamp {
                        parent: binding.parent.clone(),
                        control: binding.control,
                    };
                    self.breakables.bind(kind, record, rules);
                }
            }
        }
        // Lamp trigger colliders are the structure colliders on the breakablescript node.
        for index in 0..self.breakables.targets().len() {
            let target = &self.breakables.targets()[index];
            let BreakKind::Lamp { parent, .. } = &target.kind else {
                continue;
            };
            let record = target.record;
            let instance = structures
                .iter()
                .find(|s| &s.placement.id == parent)
                .expect("bound lamp structure");
            let node = world_node(&records[record], &instance.placement.prototype)
                .expect("bound lamp owner");
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .expect("structure prototype");
            for (source, handle) in &instance.colliders {
                if prototype
                    .colliders
                    .iter()
                    .any(|c| &c.source == source && c.node == node)
                {
                    self.breakables.bind_collider(*handle, index);
                }
            }
        }
    }
}
