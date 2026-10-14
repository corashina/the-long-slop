//! Source fuel station prototypes and their roadside rules.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationsReference {
    /// `roadBuildingScript` instance.
    pub source: SourceId,
    pub types: Vec<StationReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationReference {
    /// Prefab GameObject.
    pub source: SourceId,
    /// `itemdatabase.b` value.
    pub kind: u8,
    pub asset: AssetId,
    pub root: NodeId,
    pub sides: Vec<StationSide>,
    pub ground: Vec<StationGround>,
    /// Static shapes in root-local meters.
    pub colliders: Vec<StationCollider>,
}

/// Source X offset from the road center and Y angle, both before the road turn.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationSide {
    pub lateral_meters: [f32; 2],
    pub yaw_degrees: [f32; 2],
}

/// Terrain takes the marker height inside `radius` and blends out at `blend_radius`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationGround {
    pub source: SourceId,
    pub center: [f32; 3],
    pub radius: f32,
    pub blend_radius: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationCollider {
    pub source: SourceId,
    pub geometry: ColliderGeometry,
}

impl StationsReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.source.file == "level1" && self.source.path_id == 64750,
            "station generator source",
        )?;
        require(
            self.types.len() == 3
                && ids(self.types.iter().map(|t| t.kind), "station kinds")?.len() == 3,
            "station types",
        )?;
        for t in &self.types {
            require(
                m.assets
                    .iter()
                    .any(|a| a.id == t.asset && a.usage == AssetUsage::StationPrototype)
                    && m.nodes
                        .iter()
                        .any(|n| n.id == t.root && n.asset == t.asset && n.parent.is_none()),
                "station asset",
            )?;
            require(
                !t.sides.is_empty()
                    && t.sides.iter().all(|s| {
                        finite(&s.lateral_meters)
                            && finite(&s.yaw_degrees)
                            && s.lateral_meters[0] <= s.lateral_meters[1]
                            && s.yaw_degrees[0] <= s.yaw_degrees[1]
                    }),
                "station sides",
            )?;
            require(
                !t.ground.is_empty()
                    && t.ground.iter().all(|g| {
                        finite(&g.center)
                            && g.radius.is_finite()
                            && g.radius > 0.
                            && g.blend_radius.is_finite()
                            && g.blend_radius >= g.radius
                    }),
                "station ground",
            )?;
            require(
                !t.colliders.is_empty()
                    && t.colliders
                        .iter()
                        .all(|c| weather::valid_geometry(&c.geometry)),
                "station colliders",
            )?;
        }
        Ok(())
    }
}
