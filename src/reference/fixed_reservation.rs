//! Checked construction bounds for fixed Bevy imports.
use super::{
    FileCatalog, ReferenceError, lighting_scene::CloudMesh, prototype_plan::PrototypePlan, require,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedReservation {
    pub mesh_bytes: usize,
    pub cpu: usize,
    pub gpu: usize,
    pub staging: usize,
}
impl FixedReservation {
    pub fn mesh(plan: &PrototypePlan, retained: usize) -> Result<Self, ReferenceError> {
        require(
            plan.primitives
                .iter()
                .all(|p| plan.nodes.iter().any(|n| n.mesh == Some(p.mesh))),
            "fixed unreferenced mesh bound",
        )?;
        let mut bytes = 0usize;
        let mut count = 0usize;
        for node in &plan.nodes {
            require(node.skin.is_none(), "fixed mesh skin bound")?;
            let Some(mesh) = node.mesh else { continue };
            let mut found = false;
            for primitive in plan.primitives.iter().filter(|p| p.mesh == mesh) {
                found = true;
                require(
                    primitive.vertices > 0
                        && primitive.indices > 0
                        && primitive.attributes.iter().any(|a| a == "POSITION")
                        && !primitive
                            .attributes
                            .iter()
                            .any(|a| a.starts_with("JOINTS_") || a.starts_with("WEIGHTS_")),
                    "fixed mesh vertex contract",
                )?;
                let indices = primitive.indices.checked_mul(4).ok_or_else(overflow)?;
                let authored = primitive.bytes.checked_sub(indices).ok_or_else(overflow)?;
                require(
                    authored % primitive.vertices == 0,
                    "fixed mesh authored stride",
                )?;
                let stride = authored / primitive.vertices;
                require(
                    stride >= 12 && stride % 4 == 0,
                    "fixed mesh authored attributes",
                )?;
                let expanded = primitive
                    .vertices
                    .max(primitive.indices)
                    .checked_mul(stride.checked_add(32).ok_or_else(overflow)?)
                    .and_then(|n| n.checked_add(indices))
                    .ok_or_else(overflow)?;
                bytes = bytes.checked_add(expanded).ok_or_else(overflow)?;
                count = count.checked_add(1).ok_or_else(overflow)?;
            }
            require(found, "fixed mesh primitive coverage")?;
        }
        Self::checked(
            plan.json_bytes,
            plan.buffer_bytes,
            bytes.max(plan.mesh_bytes),
            plan.scene_bytes,
            count,
            retained,
        )
    }

    pub fn cloud(mesh: &CloudMesh, catalog: &FileCatalog) -> Result<Self, ReferenceError> {
        require(
            mesh.vertex_count == 829 && mesh.index_count == 4596,
            "fixed cloud geometry counts",
        )?;
        let json = catalog.file(&mesh.file)?;
        let buffer = catalog.file(&mesh.buffer)?;
        json.validate()?;
        buffer.validate()?;
        require(
            json.bytes > 0 && buffer.bytes == 829 * (12 + 8) + 4596 * 4,
            "fixed cloud input contract",
        )?;
        // The checked producer writes POSITION f32x3 and TEXCOORD_0 f32x2 only.
        let bytes = 4596usize
            .checked_mul(12 + 8 + 32)
            .and_then(|n| n.checked_add(4596 * 4))
            .ok_or_else(overflow)?;
        Self::checked(
            json.bytes as usize,
            buffer.bytes as usize,
            bytes,
            4096,
            1,
            0,
        )
    }

    fn checked(
        json: usize,
        buffer: usize,
        mesh_bytes: usize,
        scene: usize,
        count: usize,
        retained: usize,
    ) -> Result<Self, ReferenceError> {
        let cpu = json
            .checked_mul(18)
            .and_then(|n| n.checked_add(buffer.checked_mul(2)?))
            .and_then(|n| n.checked_add(mesh_bytes.checked_mul(3)?))
            .and_then(|n| n.checked_add(scene))
            .and_then(|n| n.checked_add(count.checked_mul(4096)?))
            .and_then(|n| n.checked_add(retained))
            .ok_or_else(overflow)?;
        let gpu = count
            .checked_mul(8)
            .and_then(|n| n.checked_add(mesh_bytes))
            .ok_or_else(overflow)?;
        let staging = json
            .checked_add(buffer)
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(overflow)?;
        Ok(Self {
            mesh_bytes,
            cpu,
            gpu,
            staging,
        })
    }
}
fn overflow() -> ReferenceError {
    ReferenceError("Fixed Bevy reservation overflow".into())
}

/// Per-stream snapshot of global fixed ownership in the existing presentation ledger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedAllocation {
    pub cpu: usize,
    pub gpu: usize,
    pub jobs: usize,
    pub pending: usize,
    pub staging: usize,
}
impl FixedAllocation {
    pub fn replace(
        self,
        old: Self,
        mut a: crate::world::PresentationAllocation,
    ) -> Result<crate::world::PresentationAllocation, ReferenceError> {
        let replace = |value: usize, before: usize, after: usize| {
            value
                .checked_sub(before)
                .and_then(|n| n.checked_add(after))
                .ok_or_else(overflow)
        };
        a.prototype_cpu = replace(a.prototype_cpu, old.cpu, self.cpu)?;
        a.prototype_gpu = replace(a.prototype_gpu, old.gpu, self.gpu)?;
        a.jobs = replace(a.jobs, old.jobs, self.jobs)?;
        a.pending = replace(a.pending, old.pending, self.pending)?;
        a.staging_bytes = replace(a.staging_bytes, old.staging, self.staging)?;
        a.dependency_staging_bytes =
            replace(a.dependency_staging_bytes, old.staging, self.staging)?;
        Ok(a)
    }
    pub fn exclude_shared(
        self,
        metrics: &mut crate::world::WorldMetrics,
    ) -> Result<(), ReferenceError> {
        metrics.prototype_cpu_bytes = metrics
            .prototype_cpu_bytes
            .checked_sub(self.cpu)
            .ok_or_else(overflow)?;
        metrics.prototype_gpu_bytes = metrics
            .prototype_gpu_bytes
            .checked_sub(self.gpu)
            .ok_or_else(overflow)?;
        Ok(())
    }
}

/// Production lifetime of one fixed assembled image and its bounded row scratch.
#[derive(Clone, Copy, Debug)]
pub struct FixedImageOwnership {
    pixels: usize,
    main_world: bool,
    scratch: usize,
    started: bool,
    row_live: bool,
    assembled: bool,
    complete: bool,
    cancelled: bool,
}
impl FixedImageOwnership {
    pub fn new(pixels: usize, main_world: bool, scratch: usize) -> Result<Self, ReferenceError> {
        pixels
            .checked_mul(3)
            .and_then(|n| n.checked_add(scratch))
            .ok_or_else(overflow)?;
        Ok(Self {
            pixels,
            main_world,
            scratch,
            started: false,
            row_live: false,
            assembled: false,
            complete: false,
            cancelled: false,
        })
    }
    pub fn begin_row(&mut self) -> Result<bool, ReferenceError> {
        if self.cancelled {
            return Ok(false);
        }
        require(
            !self.row_live && !self.assembled,
            "fixed row admission state",
        )?;
        self.started = true;
        self.row_live = true;
        Ok(true)
    }
    pub fn finish_row(&mut self, terminal: bool) -> bool {
        if terminal {
            self.row_live = false;
        }
        terminal
    }
    pub fn assemble(&mut self) -> Result<(), ReferenceError> {
        require(
            self.started && !self.row_live && !self.cancelled,
            "fixed row assembly state",
        )?;
        self.assembled = true;
        Ok(())
    }
    pub fn uploaded(&mut self) -> Result<(), ReferenceError> {
        require(self.assembled && !self.row_live, "fixed image upload state")?;
        self.complete = true;
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub fn discard(&mut self) -> Result<(), ReferenceError> {
        require(
            self.cancelled && !self.row_live && !self.assembled,
            "fixed row physical retirement",
        )?;
        self.started = false;
        Ok(())
    }
    pub fn bytes(&self) -> (usize, usize) {
        if !self.started {
            return (0, 0);
        }
        let cpu = if self.complete {
            if self.main_world { self.pixels } else { 0 }
        } else if self.cancelled && !self.assembled {
            self.pixels
        } else {
            3 * self.pixels
        };
        (
            cpu + if self.row_live { self.scratch } else { 0 },
            self.pixels,
        )
    }
    pub fn io(&self) -> (usize, usize, usize) {
        (
            usize::from(self.row_live),
            usize::from(self.started && !self.complete),
            if self.row_live { self.scratch } else { 0 },
        )
    }
}

pub fn png_staging(encoded: usize, width: u32) -> Result<usize, ReferenceError> {
    encoded
        .checked_mul(3)
        .and_then(|n| n.checked_add(2 * 1024 * 1024))
        .and_then(|n| n.checked_add((width as usize).checked_mul(128)?))
        .ok_or_else(overflow)
}
pub fn png_cpu(pixels: usize, max_staging: usize) -> Result<usize, ReferenceError> {
    pixels
        .checked_mul(2)
        .and_then(|n| n.checked_add(max_staging.checked_mul(2)?))
        .ok_or_else(overflow)
}
pub fn png_pixels(mips: &[super::TextureMip]) -> Result<usize, ReferenceError> {
    require(!mips.is_empty(), "fixed PNG mip count")?;
    mips.iter().try_fold(0usize, |sum, mip| {
        require(mip.width > 0 && mip.height > 0, "fixed PNG mip extent")?;
        (mip.width as usize)
            .checked_mul(mip.height as usize)
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(sum))
            .ok_or_else(overflow)
    })
}

pub fn fixed_room(
    stream: &crate::world::WorldStream,
    cpu: usize,
    gpu: usize,
    staging: usize,
    pending: bool,
) -> Result<bool, ReferenceError> {
    if cpu > 256 * 1024 * 1024 || gpu > 512 * 1024 * 1024 || staging > 32 * 1024 * 1024 {
        return Err(ReferenceError("Fixed input exceeds shared limits".into()));
    }
    Ok((staging == 0 || stream.available_external_jobs() > 0)
        && (!pending || stream.metrics().pending_work < 32)
        && staging <= stream.available_dependency_staging_bytes()
        && stream
            .metrics()
            .prototype_cpu_bytes
            .checked_add(cpu)
            .is_some_and(|n| n <= 256 * 1024 * 1024)
        && stream
            .metrics()
            .prototype_gpu_bytes
            .checked_add(gpu)
            .is_some_and(|n| n <= 512 * 1024 * 1024))
}

pub const FIXED_SHADER_PATHS: [&str; 36] = [
    "shaders/source_lod.wgsl",
    "shaders/source_float_store.wgsl",
    "shaders/source_view_lighting.wgsl",
    "shaders/source_caster_math.wgsl",
    "shaders/source_world_caster.wgsl",
    "shaders/source_lighting.wgsl",
    "shaders/source_surface.wgsl",
    "shaders/source_mesh.wgsl",
    "shaders/source_glass.wgsl",
    "shaders/source_sky.wgsl",
    "shaders/source_cloud.wgsl",
    "shaders/source_render_sky.wgsl",
    "shaders/source_copy.wgsl",
    "shaders/source_histogram.wgsl",
    "shaders/source_exposure.wgsl",
    "shaders/source_probe.wgsl",
    "shaders/source_taa.wgsl",
    "shaders/source_caster.wgsl",
    "shaders/source_motion_vectors.wgsl",
    "shaders/source_motion_objects.wgsl",
    "shaders/source_motion_blur.wgsl",
    "shaders/source_bloom.wgsl",
    "shaders/source_lut.wgsl",
    "shaders/source_uber.wgsl",
    "shaders/source_ao_depth.wgsl",
    "shaders/source_ao_render.wgsl",
    "shaders/source_ao_upsample.wgsl",
    "shaders/source_ao_composite.wgsl",
    "shaders/source_plate_text.wgsl",
    "shaders/source_grass_displacement.wgsl",
    "shaders/source_grass.wgsl",
    "shaders/source_terrain.wgsl",
    "shaders/source_impostor.wgsl",
    "shaders/source_lod_caster.wgsl",
    "shaders/source_particles.wgsl",
    "shaders/source_shadow_collector.wgsl",
];

/// Source and parser copies, retained main/render shader data, and input scratch.
pub fn shader_input(catalog: &FileCatalog, path: &str) -> Result<FixedReservation, ReferenceError> {
    require(
        FIXED_SHADER_PATHS.contains(&path),
        "unselected shader input",
    )?;
    let receipt = catalog.file(path)?;
    receipt.validate()?;
    let bytes = receipt.bytes as usize;
    require(bytes > 0, "empty selected shader input")?;
    Ok(FixedReservation {
        mesh_bytes: 0,
        cpu: bytes
            .checked_mul(18)
            .and_then(|n| n.checked_add(4096))
            .ok_or_else(overflow)?,
        gpu: 0,
        staging: bytes.checked_mul(2).ok_or_else(overflow)?,
    })
}
