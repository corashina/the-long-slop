//! Test-only source loot rates. Expected item counts per building spawn, from the
//! decompiled source rules as probabilities. It does not use the draw-based port code.
use super::SpawnContext;
use super::spawn_rules::*;
use crate::reference::{ReferenceError, SourceId, WorldCategory, WorldReference, WorldValue};
use std::collections::HashMap;

/// Expected counts for one building spawn.
#[derive(Debug, Default)]
pub(super) struct Expected {
    /// Spawned items by catalog index.
    pub items: HashMap<u32, f64>,
    /// Spawned prototypes that no catalog item names, such as row `G` prefabs.
    pub unlisted: HashMap<SourceId, f64>,
    /// Selected catalog items without a prototype, by exclusion reason.
    pub dropped: HashMap<String, f64>,
}

/// Expected loot of one spawn of the building `prototype`, in the `contents.rs` caller order.
pub(super) fn expected(
    reference: &WorldReference,
    rules: &SourceRules,
    prototype: &SourceId,
    context: SpawnContext,
) -> Result<Expected, ReferenceError> {
    let mut walk = Walk {
        reference,
        rules,
        context,
        out: Expected::default(),
    };
    let building = rules
        .component(reference, prototype, "buildingscript")?
        .ok_or_else(|| ReferenceError("Audit prototype is not a building".into()))?;
    let f = building.fields;
    // buildingscript: randomSpawn = Random.Range(minRandomSpawn, maxRandomSpawn).
    let (low, high) = (number(f, "minRandomSpawn")?, number(f, "maxRandomSpawn")?);
    let random_spawn = (low != 0. || high != 0.).then(|| building_gate(low as f64, high as f64));
    let mut underground = Vec::new();
    rules.children(
        reference,
        prototype,
        "undergroundSpawnScript",
        true,
        &mut underground,
    )?;
    for caller in underground {
        walk.underground(caller)?;
    }
    let mut started = Vec::new();
    for save in array(f, "toStart")? {
        if let Ok(save) = value_link(save)
            && !started.contains(&save)
        {
            started.push(save);
            walk.start_save(save, 1., 0)?;
        }
    }
    let mut callers = Vec::new();
    rules.children(
        reference,
        prototype,
        "newRandomStuffSpawnScript",
        true,
        &mut callers,
    )?;
    for caller in callers {
        let f = rules.get(reference, caller)?.fields;
        if flag(f, "dontSpawn")? {
            continue;
        }
        let gate = if flag(f, "buildingAffectsThis")? {
            random_spawn
        } else {
            None
        };
        walk.spawn(caller, gate, 1., 0)?;
    }
    Ok(walk.out)
}

/// Source gate `!(Random.Range(0.001f, 1f) > chance)` with a uniform float draw.
fn spawn_gate(chance: f64) -> f64 {
    ((chance - 0.001) / 0.999).clamp(0., 1.)
}

/// `spawn_gate` averaged over a chance that is uniform in [low, high].
fn building_gate(low: f64, high: f64) -> f64 {
    if low == high {
        return spawn_gate(low);
    }
    // Antiderivative of spawn_gate.
    let integral = |c: f64| {
        if c <= 0.001 {
            0.
        } else if c <= 1. {
            (c - 0.001).powi(2) / (2. * 0.999)
        } else {
            0.999 / 2. + (c - 1.)
        }
    };
    (integral(high) - integral(low)) / (high - low)
}

/// Mean of `Random.Range(min, max)` for ints: max is exclusive, and reversed bounds mirror it.
fn int_range_mean(min: i32, max: i32) -> f64 {
    match min.cmp(&max) {
        std::cmp::Ordering::Equal => min as f64,
        std::cmp::Ordering::Less => (min + max - 1) as f64 / 2.,
        std::cmp::Ordering::Greater => (min + max + 1) as f64 / 2.,
    }
}

/// SelectSpawn: `num = Random.Range(0, ci + 1)`; the first row with `num <= chance` that is not
/// skipped wins. Returns the chance of each row and the chance that no row wins.
fn rows(weights: &[i32], skip: &[bool]) -> (Vec<f64>, f64) {
    let total: i32 = weights.iter().sum();
    let values = (total + 1) as f64;
    let mut chances = vec![0.; weights.len()];
    let mut none = 0.;
    let mut low = 0;
    let mut cumulative = 0;
    for (i, w) in weights.iter().enumerate() {
        cumulative += w;
        // Draws in [low, cumulative] stop at the first row from i that is not skipped.
        let share = (cumulative - low + 1).max(0) as f64 / values;
        match (i..weights.len()).find(|j| !skip[*j]) {
            Some(j) => chances[j] += share,
            None => none += share,
        }
        low = low.max(cumulative + 1);
    }
    (chances, none)
}

/// selectId(categoryId) as item index chances. A missing branch array selects nothing.
fn select_id(
    categories: &[WorldCategory],
    index: usize,
    current_car: i32,
    depth: usize,
) -> Result<HashMap<usize, f64>, ReferenceError> {
    let mut out = HashMap::new();
    if depth > categories.len() {
        return Err(ReferenceError("Audit category cycle".into()));
    }
    let Some(category) = categories.get(index) else {
        return Ok(out);
    };
    let sum = |values: &Option<Vec<u32>>, weights: &Option<Vec<u32>>| {
        weights
            .as_ref()
            .map_or(values.as_ref().map_or(0, Vec::len) as i32, |w| {
                w.iter().map(|n| *n as i32).sum()
            })
    };
    let item_total = sum(&category.items, &category.item_chances);
    let category_total = sum(&category.categories, &category.category_chances);
    // num3 = Random.Range(1, num + num2 + 1); equal bounds return 1.
    let draws = (item_total + category_total).max(1);
    // Group draws by outcome first, so each subcategory recurses once.
    let mut outcomes: HashMap<(bool, usize), f64> = HashMap::new();
    for draw in 1..=draws {
        let p = 1. / draws as f64;
        let (values, weights, recursive) = if draw <= item_total {
            (&category.items, &category.item_chances, false)
        } else {
            (&category.categories, &category.category_chances, true)
        };
        let Some(values) = values else { continue };
        if category.depend_on_car {
            // The category branch returns the category index as an item index.
            for (i, q) in depend_on_car(values.len(), current_car)
                .into_iter()
                .enumerate()
            {
                *outcomes.entry((false, values[i] as usize)).or_default() += p * q;
            }
            continue;
        }
        let mut cumulative = 0;
        // No cumulative matches: selectId returns 0.
        let mut selected = (false, 0);
        for (i, value) in values.iter().enumerate() {
            cumulative += weights.as_ref().map_or(1, |w| w[i] as i32);
            // The source compares the raw draw, without subtracting num, in the category branch.
            if draw <= cumulative {
                selected = (recursive, *value as usize);
                break;
            }
        }
        *outcomes.entry(selected).or_default() += p;
    }
    for ((recursive, value), p) in outcomes {
        if recursive {
            for (item, q) in select_id(categories, value, current_car, depth + 1)? {
                *out.entry(item).or_default() += p * q;
            }
        } else {
            *out.entry(value).or_default() += p;
        }
    }
    Ok(out)
}

/// dependOnCar: 25% the current car index if it is in range, else up to 10 uniform rerolls
/// that stop at the first index that is not the current car.
fn depend_on_car(len: usize, current_car: i32) -> Vec<f64> {
    let car = usize::try_from(current_car).ok().filter(|c| *c < len);
    let mut out = vec![0.; len];
    let Some(car) = car else {
        out.iter_mut().for_each(|v| *v = 1. / len as f64);
        return out;
    };
    let stay = (1. / len as f64).powi(10);
    for (i, v) in out.iter_mut().enumerate() {
        *v = 0.75
            * if i == car {
                stay
            } else {
                (1. - stay) / (len - 1) as f64
            };
    }
    out[car] += 0.25;
    out
}

/// undergroundSpawnScript and spawnInBoxScript pools: float weights, the first cumulative above
/// a draw in [0, total).
fn pool(weights: &[f64]) -> Vec<f64> {
    let total: f64 = weights.iter().sum();
    weights
        .iter()
        .map(|w| if total > 0. { w.max(0.) / total } else { 0. })
        .collect()
}

struct Walk<'a> {
    reference: &'a WorldReference,
    rules: &'a SourceRules,
    context: SpawnContext,
    out: Expected,
}
impl Walk<'_> {
    fn named(&self, name: &str, is_item: bool) -> Result<HashMap<usize, f64>, ReferenceError> {
        let catalog = &self.reference.catalog;
        if is_item {
            return Ok(catalog
                .items
                .iter()
                .position(|i| i.name == name)
                .map(|i| (i, 1.))
                .into_iter()
                .collect());
        }
        match catalog.categories.iter().position(|c| c.name == name) {
            Some(i) => select_id(&catalog.categories, i, self.context.current_car, 0),
            None => Ok(HashMap::new()),
        }
    }
    fn pool(&self, rows: &[WorldValue]) -> Result<HashMap<usize, f64>, ReferenceError> {
        let weights = rows
            .iter()
            .map(|r| Ok(number(fields(r)?, "chance")? as f64))
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let mut out = HashMap::new();
        for (row, p) in rows.iter().zip(pool(&weights)) {
            let row = fields(row)?;
            let named = self.named(
                text(row, "category")?,
                flag(row, "itemNameInsteadOfCategory")?,
            )?;
            for (item, q) in named {
                *out.entry(item).or_default() += p * q;
            }
        }
        Ok(out)
    }
    /// Counts a published placement of `root`.
    fn publish(&mut self, root: &SourceId, m: f64) {
        let items = &self.reference.catalog.items;
        match items
            .iter()
            .position(|i| i.prototype.as_ref() == Some(root))
        {
            Some(i) => *self.out.items.entry(i as u32).or_default() += m,
            None => *self.out.unlisted.entry(root.clone()).or_default() += m,
        }
    }
    /// Records a selected catalog item without a prototype. Returns true when it has one.
    fn listed(&mut self, item: usize, m: f64) -> bool {
        let item = &self.reference.catalog.items[item];
        if item.prototype.is_none() {
            *self.out.dropped.entry(item.exclusion.clone()).or_default() += m;
        }
        item.prototype.is_some()
    }
    /// newRandomStuffSpawnScript.Spawn with `gate` as the chanceToSpawn override chance.
    fn spawn(
        &mut self,
        caller: &SourceId,
        gate: Option<f64>,
        m: f64,
        depth: usize,
    ) -> Result<(), ReferenceError> {
        if depth > 64 {
            return Err(ReferenceError("Audit spawn recursion".into()));
        }
        let f = self.rules.get(self.reference, caller)?.fields;
        let [_, month, day] = self.context.date;
        let holiday = match integer(f, "holiday")? {
            0 => true,
            1 => (month == 10 && day >= 15) || (month == 11 && day <= 15),
            2 => (month == 12 && day >= 15) || (month == 1 && day <= 15),
            _ => false,
        };
        if !holiday {
            return Ok(());
        }
        let m = m * gate.map_or_else(|| Ok(spawn_gate(number(f, "chanceToSpawn")? as f64)), Ok)?;
        let items = array(f, "items")?;
        let mut weights = Vec::new();
        let mut skip = Vec::new();
        for row in items {
            let row = fields(row)?;
            let n = integer(row, "chance")?;
            weights.push(if n == 0 { 1 } else { n });
            skip.push(self.context.peaceful_enemies == 2 && text(row, "prefab")? == "humanmonster");
        }
        let (chances, none) = rows(&weights, &skip);
        for (row, p) in items.iter().zip(chances) {
            self.row(fields(row)?, true, m * p, depth)?;
        }
        // The port keeps row 0 when no row wins; only its G target spawns.
        if let Some(row) = items.first() {
            self.row(fields(row)?, false, m * none, depth)?;
        }
        Ok(())
    }
    /// SpawnItem: the row G prefab, else the selected item or category.
    fn row(
        &mut self,
        row: Fields,
        selected: bool,
        m: f64,
        depth: usize,
    ) -> Result<(), ReferenceError> {
        if m == 0. {
            return Ok(());
        }
        if let Some(target) = optional(row, "G")? {
            return self.place(target, m, depth);
        }
        if !selected {
            return Ok(());
        }
        for (item, q) in self.named(text(row, "prefab")?, flag(row, "isitem")?)? {
            self.listed(item, m * q);
            let source = &self.reference.catalog.items[item].source;
            self.place(source, m * q, depth)?;
        }
        Ok(())
    }
    /// One instantiated prefab: nested spawners, then its item and box contents.
    fn place(&mut self, target: &SourceId, m: f64, depth: usize) -> Result<(), ReferenceError> {
        let (reference, rules) = (self.reference, self.rules);
        let root = rules.root(reference, target)?;
        if let Some(spawner) = rules.component(reference, target, "newRandomStuffSpawnScript")? {
            return self.spawn(spawner.source, None, m, depth + 1);
        }
        let mut nested = Vec::new();
        rules.children(
            reference,
            root,
            "newRandomStuffSpawnScript",
            false,
            &mut nested,
        )?;
        for spawner in nested {
            self.spawn(spawner, None, m, depth + 1)?;
        }
        let actual = match rules.component(reference, target, "childunparent")? {
            Some(unparent) => link(unparent.fields, "g")?,
            None => target,
        };
        if let Some(save) = rules.component(reference, actual, "tosaveitemscript")? {
            self.start_save(save.source, m, 0)?;
            if reference.prototypes.iter().any(|p| &p.source == root) {
                self.publish(root, m);
            }
        }
        Ok(())
    }
    /// tosaveitemscript start: attached part saves and spawnInBoxScript contents.
    fn start_save(&mut self, save: &SourceId, m: f64, depth: usize) -> Result<(), ReferenceError> {
        if depth > 64 {
            return Err(ReferenceError("Audit save recursion".into()));
        }
        let (reference, rules) = (self.reference, self.rules);
        let f = rules.get(reference, save)?.fields;
        if flag(f, "started")? {
            return Ok(());
        }
        for slot in array(f, "partslotscripts")? {
            let Ok(slot) = value_link(slot) else { continue };
            if let Some(part) = optional(rules.get(reference, slot)?.fields, "part")?
                && let Some(save) = optional(rules.get(reference, part)?.fields, "tosaveitem")?
            {
                self.start_save(save, m, depth + 1)?;
            }
        }
        let Some(spawn_box) = optional(f, "spawnBox")? else {
            return Ok(());
        };
        let f = rules.get(reference, spawn_box)?.fields;
        if flag(f, "disableSpawn")? || !flag(f, "spawnBeforeOpen")? || flag(f, "spawned")? {
            return Ok(());
        }
        for (item, q) in self.pool(array(f, "spawn")?)? {
            self.listed(item, m * q);
            let root = rules.root(reference, &reference.catalog.items[item].source)?;
            let count = match rules.component(reference, root, "massScript")? {
                Some(mass) => box_mean(integer(mass.fields, "maxCountIfSpawnedInBox")?),
                None => 1.,
            };
            if let Some(save) = rules.component(reference, root, "tosaveitemscript")? {
                self.start_save(save.source, m * q * count, depth + 1)?;
                if reference.prototypes.iter().any(|p| &p.source == root) {
                    self.publish(root, m * q * count);
                }
            }
        }
        Ok(())
    }
    /// undergroundSpawnScript: `Random.Range(0f, 1f) < chance`, then minCount..=maxCount pool picks.
    fn underground(&mut self, caller: &SourceId) -> Result<(), ReferenceError> {
        let (reference, rules) = (self.reference, self.rules);
        let f = rules.get(reference, caller)?.fields;
        let m = (number(f, "chance")? as f64).clamp(0., 1.)
            * int_range_mean(integer(f, "minCount")?, integer(f, "maxCount")? + 1);
        for (item, q) in self.pool(array(f, "spawn")?)? {
            let listed = self.listed(item, m * q);
            let source = &reference.catalog.items[item].source;
            if let Some(save) = rules.component(reference, source, "tosaveitemscript")? {
                self.start_save(save.source, m * q, 0)?;
                if listed {
                    self.publish(rules.root(reference, source)?, m * q);
                }
            }
        }
        Ok(())
    }
}

/// spawnInBoxScript copies: `Random.Range(1, max + 1)` when max > 0, else 1.
fn box_mean(max: i32) -> f64 {
    if max > 0 { (1 + max) as f64 / 2. } else { 1. }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
    fn category(
        items: &[u32],
        chances: Option<&[u32]>,
        categories: &[u32],
        car: bool,
    ) -> WorldCategory {
        WorldCategory {
            invalid_category_indices: Vec::new(),
            initialized: true,
            index: 0,
            name: String::new(),
            items: (!items.is_empty()).then(|| items.to_vec()),
            categories: (!categories.is_empty()).then(|| categories.to_vec()),
            item_chances: chances.map(<[u32]>::to_vec),
            category_chances: None,
            depend_on_car: car,
        }
    }

    #[test]
    fn row_zero_gets_one_extra_value() {
        // ci = 4, draws 0..=4: row 0 wins 0..=2, row 1 wins 3..=4.
        let (chances, none) = rows(&[2, 2], &[false, false]);
        close(chances[0], 3. / 5.);
        close(chances[1], 2. / 5.);
        close(none, 0.);
    }

    #[test]
    fn humanmonster_skip_moves_to_next_row() {
        let (chances, none) = rows(&[1, 1, 1], &[false, true, false]);
        close(chances[0], 2. / 4.);
        close(chances[2], 2. / 4.);
        let (chances, none_last) = rows(&[1, 1], &[false, true]);
        close(chances[0], 2. / 3.);
        close(none + none_last, 1. / 3.);
    }

    #[test]
    fn category_branch_compares_raw_draw() {
        // Root: items [10] (total 1), categories [1, 2] (total 2). Draws 1..=3.
        // Draw 1: item 10. Draw 2: category 2 (cumulative 2). Draw 3: no match, item 0.
        let categories = [
            category(&[10], None, &[1, 2], false),
            category(&[11], None, &[], false),
            category(&[12], None, &[], false),
        ];
        let out = select_id(&categories, 0, 0, 0).unwrap();
        close(out[&10], 1. / 3.);
        close(out[&12], 1. / 3.);
        close(out[&0], 1. / 3.);
        assert!(!out.contains_key(&11));
    }

    #[test]
    fn depend_on_car_prefers_current_car() {
        let categories = [category(&[5, 6], None, &[], true)];
        let out = select_id(&categories, 0, 1, 0).unwrap();
        let stay = 0.5f64.powi(10);
        close(out[&6], 0.25 + 0.75 * stay);
        close(out[&5], 0.75 * (1. - stay));
        // Out of range: uniform.
        let out = select_id(&categories, 0, 7, 0).unwrap();
        close(out[&5], 0.5);
        close(out[&6], 0.5);
    }

    #[test]
    fn building_gate_integrates_uniform_chance() {
        close(spawn_gate(0.001), 0.);
        close(spawn_gate(1.5), 1.);
        // Fully inside [0.001, 1]: the gate of the mean.
        close(building_gate(0.2, 0.6), spawn_gate(0.4));
        // Half the range above 1: mean of spawn_gate(0.75) and 1.
        close(building_gate(0.5, 1.5), (spawn_gate(0.75) + 1.) / 2.);
        close(building_gate(0.5, 0.5), spawn_gate(0.5));
        close(int_range_mean(2, 5), 3.);
        close(box_mean(10), 5.5);
    }

    fn reference() -> &'static WorldReference {
        static REFERENCE: std::sync::OnceLock<crate::reference::ReferenceManifest> =
            std::sync::OnceLock::new();
        &REFERENCE
            .get_or_init(|| {
                crate::reference::ReferenceManifest::from_runtime_json(
                    &std::fs::read("web/assets/reference/manifest.json").unwrap(),
                )
                .unwrap()
            })
            .world
    }

    #[test]
    #[ignore = "requires the installed source export"]
    fn print_expected_loot() {
        let start = std::time::Instant::now();
        let reference = reference();
        let rules = SourceRules::new(reference);
        println!("loaded in {:?}", start.elapsed());
        let context = SpawnContext {
            current_car: 1,
            peaceful_enemies: 0,
            date: reference.spawn_context.date,
            chance_bits: None,
        };
        for kind in &reference.buildings.types {
            let out = expected(reference, &rules, &kind.prototype, context).unwrap();
            let catalog = &reference.catalog.items;
            let mut rows: Vec<(&str, f64)> = out
                .items
                .iter()
                .map(|(i, n)| (catalog[*i as usize].name.as_str(), *n))
                .collect();
            for (prototype, n) in &out.unlisted {
                let p = reference.prototypes.iter().find(|p| &p.source == prototype);
                rows.push((p.map_or("?", |p| p.nodes[0].name.as_str()), *n));
            }
            rows.sort_by(|a, b| b.1.total_cmp(&a.1));
            let total: f64 = rows.iter().map(|(_, n)| n).sum();
            println!(
                "type {} total {total:.3} dropped {:?}",
                kind.type_id, out.dropped
            );
            for (name, n) in rows.iter().take(12) {
                println!("  {name:<24} {n:.4}");
            }
        }
    }
}
