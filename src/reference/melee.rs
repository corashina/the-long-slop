//! Checked melee hit effects: mainscript hit particles and hit volumes.
use super::*;

/// mainscript.particle order used by meleeweaponscript.
pub const MELEE_HIT_PARTICLES: [&str; 5] = ["spark", "blood", "plant", "wood", "sand"];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeleeReference {
    pub source: SourceId,
    /// mainscript.hitVolume in soundHitType order: metal, blood, plant, wood, sand, glass.
    pub hit_volumes: [f32; 6],
    /// mainscript.Particles in `MELEE_HIT_PARTICLES` order.
    pub particles: [ParticleReference; 5],
}
impl MeleeReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.source.file == "level1"
                && self.hit_volumes.iter().all(|v| v.is_finite() && *v > 0.)
                && self
                    .particles
                    .iter()
                    .zip(MELEE_HIT_PARTICLES)
                    .all(|(p, name)| p.role.strip_prefix("melee_") == Some(name)),
            "melee hit effects",
        )?;
        for p in &self.particles {
            conditions::validate_particle(&self.source, p, m)?;
        }
        Ok(())
    }
}
