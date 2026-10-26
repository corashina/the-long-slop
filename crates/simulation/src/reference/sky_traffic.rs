use super::*;
/// Source AirPlaneSpawnScript on Main. Ranges are [min, max].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkyReference {
    pub source: SourceId,
    /// Raw prefab chances in prefab order. The source picks by cumulative chance.
    pub chances: Vec<f32>,
    pub max_planes: u32,
    pub spawn_time_range: [f32; 2],
    pub spawn_distance: f32,
    /// Anchor height above the player.
    pub height: f32,
    pub speed: f32,
    /// Degrees of yaw from the source +Z axis.
    pub max_angle: f32,
    pub destroy_distance: f32,
    pub contrail_rate: f32,
    pub prefabs: Vec<SkyPrefab>,
    pub evidence: ManagedEvidence,
}
/// One AirPlanePrefabs entry, exported as a sky_traffic asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkyPrefab {
    /// Prefab root Transform.
    pub source: SourceId,
    pub asset: AssetId,
    pub root: NodeId,
    /// First active ParticleSystem. It simulates in the anchor space.
    pub contrail: Option<ParticleReference>,
    /// Audio voice IDs of the prefab AudioSources.
    pub voices: Vec<u32>,
    pub spin: Option<SkySpin>,
}
/// Source fedoscript rotor spin.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkySpin {
    pub node: NodeId,
    /// Port space.
    pub axis: [f32; 3],
    pub degrees_per_second: f32,
}
impl SkyReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        let range = self.spawn_time_range;
        require(
            self.evidence.valid()
                && finite(&self.chances)
                && finite(&range)
                && finite(&[
                    self.spawn_distance,
                    self.height,
                    self.speed,
                    self.max_angle,
                    self.destroy_distance,
                    self.contrail_rate,
                ])
                && self.max_planes > 0
                && 0. <= range[0]
                && range[0] <= range[1]
                && self.spawn_distance > 0.
                && self.speed > 0.
                && self.destroy_distance > 0.
                && self.contrail_rate >= 0.
                && (0. ..=180.).contains(&self.max_angle),
            "sky spawner",
        )?;
        require(
            self.chances.len() == self.prefabs.len()
                && self.chances.iter().all(|c| *c >= 0.)
                && self.chances.iter().sum::<f32>() > 0.,
            "sky chances",
        )?;
        let node = |id: NodeId| m.nodes.iter().find(|n| n.id == id);
        for p in &self.prefabs {
            require(
                m.assets.iter().any(|a| {
                    a.id == p.asset && a.usage == AssetUsage::SkyTraffic && a.root == p.source
                }) && node(p.root).is_some_and(|n| {
                    n.source == p.source && n.asset == p.asset && n.parent.is_none()
                }),
                "sky prefab root",
            )?;
            if let Some(s) = &p.spin {
                require(
                    node(s.node).is_some_and(|n| n.asset == p.asset)
                        && finite(&s.axis)
                        && s.axis.iter().any(|v| *v != 0.)
                        && s.degrees_per_second.is_finite(),
                    "sky spin",
                )?;
            }
            if let Some(c) = &p.contrail {
                require(
                    c.role.starts_with("sky_") && c.role.ends_with("_contrail"),
                    "sky contrail role",
                )?;
                particles::validate_particle(c)?;
            }
        }
        Ok(())
    }
}
