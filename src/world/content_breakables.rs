//! Binds source breakablescripts of active content to `damage::Breakables` targets, steps their hits
//! and removes broken items.
//! Items bind by their world prototype. Lamp triggers bind by their structure lamp control.
use super::WorldStream;
use crate::damage::BreakKind;
use crate::interaction::items::{ItemLocation, ItemStore};
use crate::interaction::storage::StorageLocks;
use crate::reference::{
    BreakableOwnerKind, BreakableReference, ItemId, ReferenceError, ReferenceManifest, SourceId,
};
use crate::vehicle::PhysicsWorld;
use rapier3d::prelude::ActiveEvents;

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
        physics: &mut PhysicsWorld,
        dt: f32,
    ) {
        self.refresh_boxes();
        self.refresh_breakables(reference, items, physics);
        self.breakables.step_hits(
            &reference.breakables,
            physics,
            &self.trigger_roles,
            |item| items.get(item).and_then(|s| s.body),
            dt,
        );
    }
    /// Next target that broke in `step_breakables`, in break order: its kind and record index.
    pub fn take_break(&mut self) -> Option<(BreakKind, usize)> {
        let target = self.breakables.take_break()?;
        let target = &self.breakables.targets()[target];
        Some((target.kind.clone(), target.record))
    }
    /// Source Break: one random clip of the record.
    pub fn pick_break_clip<'a>(&mut self, record: &'a BreakableReference) -> Option<&'a SourceId> {
        self.breakables.pick_clip(record)
    }
    /// Source Break destroys the item unless noDestroy. A box releases its content first.
    /// Items locked in the item unlock at their current pose. The removal is saved.
    pub fn remove_broken_item(
        &mut self,
        reference: &ReferenceManifest,
        record: usize,
        item: ItemId,
        items: &mut ItemStore,
        storage: &mut StorageLocks,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let Some(body) = items
            .get(item)
            .filter(|s| !s.removed && s.location == ItemLocation::World)
            .and_then(|s| s.body)
        else {
            return Ok(());
        };
        if reference.breakables[record].source == reference.acquisition.r#box.breakable {
            self.break_box(item, items);
        }
        storage.unbind_owner(item, |_, _| false, items, physics)?;
        self.changes.remove(items.object_key(item));
        physics.edit_body(body, |body| body.set_enabled(false));
        physics.refresh_queries();
        items.get_mut(item).expect("broken item").removed = true;
        items.mark_dirty(item);
        Ok(())
    }
    /// Rebinds targets on a content, item or lamp revision change only.
    fn refresh_breakables(
        &mut self,
        reference: &ReferenceManifest,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
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
        // Source OnCollisionEnter runs on every contact. Item colliders without a hit sound report
        // no contacts, so bound item colliders report them here.
        for target in self.breakables.targets() {
            let BreakKind::Item(item) = target.kind else {
                continue;
            };
            let Some(body) = items.get(item).and_then(|s| s.body) else {
                continue;
            };
            for index in 0..physics.rapier().bodies[body].colliders().len() {
                let collider = physics.rapier().bodies[body].colliders()[index];
                let events = physics.rapier().colliders[collider].active_events();
                if !events.contains(ActiveEvents::COLLISION_EVENTS) {
                    physics.edit_collider(collider, |c| {
                        c.set_active_events(events | ActiveEvents::COLLISION_EVENTS)
                    });
                }
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
