//! Source player needs, toilet actions, and death timing.

use crate::fluids::FluidContainer;
use crate::reference::{ConsumptionEffect, NeedValues, SurvivalReference};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ToiletInput {
    pub urinate: bool,
    pub burp_held: bool,
    pub burp_pressed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ToiletOutput {
    pub burp: bool,
    pub fart: bool,
    pub defecated: bool,
    pub poured_liters: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeedsOutput {
    pub needs: NeedValues,
    pub bladder_liters: f32,
    pub can_eat: bool,
    pub opacity: f32,
    pub dead: bool,
    pub death_seconds: f64,
    pub look_scale: f32,
    pub menu_due: bool,
}

pub struct SurvivalState {
    needs: NeedValues,
    pub bladder: FluidContainer,
    pending: ConsumptionEffect,
    maximum: NeedValues,
    show_seconds: f32,
    hide_seconds: f32,
    death_duration: f32,
    elapsed: f64,
    stats_shown_at: f64,
    food_hurt: bool,
    water_hurt: bool,
    hp_hurt: bool,
    dead: bool,
    death_elapsed: f64,
    valve: f32,
    emptied_bladder: bool,
    burp_gate: f32,
    fart_gate: f32,
}

impl SurvivalState {
    pub fn new(reference: &SurvivalReference) -> Self {
        Self {
            needs: reference.initial,
            bladder: FluidContainer::new(
                reference.bladder_capacity_liters,
                &reference.initial_bladder,
            ),
            pending: reference.initial_queue,
            maximum: reference.maximum,
            show_seconds: reference.food_stat_show_time,
            hide_seconds: reference.food_stat_hide_time,
            death_duration: reference.death_seconds,
            elapsed: 0.,
            stats_shown_at: -10.,
            food_hurt: false,
            water_hurt: false,
            hp_hurt: false,
            dead: reference.initial.hp <= 0.,
            death_elapsed: 0.,
            valve: 0.,
            emptied_bladder: false,
            burp_gate: 0.,
            fart_gate: 0.,
        }
    }

    pub fn queue(&mut self, effect: ConsumptionEffect) {
        if ![effect.food, effect.water, effect.bowel, effect.hp]
            .iter()
            .all(|v| v.is_finite())
        {
            return;
        }
        self.pending.food += effect.food;
        self.pending.water += effect.water;
        self.pending.bowel += effect.bowel;
        self.pending.hp += effect.hp;
    }

    pub fn pending(&self) -> ConsumptionEffect {
        self.pending
    }

    pub fn capture_save(&self) -> crate::save::NeedsSnapshot {
        crate::save::NeedsSnapshot {
            needs: self.needs,
            pending: self.pending,
            bladder: crate::fluids::FluidSnapshot::from(&self.bladder),
        }
    }

    /// Source LoadPlayer restores needs and queues without showing stats.
    pub fn restore_save(&mut self, snapshot: &crate::save::NeedsSnapshot) {
        self.needs = snapshot.needs;
        self.pending = snapshot.pending;
        self.bladder.restore_entries(snapshot.bladder.as_slice());
        self.food_hurt = self.needs.food <= 0.;
        self.water_hurt = self.needs.water <= 0.;
        self.hp_hurt = self.needs.hp < self.maximum.hp * 0.25;
        self.dead = self.needs.hp <= 0.;
        self.death_elapsed = 0.;
    }

    pub fn show_stats(&mut self) {
        self.stats_shown_at = self.elapsed;
    }

    /// Fractions use the source order: gas, oil, alcohol, water, blood, diesel.
    pub fn apply_drink(
        &mut self,
        reference: &SurvivalReference,
        fractions: [f32; 6],
        requested_liters: f32,
    ) {
        if !requested_liters.is_finite()
            || requested_liters <= 0.
            || !fractions.iter().all(|v| v.is_finite() && *v >= 0.)
        {
            return;
        }
        self.pending.water += requested_liters * fractions[3];
        self.pending.hp += requested_liters
            * (fractions[4] * reference.blood_to_hp_ratio + fractions[2] * reference.alcohol_heal
                - (fractions[0] + fractions[1] + fractions[5]) * reference.car_fluids_damage);
        self.convert_water_to_bladder(
            reference,
            requested_liters * fractions[2] * reference.alcohol_convert_to_piss,
        );
        self.show_stats();
    }

    pub fn damage_instant(&mut self, reference: &SurvivalReference, amount: f32) {
        if !amount.is_finite() || self.dead {
            return;
        }
        self.needs.hp = (self.needs.hp - amount).clamp(0., reference.maximum.hp);
        if self.needs.hp <= 0. {
            self.dead = true;
            self.death_elapsed = 0.;
        }
    }

    pub fn advance(
        &mut self,
        reference: &SurvivalReference,
        dt: f32,
        temperature: f32,
        need_scale: f32,
    ) -> NeedsOutput {
        if !dt.is_finite()
            || dt < 0.
            || !temperature.is_finite()
            || !need_scale.is_finite()
            || need_scale < 0.
        {
            return self.output();
        }
        self.elapsed += f64::from(dt);
        if self.dead {
            self.death_elapsed += f64::from(dt);
        }
        let step = dt.min(1.);
        let cold = inverse_lerp(reference.def_temp, reference.cold_temp, temperature);
        let hot = inverse_lerp(reference.def_temp, reference.hot_temp, temperature);
        let food_loss = reference.food_loss
            * need_scale
            * lerp(
                1.,
                reference.cold_food_loss_modifier,
                cold * lerp(1., reference.hot_food_loss_modifier, hot),
            );
        // The recovered update uses foodLoss for water as well.
        let water_loss = reference.food_loss
            * need_scale
            * lerp(
                1.,
                reference.cold_water_loss_modifier,
                cold * lerp(1., reference.hot_water_loss_modifier, hot),
            );
        self.needs.food = (self.needs.food - food_loss * step).clamp(0., reference.maximum.food);
        self.needs.bowel += food_loss * reference.food_to_shit_ratio * step;
        self.convert_water_to_bladder(reference, water_loss * step);
        if !self.food_hurt && self.needs.food <= 0. {
            self.food_hurt = true;
            self.show_stats();
        }
        if !self.water_hurt && self.needs.water <= 0. {
            self.water_hurt = true;
            self.show_stats();
        }
        if self.hp_hurt {
            if self.needs.hp > reference.maximum.hp * 0.25 {
                self.hp_hurt = false;
            }
        } else if self.needs.hp < reference.maximum.hp * 0.25 {
            self.hp_hurt = true;
            self.show_stats();
        }
        let mut damage = 0.;
        if self.needs.water <= 0. {
            damage += reference.water_damage;
        } else {
            self.water_hurt = false;
        }
        if self.needs.food <= 0. {
            damage += reference.food_damage;
        } else {
            self.food_hurt = false;
        }
        let fullness = inverse_lerp(reference.maximum.bowel, 0., self.needs.bowel).min(
            inverse_lerp(self.bladder.capacity_liters, 0., self.bladder.total()),
        );
        let heal = inverse_lerp(0., reference.maximum.water, self.needs.water).min(inverse_lerp(
            0.,
            reference.maximum.food,
            self.needs.food,
        )) * lerp(
            reference.max_healing_speed * reference.healing_toilet_full_modfiier,
            reference.max_healing_speed,
            fullness,
        );
        self.damage_instant(reference, damage * step * need_scale);
        if !self.dead {
            self.needs.hp =
                (self.needs.hp + heal * step * need_scale).clamp(0., reference.maximum.hp);
        }
        self.needs.bowel = self.needs.bowel.clamp(0., reference.maximum.bowel);

        let fraction = (dt * reference.delay_speed).min(1.);
        let applied = ConsumptionEffect {
            food: self.pending.food * fraction,
            water: self.pending.water * fraction,
            bowel: self.pending.bowel * fraction,
            hp: self.pending.hp * fraction,
        };
        self.pending.food -= applied.food;
        self.pending.water -= applied.water;
        self.pending.bowel -= applied.bowel;
        self.pending.hp -= applied.hp;
        self.eat_instant(reference, applied);
        self.output()
    }

    pub fn toilet_step(
        &mut self,
        reference: &SurvivalReference,
        input: ToiletInput,
        dt: f32,
        draws: [f32; 2],
        gate_seconds: f32,
        fluid_scale: f32,
    ) -> ToiletOutput {
        if !dt.is_finite()
            || dt < 0.
            || !fluid_scale.is_finite()
            || fluid_scale < 0.
            || !gate_seconds.is_finite()
            || gate_seconds < 0.
            || !draws.iter().all(|v| v.is_finite() && *v >= 0. && *v <= 1.)
        {
            return ToiletOutput::default();
        }
        self.burp_gate = (self.burp_gate - dt).max(0.);
        self.fart_gate = (self.fart_gate - dt).max(0.);
        let mut result = ToiletOutput::default();
        let target = if input.urinate && !self.emptied_bladder {
            1.
        } else {
            0.
        };
        self.valve = lerp(
            self.valve,
            target,
            (dt * reference.toilet.valve_gain).clamp(0., 1.),
        );
        if input.urinate {
            self.show_stats();
            if self.bladder.total() <= reference.toilet.empty_amount {
                self.emptied_bladder = true;
            }
        } else {
            self.emptied_bladder = false;
        }
        if self.valve > 0. && self.bladder.total() > 0. {
            let area =
                reference.toilet.radius * reference.toilet.radius * std::f32::consts::PI * 0.025;
            let poured = area * self.valve * 2. * dt * fluid_scale;
            self.bladder.change_without_mix(-poured);
            self.bladder.cleanup_one();
            result.poured_liters = poured;
        }
        if input.burp_held {
            self.show_stats();
        }
        if input.burp_pressed {
            if draws[0] < reference.toilet.fart_burp_ratio {
                if self.fart_gate <= 0. {
                    if self.needs.bowel >= reference.maximum.bowel * reference.toilet.bowel_fraction
                        && draws[1] > reference.toilet.fart_shit_ratio
                    {
                        self.needs.bowel -=
                            reference.maximum.bowel * reference.toilet.bowel_fraction;
                        result.defecated = true;
                    } else {
                        result.fart = true;
                    }
                    self.fart_gate = gate_seconds;
                }
            } else if self.burp_gate <= 0. {
                result.burp = true;
                self.burp_gate = gate_seconds;
            }
        }
        result
    }

    pub fn output(&self) -> NeedsOutput {
        let age = (self.elapsed - self.stats_shown_at) as f32;
        let opacity = if age >= self.show_seconds {
            0.
        } else if age < self.hide_seconds {
            1.
        } else {
            inverse_lerp(
                self.show_seconds,
                self.show_seconds - self.hide_seconds,
                age,
            )
        };
        let progress = if self.dead {
            (self.death_elapsed as f32 / self.death_duration).clamp(0., 1.)
        } else {
            0.
        };
        NeedsOutput {
            needs: self.needs,
            bladder_liters: self.bladder.total(),
            can_eat: !self.dead && self.needs.bowel + self.pending.bowel < self.maximum.bowel,
            opacity,
            dead: self.dead,
            death_seconds: self.death_elapsed,
            look_scale: if self.dead {
                lerp(0.8, 0., progress)
            } else {
                1.
            },
            menu_due: self.dead && progress >= 1.,
        }
    }

    fn convert_water_to_bladder(&mut self, reference: &SurvivalReference, amount: f32) {
        self.needs.water = (self.needs.water - amount).clamp(0., reference.maximum.water);
        let addition = (amount * reference.water_to_piss_ratio)
            .min((self.bladder.capacity_liters - self.bladder.total()).max(0.));
        self.bladder.change_one(addition, 3);
    }

    fn eat_instant(&mut self, reference: &SurvivalReference, effect: ConsumptionEffect) {
        let free_food = reference.maximum.food - self.needs.food;
        if effect.food <= free_food {
            self.needs.food += effect.food;
            self.needs.bowel = (self.needs.bowel + effect.bowel).clamp(0., reference.maximum.bowel);
        } else {
            self.needs.food = reference.maximum.food;
            self.needs.bowel = (self.needs.bowel + effect.bowel + effect.food - free_food)
                .clamp(0., reference.maximum.bowel);
        }
        let to_water = (reference.maximum.water - self.needs.water).min(effect.water);
        self.needs.water = (self.needs.water + to_water).clamp(0., reference.maximum.water);
        self.bladder.change_without_mix(effect.water - to_water);
        if effect.hp > 0. {
            if !self.dead {
                self.needs.hp = (self.needs.hp + effect.hp).clamp(0., reference.maximum.hp);
            }
        } else if effect.hp < 0. {
            self.damage_instant(reference, -effect.hp);
        }
        if effect.hp.abs() > 0.01 {
            self.show_stats();
        }
    }
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    ((value - a) / (b - a)).clamp(0., 1.)
}

fn lerp(a: f32, b: f32, fraction: f32) -> f32 {
    a + (b - a) * fraction.clamp(0., 1.)
}

#[cfg(test)]
mod source_rules {
    use super::*;
    use crate::reference::NeedValues;

    const GAS: usize = 0;
    const OIL: usize = 1;
    const ALCOHOL: usize = 2;
    const WATER: usize = 3;
    const BLOOD: usize = 4;

    fn reference() -> SurvivalReference {
        let id = serde_json::json!({"file": "level1", "path_id": 1});
        let evidence = serde_json::json!({"assembly_sha256": "", "methods": []});
        let needs = serde_json::json!({"hp": 100., "food": 100., "water": 100., "bowel": 100.});
        let zero = serde_json::json!({"food": 0., "water": 0., "bowel": 0., "hp": 0.});
        serde_json::from_value(serde_json::json!({
            "mode_source": id, "mode": 2, "death_seconds": 14.,
            "drinking": {
                "source": id, "tank": id, "collider": id,
                "geometry": {"kind": "sphere", "center": [0., 0., 0.], "radius": 0.1},
                "local_position": [0., 0., 0.], "radius": 0.1, "valve": 1.,
                "no_angle": false, "no_pour": false, "is_drink_cap": true,
                "particle_contact_radius_multiplier": 1., "evidence": evidence
            },
            "source": id, "player": id, "blood": id,
            "initial": {"hp": 100., "food": 75., "water": 60., "bowel": 50.},
            "maximum": needs, "initial_bladder": [], "bladder_capacity_liters": 1.,
            "initial_queue": zero, "starter_food": 0, "foods": [], "starter_parts": [],
            "starter_parent": id,
            "starter_pose": {"translation": [0., 0., 0.], "rotation": [0., 0., 0., 1.], "scale": [1., 1., 1.]},
            "evidence": evidence,
            "toilet": {
                "source": id, "bladder": id, "radius": 0.01, "no_angle": false,
                "valve_gain": 3., "empty_amount": 0.01, "bowel_fraction": 0.3,
                "fart_burp_ratio": 0.5, "fart_shit_ratio": 0.5,
                "min_fart_pitch": 1., "max_fart_pitch": 1., "min_fart_volume": 1., "max_fart_volume": 1.,
                "min_burp_pitch": 1., "max_burp_pitch": 1., "min_burp_volume": 1., "max_burp_volume": 1.,
                "max_rate": 1., "min_rate": 0., "max_speed": 1., "force_color": [1., 1., 1., 1.], "force_color_m": 1.
            },
            "inputs": [], "drink_modifier": 1., "death_velocity_change": 20.,
            "food_stat_show_time": 5., "food_stat_hide_time": 1.,
            "water_to_piss_ratio": 0.01, "food_to_shit_ratio": 0.5, "blood_to_hp_ratio": 10.,
            "alcohol_heal": 20., "alcohol_convert_to_piss": 30., "car_fluids_damage": 40.,
            "max_healing_speed": 0.2, "healing_toilet_full_modfiier": 0.25,
            "water_damage": 0.5, "food_damage": 0.25,
            "def_temp": 20., "cold_temp": 0., "hot_temp": 40.,
            "food_loss": 0.1, "cold_food_loss_modifier": 2., "hot_food_loss_modifier": 3.,
            "water_loss": 0.7, "cold_water_loss_modifier": 4., "hot_water_loss_modifier": 5.,
            "run_food_loss_modifier": 1., "run_water_loss_modifier": 1., "delay_speed": 1.
        }))
        .unwrap()
    }

    fn with_needs(reference: &SurvivalReference, needs: NeedValues) -> SurvivalState {
        let mut state = SurvivalState::new(reference);
        state.restore_save(&crate::save::NeedsSnapshot {
            needs,
            pending: ConsumptionEffect::default(),
            bladder: crate::fluids::FluidSnapshot::default(),
        });
        state
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn needs_drop_at_the_source_rate_and_water_uses_food_loss() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 100.,
                food: 50.,
                water: 50.,
                bowel: 0.,
            },
        );
        let out = state.advance(&r, 1., r.def_temp, 1.);
        assert!(close(out.needs.food, 50. - r.food_loss));
        // currentWaterLoss = foodLoss * ...; waterLoss is never read by the source.
        assert!(close(out.needs.water, 50. - r.food_loss));
        assert!(close(out.needs.bowel, r.food_loss * r.food_to_shit_ratio));
        assert!(close(
            out.bladder_liters,
            r.food_loss * r.water_to_piss_ratio
        ));
    }

    #[test]
    fn cold_raises_the_loss_rate() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 100.,
                food: 50.,
                water: 50.,
                bowel: 0.,
            },
        );
        let out = state.advance(&r, 1., r.cold_temp, 1.);
        assert!(close(
            out.needs.food,
            50. - r.food_loss * r.cold_food_loss_modifier
        ));
        assert!(close(
            out.needs.water,
            50. - r.food_loss * r.cold_water_loss_modifier
        ));
    }

    #[test]
    fn starving_and_thirst_damage_until_death() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 1.,
                food: 0.,
                water: 0.,
                bowel: 0.,
            },
        );
        let out = state.advance(&r, 1., r.def_temp, 1.);
        assert!(close(out.needs.hp, 1. - r.food_damage - r.water_damage));
        assert!(!out.dead);
        let out = state.advance(&r, 1., r.def_temp, 1.);
        assert!(out.dead);
        assert_eq!(out.needs.hp, 0.);
        assert!(!out.can_eat);
    }

    #[test]
    fn healing_slows_when_the_toilet_is_full() {
        let r = reference();
        let mut empty = with_needs(
            &r,
            NeedValues {
                hp: 50.,
                food: 100.,
                water: 100.,
                bowel: 0.,
            },
        );
        let mut full = with_needs(
            &r,
            NeedValues {
                hp: 50.,
                food: 100.,
                water: 100.,
                bowel: 100.,
            },
        );
        let healed = empty.advance(&r, 1., r.def_temp, 0.).needs.hp;
        assert_eq!(healed, 50., "sleep scale 0 stops healing");
        let healed = empty.advance(&r, 0.1, r.def_temp, 1.).needs.hp - 50.;
        let slowed = full.advance(&r, 0.1, r.def_temp, 1.).needs.hp - 50.;
        assert!(close(healed, r.max_healing_speed * 0.1));
        assert!(close(
            slowed,
            r.max_healing_speed * r.healing_toilet_full_modfiier * 0.1
        ));
    }

    #[test]
    fn alcohol_heals_and_turns_water_into_urine() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 50.,
                food: 100.,
                water: 100.,
                bowel: 0.,
            },
        );
        let mut fractions = [0.; 6];
        fractions[ALCOHOL] = 1.;
        state.apply_drink(&r, fractions, 0.01);
        assert!(close(state.pending().hp, 0.01 * r.alcohol_heal));
        assert!(close(state.pending().water, 0.));
        let out = state.output();
        assert!(close(
            out.needs.water,
            100. - 0.01 * r.alcohol_convert_to_piss
        ));
        assert!(close(
            out.bladder_liters,
            0.01 * r.alcohol_convert_to_piss * r.water_to_piss_ratio
        ));
    }

    #[test]
    fn car_fluids_hurt_and_blood_heals() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 50.,
                food: 100.,
                water: 100.,
                bowel: 0.,
            },
        );
        let mut fractions = [0.; 6];
        fractions[GAS] = 0.5;
        fractions[OIL] = 0.25;
        fractions[BLOOD] = 0.25;
        state.apply_drink(&r, fractions, 0.1);
        let expected = 0.1 * (0.25 * r.blood_to_hp_ratio - 0.75 * r.car_fluids_damage);
        assert!(close(state.pending().hp, expected));
        // The queue drains at delaySpeed per second, like source toHp.
        let out = state.advance(&r, 0.5, r.def_temp, 0.);
        assert!(close(out.needs.hp, 50. + expected * 0.5));
        assert!(close(state.pending().hp, expected * 0.5));
    }

    #[test]
    fn water_past_full_goes_to_the_bladder() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 100.,
                food: 100.,
                water: 99.,
                bowel: 0.,
            },
        );
        let mut fractions = [0.; 6];
        fractions[WATER] = 1.;
        // ChangeWithoutMix keeps the bladder's own mixture, so it needs fluid to grow.
        state.bladder.change_one(0.1, WATER);
        state.apply_drink(&r, fractions, 1.5);
        let out = state.advance(&r, 1., r.def_temp, 0.);
        assert!(close(out.needs.water, 100.));
        assert!(close(out.bladder_liters, 0.6));
    }

    #[test]
    fn saved_needs_round_trip() {
        let r = reference();
        let mut state = with_needs(
            &r,
            NeedValues {
                hp: 20.,
                food: 30.,
                water: 40.,
                bowel: 50.,
            },
        );
        state.queue(ConsumptionEffect {
            food: 1.,
            water: 2.,
            bowel: 3.,
            hp: 4.,
        });
        state.bladder.change_one(0.25, WATER);
        let saved = state.capture_save();
        let json = serde_json::to_string(&saved).unwrap();
        let mut loaded = SurvivalState::new(&r);
        loaded.restore_save(&serde_json::from_str(&json).unwrap());
        assert_eq!(loaded.capture_save(), saved);
        assert!(!loaded.output().dead);
        assert_eq!(loaded.output().opacity, 0., "loading does not flash stats");
    }
}
