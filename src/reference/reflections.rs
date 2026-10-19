//! Native IndividualFaces order. A job advances only after its GPU work is encoded.

pub const PROBE_COUNT: usize = 2;

/// Fixed renderer slots registered by the selected source vehicle.
pub fn probe_slots(count: usize) -> Result<[bool; PROBE_COUNT], super::ReferenceError> {
    super::require(
        (1..=PROBE_COUNT).contains(&count),
        "Selected vehicle reflection probe count must be between one and two",
    )?;
    Ok(std::array::from_fn(|index| index < count))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeJob {
    Face(u8),
    FirstMip,
    RemainingMipsAndPublish,
}
/// Warmup prepares every face. Runtime prepares only the current face job.
pub fn prepare_probe_face(job: Option<ProbeJob>, face: usize, warmup: bool) -> bool {
    face < 6 && (warmup || job == Some(ProbeJob::Face(face as u8)))
}
#[derive(Default)]
pub struct ProbeSchedule {
    next: u8,
    published: bool,
}
impl ProbeSchedule {
    pub fn job(&self) -> ProbeJob {
        match self.next {
            0..=5 => ProbeJob::Face(self.next),
            6 => ProbeJob::FirstMip,
            7 => ProbeJob::RemainingMipsAndPublish,
            _ => unreachable!(),
        }
    }
    pub fn published(&self) -> bool {
        self.published
    }
    /// Call after encoding this frame's job. Pipeline waits must leave the job unchanged.
    pub fn complete(&mut self, job: ProbeJob) -> Result<(), &'static str> {
        if job != self.job() {
            return Err("Source probe job completed out of order");
        }
        if self.next == 7 {
            self.published = true;
            self.next = 0;
        } else {
            self.next += 1;
        }
        Ok(())
    }
}

/// Native kCubemapOrthoBases, RVA0x1a62fa0. Each face stores right, up, back.
pub const CUBE_BASES: [[[f32; 3]; 3]; 6] = [
    [[0., 0., -1.], [0., -1., 0.], [-1., 0., 0.]],
    [[0., 0., 1.], [0., -1., 0.], [1., 0., 0.]],
    [[1., 0., 0.], [0., 0., 1.], [0., -1., 0.]],
    [[1., 0., 0.], [0., 0., -1.], [0., 1., 0.]],
    [[1., 0., 0.], [0., -1., 0.], [0., 0., -1.]],
    [[-1., 0., 0.], [0., -1., 0.], [0., 0., 1.]],
];
/// Native remap increments its roughness fraction in f32. It does not divide each index.
pub fn source_filter_levels() -> [f32; 6] {
    let step = 0.1666666716337204_f32;
    let mut fraction = step;
    std::array::from_fn(|index| {
        let radius = (128_f64 * f64::from(fraction.powf(1.899999976158142_f32))) as f32;
        fraction += step;
        if index == 5 {
            return 7.;
        }
        let mut level = 7;
        let mut lower = 256_f32;
        while lower > radius {
            lower *= 0.5;
            level -= 1;
        }
        level as f32 + (radius - lower) / lower
    })
}

/// Native ReflectionProbes::UpdateProbe uses position plus offset and unscaled box size.
/// Rotation and transform scale do not enter this axis-aligned influence volume.
pub fn probe_influence_bounds(
    position: [f32; 3],
    size: [f32; 3],
    offset: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    (
        std::array::from_fn(|axis| position[axis] + offset[axis]),
        size.map(|value| (value * 0.5).abs()),
    )
}

/// Native cube view rows are right/up/back, followed by translation(-source_position).
/// Target world coordinates reflect Z. Preserve the resulting improper view matrix.
pub fn capture_view_from_world(face: usize, target_position: [f32; 3]) -> [[f32; 4]; 4] {
    let basis = CUBE_BASES[face];
    let source_position = [target_position[0], target_position[1], -target_position[2]];
    let mut matrix = [[0.; 4]; 4];
    for row in 0..3 {
        for column in 0..3 {
            matrix[column][row] = basis[row][column] * if column == 2 { -1. } else { 1. };
        }
        // Native Matrix4x4::operator*= adds Y, X, Z, then W, without FMA.
        matrix[3][row] = ((basis[row][1] * -source_position[1]
            + basis[row][0] * -source_position[0])
            + basis[row][2] * -source_position[2])
            + 0.;
    }
    matrix[3][3] = 1.;
    matrix
}

/// Retain a valid source slot when no other capture is pending.
pub fn pending_probe_index(
    current: usize,
    probes: &[ProbeSchedule; PROBE_COUNT],
    enabled: [bool; PROBE_COUNT],
) -> usize {
    let wanted = |index: usize| enabled[index] && !probes[index].published();
    if !wanted(current) && probes[current].job() == ProbeJob::Face(0) {
        return (1..PROBE_COUNT)
            .map(|offset| (current + offset) % PROBE_COUNT)
            .find(|&index| wanted(index))
            .unwrap_or(current);
    }
    current
}
