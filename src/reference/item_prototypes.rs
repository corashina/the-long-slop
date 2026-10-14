use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrototypeId(pub SourceId);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemPrototype {
    pub id: PrototypeId,
    pub source: SourceId,
    pub item_id: Option<u32>,
    pub asset: AssetId,
    pub root: NodeId,
    pub bodies: Vec<BodyRecord>,
    pub colliders: Vec<ColliderRecord>,
    pub pickups: Vec<PrototypePickup>,
    pub variants: Vec<PrototypeVariant>,
    pub renderers: Vec<PrototypeRenderer>,
    pub behavior: PrototypeBehavior,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypePickup {
    pub node: NodeId,
    pub body: BodyId,
    pub pickup: PickupReference,
    pub collision_clip: Option<SourceId>,
    pub collision_gain: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypeVariant {
    pub source: SourceId,
    pub node: NodeId,
    pub nodes: Vec<NodeId>,
    pub authored_index: u32,
    pub random_start: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypeRenderer {
    pub source: SourceId,
    pub node: NodeId,
    pub mesh: SourceId,
    pub enabled: bool,
    pub materials: Vec<SourceId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypeBehavior {
    pub pack: Option<SourceId>,
    pub ammo: Option<SourceId>,
    pub cigarette_prefab: Option<SourceId>,
    pub spawn: Option<SourceId>,
    pub breakable: Option<SourceId>,
    pub mount: Option<SourceId>,
    pub debris: Option<PrototypeId>,
    pub reduced_debris: Option<PrototypeId>,
    pub melee: Option<SourceId>,
    pub attack_target: Option<SourceId>,
    pub attack_trigger: Option<SourceId>,
    pub gib: Option<SourceId>,
    pub gib_children: Vec<NodeId>,
    pub clips: Vec<SourceId>,
    pub contact_sound_source: Option<SourceId>,
    pub contact_clips: Vec<Vec<SourceId>>,
}

fn checked(source: &SourceId, path: i64) -> bool {
    source.file == "sharedassets1.assets" && source.path_id == path
}
fn optional_checked(source: &Option<SourceId>, path: i64) -> bool {
    source.as_ref().is_some_and(|s| checked(s, path))
}
fn clip_exists(m: &ReferenceManifest, source: &SourceId) -> bool {
    m.audio["clips"].as_array().is_some_and(|clips| {
        clips.iter().any(|c| {
            c["source"]["file"].as_str() == Some(source.file.as_str())
                && c["source"]["path_id"].as_i64() == Some(source.path_id)
        })
    })
}

impl PickupReference {
    pub fn validate_links(
        &self,
        m: &ReferenceManifest,
        asset: AssetId,
    ) -> Result<(), ReferenceError> {
        require(
            self.disable_when_stored.is_some(),
            "effective pickup storage owner",
        )?;
        for source in [
            &self.right_hand_grip,
            &self.left_hand_grip,
            &self.disable_when_stored,
            &self.keep_when_stored,
        ]
        .into_iter()
        .flatten()
        {
            require(
                m.nodes
                    .iter()
                    .any(|n| &n.source == source && n.asset == asset),
                "pickup Transform owner",
            )?;
        }
        require(
            self.inventory_icon
                .as_ref()
                .is_none_or(|s| m.ui.sprites.iter().any(|sprite| &sprite.source == s)),
            "pickup sprite",
        )?;
        require(
            finite(&self.right_hand_position)
                && finite(&self.recenter_position)
                && self.recenter_distance.is_finite()
                && [&self.right_hand_rotation, &self.recenter_rotation]
                    .iter()
                    .all(|q| {
                        finite(*q) && (q.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
                    }),
            "pickup pose",
        )
    }
}

pub fn validate_prototypes(m: &ReferenceManifest) -> Result<(), ReferenceError> {
    require(
        m.item_prototypes
            .iter()
            .map(|p| p.id.0.path_id)
            .eq([75443, 51722, 61587, 81642, 81645]),
        "selected item prototypes",
    )?;
    ids(
        m.item_prototypes.iter().map(|p| &p.id),
        "duplicate prototype",
    )?;
    ids(
        m.item_prototypes.iter().map(|p| p.asset),
        "duplicate prototype asset",
    )?;
    for p in &m.item_prototypes {
        p.validate(m)?;
    }
    Ok(())
}

impl ItemPrototype {
    fn within(&self, m: &ReferenceManifest, mut node: NodeId) -> bool {
        for _ in 0..=m.nodes.len() {
            let Some(n) = m
                .nodes
                .iter()
                .find(|n| n.id == node && n.asset == self.asset)
            else {
                return false;
            };
            if node == self.root {
                return true;
            }
            let Some(parent) = n.parent else {
                return false;
            };
            node = parent;
        }
        false
    }
    fn body_for(&self, m: &ReferenceManifest, mut node: Option<NodeId>) -> Option<BodyId> {
        for _ in 0..=m.nodes.len() {
            let current = node?;
            if !self.within(m, current) {
                return None;
            }
            if let Some(b) = self.bodies.iter().find(|b| b.node == current) {
                return Some(b.id);
            }
            node = m.nodes.iter().find(|n| n.id == current)?.parent;
        }
        None
    }
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        let (item_id, count, icon, inventory, hand, pickup_source) = match self.id.0.path_id {
            75443 => (Some(74), 1, Some(4624), true, false, Some(149224)),
            51722 => (Some(62), 1, None, false, false, Some(154651)),
            61587 => (Some(216), 1, Some(4553), true, true, Some(144067)),
            81642 => (None, 56, Some(4623), true, false, None),
            81645 => (None, 14, Some(4623), true, false, None),
            _ => return Err(ReferenceError("unknown item prototype".into())),
        };
        require(
            checked(&self.id.0, self.id.0.path_id)
                && self.source == self.id.0
                && self.item_id == item_id
                && m.assets.iter().any(|a| {
                    a.id == self.asset
                        && a.usage == AssetUsage::ItemPrototype
                        && a.root == self.source
                })
                && m.nodes
                    .iter()
                    .any(|n| n.id == self.root && n.source == self.source && n.parent.is_none())
                && self.within(m, self.root),
            "prototype identity",
        )?;
        let (collider_count, renderer_count) = match self.id.0.path_id {
            75443 => (1, 3),
            51722 => (2, 3),
            61587 => (2, 3),
            81642 => (58, 56),
            81645 => (15, 14),
            _ => unreachable!(),
        };
        require(
            self.colliders.len() == collider_count && self.renderers.len() == renderer_count,
            "complete selected geometry",
        )?;
        let bodies = ids(self.bodies.iter().map(|b| b.id), "duplicate local body")?;
        ids(self.bodies.iter().map(|b| b.node), "duplicate body node")?;
        ids(
            self.bodies.iter().map(|b| &b.source),
            "duplicate body source",
        )?;
        ids(
            self.colliders.iter().map(|c| c.id),
            "duplicate local collider",
        )?;
        ids(
            self.colliders.iter().map(|c| &c.source),
            "duplicate collider source",
        )?;
        require(
            self.bodies.len() == count && self.pickups.len() == count && !self.colliders.is_empty(),
            "complete prototype physics",
        )?;
        for b in &self.bodies {
            let parent = m
                .nodes
                .iter()
                .find(|n| n.id == b.node)
                .and_then(|n| n.parent);
            require(
                self.within(m, b.node)
                    && checked(&b.source, b.source.path_id)
                    && b.source.path_id > 0
                    && b.id.0 > 0
                    && b.mass_kg.is_finite()
                    && b.mass_kg > 0.
                    && b.linear_damping.is_finite()
                    && b.linear_damping >= 0.
                    && b.angular_damping.is_finite()
                    && b.angular_damping >= 0.
                    && transform(&b.pose)
                    && b.parent_body == self.body_for(m, parent),
                "prototype body",
            )?;
        }
        for c in &self.colliders {
            require(
                self.within(m, c.node)
                    && checked(&c.source, c.source.path_id)
                    && c.source.path_id > 0
                    && bodies.contains(&c.body)
                    && self.body_for(m, Some(c.node)) == Some(c.body)
                    && c.id.0 > 0
                    && c.membership.is_power_of_two()
                    && finite(&[c.friction, c.restitution])
                    && c.friction >= 0.
                    && c.restitution >= 0.
                    && super::weather::valid_geometry(&c.geometry),
                "prototype collider owner or shape",
            )?;
        }
        ids(self.pickups.iter().map(|p| p.node), "duplicate pickup node")?;
        ids(
            self.pickups.iter().map(|p| &p.pickup.source),
            "duplicate pickup source",
        )?;
        for p in &self.pickups {
            let ineligible = match (self.id.0.path_id, p.pickup.source.path_id) {
                (81642, 135313) => Some(77169),
                (81642, 140775) => Some(82214),
                (81645, 135314) => Some(77170),
                _ => None,
            };
            let expected_inventory = inventory && ineligible.is_none();
            let expected_icon = if ineligible.is_some() { None } else { icon };
            require(
                ineligible.is_none_or(|source| {
                    m.nodes
                        .iter()
                        .any(|n| n.id == p.node && checked(&n.source, source))
                }),
                "ineligible debris source owner",
            )?;
            require(
                self.within(m, p.node)
                    && checked(&p.pickup.source, p.pickup.source.path_id)
                    && self.body_for(m, Some(p.node)) == Some(p.body)
                    && bodies.contains(&p.body)
                    && p.pickup.can_inventory == expected_inventory
                    && p.pickup.can_hold_right_hand == hand
                    && p.pickup.inventory_icon.as_ref().map(|s| s.path_id) == expected_icon
                    && pickup_source
                        .is_none_or(|s| checked(&p.pickup.source, s) && p.node == self.root)
                    && p.collision_gain.is_finite()
                    && p.collision_gain >= 0.
                    && p.collision_clip.as_ref().is_some_and(|s| {
                        checked(s, if item_id == Some(74) { 4270 } else { 4166 })
                            && clip_exists(m, s)
                    }),
                "prototype pickup owner",
            )?;
            p.pickup.validate_links(m, self.asset)?;
            for source in [
                &p.pickup.right_hand_grip,
                &p.pickup.left_hand_grip,
                &p.pickup.disable_when_stored,
                &p.pickup.keep_when_stored,
            ]
            .into_iter()
            .flatten()
            {
                require(
                    m.nodes
                        .iter()
                        .any(|n| &n.source == source && self.within(m, n.id)),
                    "prototype pickup subtree",
                )?;
            }
        }
        ids(
            self.renderers.iter().map(|r| r.node),
            "duplicate renderer node",
        )?;
        ids(
            self.renderers.iter().map(|r| &r.source),
            "duplicate prototype renderer",
        )?;
        require(!self.renderers.is_empty(), "prototype renderers")?;
        for r in &self.renderers {
            require(
                self.within(m, r.node)
                    && checked(&r.source, r.source.path_id)
                    && r.source.path_id > 0
                    && !r.materials.is_empty()
                    && r.materials.iter().all(|source| {
                        m.rendering
                            .materials
                            .iter()
                            .any(|mat| mat.asset == self.asset && &mat.source == source)
                    }),
                "prototype renderer material slots",
            )?;
        }
        if item_id == Some(74) {
            require(self.variants.len() == 1, "pack selector")?;
            let v = &self.variants[0];
            ids(v.nodes.iter().copied(), "duplicate pack variant node")?;
            require(
                checked(&v.source, 140392)
                    && v.node == self.root
                    && v.authored_index == 0
                    && v.random_start
                    && v.nodes.len() == 3
                    && v.nodes
                        .iter()
                        .zip([56803, 56801, 56802])
                        .all(|(id, source)| {
                            m.nodes.iter().any(|n| {
                                n.id == *id && checked(&n.source, source) && self.within(m, n.id)
                            })
                        }),
                "pack variant branches",
            )?;
        } else {
            require(self.variants.is_empty(), "unexpected item variants")?;
        }
        let b = &self.behavior;
        let expected_clips: &[i64] = match item_id {
            Some(62) => &[4223, 4140],
            Some(216) => &[4241, 4452, 4144, 4208],
            _ => &[],
        };
        require(
            b.clips
                .iter()
                .map(|s| s.path_id)
                .eq(expected_clips.iter().copied())
                && b.clips
                    .iter()
                    .all(|s| checked(s, s.path_id) && clip_exists(m, s)),
            "prototype behavior clips",
        )?;
        if item_id == Some(216) {
            let groups: &[&[i64]] = &[
                &[4166],
                &[4151, 4471, 4106],
                &[4164, 4223, 4140],
                &[4164, 4223, 4140],
                &[4354, 4119, 4406, 4305],
                &[4370],
            ];
            require(
                b.contact_sound_source
                    .as_ref()
                    .is_some_and(|s| s.file == "level1" && s.path_id == 64919)
                    && b.contact_clips.len() == groups.len()
                    && b.contact_clips.iter().zip(groups).all(|(clips, expected)| {
                        clips.iter().map(|s| s.path_id).eq(expected.iter().copied())
                            && clips
                                .iter()
                                .all(|s| checked(s, s.path_id) && clip_exists(m, s))
                    }),
                "shovel contact clips",
            )?;
        } else {
            require(
                b.contact_sound_source.is_none() && b.contact_clips.is_empty(),
                "unexpected contact clips",
            )?;
        }
        require(
            b.pack.is_some() == (item_id == Some(74))
                && b.ammo.is_some() == (item_id == Some(74))
                && b.cigarette_prefab.is_some() == (item_id == Some(74))
                && b.spawn.is_some() == (item_id == Some(62))
                && b.breakable.is_some() == (item_id == Some(62))
                && b.mount.is_some() == (item_id == Some(62))
                && b.debris.is_some() == (item_id == Some(62))
                && b.reduced_debris.is_some() == (item_id == Some(62))
                && b.melee.is_some() == (item_id == Some(216))
                && b.attack_target.is_some() == (item_id == Some(216))
                && b.attack_trigger.is_some() == (item_id == Some(216))
                && b.gib.is_some() == item_id.is_none(),
            "prototype behavior kinds",
        )?;
        match item_id {
            Some(74) => require(
                optional_checked(&b.pack, 141307)
                    && optional_checked(&b.ammo, 141757)
                    && optional_checked(&b.cigarette_prefab, 11985),
                "pack source links",
            )?,
            Some(62) => require(
                optional_checked(&b.spawn, 158204)
                    && optional_checked(&b.breakable, 135113)
                    && optional_checked(&b.mount, 146589)
                    && b.debris.as_ref().is_some_and(|id| {
                        checked(&id.0, 81642) && m.item_prototypes.iter().any(|p| &p.id == id)
                    })
                    && b.reduced_debris.as_ref().is_some_and(|id| {
                        checked(&id.0, 81645) && m.item_prototypes.iter().any(|p| &p.id == id)
                    }),
                "Box source links",
            )?,
            Some(216) => require(
                optional_checked(&b.melee, 132938)
                    && optional_checked(&b.attack_target, 53759)
                    && optional_checked(&b.attack_trigger, 70911)
                    && [&b.attack_target, &b.attack_trigger].iter().all(|source| {
                        m.nodes
                            .iter()
                            .any(|n| Some(&n.source) == source.as_ref() && self.within(m, n.id))
                    }),
                "shovel source links",
            )?,
            None => require(
                optional_checked(
                    &b.gib,
                    if self.id.0.path_id == 81642 {
                        153704
                    } else {
                        153707
                    },
                ) && b
                    .gib_children
                    .iter()
                    .copied()
                    .eq(self.pickups.iter().map(|p| p.node))
                    && self.pickups.iter().all(|p| p.node != self.root),
                "debris child owners",
            )?,
            _ => unreachable!(),
        }
        require(
            item_id.is_none() || b.gib_children.is_empty(),
            "unexpected gib children",
        )?;
        require(
            m.bodies.iter().all(|b| !self.within(m, b.node))
                && m.items.iter().all(|i| !self.within(m, i.node))
                && m.colliders.iter().all(|c| {
                    !m.nodes
                        .iter()
                        .any(|n| n.id == c.node && n.asset == self.asset)
                }),
            "prototype active publication",
        )
    }
}
