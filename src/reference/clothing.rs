use super::postprocessing::GlassesProfile;
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClothingReference {
    pub wearable_type_count: u32,
    pub player_root: SourceId,
    pub anchor_nodes: Vec<SourceAnchorNode>,
    pub anchors: Vec<SourceId>,
    pub wearables: Vec<WearableReference>,
    pub glasses_profiles: Vec<GlassesProfile>,
    pub evidence: ManagedEvidence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoordinateSpace {
    SourceParentLocal,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAnchorNode {
    pub source: SourceId,
    pub parent: Option<SourceId>,
    pub coordinate_space: SourceCoordinateSpace,
    pub transform: Transform,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WearableReference {
    pub source: SourceId,
    pub prototype: PrototypeId,
    pub type_index: u32,
    pub colliders: Vec<ColliderId>,
    pub renderers: Vec<NodeId>,
    pub use_center_object: bool,
    pub center: Option<NodeId>,
    pub selectors: Vec<WearableSelector>,
    pub lenses: Vec<GlassesLens>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlassesLens {
    pub source: SourceId,
    pub node: NodeId,
    pub wearable: SourceId,
    pub profile: Option<u32>,
    pub disable_lens_flare: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WearableSelectorKind {
    Random,
    Material,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WearableSelector {
    pub source: SourceId,
    pub node: NodeId,
    pub kind: WearableSelectorKind,
    pub authored_index: u32,
    pub random_start: bool,
    pub save_index: Option<u32>,
    pub children: Vec<SourceId>,
    pub material_rules: Option<WearableMaterialRules>,
    pub variants: Vec<WearableVariant>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WearableMaterialRules {
    #[serde(rename = "randomMin")]
    pub random_min: u32,
    #[serde(rename = "randomMax")]
    pub random_max: u32,
    #[serde(rename = "isChild")]
    pub is_child: bool,
    #[serde(rename = "noInheritState")]
    pub no_inherit_state: bool,
    #[serde(rename = "sameStateAsParent")]
    pub same_state_as_parent: bool,
    #[serde(rename = "DontUseWorn")]
    pub dont_use_worn: bool,
    #[serde(rename = "disableColor")]
    pub disable_color: bool,
    #[serde(rename = "useMatListDontCreateNewMaterial")]
    pub use_mat_list_dont_create_new_material: bool,
    #[serde(rename = "enableOverrideMin")]
    pub enable_override_min: bool,
    #[serde(rename = "enableOverrideMax")]
    pub enable_override_max: bool,
    #[serde(rename = "overRideMin")]
    pub override_min: u32,
    #[serde(rename = "overRideMax")]
    pub override_max: u32,
    pub loaded: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantMaterialSlot {
    pub node: NodeId,
    pub slot: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantFluid {
    pub all: f32,
    pub f: f32,
    pub max_c: f32,
    pub fluids: Vec<VariantFluidAmount>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantFluidAmount {
    pub kind: u32,
    pub amount: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WearableVariant {
    pub nodes: Vec<NodeId>,
    pub weight: Option<f32>,
    pub material: Option<SourceId>,
    pub renderers: Vec<VariantMaterialSlot>,
    pub moves: Vec<NodeId>,
    pub targets: Vec<NodeId>,
    pub part_slots: Vec<SourceId>,
    pub parts: Vec<SourceId>,
    pub tank: Option<SourceId>,
    pub fluid: VariantFluid,
    pub lens: Option<SourceId>,
    pub profile: Option<u32>,
    pub disable_lens_flare: Option<bool>,
}
fn sid(path_id: i64) -> SourceId {
    SourceId {
        file: "sharedassets1.assets".into(),
        path_id,
    }
}
fn scene(path_id: i64) -> SourceId {
    SourceId {
        file: "level1".into(),
        path_id,
    }
}
impl ClothingReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        const ANCESTRY: [i64; 6] = [39843, 39851, 39811, 22249, 39820, 25362];
        const ANCHORS: [i64; 5] = [26477, 28766, 35477, 36615, 39162];
        require(
            self.wearable_type_count == 5
                && self.player_root == scene(39843)
                && self.anchors.iter().eq(ANCHORS
                    .iter()
                    .map(|p| scene(*p))
                    .collect::<Vec<_>>()
                    .iter())
                && self.evidence.valid(),
            "wearable anchor identities",
        )?;
        let sources: Vec<_> = ANCESTRY.into_iter().chain(ANCHORS).collect();
        require(
            self.anchor_nodes.len() == sources.len(),
            "complete wearable anchor ancestry",
        )?;
        for (i, (n, source)) in self.anchor_nodes.iter().zip(sources).enumerate() {
            let parent = if i == 0 {
                None
            } else {
                Some(scene(if i < 6 { ANCESTRY[i - 1] } else { 25362 }))
            };
            require(
                n.source == scene(source)
                    && n.parent == parent
                    && n.coordinate_space == SourceCoordinateSpace::SourceParentLocal
                    && transform(&n.transform),
                "wearable source anchor pose",
            )?;
        }
        require(
            self.glasses_profiles.len() == 6,
            "selected glasses profiles",
        )?;
        for (i, p) in self.glasses_profiles.iter().enumerate() {
            p.validate(i, m)?;
        }
        require(self.wearables.len() == 6, "selected wearable families")?;
        for (w, (wearable, root, item, type_index)) in self.wearables.iter().zip([
            (146739, 70812, 217, 0),
            (147668, 53838, 218, 1),
            (147667, 53837, 221, 2),
            (131348, 63605, 245, 1),
            (148817, 63259, 92, 3),
            (142834, 54495, 230, 4),
        ]) {
            let p = m
                .item_prototypes
                .iter()
                .find(|p| p.id == w.prototype)
                .ok_or_else(|| ReferenceError("wearable prototype missing".into()))?;
            let node_source = |node: NodeId, source: i64| {
                m.nodes
                    .iter()
                    .any(|n| n.id == node && n.source == sid(source) && p.within(m, node))
            };
            require(
                w.source == sid(wearable)
                    && w.prototype.0 == sid(root)
                    && p.item_id == Some(item)
                    && w.type_index == type_index,
                "wearable family source",
            )?;
            ids(w.renderers.iter().copied(), "duplicate wearable renderer")?;
            require(
                w.renderers
                    .iter()
                    .all(|n| p.renderers.iter().any(|r| r.node == *n)),
                "wearable renderer nodes",
            )?;
            let (expected_renderers, expected_colliders): (&[i64], &[i64]) = match wearable {
                146739 => (
                    &[
                        81689, 70739, 63552, 66918, 82244, 73036, 66333, 56341, 60387, 66684,
                        87658, 79878, 83500, 74142, 77647, 80742, 86499, 53868, 58013, 55142,
                        57753, 81212, 83913, 83695, 87099,
                    ],
                    &[112724, 113317, 121089],
                ),
                147668 => (&[86632, 88813], &[117484, 117486]),
                147667 => (&[80368], &[117485]),
                131348 => (
                    &[75397],
                    &[
                        112577, 119446, 118285, 113889, 112407, 110965, 119440, 111925, 121241,
                        116901, 114763, 114256,
                    ],
                ),
                148817 => (&[84062], &[111200, 116405, 124754]),
                142834 => (&[], &[112435]),
                _ => unreachable!(),
            };
            require(
                w.renderers.len() == expected_renderers.len()
                    && w.renderers
                        .iter()
                        .zip(expected_renderers)
                        .all(|(n, id)| node_source(*n, *id)),
                "exact wearable source renderers",
            )?;
            require(
                w.colliders.len() == expected_colliders.len()
                    && w.colliders
                        .iter()
                        .zip(expected_colliders)
                        .all(|(id, source)| {
                            p.colliders
                                .iter()
                                .any(|c| c.id == *id && c.source == sid(*source))
                        }),
                "exact wearable source colliders",
            )?;
            let center = match wearable {
                131348 => Some(69634),
                142834 => Some(56826),
                _ => None,
            };
            require(
                w.use_center_object == center.is_some()
                    && match (w.center, center) {
                        (Some(n), Some(s)) => node_source(n, s),
                        (None, None) => true,
                        _ => false,
                    },
                "wearable center source",
            )?;
            if wearable != 146739 {
                require(
                    w.selectors.is_empty() && w.lenses.is_empty(),
                    "unexpected wearable selector",
                )?;
                continue;
            }
            require(
                w.selectors
                    .iter()
                    .map(|s| s.source.path_id)
                    .eq([135226, 132932, 157851, 154182, 149977, 150967])
                    && w.lenses.len() == 19,
                "complete glasses selection",
            )?;
            ids(
                w.selectors.iter().map(|s| &s.source),
                "duplicate wearable selector",
            )?;
            ids(w.lenses.iter().map(|s| &s.source), "duplicate glasses lens")?;
            for l in &w.lenses {
                require(
                    l.wearable == w.source
                        && p.within(m, l.node)
                        && p.renderers.iter().any(|r| r.node == l.node)
                        && l.profile.is_some_and(|v| v < 6),
                    "glasses startup lens",
                )?;
            }
            for s in &w.selectors {
                require(
                    s.source.file == "sharedassets1.assets"
                        && p.within(m, s.node)
                        && (s.authored_index as usize) < s.variants.len()
                        && s.authored_index == 0
                        && s.random_start,
                    "wearable selector owner",
                )?;
                let (source, weights): (i64, &[f32]) = match s.source.path_id {
                    135226 => (70812, &[10., 10., 10., 10., 10.]),
                    157851 => (57452, &[25., 10., 10., 10., 10., 10.]),
                    154182 => (49311, &[25., 10., 10., 10., 10., 10.]),
                    149977 => (63724, &[10., 10., 10., 10., 10.]),
                    132932 => (70812, &[]),
                    150967 => (85646, &[]),
                    _ => return Err(ReferenceError("unknown wearable selector".into())),
                };
                require(
                    node_source(s.node, source),
                    "wearable selector node identity",
                )?;
                if weights.is_empty() {
                    require(
                        s.kind == WearableSelectorKind::Material
                            && s.variants.len() == 3
                            && s.save_index.is_none(),
                        "wearable material selector",
                    )?;
                    let rules = s
                        .material_rules
                        .as_ref()
                        .ok_or_else(|| ReferenceError("missing material selection rules".into()))?;
                    require(
                        rules.random_min == 0
                            && rules.random_max == 2
                            && rules.is_child == (s.source.path_id == 150967)
                            && !rules.no_inherit_state
                            && !rules.same_state_as_parent
                            && rules.dont_use_worn
                            && rules.disable_color
                            && rules.use_mat_list_dont_create_new_material
                            && !rules.enable_override_min
                            && !rules.enable_override_max
                            && rules.override_min == 0
                            && rules.override_max == 0
                            && !rules.loaded,
                        "material selector source rules",
                    )?;
                    let expected = if s.source.path_id == 132932 {
                        vec![sid(150967)]
                    } else {
                        vec![]
                    };
                    require(s.children == expected, "material selector children")?;
                } else {
                    require(
                        s.kind == WearableSelectorKind::Random
                            && s.save_index == Some(0)
                            && s.material_rules.is_none()
                            && s.children.is_empty()
                            && s.variants
                                .iter()
                                .map(|v| v.weight)
                                .eq(weights.iter().map(|v| Some(*v))),
                        "wearable random selection",
                    )?;
                }
                for v in &s.variants {
                    ids(v.nodes.iter().copied(), "duplicate wearable variant node")?;
                    require(
                        v.nodes.iter().all(|n| p.within(m, *n))
                            && v.moves.is_empty()
                            && v.targets.is_empty()
                            && v.part_slots.is_empty()
                            && v.parts.is_empty()
                            && v.tank.is_none()
                            && v.fluid.fluids.is_empty()
                            && v.fluid.all == 0.
                            && v.fluid.f == 0.
                            && v.fluid.max_c == 0.,
                        "wearable complete selection payload",
                    )?;
                    for r in &v.renderers {
                        require(
                            p.renderers
                                .iter()
                                .any(|n| n.node == r.node && (r.slot as usize) < n.materials.len()),
                            "wearable variant renderer material slot",
                        )?;
                    }
                    require(
                        v.material.as_ref().is_none_or(|s| {
                            m.rendering
                                .materials
                                .iter()
                                .any(|r| r.asset == p.asset && &r.source == s)
                        }),
                        "wearable variant material dependency",
                    )?;
                    let candidates: Vec<_> = w
                        .lenses
                        .iter()
                        .filter(|l| v.nodes.iter().any(|n| descendant(m, l.node, *n)))
                        .collect();
                    let expected = if candidates.len() == 1 {
                        Some(&candidates[0].source)
                    } else {
                        None
                    };
                    require(
                        v.lens.as_ref() == expected,
                        "missing effective variant lens",
                    )?;
                    if let Some(lens) = &v.lens {
                        require(
                            w.lenses.iter().any(|l| {
                                &l.source == lens
                                    && v.nodes.iter().any(|n| descendant(m, l.node, *n))
                                    && l.profile == v.profile
                                    && Some(l.disable_lens_flare) == v.disable_lens_flare
                            }),
                            "effective variant lens",
                        )?;
                    } else {
                        require(
                            v.profile.is_none() && v.disable_lens_flare.is_none(),
                            "unexpected variant profile",
                        )?;
                    }
                }
            }
            validate_selected_glasses(w, m, p)?;
        }
        Ok(())
    }
}

fn validate_selected_glasses(
    w: &WearableReference,
    m: &ReferenceManifest,
    p: &ItemPrototype,
) -> Result<(), ReferenceError> {
    let matches = |node: NodeId, source: i64| {
        m.nodes
            .iter()
            .any(|n| n.id == node && n.source == sid(source) && p.within(m, node))
    };
    let branches: &[(i64, &[i64])] = &[
        (135226, &[57452, 49311, 69685, 63724, 74215]),
        (157851, &[63552, 66918, 82244, 73036, 66333, 56341]),
        (154182, &[66684, 87658, 79878, 83500, 74142, 77647]),
        (149977, &[58013, 55142, 57753, 81212, 83913]),
    ];
    for (source, branches) in branches {
        let selector = w
            .selectors
            .iter()
            .find(|s| s.source == sid(*source))
            .ok_or_else(|| ReferenceError("missing source selector".into()))?;
        require(
            selector.variants.len() == branches.len()
                && selector.variants.iter().zip(*branches).all(|(v, node)| {
                    v.nodes.len() == 1
                        && matches(v.nodes[0], *node)
                        && v.material.is_none()
                        && v.renderers.is_empty()
                }),
            "exact selected variant nodes",
        )?;
    }
    let lenses = &[
        (154009, 63552, 0, true),
        (135972, 66918, 4, true),
        (150488, 82244, 2, true),
        (143500, 73036, 1, true),
        (156901, 66333, 5, true),
        (151834, 56341, 3, true),
        (140782, 66684, 0, true),
        (141256, 87658, 4, true),
        (153056, 79878, 2, true),
        (147364, 83500, 1, true),
        (147011, 74142, 5, true),
        (136426, 77647, 3, true),
        (157020, 86499, 0, true),
        (155894, 58013, 4, false),
        (152881, 55142, 2, false),
        (140470, 57753, 1, false),
        (131005, 81212, 5, false),
        (144723, 83913, 3, false),
        (151873, 87099, 4, false),
    ];
    require(
        w.lenses.len() == lenses.len()
            && w.lenses
                .iter()
                .zip(lenses)
                .all(|(l, (source, node, profile, flare))| {
                    l.source == sid(*source)
                        && matches(l.node, *node)
                        && l.profile == Some(*profile)
                        && l.disable_lens_flare == *flare
                }),
        "exact selected startup lenses",
    )?;
    for (source, nodes) in [(132932, &[81689, 60387, 83695][..]), (150967, &[70739][..])] {
        let selector = w
            .selectors
            .iter()
            .find(|s| s.source == sid(source))
            .ok_or_else(|| ReferenceError("missing material selector".into()))?;
        require(selector.variants.len() == 3, "selected material count")?;
        for (v, (file, material)) in selector.variants.iter().zip([
            ("sharedassets1.assets", 335),
            ("sharedassets1.assets", 253),
            ("sharedassets0.assets", 89),
        ]) {
            require(
                v.material
                    == Some(SourceId {
                        file: file.into(),
                        path_id: material,
                    })
                    && v.nodes.is_empty()
                    && v.weight.is_none()
                    && v.renderers.len() == nodes.len()
                    && v.renderers
                        .iter()
                        .zip(nodes)
                        .all(|(r, node)| r.slot == 0 && matches(r.node, *node)),
                "exact material variant slots",
            )?;
        }
    }
    Ok(())
}

fn descendant(m: &ReferenceManifest, mut node: NodeId, root: NodeId) -> bool {
    for _ in 0..m.nodes.len() {
        if node == root {
            return true;
        }
        match m.nodes.iter().find(|n| n.id == node).and_then(|n| n.parent) {
            Some(parent) => node = parent,
            None => return false,
        }
    }
    false
}
