//! Appearance editor preview stage (`settingsplayercustomscript.camStuff`).
//! All poses are local to `camStuff`. The presentation places `camStuff` away from the world.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewReference {
    pub camera: PreviewCamera,
    pub lights: Vec<PreviewLight>,
    pub figure: PreviewFigure,
}

/// `settingsplayercustomscript.cam`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewCamera {
    pub pose: Transform,
    /// Vertical field of view in degrees.
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewLightKind {
    Spot,
    Directional,
    Point,
}

/// An active `Light` under `camStuff`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewLight {
    pub kind: PreviewLightKind,
    /// Source color (gamma space).
    pub color: [f32; 4],
    pub intensity: f32,
    pub range: f32,
    /// Full cone angle in degrees. Only a spot light uses it.
    pub spot_angle: f32,
    pub pose: Transform,
}

/// `settingsplayercustomscript.outfit`: the preview figure.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewFigure {
    pub root_pose: Transform,
    /// The figure character that gives the joint poses.
    pub character: u32,
    pub joints: Vec<PreviewJoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewJoint {
    /// Node name. Both player characters use the same joint names.
    pub name: String,
    pub pose: Transform,
}

impl PreviewReference {
    /// `joint_names` are the skin joint names of each player character.
    pub fn validate(&self, joint_names: &[HashSet<&str>]) -> Result<(), ReferenceError> {
        let c = &self.camera;
        require(
            transform(&c.pose)
                && c.fov > 0.
                && c.fov < 180.
                && c.near > 0.
                && c.far > c.near
                && c.far.is_finite(),
            "preview camera",
        )?;
        for l in &self.lights {
            require(
                transform(&l.pose)
                    && finite(&l.color)
                    && l.intensity.is_finite()
                    && l.intensity >= 0.
                    && l.range.is_finite()
                    && l.range > 0.
                    && l.spot_angle.is_finite(),
                "preview light",
            )?;
        }
        let f = &self.figure;
        let names = joint_names
            .get(f.character as usize)
            .ok_or_else(|| ReferenceError("preview figure character".into()))?;
        require(transform(&f.root_pose), "preview figure root pose")?;
        ids(
            f.joints.iter().map(|j| j.name.as_str()),
            "duplicate preview joint",
        )?;
        require(
            f.joints.len() == names.len()
                && f.joints
                    .iter()
                    .all(|j| names.contains(j.name.as_str()) && transform(&j.pose)),
            "preview figure joints",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_validation() {
        let pose = r#"{"translation":[0,1,2],"rotation":[0,0,0,1],"scale":[1,1,1]}"#;
        let json = format!(
            r#"{{"camera":{{"pose":{pose},"fov":30,"near":0.1,"far":100}},
            "lights":[{{"kind":"point","color":[1,1,1,1],"intensity":1,"range":10,"spot_angle":30,"pose":{pose}}}],
            "figure":{{"root_pose":{pose},"character":0,"joints":[{{"name":"Hips","pose":{pose}}},{{"name":"Spine","pose":{pose}}}]}}}}"#
        );
        let preview: PreviewReference = serde_json::from_str(&json).unwrap();
        let names = [HashSet::from(["Hips", "Spine"])];
        assert!(preview.validate(&names).is_ok());
        assert!(
            preview
                .validate(&[HashSet::from(["Hips", "Neck"])])
                .is_err()
        );
        assert!(preview.validate(&[]).is_err());
        let mut bad = preview.clone();
        bad.camera.fov = 0.;
        assert!(bad.validate(&names).is_err());
        let mut bad = preview.clone();
        bad.figure.joints[1].pose.translation[0] = f32::NAN;
        assert!(bad.validate(&names).is_err());
        let mut bad = preview;
        bad.figure.joints[1].name = "Hips".into();
        assert!(bad.validate(&names).is_err());
        assert!(
            serde_json::from_str::<PreviewReference>(
                &json.replace("\"fov\"", "\"extra\":1,\"fov\"")
            )
            .is_err()
        );
    }
}
