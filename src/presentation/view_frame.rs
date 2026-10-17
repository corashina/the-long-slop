use super::lighting_scene::PROBE_COUNT;
// One main-world view snapshot for raster, culling, motion, and shadow fitting.
use super::surface_views::{MIRROR_START, VIEW_COUNT};
use super::{
    lighting_assets::LightingAssets, lighting_scene::SourceSceneLighting,
    materials::RenderingState, source_lighting::SourceLightingStatus,
    surface_views::SourceSurfaceView,
};
use crate::reference::{
    reflections::{ProbeJob, ProbeSchedule, capture_view_from_world},
    shadow_planes::{CullPlanes, implicit_cull_planes},
    temporal::{TaaJitter, TaaSequence, jitter_perspective},
};
use bevy::render::{extract_resource::ExtractResource, view::ExtractedView};
use bevy::{ecs as bevy_ecs, prelude::*, render as bevy_render};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

/// CPU submission acknowledgements. GPU completion remains SourceExecutionGate's responsibility.
#[derive(Resource, Clone, Default)]
pub struct SourceSubmissionAcks(Arc<SubmissionAcks>);
#[derive(Default)]
struct SubmissionAcks {
    main: AtomicU32,
    probe: AtomicU32,
    publication: AtomicU32,
}
impl SourceSubmissionAcks {
    pub fn main_token(&self) -> u32 {
        self.0.main.load(Ordering::Acquire)
    }
    /// Called only after the owner's complete commands have entered the submitted frame.
    pub fn main_submitted(&self, token: u32) {
        self.0.main.fetch_max(token, Ordering::Release);
    }
    pub fn probe_submitted(&self, token: u32) {
        self.0.probe.fetch_max(token, Ordering::Release);
    }
}
#[derive(Resource)]
pub struct SourceSceneEpoch(pub u32);
impl Default for SourceSceneEpoch {
    fn default() -> Self {
        Self(1)
    }
}
/// The source restart caller recreates its scene. GPU allocations remain owned by the renderer.
pub fn restart_scene(world: &mut World) {
    let acks = world.resource::<SourceSubmissionAcks>().clone();
    if let Err(error) = world.resource_mut::<SourceViewClock>().restart(&acks) {
        *world.resource::<SourceLightingStatus>().0.lock().unwrap() =
            RenderingState::Failed(error.into());
        return;
    }
    let next = world
        .resource::<SourceSceneEpoch>()
        .0
        .checked_add(1)
        .expect("Source scene generation exhausted");
    world.resource_mut::<SourceSceneEpoch>().0 = next;
    let mut lighting = world.resource_mut::<super::source_lighting::SourceLighting>();
    lighting.state = RenderingState::Loading;
    lighting.first_probe_ready = false;
    lighting.first_submission_validated = false;
    *world.resource::<SourceLightingStatus>().0.lock().unwrap() = RenderingState::Loading;
}
#[derive(Resource, Default)]
pub struct SourceViewClock {
    taa: TaaSequence,
    jitter: Option<TaaJitter>,
    probe: [ProbeSchedule; PROBE_COUNT],
    probe_index: usize,
    main_token: u32,
    probe_token: u32,
    frame: u32,
}
impl SourceViewClock {
    pub fn main_token(&self) -> u32 {
        self.main_token
    }
    fn consume(
        &mut self,
        acks: &SourceSubmissionAcks,
        spread: f32,
        extent: [u32; 2],
        probe_enabled: [bool; PROBE_COUNT],
    ) -> Result<(), &'static str> {
        if self.main_token == 0 {
            self.main_token = 1;
            self.probe_token = 1;
        }
        let main = acks.0.main.load(Ordering::Acquire);
        let probe = acks.0.probe.load(Ordering::Acquire);
        if main > self.main_token || probe > self.probe_token {
            return Err("Source view acknowledgement is ahead of its token");
        }
        if main == self.main_token {
            self.main_token = self
                .main_token
                .checked_add(1)
                .ok_or("Source main render token exhausted")?;
            self.jitter = None;
        }
        if probe == self.probe_token {
            if self.probe[self.probe_index].job() == ProbeJob::RemainingMipsAndPublish
                && acks.0.publication.load(Ordering::Acquire) != probe
            {
                return Err("Source probe final job lacks publication acknowledgement");
            }
            let completed = self.probe[self.probe_index].job();
            self.probe[self.probe_index].complete(completed)?;
            if completed == ProbeJob::RemainingMipsAndPublish {
                self.probe_index = (self.probe_index + 1) % PROBE_COUNT;
            }
            self.probe_token = self
                .probe_token
                .checked_add(1)
                .ok_or("Source probe job token exhausted")?;
        }
        if !probe_enabled[self.probe_index]
            && self.probe[self.probe_index].job() == ProbeJob::Face(0)
        {
            if let Some(index) = (1..PROBE_COUNT)
                .map(|offset| (self.probe_index + offset) % PROBE_COUNT)
                .find(|&index| probe_enabled[index])
            {
                self.probe_index = index;
                self.probe_token = self
                    .probe_token
                    .checked_add(1)
                    .ok_or("Source probe job token exhausted")?;
            }
        }
        if self.jitter.is_none() {
            self.jitter = Some(self.taa.advance(spread, extent)?);
        }
        // A resize changes normalized jitter, not the source sample index.
        let jitter = self.jitter.as_mut().unwrap();
        jitter.uv = std::array::from_fn(|i| jitter.pixels[i] / extent[i] as f32);
        self.frame = self
            .frame
            .checked_add(1)
            .ok_or("Source view frame stamp exhausted")?;
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct ViewSnapshot {
    pub extent: UVec2,
    pub view_from_world: Mat4,
    pub implicit_projection: Mat4,
    pub nonjittered_projection: Mat4,
    pub raster_projection: Mat4,
    pub culling_projection: Mat4,
    pub source_world_from_view: Mat4,
    pub source_camera_planes: [[f32; 4]; 6],
    pub source_shadow_planes: [[f32; 4]; 6],
    pub frustum_points: [[f32; 3]; 8],
    pub near: f32,
    pub far: f32,
    pub fov_degrees: f32,
    pub mask: u32,
    pub shadow_distance: f32,
}
#[derive(Resource, Clone, Default, ExtractResource)]
pub struct SourceViewFrame {
    pub motion_revision: u32,
    pub scene: u32,
    pub probe_enabled: [bool; PROBE_COUNT],
    pub probe_checks: u32,
    pub probe_distance_squared: [f32; PROBE_COUNT],
    pub probe_index: usize,
    pub sky_clip_from_world: Option<Mat4>,
    pub skybox_clip_from_world: Option<Mat4>,
    pub frame: u32,
    pub main_token: u32,
    pub probe_token: u32,
    pub probe_job: Option<ProbeJob>,
    pub jitter: Option<TaaJitter>,
    pub views: [Option<ViewSnapshot>; VIEW_COUNT],
}
fn source_transform(m: Mat4, p: Vec4) -> Vec4 {
    // Matrix4x4::PerspectiveMultiplyPoint3 sums Y, X, Z, then W.
    ((m.y_axis * p.y + m.x_axis * p.x) + m.z_axis * p.z) + m.w_axis * p.w
}
fn corners(inverse: Mat4) -> [[f32; 3]; 8] {
    [
        (-1., -1., -1.),
        (1., -1., -1.),
        (1., 1., -1.),
        (-1., 1., -1.),
        (-1., -1., 1.),
        (1., -1., 1.),
        (1., 1., 1.),
        (-1., 1., 1.),
    ]
    .map(|(x, y, z)| {
        let v = source_transform(inverse, Vec4::new(x, y, z, 1.));
        (v.truncate() * (1. / v.w)).to_array()
    })
}
pub fn gpu_projection(source: Mat4) -> Mat4 {
    let mut p = source.to_cols_array_2d();
    for c in &mut p {
        c[1] = -c[1];
        c[2] = 0.5 * c[3] - 0.5 * c[2];
    }
    Mat4::from_cols_array_2d(&p)
}
/// Runs after current physical extents and transforms, before source frustum publication.
pub(crate) fn snapshot(
    drive: Res<crate::app::Drive>,
    reference: Res<crate::app::RuntimeReference>,
    assets: Option<Res<LightingAssets>>,
    scene: Res<SourceSceneLighting>,
    epoch: Res<SourceSceneEpoch>,
    activation: Res<super::lighting_scene::ProbeActivation>,
    acks: Res<SourceSubmissionAcks>,
    mut clock: ResMut<SourceViewClock>,
    mut frame: ResMut<SourceViewFrame>,
    status: Res<SourceLightingStatus>,
    views: Query<(&SourceSurfaceView, &Camera, &GlobalTransform, &Projection)>,
    sky_view: Query<(&Camera, &GlobalTransform), With<super::frame_graph::SourceSkyCamera>>,
) {
    frame.scene = epoch.0;
    frame.probe_job = None;
    frame.views = [None; VIEW_COUNT];
    frame.sky_clip_from_world = None;
    frame.skybox_clip_from_world = None;
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let (Some(assets), Some(scene)) = (assets, scene.0) else {
        return;
    };
    if !assets.loaded {
        return;
    }
    let mut input = [None; VIEW_COUNT];
    for (view, camera, pose, projection) in &views {
        if view.index >= VIEW_COUNT || input[view.index].is_some() {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source snapshot view identity is invalid".into());
            return;
        }
        if !camera.is_active {
            if view.index >= MIRROR_START {
                continue;
            }
            return;
        }
        let Some(extent) = camera.physical_viewport_size() else {
            return;
        };
        if extent.x == 0 || extent.y == 0 {
            return;
        }
        let mirror = if view.index >= MIRROR_START {
            let Projection::Custom(custom) = projection else {
                return;
            };
            let p = &custom
                .get::<super::mirrors::MirrorProjection>()
                .expect("mirror projection")
                .perspective;
            Some((
                p.near,
                p.far,
                p.fov.to_degrees(),
                projection.get_clip_from_view(),
            ))
        } else {
            None
        };
        input[view.index] = Some((extent, pose.to_matrix(), mirror));
    }
    if input[..MIRROR_START].iter().any(Option::is_none) {
        return;
    }
    let Ok((sky_camera, sky_pose)) = sky_view.single() else {
        return;
    };
    if !sky_camera.is_active
        || sky_camera
            .physical_viewport_size()
            .is_none_or(|size| size.x == 0 || size.y == 0)
    {
        return;
    }
    let main = input[1].unwrap().0;
    if let Err(error) = clock.consume(
        &acks,
        assets.record.postprocessing.taa.jitter_spread,
        main.to_array(),
        activation.enabled,
    ) {
        *status.0.lock().unwrap() = RenderingState::Failed(error.into());
        return;
    }
    let snapshots = std::array::from_fn(|index| {
        let (extent, world_from_view, mirror) = input[index]?;
        let (near, far, fov, mask, distance) = if index < 3 {
            let c = &assets.record.cameras[index + 1];
            (
                c.near,
                c.far,
                c.field_of_view,
                c.mask,
                assets.record.shadows.distance,
            )
        } else if index < MIRROR_START {
            let p = &assets.record.probes[clock.probe_index];
            (p.near, p.far, 90., p.mask, p.shadow_distance)
        } else {
            let (near, far, fov, _) = mirror.unwrap();
            (
                near,
                far,
                fov,
                reference.0.mirrors.mirrors[index - MIRROR_START].culling_mask,
                assets.record.shadows.distance,
            )
        };
        let view = if index < 3 || index >= MIRROR_START {
            world_from_view.inverse()
        } else {
            Mat4::from_cols_array_2d(&capture_view_from_world(
                index - 3,
                scene.probe_position[clock.probe_index].to_array(),
            ))
        };
        Some(camera_snapshot(
            extent,
            view,
            near,
            far,
            fov,
            mask,
            distance,
            (index == 1).then_some(clock.jitter.unwrap()),
            mirror.map(|p| p.3),
        ))
    });
    let sky_clips = Some(sky_pose).map(|pose| {
        let sky = &assets.record.cameras[0];
        let projection = Mat4::perspective_rh_gl(
            sky.field_of_view.to_radians(),
            main.x as f32 / main.y as f32,
            sky.near,
            sky.far,
        );
        let view = pose.to_matrix().inverse();
        let skybox_projection =
            Mat4::from_cols_array_2d(&crate::reference::sky::source_sky_gpu_projection(
                projection.to_cols_array_2d(),
                sky.near,
            ));
        (gpu_projection(projection) * view, skybox_projection * view)
    });
    *frame = SourceViewFrame {
        scene: epoch.0,
        motion_revision: drive.runtime.audio_motion_revision,
        probe_enabled: activation.enabled,
        probe_checks: activation.checks,
        probe_index: clock.probe_index,
        probe_distance_squared: activation.distance_squared,
        sky_clip_from_world: sky_clips.map(|(camera, _)| camera),
        skybox_clip_from_world: sky_clips.map(|(_, skybox)| skybox),
        frame: clock.frame,
        main_token: clock.main_token,
        probe_token: clock.probe_token,
        probe_job: (activation.enabled[clock.probe_index]
            || clock.probe[clock.probe_index].job() != ProbeJob::Face(0))
        .then(|| clock.probe[clock.probe_index].job()),
        jitter: clock.jitter,
        views: snapshots,
    };
}
/// Replace Bevy's frusta after its update, before either camera visibility system runs.
pub fn publish_frusta(
    frame: Res<SourceViewFrame>,
    mut views: Query<(&SourceSurfaceView, &mut bevy::camera::primitives::Frustum)>,
) {
    for (view, mut frustum) in &mut views {
        if let Some(value) = frame.views.get(view.index).copied().flatten() {
            for (output, p) in frustum
                .half_spaces
                .iter_mut()
                .zip(value.source_camera_planes)
            {
                *output =
                    bevy::math::primitives::HalfSpace::new(Vec4::new(p[0], p[1], -p[2], p[3]));
            }
        }
    }
}
/// Runs before render preparation. No pose, projection, or extent is recomputed here.
pub fn apply_render_snapshot(
    frame: Res<SourceViewFrame>,
    mut views: Query<(&mut SourceSurfaceView, &mut ExtractedView)>,
) {
    for (mut state, mut view) in &mut views {
        if let Some(snapshot) = frame.views.get(state.index).copied().flatten() {
            view.world_from_view = GlobalTransform::from(snapshot.view_from_world.inverse());
            view.clip_from_view = snapshot.raster_projection;
            view.clip_from_world = Some(snapshot.raster_projection * snapshot.view_from_world);
            view.invert_culling = state.index < 3;
            state.nonjittered_clip_from_world =
                snapshot.nonjittered_projection * snapshot.view_from_world;
            state.near = snapshot.near;
            state.far = snapshot.far;
        }
    }
}
/// Explicit graph setup. Final application registration must also install source restart semantics.
pub fn install(app: &mut App) {
    use bevy::camera::visibility::VisibilitySystems;
    use bevy::render::{Render, RenderApp, RenderSystems};
    let acks = SourceSubmissionAcks::default();
    app.insert_resource(acks.clone())
        .init_resource::<SourceViewClock>()
        .init_resource::<SourceSceneEpoch>()
        .init_resource::<super::lighting_scene::ProbeActivation>()
        .init_resource::<SourceViewFrame>()
        .add_plugins(bevy::render::extract_resource::ExtractResourcePlugin::<
            SourceViewFrame,
        >::default())
        .add_systems(
            PostUpdate,
            snapshot
                .after(bevy::camera::CameraUpdateSystems)
                .after(bevy::transform::TransformSystems::Propagate)
                .after(super::lighting_scene::sync_lighting)
                .before(VisibilitySystems::UpdateFrusta),
        )
        .add_systems(
            PostUpdate,
            publish_frusta
                .after(VisibilitySystems::UpdateFrusta)
                .before(VisibilitySystems::CheckVisibility),
        );
    app.sub_app_mut(RenderApp)
        .insert_resource(acks)
        .add_systems(
            Render,
            (apply_render_snapshot, apply_sky_snapshot)
                .chain()
                .after(RenderSystems::ExtractCommands)
                .before(RenderSystems::PrepareAssets),
        );
}

impl SourceSubmissionAcks {
    /// Call after the validated complete cube copy enters its submission.
    pub fn probe_publication_submitted(&self, token: u32) {
        self.0.publication.fetch_max(token, Ordering::Release);
        self.probe_submitted(token);
    }
}

impl SourceViewClock {
    /// Retained GPU resources require monotonic tokens across source scene recreation.
    pub fn restart(&mut self, acks: &SourceSubmissionAcks) -> Result<(), &'static str> {
        self.main_token = self
            .main_token
            .checked_add(1)
            .ok_or("Source main render token exhausted")?;
        self.probe_token = self
            .probe_token
            .checked_add(1)
            .ok_or("Source probe render token exhausted")?;
        self.taa = TaaSequence::default();
        self.jitter = None;
        self.probe = std::array::from_fn(|_| ProbeSchedule::default());
        self.probe_index = 0;
        acks.0.main.store(0, Ordering::Release);
        acks.0.probe.store(0, Ordering::Release);
        acks.0.publication.store(0, Ordering::Release);
        Ok(())
    }
    /// The clock stores no world-space history. Preserve jitter and probe job order.
    pub fn rebase(&mut self, delta: Vec3) -> Result<(), &'static str> {
        if !delta.is_finite() {
            return Err("Source view rebase delta is invalid");
        }
        Ok(())
    }
}
pub fn apply_sky_snapshot(
    frame: Res<SourceViewFrame>,
    mut views: Query<&mut ExtractedView, With<super::frame_graph::SourceSkyCamera>>,
) {
    if let Some(clip) = frame.sky_clip_from_world {
        for mut view in &mut views {
            view.clip_from_view = clip * view.world_from_view.to_matrix();
            view.clip_from_world = Some(clip);
            view.invert_culling = true;
        }
    }
}

/// Shared source projection and culling math for live and private candidate views.
pub(crate) fn camera_snapshot(
    extent: UVec2,
    view: Mat4,
    near: f32,
    far: f32,
    fov: f32,
    mask: u32,
    distance: f32,
    jitter: Option<TaaJitter>,
    raster_override: Option<Mat4>,
) -> ViewSnapshot {
    let implicit = Mat4::perspective_rh_gl(
        fov.to_radians(),
        extent.x as f32 / extent.y as f32,
        near,
        far,
    );
    let current = if let Some(jitter) = jitter {
        // The source helper is row-major; glam stores columns.
        let rows = jitter_perspective(
            implicit.transpose().to_cols_array_2d(),
            near,
            fov,
            extent.x as f32 / extent.y as f32,
            extent.to_array(),
            jitter,
        )
        .expect("validated source view jitter");
        Mat4::from_cols_array_2d(&rows).transpose()
    } else {
        implicit
    };
    let source_view = Mat4::from_cols_array_2d(&crate::reference::view_math::source_culling_view(
        view.to_cols_array_2d(),
    ));
    let source_world_from_view = source_view.inverse();
    let world_corners = corners((current * source_view).inverse());
    let selected = if crate::reference::view_math::requires_implicit_projection(
        world_corners,
        source_world_from_view.w_axis.truncate().to_array(),
        near,
        far,
    ) {
        implicit
    } else {
        current
    };
    // GetScriptableCullingParameters450659..450663 sets shadowNearOffset=0.
    // This is separate from the quality projection near_offset.
    let CullPlanes { camera, shadow } = implicit_cull_planes(
        (selected * source_view).to_cols_array(),
        source_world_from_view.to_cols_array(),
        near,
        far,
        distance,
        0.,
    );
    ViewSnapshot {
        extent,
        view_from_world: view,
        implicit_projection: implicit,
        nonjittered_projection: raster_override.unwrap_or_else(|| gpu_projection(implicit)),
        raster_projection: raster_override.unwrap_or_else(|| gpu_projection(current)),
        culling_projection: selected,
        source_world_from_view,
        source_camera_planes: camera,
        source_shadow_planes: shadow,
        frustum_points: corners(implicit.inverse()),
        near,
        far,
        fov_degrees: fov,
        mask,
        shadow_distance: distance,
    }
}

pub(crate) fn required_view(
    snapshot: &ViewSnapshot,
    origin: [f64; 3],
    light_forward: [f32; 3],
    light_mask: u32,
) -> Result<crate::world::RequiredView, crate::reference::ReferenceError> {
    Ok(crate::world::RequiredView {
        camera: snapshot.source_camera_planes,
        shadow: crate::reference::shadow_candidates::DirectionalHull::new(
            snapshot.source_shadow_planes,
            light_forward,
        )
        .map_err(|e| crate::reference::ReferenceError(e.into()))?,
        mask: snapshot.mask,
        light_mask,
        origin,
    })
}
