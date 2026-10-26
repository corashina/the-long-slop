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
/// One `rendszamscript` label at startup. `same_as` is the index of the linked label.
pub struct StartupLabel<'a> {
    pub text: &'a str,
    pub same: bool,
    pub same_as: Option<usize>,
}
/// Source `rendszamscript` startup in label order. A label that is not `same` draws "AAA-000"
/// and sets it on itself and on its `same_as` chain. Returns each label's text and `same`.
pub fn startup_texts(
    labels: &[StartupLabel],
    draw: &mut dyn FnMut(i32, i32) -> i32,
) -> Result<Vec<(String, bool)>, ReferenceError> {
    let mut result: Vec<_> = labels.iter().map(|l| (l.text.to_owned(), l.same)).collect();
    for label in labels {
        let mut next = label.same_as;
        for step in 0..=labels.len() {
            let Some(i) = next else { break };
            if step == labels.len() {
                return Err(ReferenceError("plate sharing cycle".into()));
            }
            next = labels
                .get(i)
                .ok_or_else(|| ReferenceError("missing shared plate".into()))?
                .same_as;
        }
    }
    for i in 0..labels.len() {
        if result[i].1 {
            continue;
        }
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
        // Set updates this label. Updated invokes Same on its linked label.
        let mut index = i;
        for step in 0..labels.len() {
            result[index].0.clone_from(&text);
            if step > 0 {
                result[index].1 = true;
            }
            let Some(next) = labels[index].same_as else {
                break;
            };
            index = next;
        }
    }
    Ok(result)
}
impl PlateState {
    pub fn new(
        record: &PlateRecord,
        draw: &mut dyn FnMut(i32, i32) -> i32,
    ) -> Result<Self, ReferenceError> {
        for (i, p) in record.plates.iter().enumerate() {
            if record.plates[..i].iter().any(|o| o.item == p.item) {
                return Err(ReferenceError("duplicate plate item".into()));
            }
        }
        let labels = record
            .plates
            .iter()
            .map(|p| {
                Ok(StartupLabel {
                    text: &p.initial_text,
                    same: p.initial_same,
                    same_as: p
                        .same_as
                        .map(|id| {
                            record
                                .plates
                                .iter()
                                .position(|o| o.item == id)
                                .ok_or_else(|| ReferenceError("missing shared plate".into()))
                        })
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let texts = startup_texts(&labels, draw)?;
        Ok(Self {
            labels: record
                .plates
                .iter()
                .zip(texts)
                .map(|(p, (text, same))| Label {
                    item: p.item,
                    revision: u64::from(text != p.initial_text),
                    text,
                    same,
                    same_as: p.same_as,
                })
                .collect(),
        })
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
/// Plate texts of one world car copy, in `plates` order. `seed` comes from the copy's object key,
/// so each copy draws its own texts and a reload draws the same texts.
pub fn copy_texts(
    plates: &[crate::reference::WorldPlateReference],
    seed: u64,
) -> Result<Vec<String>, ReferenceError> {
    let labels = plates
        .iter()
        .map(|p| StartupLabel {
            text: &p.initial_text,
            same: p.initial_same,
            same_as: p
                .same_as
                .and_then(|node| plates.iter().position(|o| o.node == node)),
        })
        .collect::<Vec<_>>();
    let mut draw = StartupDraw::new(seed ^ COPY_DRAW);
    Ok(startup_texts(&labels, &mut |a, b| draw.draw(a, b))?
        .into_iter()
        .map(|(text, _)| text)
        .collect())
}
/// Separates the plate draw from other random streams seeded by the object key.
const COPY_DRAW: u64 = 0x706c_6174_6573;
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
