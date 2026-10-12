use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShelterReference {
    pub source: SourceId,
    pub root: BodyId,
    pub geometry: ColliderGeometry,
    pub local_pose: Transform,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleCurve {
    pub mode: u8,
    pub min_scalar: f32,
    pub scalar: f32,
    pub min_curve: TorqueCurve,
    pub max_curve: TorqueCurve,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleColor {
    pub mode: u8,
    pub min_color: [f32; 4],
    pub max_color: [f32; 4],
    pub min_gradient: GradientReference,
    pub max_gradient: GradientReference,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleLimit {
    pub source: SourceId,
    pub module: String,
    pub visible_effect: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleReference {
    pub source: SourceId,
    pub role: String,
    pub capacity: u32,
    pub rate_over_time: f32,
    pub rate_over_distance: f32,
    pub duration: f32,
    pub simulation_space: u8,
    pub local_pose: Transform,
    pub module_flags: HashMap<String, bool>,
    pub curves: HashMap<String, ParticleCurve>,
    pub colors: HashMap<String, ParticleColor>,
    pub vectors: HashMap<String, [f32; 3]>,
    pub scalars: HashMap<String, f32>,
    pub flags: HashMap<String, bool>,
    pub links: HashMap<String, SourceId>,
    pub materials: Vec<SkyMaterialReference>,
    pub limits: Vec<ParticleLimit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StormReference {
    pub source: SourceId,
    pub biome: u8,
    pub life_loss_on_other_biome: f32,
    pub life_gain: f32,
    pub destroy_dist: f32,
    pub life: f32,
    pub radius: f32,
    pub fog_density: f32,
    pub atmosphere_thickness: f32,
    pub intensity_multipler: f32,
    pub ambient_multipler: f32,
    pub volume: f32,
    pub ambient_decrease: f32,
    pub yoffset: f32,
    pub wind_force: f32,
    pub current_force: [f32; 3],
    pub next_force: [f32; 3],
    pub distance_lerp: f32,
    pub def_s_rate_t: f32,
    pub def_s_rate_d: f32,
    pub def_b_rate_t: f32,
    pub def_b_rate_d: f32,
    pub night_color: [f32; 4],
    pub sky_color: [f32; 4],
    pub sky_ground_color: [f32; 4],
    pub fog_color: [f32; 4],
    pub cloud_color: [f32; 4],
    pub day_color1: GradientReference,
    pub day_color2: GradientReference,
    pub wind_lm: u32,
    pub size_range: [f32; 2],
    pub speed_range: [f32; 2],
    pub wind_time_range: [f32; 2],
    pub wind_scan_seconds: f32,
    pub wind_bounds_range: [f32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherReference {
    pub source: SourceId,
    pub min_interval_seconds: f32,
    pub max_interval_seconds: f32,
    pub spawn_radius: f32,
    pub storm: StormReference,
    pub shelters: Vec<ShelterReference>,
    pub particles: Vec<ParticleReference>,
    pub evidence: ManagedEvidence,
}
pub(super) fn valid_geometry(g: &ColliderGeometry) -> bool {
    match g {
        ColliderGeometry::Box {
            center,
            half_extents,
            rotation,
        } => {
            finite(center)
                && finite(half_extents)
                && half_extents.iter().all(|v| *v > 0.)
                && finite(rotation)
        }
        ColliderGeometry::Sphere { center, radius } => {
            finite(center) && radius.is_finite() && *radius > 0.
        }
        ColliderGeometry::Capsule { a, b, radius } => {
            finite(a) && finite(b) && radius.is_finite() && *radius > 0.
        }
        ColliderGeometry::Convex { points } => {
            points.len() >= 4 && points.iter().all(|p| finite(p))
        }
        ColliderGeometry::Trimesh { points, triangles } => {
            points.len() >= 3
                && points.iter().all(|p| finite(p))
                && !triangles.is_empty()
                && triangles
                    .iter()
                    .flatten()
                    .all(|i| (*i as usize) < points.len())
        }
    }
}
fn gradient_valid(g: &GradientReference) -> bool {
    g.mode <= 1
        && !g.colors.is_empty()
        && !g.alphas.is_empty()
        && g.colors
            .iter()
            .all(|k| k.time.is_finite() && (0. ..=1.).contains(&k.time) && finite(&k.value))
        && g.alphas
            .iter()
            .all(|k| finite(&[k.time, k.value]) && (0. ..=1.).contains(&k.time))
        && g.colors.windows(2).all(|w| w[0].time <= w[1].time)
        && g.alphas.windows(2).all(|w| w[0].time <= w[1].time)
}
fn curve_valid(c: &TorqueCurve) -> bool {
    c.keys.iter().all(|k| {
        finite(&[
            k.time,
            k.value,
            k.in_tangent,
            k.out_tangent,
            k.in_weight,
            k.out_weight,
        ]) && k.weighted_mode <= 3
    }) && c.keys.windows(2).all(|w| w[0].time < w[1].time)
}
impl WeatherReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.evidence.valid()
                && finite(&[
                    self.min_interval_seconds,
                    self.max_interval_seconds,
                    self.spawn_radius,
                ])
                && self.min_interval_seconds > 0.
                && self.max_interval_seconds >= self.min_interval_seconds
                && self.spawn_radius > 0.,
            "weather schedule",
        )?;
        let s = &self.storm;
        require(
            finite(&[
                s.life_loss_on_other_biome,
                s.life_gain,
                s.destroy_dist,
                s.life,
                s.radius,
                s.fog_density,
                s.atmosphere_thickness,
                s.intensity_multipler,
                s.ambient_multipler,
                s.volume,
                s.ambient_decrease,
                s.yoffset,
                s.wind_force,
                s.distance_lerp,
                s.def_s_rate_t,
                s.def_s_rate_d,
                s.def_b_rate_t,
                s.def_b_rate_d,
                s.wind_scan_seconds,
            ]) && finite(&s.current_force)
                && finite(&s.next_force)
                && s.distance_lerp >= 0.
                && s.radius > 0.
                && s.destroy_dist >= s.radius
                && s.life_gain > 0.
                && s.life_loss_on_other_biome > 0.
                && s.wind_scan_seconds > 0.,
            "storm rates",
        )?;
        require(
            [
                s.size_range,
                s.speed_range,
                s.wind_time_range,
                s.wind_bounds_range,
            ]
            .iter()
            .all(|r| finite(r) && r[0] > 0. && r[1] >= r[0])
                && [
                    s.night_color,
                    s.sky_color,
                    s.sky_ground_color,
                    s.fog_color,
                    s.cloud_color,
                ]
                .iter()
                .all(|c| finite(c))
                && gradient_valid(&s.day_color1)
                && gradient_valid(&s.day_color2),
            "storm curves",
        )?;
        require(self.shelters.len() == 6, "selected shelters")?;
        ids(self.shelters.iter().map(|s| &s.source), "duplicate shelter")?;
        for s in &self.shelters {
            let owner = match (s.source.file.as_str(), s.source.path_id) {
                ("level1", 73428 | 79089 | 72174 | 71312) => 21400,
                ("level1", 76881 | 68729) => 37191,
                _ => return Err(ReferenceError("Invalid selected shelter source".into())),
            };
            require(
                m.bodies.iter().find(|b| b.id == s.root).is_some_and(|b| {
                    b.parent_body.is_none()
                        && m.nodes.iter().any(|n| {
                            n.id == b.node && n.source.file == "level1" && n.source.path_id == owner
                        })
                }) && valid_geometry(&s.geometry)
                    && transform(&s.local_pose),
                "shelter root and geometry",
            )?;
        }
        require(self.particles.len() == 4, "selected particles")?;
        ids(
            self.particles.iter().map(|p| &p.source),
            "duplicate particle",
        )?;
        ids(
            self.particles.iter().map(|p| p.role.as_str()),
            "duplicate particle role",
        )?;
        for p in &self.particles {
            require(
                p.capacity > 0
                    && p.capacity <= 1_000_000
                    && p.scalars.get("InitialModule.maxNumParticles") == Some(&(p.capacity as f32)),
                "particle capacity",
            )?;
            require(
                finite(&[p.rate_over_time, p.rate_over_distance, p.duration])
                    && p.rate_over_time >= 0.
                    && p.rate_over_distance >= 0.
                    && p.duration > 0.
                    && p.simulation_space <= 2
                    && transform(&p.local_pose),
                "particle rates",
            )?;
            require(
                p.module_flags.get("InitialModule") == Some(&true)
                    && p.curves.contains_key("InitialModule.startLifetime")
                    && !p.materials.is_empty(),
                "particle modules",
            )?;
            require(
                p.scalars.values().all(|v| v.is_finite()) && p.vectors.values().all(|v| finite(v)),
                "particle values",
            )?;
            for c in p.curves.values() {
                require(
                    c.mode <= 3
                        && finite(&[c.scalar, c.min_scalar])
                        && curve_valid(&c.min_curve)
                        && curve_valid(&c.max_curve),
                    "particle curve",
                )?;
            }
            for c in p.colors.values() {
                require(
                    c.mode <= 4
                        && finite(&c.min_color)
                        && finite(&c.max_color)
                        && gradient_valid(&c.min_gradient)
                        && gradient_valid(&c.max_gradient),
                    "particle color",
                )?;
            }
            for l in &p.limits {
                require(
                    l.source == p.source
                        && p.module_flags.get(&l.module) == Some(&true)
                        && !l.visible_effect.is_empty(),
                    "particle limit",
                )?;
            }
        }
        Ok(())
    }
}
