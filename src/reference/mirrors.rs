use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorRecord {
    pub rules: MirrorRulesReference,
    pub target: MirrorTargetReference,
    pub mirrors: Vec<MirrorReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorRulesReference {
    pub source: SourceId,
    pub settings_source: SourceId,
    pub draw_distance_source: SourceId,
    pub enabled_source: SourceId,
    pub enabled: bool,
    pub draw_distance_meters: f32,
    pub fov_bounds_degrees: [f32; 2],
    pub distance_bounds_meters: [f32; 2],
    pub count_limit: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorTargetReference {
    pub source: SourceId,
    pub extent: [u32; 2],
    pub samples: u32,
    pub color_format_code: u32,
    pub depth_stencil_format_code: u32,
    pub linear: bool,
    pub filter: u8,
    pub wrap: [u8; 3],
    pub mip_sampling: bool,
    pub generate_mips: bool,
    pub mip_count: i32,
    pub dynamic_scale: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorReference {
    pub source: SourceId,
    pub owner: ItemId,
    pub owner_node: NodeId,
    pub surface_node: NodeId,
    pub surface_source: SourceId,
    pub plane_node: NodeId,
    pub camera_node: NodeId,
    pub camera_source: SourceId,
    pub camera_script_source: SourceId,
    pub near_plane: f32,
    pub serialized_far_plane: f32,
    pub initial_far_plane: f32,
    pub serialized_fov_degrees: f32,
    pub culling_mask: u32,
    pub hdr: bool,
    pub clear_flags: u8,
    pub clear_color: [f32; 4],
    pub occlusion_culling: bool,
    pub allow_msaa: bool,
    pub serialized_rendering_path: i32,
    pub rendering_path: i32,
    pub orthographic: bool,
    pub update_mirrors: bool,
    pub postprocessing_sources: Vec<SourceId>,
    pub depth_source: Option<SourceId>,
    pub heat_haze_source: Option<SourceId>,
    pub live_material: MaterialBindingReference,
    pub disabled_material: MaterialBindingReference,
    pub live_texture_slots: [String; 2],
}
impl MirrorRecord {
    pub fn validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        let valid_source = |s: &SourceId| {
            s.path_id != 0
                && manifest
                    .source_hashes
                    .iter()
                    .any(|hash| hash.file == s.file)
        };
        let descendant = |mut node: NodeId, owner: NodeId| {
            for _ in 0..=manifest.nodes.len() {
                if node == owner {
                    return true;
                }
                match manifest
                    .nodes
                    .iter()
                    .find(|n| n.id == node)
                    .and_then(|n| n.parent)
                {
                    Some(parent) => node = parent,
                    None => return false,
                }
            }
            false
        };
        let r = &self.rules;
        require(
            r.source.file == "level1"
                && r.source.path_id == 64919
                && [
                    &r.settings_source,
                    &r.draw_distance_source,
                    &r.enabled_source,
                ]
                .into_iter()
                .all(valid_source)
                && finite(&[r.draw_distance_meters])
                && r.draw_distance_meters > 0.
                && finite(&r.fov_bounds_degrees)
                && r.fov_bounds_degrees[0] > 0.
                && r.fov_bounds_degrees[0] <= r.fov_bounds_degrees[1]
                && r.fov_bounds_degrees[1] < 180.
                && finite(&r.distance_bounds_meters)
                && r.distance_bounds_meters[0] >= 0.
                && r.distance_bounds_meters[0] < r.distance_bounds_meters[1]
                && r.count_limit == 5,
            "level1:64919 mirror rules",
        )?;
        let t = &self.target;
        require(
            t.source.file == "sharedassets1.assets"
                && t.source.path_id == 4495
                && valid_source(&t.source)
                && t.extent == [256, 256]
                && t.samples == 1
                && t.color_format_code == 8
                && t.depth_stencil_format_code == 92
                && t.linear
                && t.filter == 1
                && t.wrap == [1; 3]
                && !t.mip_sampling
                && !t.dynamic_scale
                && t.generate_mips
                && t.mip_count == -1,
            "sharedassets1.assets:4495 mirror target",
        )?;
        require(self.mirrors.len() == 3, "three selected mirror captures")?;
        let expected = [
            (66131, 40260, 43665, 31962, 28414, 39254),
            (67237, 40324, 40735, 33202, 38060, 31061),
            (70638, 40222, 44371, 28868, 32190, 26965),
        ];
        let mut owners = HashSet::new();
        let mut cameras = HashSet::new();
        for (m, (component, camera, surface, screen, plane, camera_transform)) in
            self.mirrors.iter().zip(expected)
        {
            let label = format!("{}:{}", m.source.file, m.source.path_id);
            let check = |ok, field: &str| require(ok, &format!("{label} {field}"));
            check(
                m.source.file == "level1" && m.source.path_id == component,
                "source",
            )?;
            check(
                owners.insert(m.owner) && cameras.insert(m.camera_node),
                "duplicate owner/camera",
            )?;
            check(
                manifest
                    .items
                    .iter()
                    .any(|i| i.id == m.owner && i.node == m.owner_node),
                "owner",
            )?;
            for (id, field, source) in [
                (m.owner_node, "owner_node", None),
                (m.surface_node, "surface_node", Some(screen)),
                (m.plane_node, "plane_node", Some(plane)),
                (m.camera_node, "camera_node", Some(camera_transform)),
            ] {
                check(
                    manifest.nodes.iter().any(|n| {
                        n.id == id
                            && n.asset == AssetId(2)
                            && source
                                .is_none_or(|s| n.source.file == "level1" && n.source.path_id == s)
                    }),
                    field,
                )?;
            }
            check(
                m.camera_source.file == "level1"
                    && m.camera_source.path_id == camera
                    && m.surface_source.file == "level1"
                    && m.surface_source.path_id == surface
                    && valid_source(&m.camera_script_source),
                "camera/surface source",
            )?;
            check(
                finite(&[
                    m.near_plane,
                    m.serialized_far_plane,
                    m.initial_far_plane,
                    m.serialized_fov_degrees,
                ]) && m.near_plane > 0.
                    && m.serialized_far_plane > m.near_plane
                    && m.initial_far_plane > m.near_plane
                    && m.serialized_fov_degrees > 0.
                    && m.serialized_fov_degrees < 180.
                    && finite(&m.clear_color),
                "projection",
            )?;
            check(
                m.culling_mask == 73113395
                    && m.clear_flags == 1
                    && !m.orthographic
                    && !m.update_mirrors
                    && m.postprocessing_sources.is_empty()
                    && m.depth_source.is_none()
                    && m.heat_haze_source.is_none()
                    && matches!(m.rendering_path, 1 | 3),
                "camera policy",
            )?;
            for (link, field) in [
                (&m.live_material, "live_material"),
                (&m.disabled_material, "disabled_material"),
            ] {
                check(
                    link.asset == AssetId(2)
                        && valid_source(&link.source)
                        && manifest.rendering.materials.iter().any(|r| {
                            r.asset == link.asset
                                && r.gltf_material == link.gltf_material
                                && r.source == link.source
                        }),
                    field,
                )?;
            }
            check(
                m.live_material.source.file == "sharedassets1.assets"
                    && m.live_material.source.path_id == 624
                    && m.disabled_material.source.file == "sharedassets0.assets"
                    && m.disabled_material.source.path_id == 116
                    && manifest
                        .nodes
                        .iter()
                        .any(|n| n.id == m.surface_node && n.parent == Some(m.plane_node))
                    && [m.surface_node, m.plane_node, m.camera_node]
                        .into_iter()
                        .all(|node| descendant(node, m.owner_node))
                    && m.live_material.source != m.disabled_material.source
                    && m.live_texture_slots == ["_MainTex", "_EmissionMap"],
                "live texture bindings",
            )?;
        }
        Ok(())
    }
}
