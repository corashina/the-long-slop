//! Retained source plate script state. Activation follows record order.
pub mod layout;
use crate::reference::{ItemId, PlateRecord, ReferenceError};
struct Label {
    item: ItemId,
    text: String,
    same: bool,
    same_as: Option<ItemId>,
    revision: u64,
}
pub struct PlateState {
    labels: Vec<Label>,
}
impl PlateState {
    pub fn new(
        record: &PlateRecord,
        draw: &mut dyn FnMut(i32, i32) -> i32,
    ) -> Result<Self, ReferenceError> {
        let mut state = Self {
            labels: record
                .plates
                .iter()
                .map(|p| Label {
                    item: p.item,
                    text: p.initial_text.clone(),
                    same: p.initial_same,
                    same_as: p.same_as,
                    revision: 0,
                })
                .collect(),
        };
        for i in 0..state.labels.len() {
            if state.labels[..i]
                .iter()
                .any(|l| l.item == state.labels[i].item)
            {
                return Err(ReferenceError("duplicate plate item".into()));
            }
            let mut next = state.labels[i].same_as;
            for step in 0..=state.labels.len() {
                let Some(id) = next else { break };
                let l = state
                    .labels
                    .iter()
                    .find(|l| l.item == id)
                    .ok_or_else(|| ReferenceError("missing shared plate".into()))?;
                if step == state.labels.len() {
                    return Err(ReferenceError("plate sharing cycle".into()));
                }
                next = l.same_as;
            }
        }
        for i in 0..state.labels.len() {
            if !state.labels[i].same {
                let mut text = String::with_capacity(7);
                for j in 0..7 {
                    if j == 3 {
                        text.push('-');
                        continue;
                    }
                    let (a, b) = if j < 3 { (65, 91) } else { (48, 58) };
                    let value = draw(a, b);
                    if !(a..b).contains(&value) {
                        return Err(ReferenceError("plate draw outside source range".into()));
                    }
                    text.push(char::from_u32(value as u32).unwrap());
                }
                state.set(state.labels[i].item, &text)?;
            }
        }
        Ok(state)
    }
    pub fn text(&self, item: ItemId) -> Option<&str> {
        self.labels
            .iter()
            .find(|l| l.item == item)
            .map(|l| l.text.as_str())
    }
    pub fn revision(&self, item: ItemId) -> Option<u64> {
        self.labels
            .iter()
            .find(|l| l.item == item)
            .map(|l| l.revision)
    }
    pub fn set(&mut self, item: ItemId, text: &str) -> Result<(), ReferenceError> {
        let mut index = self
            .labels
            .iter()
            .position(|l| l.item == item)
            .ok_or_else(|| ReferenceError("unknown plate item".into()))?;
        // Set updates this label. Updated invokes Same on its linked label.
        for step in 0..self.labels.len() {
            let label = &mut self.labels[index];
            if label.text != text {
                label.text.clear();
                label.text.push_str(text);
                label.revision += 1;
            }
            if step > 0 {
                label.same = true;
            }
            let Some(next) = label.same_as else { break };
            index = self
                .labels
                .iter()
                .position(|l| l.item == next)
                .ok_or_else(|| ReferenceError("missing shared plate".into()))?;
        }
        Ok(())
    }
}
/// Startup range adapter. This does not reproduce Unity global random state.
pub struct StartupDraw {
    seed: u64,
    counter: u64,
}
impl StartupDraw {
    pub fn new(seed: u64) -> Self {
        Self { seed, counter: 0 }
    }
    pub fn draw(&mut self, min: i32, max: i32) -> i32 {
        assert!(min < max);
        let width = (i64::from(max) - i64::from(min)) as u64;
        let threshold = width.wrapping_neg() % width;
        loop {
            let bits = crate::world::hash(self.seed ^ self.counter);
            self.counter = self.counter.wrapping_add(1);
            if bits >= threshold {
                return (i64::from(min) + (bits % width) as i64) as i32;
            }
        }
    }
}
