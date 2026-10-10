use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialLink {
    pub source: SourceId,
    pub asset: AssetId,
    pub gltf_material: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedleReference {
    pub source: SourceId,
    pub node: NodeId,
    pub signal: String,
    pub min_value: f32,
    pub max_value: f32,
    pub min_angle: f32,
    pub max_angle: f32,
    pub axis: u8,
    pub rest: [f32; 3],
    pub offset: f32,
    pub inv: bool,
    pub value_smoothing: f32,
    pub signal_smoothing: f32,
    pub no_value_lerp: bool,
    pub lerp_angle: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LampReference {
    pub source: SourceId,
    pub node: NodeId,
    pub signal: String,
    pub on: MaterialLink,
    pub off: MaterialLink,
    pub min_value: f32,
    pub max_value: f32,
    pub stay_on: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigitReference {
    pub node: NodeId,
    pub place: i32,
    pub axis: u8,
    pub inv: bool,
    pub direct: bool,
    pub continuous: bool,
    pub initial_value: f32,
    pub initial_angle: f32,
    pub on: MaterialLink,
    pub off: MaterialLink,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OdometerReference {
    pub source: SourceId,
    pub initial_value: f32,
    pub base: u32,
    pub decimals: f32,
    pub use_multiplier: bool,
    pub multiplier: f32,
    pub digits: Vec<DigitReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentReference {
    pub electronics_output: f32,
    pub electronics_off_threshold: f32,
    pub needles: Vec<NeedleReference>,
    pub lamps: Vec<LampReference>,
    pub odometer: OdometerReference,
    pub low_fuel: f32,
    pub low_oil: f32,
    pub oil_test_rpm: f32,
    pub battery_test_rpm: f32,
}
impl InstrumentReference {
    pub fn validate(
        &self,
        nodes: &HashSet<NodeId>,
        assets: &[AssetRecord],
    ) -> Result<(), ReferenceError> {
        require(
            self.needles.len() == 3 && self.lamps.len() == 4 && self.odometer.digits.len() == 5,
            "Lada instruments",
        )?;
        ids(
            self.needles.iter().map(|n| &n.signal),
            "duplicate needle signal",
        )?;
        for n in &self.needles {
            require(
                nodes.contains(&n.node)
                    && ["speed", "temp", "fuel"].contains(&n.signal.as_str())
                    && n.axis < 3
                    && n.min_value < n.max_value
                    && finite(&[
                        n.min_value,
                        n.max_value,
                        n.min_angle,
                        n.max_angle,
                        n.offset,
                        n.value_smoothing,
                        n.signal_smoothing,
                    ])
                    && finite(&n.rest)
                    && n.value_smoothing >= 0.
                    && n.signal_smoothing >= 0.,
                "instrument needle",
            )?;
        }
        let material = |m: &MaterialLink| {
            require(
                assets
                    .iter()
                    .any(|a| a.id == m.asset && m.gltf_material < a.material_count),
                "instrument material",
            )
        };
        for l in &self.lamps {
            require(
                nodes.contains(&l.node)
                    && ["fuel", "oil", "battery", "headlight2"].contains(&l.signal.as_str())
                    && finite(&[l.min_value, l.max_value]),
                "instrument lamp",
            )?;
            material(&l.on)?;
            material(&l.off)?;
        }
        let o = &self.odometer;
        require(
            o.base == 10
                && o.decimals == 1.
                && finite(&[o.initial_value, o.multiplier])
                && o.initial_value >= 0.,
            "odometer",
        )?;
        for d in &o.digits {
            require(
                nodes.contains(&d.node)
                    && d.axis < 3
                    && (0..5).contains(&d.place)
                    && finite(&[d.initial_value, d.initial_angle]),
                "odometer digit",
            )?;
            material(&d.on)?;
            material(&d.off)?;
        }
        ids(o.digits.iter().map(|d| d.place), "duplicate digit place")?;
        require(
            finite(&[
                self.electronics_output,
                self.electronics_off_threshold,
                self.low_fuel,
                self.low_oil,
                self.oil_test_rpm,
                self.battery_test_rpm,
            ]) && self.low_fuel > 0.
                && self.low_oil > 0.
                && self.oil_test_rpm > 0.
                && self.battery_test_rpm > 0.,
            "instrument thresholds",
        )?;
        Ok(())
    }
}
