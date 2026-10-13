//! Per-instance Grass preparation. The owning world scheduler supplies the job slots and admission.
use super::{GrassAnchor, GrassPatch, GrassSettings, GrassView, domain::GrassDomain};
use crate::reference::ReferenceError;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct PatchPlan {
    real: f32,
    factor: u32,
}
#[derive(Default)]
pub struct GrassMeshData {
    pub anchors: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Fraction along blade, side, source domain smoothing, previous fraction.
    pub blades: Vec<[f32; 4]>,
    pub patches: Vec<u32>,
    pub indices: Vec<u32>,
}
impl GrassMeshData {
    pub fn retained_bytes(&self) -> usize {
        self.anchors.capacity() * 12
            + self.uvs.capacity() * 8
            + self.blades.capacity() * 16
            + self.patches.capacity() * 4
            + self.indices.capacity() * 4
    }
}
pub struct GrassExpansion {
    pub session: u64,
    pub request: u64,
    pub view: GrassView,
    settings: GrassSettings,
    patches: Arc<[GrassPatch]>,
    domains: Arc<[GrassDomain; 4]>,
    plans: Vec<PatchPlan>,
    patch: usize,
    triangle: usize,
    vertices: usize,
    indices: usize,
    admitted: bool,
    pub output: GrassMeshData,
}
impl GrassExpansion {
    pub fn new(
        session: u64,
        request: u64,
        view: GrassView,
        settings: GrassSettings,
        patches: Arc<[GrassPatch]>,
        domains: Arc<[GrassDomain; 4]>,
        admit: impl FnOnce(usize, usize) -> Result<(), ReferenceError>,
    ) -> Result<Self, ReferenceError> {
        let plan_bytes = patches
            .len()
            .checked_mul(std::mem::size_of::<PatchPlan>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .ok_or_else(|| ReferenceError("Grass planning allocation overflow".into()))?;
        admit(plan_bytes, 0)?;
        let plans = Vec::with_capacity(patches.len());
        Ok(Self {
            session,
            request,
            view,
            settings,
            patches,
            domains,
            plans,
            patch: 0,
            triangle: 0,
            vertices: 0,
            indices: 0,
            admitted: false,
            output: GrassMeshData::default(),
        })
    }
    /// Admission covers the complete pending result while the accepted mesh remains resident.
    /// At most 4096 prospective or emitted vertices are processed by one update.
    pub fn prepare(
        &mut self,
        admit: impl FnOnce(usize, usize) -> Result<(), ReferenceError>,
    ) -> Result<bool, ReferenceError> {
        let mut work = 0usize;
        while self.patch < self.patches.len() {
            if self.plans.len() == self.patch {
                let (real, factor) = self
                    .settings
                    .tessellation(&self.patches[self.patch], self.view);
                self.plans.push(PatchPlan { real, factor });
            }
            let plan = self.plans[self.patch];
            if plan.factor == 0 {
                self.patch += 1;
                self.triangle = 0;
                // Culled patches also consume work, so long invisible instances stay bounded.
                work += 1;
                if work >= 4096 {
                    return Ok(false);
                }
                continue;
            }
            let domain_index = match plan.factor {
                2 => 0,
                4 => 1,
                8 => 2,
                16 => 3,
                _ => {
                    return Err(ReferenceError(
                        "Grass hull produced an unsupported partition".into(),
                    ));
                }
            };
            let domain = &self.domains[domain_index];
            if self.triangle == domain.triangles.len() {
                self.patch += 1;
                self.triangle = 0;
                continue;
            }
            let leading = domain.triangles[self.triangle][0] as usize;
            let anchor = self.settings.anchor(
                &self.patches[self.patch],
                domain.points[leading],
                plan.real,
                plan.factor,
            );
            let segments = self
                .settings
                .segments(anchor.position.distance(self.view.position));
            let emitted = (segments as usize + 1) * 2;
            if work + emitted > 4096 {
                return Ok(false);
            }
            work += emitted;
            if self.admitted {
                self.emit(anchor, segments);
            } else {
                self.vertices = self
                    .vertices
                    .checked_add(emitted)
                    .ok_or_else(|| ReferenceError("Grass vertex count overflow".into()))?;
                self.indices = self
                    .indices
                    .checked_add(segments as usize * 6)
                    .ok_or_else(|| ReferenceError("Grass index count overflow".into()))?;
            }
            self.triangle += 1;
        }
        if self.admitted {
            return Ok(true);
        }
        if self.vertices > u32::MAX as usize {
            return Err(ReferenceError("Grass vertex address range exceeded".into()));
        }
        let geometry = self
            .vertices
            .checked_mul(40)
            .and_then(|bytes| bytes.checked_add(self.indices.checked_mul(4)?))
            .ok_or_else(|| ReferenceError("Grass allocation size overflow".into()))?;
        admit(geometry, geometry)?;
        self.output = GrassMeshData {
            anchors: Vec::with_capacity(self.vertices),
            uvs: Vec::with_capacity(self.vertices),
            blades: Vec::with_capacity(self.vertices),
            patches: Vec::with_capacity(self.vertices),
            indices: Vec::with_capacity(self.indices),
        };
        self.admitted = true;
        self.patch = 0;
        self.triangle = 0;
        Ok(false)
    }
    fn emit(&mut self, anchor: GrassAnchor, segments: u32) {
        let start = self.output.anchors.len() as u32;
        for step in 0..=segments {
            let fraction = step as f32 / segments as f32;
            let previous = step.saturating_sub(1) as f32 / segments as f32;
            for side in [-1.0, 1.0] {
                self.output.anchors.push(anchor.position.to_array());
                self.output.uvs.push(anchor.uv.to_array());
                self.output
                    .blades
                    .push([fraction, side, anchor.smoothing, previous]);
                self.output.patches.push(self.patch as u32);
            }
            if step < segments {
                let a = start + step * 2;
                // Source triangle strip, with one source-to-runtime winding reflection.
                self.output
                    .indices
                    .extend_from_slice(&[a, a + 2, a + 1, a + 2, a + 3, a + 1]);
            }
        }
    }
    pub fn retained_bytes(&self) -> usize {
        self.plans.capacity() * std::mem::size_of::<PatchPlan>() + self.output.retained_bytes()
    }
}
