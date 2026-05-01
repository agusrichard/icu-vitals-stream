#[derive(Debug, Clone, serde::Deserialize)]
pub struct VitalSigns {
    pub patient_id: String,
    pub timestamp: i64,
    pub simulator_state: String,
    pub respiration_rate: i32,
    pub oxygen_saturation: i32,
    pub supplemental_o2: bool,
    pub temperature: f64,
    pub systolic_bp: i32,
    pub heart_rate: i32,
    pub consciousness_level: String,
}