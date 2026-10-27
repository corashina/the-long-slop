//! Renderer facts. The renderer records them during its step. The world step applies them in order.
use super::{GeneratedId, SurfaceKey, WorldRequest};
use crate::reference::SourceId;

#[derive(Clone, Debug, PartialEq)]
pub enum RendererFact {
    /// The prototype materials are ready.
    BindPrototype(SourceId),
    /// The renderer dropped the prototype.
    ReleasePrototype(SourceId),
    /// The structure materials of the prototype are ready.
    BindRecipe(SourceId),
    /// The renderer started the prepared surface.
    ReserveSurface(WorldRequest, SurfaceKey),
    /// The renderer finished the prepared surface.
    AcceptSurface(WorldRequest, SurfaceKey),
    /// The renderer dropped the prepared surface after its work drained.
    DiscardSurface(WorldRequest, SurfaceKey),
    /// The renderer finished the structure. Its claim credit moves to the ready queue.
    AcceptStructure(WorldRequest, GeneratedId),
}

/// Facts of one renderer step, in order.
#[derive(Debug, Default)]
pub struct RendererFacts(Vec<RendererFact>);

impl RendererFacts {
    pub fn push(&mut self, fact: RendererFact) {
        self.0.push(fact);
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(super) fn drain(&mut self) -> std::vec::Drain<'_, RendererFact> {
        self.0.drain(..)
    }
}
