use crate::reference::{ItemId, ReferenceError, ReferenceManifest};
use crate::save::{SaveError, SourceKey, ToolSnapshot};
#[derive(Clone, Debug)]
pub struct ToolState {
    pub item: ItemId,
    pub quantity: Option<f32>,
    pub color: Option<[f32; 4]>,
    pub disabled: bool,
}
pub struct CleaningTools {
    pub states: Vec<ToolState>,
}
impl CleaningTools {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        reference.conditions.validate(reference)?;
        Ok(Self {
            states: reference
                .conditions
                .brushes
                .iter()
                .map(|t| ToolState {
                    item: t.item,
                    quantity: None,
                    color: None,
                    disabled: t.disabled,
                })
                .chain(reference.conditions.sprays.iter().map(|t| ToolState {
                    item: t.item,
                    quantity: Some(t.startup_quantity),
                    color: Some(t.startup_color),
                    disabled: false,
                }))
                .collect(),
        })
    }
    pub fn get(&self, item: ItemId) -> Option<&ToolState> {
        self.states.iter().find(|t| t.item == item)
    }
    pub fn capture_save(
        &self,
        reference: &ReferenceManifest,
    ) -> Result<Vec<ToolSnapshot>, SaveError> {
        self.states
            .iter()
            .map(|t| {
                let source = reference
                    .conditions
                    .brushes
                    .iter()
                    .find(|b| b.item == t.item)
                    .map(|b| &b.source)
                    .or_else(|| {
                        reference
                            .conditions
                            .sprays
                            .iter()
                            .find(|b| b.item == t.item)
                            .map(|b| &b.source)
                    })
                    .ok_or_else(|| SaveError::InvalidLinks("Unknown tool owner".into()))?;
                Ok(ToolSnapshot {
                    source: SourceKey::from(source),
                    quantity: t.quantity,
                    color: t.color,
                    disabled: t.disabled,
                })
            })
            .collect()
    }
    pub fn restore_save(
        &mut self,
        reference: &ReferenceManifest,
        saved: &[ToolSnapshot],
    ) -> Result<(), SaveError> {
        validate_tools(reference, saved)?;
        for t in &mut self.states {
            let source = reference
                .conditions
                .brushes
                .iter()
                .find(|b| b.item == t.item)
                .map(|b| &b.source)
                .or_else(|| {
                    reference
                        .conditions
                        .sprays
                        .iter()
                        .find(|b| b.item == t.item)
                        .map(|b| &b.source)
                })
                .unwrap();
            let v = saved
                .iter()
                .find(|v| v.source == SourceKey::from(source))
                .unwrap();
            t.quantity = v.quantity;
            t.color = v.color;
            t.disabled = v.disabled;
        }
        Ok(())
    }
}
pub(crate) fn validate_tools(
    reference: &ReferenceManifest,
    saved: &[ToolSnapshot],
) -> Result<(), SaveError> {
    let invalid = || SaveError::CorruptData("Invalid tool state or owner".into());
    if saved.len() != reference.conditions.brushes.len() + reference.conditions.sprays.len() {
        return Err(invalid());
    }
    for r in &reference.conditions.brushes {
        let mut matches = saved
            .iter()
            .filter(|v| v.source == SourceKey::from(&r.source));
        let v = matches.next().ok_or_else(invalid)?;
        if matches.next().is_some() || v.quantity.is_some() || v.color.is_some() {
            return Err(invalid());
        }
    }
    for r in &reference.conditions.sprays {
        let mut matches = saved
            .iter()
            .filter(|v| v.source == SourceKey::from(&r.source));
        let v = matches.next().ok_or_else(invalid)?;
        if matches.next().is_some()
            || v.disabled
            || v.quantity
                .is_none_or(|q| !q.is_finite() || !(0. ..=r.maximum_quantity).contains(&q))
            || v.color.is_none_or(|c| !c.iter().all(|v| v.is_finite()))
        {
            return Err(invalid());
        }
    }
    Ok(())
}
