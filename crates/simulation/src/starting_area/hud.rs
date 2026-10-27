//! HUD values of one tick. The renderer passes them to the browser shell.
use super::GarageRuntime;
use crate::boundary::{Change, Feedback, HudBlock, SimFrame};
use crate::interaction::feedback::FeedbackContext;
use crate::interaction::labels::display_name;
use crate::reference::{ItemId, OpeningId, ReferenceManifest};

/// Feedback values that change the shell text. The simulation sends feedback when they change.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct FeedbackStamp {
    actions: u32,
    context: u32,
    light_control: u32,
    lmb_use: bool,
    prompts: u8,
    computer: u8,
    item: Option<ItemId>,
    opening: Option<OpeningId>,
    carrying: bool,
    notice: &'static str,
    amount: u32,
    empty: bool,
    capacity: f32,
    percents: [u32; 6],
    order: u32,
    condition: u32,
    held: Option<ItemId>,
    throw_charge: u32,
    throw_active: bool,
    rpm: u32,
    engine_flags: [bool; 5],
}

impl GarageRuntime {
    pub(super) fn fill_hud(&mut self, reference: &ReferenceManifest, frame: &mut SimFrame) {
        let mut hud = HudBlock::default();
        let inventory = &self.area.inventory;
        hud.slots = inventory.slots.map(|id| id.map_or(0, |id| id.0));
        hud.selected = u32::from(inventory.selected);
        hud.swap_progress = inventory.swap_progress;
        let needs = self.needs_output();
        hud.hp = needs.needs.hp;
        hud.food = needs.needs.food;
        hud.water = needs.needs.water;
        hud.bowel = needs.needs.bowel;
        hud.bladder = needs.bladder_liters;
        hud.opacity = needs.opacity;
        hud.look_scale = needs.look_scale;
        let sleep = self.sleep.output();
        hud.sleep_phase = sleep.phase as u32;
        hud.sleep_charge = sleep.charge;
        hud.cooldown_remaining = sleep.cooldown_remaining;
        hud.sleep_progress = sleep.progress;
        for (on, flag) in [
            (needs.dead, HudBlock::DEAD),
            (needs.menu_due, HudBlock::MENU_DUE),
            (sleep.eligible, HudBlock::SLEEP_ELIGIBLE),
            (sleep.show_hint, HudBlock::SLEEP_HINT),
            (self.autosave_safe(false), HudBlock::AUTOSAVE_SAFE),
            (self.area.player.driving(), HudBlock::DRIVING),
            (self.stream.computers.typing, HudBlock::TYPING),
        ] {
            if on {
                hud.flags |= flag;
            }
        }
        let assists = self.driving_assists;
        hud.assists = assists.automatic_clutch as u32 | ((assists.automatic_gears as u32) << 1);
        let controls = self.driving_input.controls;
        hud.controls = [
            controls.steer,
            controls.throttle,
            controls.brake,
            controls.clutch,
            controls.handbrake,
        ];
        hud.gear = self
            .vehicle
            .as_ref()
            .map_or(0, |key| self.vehicles[key].drivetrain.gear() as i32 - 1);
        if let Some(c) = self.area.interaction.guns.crosshair() {
            hud.weapon_icon = c.sprite.map_or(0, |s| s + 1);
            hud.weapon_color = c
                .color
                .iter()
                .enumerate()
                .map(|(i, v)| ((v.clamp(0., 1.) * 255.).round() as u32) << (8 * i))
                .sum();
        }
        let Some(key) = self.vehicle.as_ref() else {
            frame.hud = hud;
            return;
        };
        hud.flags |= HudBlock::CAR;
        let current = &self.vehicles[key];
        let snapshot = self.physics.snapshot(key);
        let position = self.area.player.focus_position(&self.physics);
        hud.speed_kmh = snapshot.speed_mps * 3.6;
        hud.focus = position;
        hud.absolute = self.stream.origin.absolute(position);
        hud.origin = self.stream.origin.offset;
        hud.contacts = self
            .physics
            .wheel_contacts(key)
            .enumerate()
            .fold(0, |bits, (i, contact)| bits | ((contact as u32) << i));
        hud.surfaces = self.stream.frame().surfaces.len() as u32;
        hud.colliders = self.physics.collider_count() as u32;
        hud.rebases = self.rebases;
        hud.trip_m = current.trip_m;
        hud.fuel = current
            .fuel_tank
            .and_then(|tank| self.area.items.get(tank))
            .and_then(|item| item.fluid.as_ref())
            .map_or(0.0, |fluid| fluid.finite_total());
        let available = self.area.interaction.available_actions(
            &self.aim,
            &self.area.player,
            &self.area.items,
            &self.physics,
            self.survival.output(),
        );
        // The radio knob hint shows the volume or frequency of the aimed radio.
        if let Some(radio) = self
            .aim
            .use_target
            .as_ref()
            .and_then(|t| self.area.interaction.dial_knob(t, &self.physics))
            .and_then(|(knob, _)| self.stream.knob_radio(knob))
        {
            hud.flags |= HudBlock::RADIO | if radio.am { HudBlock::RADIO_AM } else { 0 };
            hud.radio_volume = radio.volume;
            hud.radio_frequency = radio.frequency();
        }
        frame.hud = hud;
        let info = self
            .area
            .interaction
            .feedback(&available, &self.area.items, reference);
        let (mut amount, mut capacity, quantities, order, fluid_name, has_name, mut no_text) = info
            .fluid
            .as_ref()
            .map_or((0., 0., [0.; 6], 0, info.name, false, true), |f| {
                (
                    f.amount,
                    f.capacity,
                    f.quantities,
                    f.order,
                    f.presentation.name.as_str(),
                    f.presentation.has_name,
                    f.presentation.no_text,
                )
            });
        let paint = self.area.interaction.paint_feedback(&self.aim, reference);
        if let Some((quantity, maximum)) = paint {
            amount = quantity;
            capacity = maximum;
            no_text = false;
        }
        let condition = available
            .item
            .and_then(|id| {
                self.area
                    .conditions
                    .item_quality(id)
                    .or_else(|| self.area.items.get(id).and_then(|i| i.generated_condition))
            })
            .unwrap_or(-1.0);
        let held = self.area.interaction.held();
        let notice = if !self.notice.is_empty() {
            self.notice
        } else if let Some(block) = self.aim.installation_block.or(self.aim.grab_block) {
            block.notice()
        } else {
            ""
        };
        let engine = self.engine_output(key);
        let seated = self.area.player.seated();
        let throw_charge = self.area.interaction.throw_charge();
        let throw_active = self.area.interaction.throw_active();
        let stamp = FeedbackStamp {
            condition: condition.to_bits(),
            held,
            throw_charge: throw_charge.to_bits(),
            throw_active,
            rpm: engine.rpm.to_bits(),
            engine_flags: [
                current.ignition.ignition,
                engine.running,
                engine.starter,
                seated,
                current.handbrake,
            ],
            actions: info.actions.0
                | if needs.dead || self.sleeping() {
                    0
                } else {
                    self.area.cigarettes.actions(
                        self.area
                            .interaction
                            .equipment
                            .item()
                            .or(self.area.interaction.held()),
                    ) | self.area.wear.actions(&self.area.interaction)
                },
            context: if paint.is_some() {
                FeedbackContext::Paint as u32
            } else {
                info.context as u32
            },
            light_control: available.light_control.map_or(0, |id| id.0),
            lmb_use: available.lmb_use,
            prompts: available.prompts
                | self.stream.refill_prompt()
                | self.area.interaction.guns.prompt,
            computer: self.computer_prompt(),
            item: available.item,
            opening: available.opening,
            carrying: info.carrying,
            notice,
            amount: if paint.is_some() {
                amount.round() as u32
            } else {
                (amount * 10.).round() as u32
            },
            empty: amount <= 0.05,
            capacity,
            percents: std::array::from_fn(|i| {
                let p = if amount > 0. {
                    quantities[i] / amount * 100.
                } else {
                    0.
                };
                if p >= 1. { p.round() as u32 } else { u32::MAX }
            }),
            order,
        };
        if self.last_feedback == Some(stamp) {
            return;
        }
        self.last_feedback = Some(stamp);
        frame.changes.push(Change::Feedback(Box::new(Feedback {
            actions: stamp.actions,
            context: stamp.context,
            name: info.name.into(),
            notice: notice.into(),
            carrying: stamp.carrying,
            amount,
            capacity,
            has_name,
            no_text,
            fluid_name: fluid_name.into(),
            condition,
            held_name: held
                .map(|id| display_name(reference, &self.area.items, id))
                .unwrap_or("")
                .into(),
            throw_charge,
            throw_active,
            rpm: engine.rpm,
            ignition: current.ignition.ignition,
            running: engine.running,
            starter: engine.starter,
            seated,
            handbrake: current.handbrake,
            quantities,
            order: stamp.order,
            light_control: stamp.light_control,
            lmb_use: stamp.lmb_use,
            prompts: stamp.prompts,
            computer: stamp.computer,
        })));
    }
}
