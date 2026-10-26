use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialLink {
    pub source: SourceId,
    pub asset: AssetId,
    pub gltf_material: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterialSwitchReference {
    Absent,
    Renderer {
        node: NodeId,
        on: MaterialLink,
        off: MaterialLink,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedleReference {
    pub source: SourceId,
    pub node: NodeId,
    pub backlight: MaterialSwitchReference,
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
/// Gauges of one car: every meter in the car prefab, with any needle and lamp count.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentReference {
    /// One `meterscript` backlight per meter.
    pub backlights: Vec<MaterialSwitchReference>,
    pub electronics_output: f32,
    pub electronics_off_threshold: f32,
    pub needles: Vec<NeedleReference>,
    pub lamps: Vec<LampReference>,
    /// None: no meter has a digit counter.
    pub odometer: Option<OdometerReference>,
    pub low_fuel: f32,
    pub low_oil: f32,
    pub oil_test_rpm: f32,
    pub battery_test_rpm: f32,
}
impl InstrumentReference {
    /// Odometer digits. Empty without an odometer.
    pub fn digits(&self) -> &[DigitReference] {
        self.odometer.as_ref().map_or(&[], |o| &o.digits)
    }
    /// The meter backlights and the needle backlights.
    pub fn all_backlights(&self) -> impl Iterator<Item = &MaterialSwitchReference> {
        self.backlights
            .iter()
            .chain(self.needles.iter().map(|n| &n.backlight))
    }
}
