use crate::vitals::VitalSigns;

pub fn score_rr(rr: i32) -> u8 {
    match rr {
        ..=8   => 3,
        9..=11 => 1,
        12..=20 => 0,
        21..=24 => 2,
        25..   => 3,
    }
}

pub fn score_spo2(spo2: i32) -> u8 {
    // Scale 1 (no COPD): standard thresholds
    match spo2 {
        ..=91  => 3,
        92..=93 => 2,
        94..=95 => 1,
        96..    => 0,
    }
}

pub fn score_supplemental_o2(on_o2: bool) -> u8 {
    if on_o2 { 2 } else { 0 }
}

pub fn score_temp(temp: f64) -> u8 {
    match temp {
        t if t <= 35.0            => 3,
        t if t <= 36.0            => 1,
        t if t <= 38.0            => 0,
        t if t <= 39.0            => 1,
        _                         => 2,
    }
}

pub fn score_bp(sbp: i32) -> u8 {
    match sbp {
        ..=90   => 3,
        91..=100 => 2,
        101..=110 => 1,
        111..=219 => 0,
        220..    => 3,
    }
}

pub fn score_hr(hr: i32) -> u8 {
    match hr {
        ..=40   => 3,
        41..=50 => 1,
        51..=90 => 0,
        91..=110 => 1,
        111..=130 => 2,
        131..    => 3,
    }
}

pub fn score_consciousness(level: &str) -> u8 {
    match level {
        "ALERT" => 0,
        _       => 3,  // NEW_CONFUSION, VOICE, PAIN, UNRESPONSIVE all score 3
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum News2Tier { Low, Medium, High }

pub struct News2Result {
    pub score: u8,
    pub tier: News2Tier,
}

pub fn score(v: &VitalSigns) -> News2Result {
    let scores = [
        score_rr(v.respiration_rate),
        score_spo2(v.oxygen_saturation),
        score_supplemental_o2(v.supplemental_o2),
        score_temp(v.temperature),
        score_bp(v.systolic_bp),
        score_hr(v.heart_rate),
        score_consciousness(&v.consciousness_level),
    ];
    let total: u8 = scores.iter().sum();
    let any_extreme = scores.iter().any(|&s| s >= 3);
    let tier = match total {
        0           => News2Tier::Low,   // score=0 means no alert needed
        1..=4 if !any_extreme => News2Tier::Low,
        1..=4                 => News2Tier::Low, // single param >=3 is still Low per NEWS2
        5..=6       => News2Tier::Medium,
        _           => News2Tier::High,
    };
    News2Result { score: total, tier }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vitals::VitalSigns;

    fn vitals(
        respiration_rate: i32,
        oxygen_saturation: i32,
        supplemental_o2: bool,
        temperature: f64,
        systolic_bp: i32,
        heart_rate: i32,
        consciousness_level: &str,
    ) -> VitalSigns {
        VitalSigns {
            patient_id: "p1".to_string(),
            timestamp: 0,
            simulator_state: "NORMAL".to_string(),
            respiration_rate,
            oxygen_saturation,
            supplemental_o2,
            temperature,
            systolic_bp,
            heart_rate,
            consciousness_level: consciousness_level.to_string(),
        }
    }

    // --- score_rr ---

    #[test]
    fn rr_boundary_bradypnoea() {
        assert_eq!(score_rr(8), 3);
        assert_eq!(score_rr(0), 3);
    }

    #[test]
    fn rr_boundary_low_normal() {
        assert_eq!(score_rr(9), 1);
        assert_eq!(score_rr(11), 1);
    }

    #[test]
    fn rr_boundary_normal() {
        assert_eq!(score_rr(12), 0);
        assert_eq!(score_rr(20), 0);
    }

    #[test]
    fn rr_boundary_elevated() {
        assert_eq!(score_rr(21), 2);
        assert_eq!(score_rr(24), 2);
    }

    #[test]
    fn rr_boundary_tachypnoea() {
        assert_eq!(score_rr(25), 3);
        assert_eq!(score_rr(40), 3);
    }

    // --- score_spo2 ---

    #[test]
    fn spo2_boundary_critical() {
        assert_eq!(score_spo2(91), 3);
        assert_eq!(score_spo2(80), 3);
    }

    #[test]
    fn spo2_boundary_low() {
        assert_eq!(score_spo2(92), 2);
        assert_eq!(score_spo2(93), 2);
    }

    #[test]
    fn spo2_boundary_borderline() {
        assert_eq!(score_spo2(94), 1);
        assert_eq!(score_spo2(95), 1);
    }

    #[test]
    fn spo2_boundary_normal() {
        assert_eq!(score_spo2(96), 0);
        assert_eq!(score_spo2(100), 0);
    }

    // --- score_supplemental_o2 ---

    #[test]
    fn supplemental_o2_on() {
        assert_eq!(score_supplemental_o2(true), 2);
    }

    #[test]
    fn supplemental_o2_off() {
        assert_eq!(score_supplemental_o2(false), 0);
    }

    // --- score_temp ---

    #[test]
    fn temp_boundary_hypothermia() {
        assert_eq!(score_temp(35.0), 3);
        assert_eq!(score_temp(32.0), 3);
    }

    #[test]
    fn temp_boundary_low() {
        assert_eq!(score_temp(35.1), 1);
        assert_eq!(score_temp(36.0), 1);
    }

    #[test]
    fn temp_boundary_normal() {
        assert_eq!(score_temp(36.1), 0);
        assert_eq!(score_temp(38.0), 0);
    }

    #[test]
    fn temp_boundary_low_fever() {
        assert_eq!(score_temp(38.1), 1);
        assert_eq!(score_temp(39.0), 1);
    }

    #[test]
    fn temp_boundary_high_fever() {
        assert_eq!(score_temp(39.1), 2);
        assert_eq!(score_temp(41.0), 2);
    }

    // --- score_bp ---

    #[test]
    fn bp_boundary_severe_hypotension() {
        assert_eq!(score_bp(90), 3);
        assert_eq!(score_bp(60), 3);
    }

    #[test]
    fn bp_boundary_moderate_hypotension() {
        assert_eq!(score_bp(91), 2);
        assert_eq!(score_bp(100), 2);
    }

    #[test]
    fn bp_boundary_mild_hypotension() {
        assert_eq!(score_bp(101), 1);
        assert_eq!(score_bp(110), 1);
    }

    #[test]
    fn bp_boundary_normal() {
        assert_eq!(score_bp(111), 0);
        assert_eq!(score_bp(219), 0);
    }

    #[test]
    fn bp_boundary_severe_hypertension() {
        assert_eq!(score_bp(220), 3);
        assert_eq!(score_bp(260), 3);
    }

    // --- score_hr ---

    #[test]
    fn hr_boundary_severe_bradycardia() {
        assert_eq!(score_hr(40), 3);
        assert_eq!(score_hr(20), 3);
    }

    #[test]
    fn hr_boundary_mild_bradycardia() {
        assert_eq!(score_hr(41), 1);
        assert_eq!(score_hr(50), 1);
    }

    #[test]
    fn hr_boundary_normal() {
        assert_eq!(score_hr(51), 0);
        assert_eq!(score_hr(90), 0);
    }

    #[test]
    fn hr_boundary_mild_tachycardia() {
        assert_eq!(score_hr(91), 1);
        assert_eq!(score_hr(110), 1);
    }

    #[test]
    fn hr_boundary_moderate_tachycardia() {
        assert_eq!(score_hr(111), 2);
        assert_eq!(score_hr(130), 2);
    }

    #[test]
    fn hr_boundary_severe_tachycardia() {
        assert_eq!(score_hr(131), 3);
        assert_eq!(score_hr(180), 3);
    }

    // --- score_consciousness ---

    #[test]
    fn consciousness_alert() {
        assert_eq!(score_consciousness("ALERT"), 0);
    }

    #[test]
    fn consciousness_impaired() {
        assert_eq!(score_consciousness("VOICE"), 3);
        assert_eq!(score_consciousness("PAIN"), 3);
        assert_eq!(score_consciousness("UNRESPONSIVE"), 3);
        assert_eq!(score_consciousness("NEW_CONFUSION"), 3);
    }

    // --- aggregate score() and tier classification ---

    #[test]
    fn score_all_normal_gives_zero_low() {
        // RR=15(0) SpO2=98(0) no-O2(0) temp=37.0(0) BP=120(0) HR=70(0) ALERT(0) = 0
        let result = score(&vitals(15, 98, false, 37.0, 120, 70, "ALERT"));
        assert_eq!(result.score, 0);
        assert_eq!(result.tier, News2Tier::Low);
    }

    #[test]
    fn score_medium_tier() {
        // RR=22(2) SpO2=98(0) no-O2(0) temp=37.0(0) BP=95(2) HR=100(1) ALERT(0) = 5
        let result = score(&vitals(22, 98, false, 37.0, 95, 100, "ALERT"));
        assert_eq!(result.score, 5);
        assert_eq!(result.tier, News2Tier::Medium);
    }

    #[test]
    fn score_high_tier() {
        // RR=25(3) SpO2=91(3) on-O2(2) temp=37.0(0) BP=120(0) HR=70(0) ALERT(0) = 8
        let result = score(&vitals(25, 91, true, 37.0, 120, 70, "ALERT"));
        assert_eq!(result.score, 8);
        assert_eq!(result.tier, News2Tier::High);
    }

    #[test]
    fn score_low_tier_with_single_extreme_param() {
        // RR=25(3) SpO2=98(0) no-O2(0) temp=37.0(0) BP=120(0) HR=70(0) ALERT(0) = 3
        // Single param >=3 but total is 3, which is 1..=4 → Low
        let result = score(&vitals(25, 98, false, 37.0, 120, 70, "ALERT"));
        assert_eq!(result.score, 3);
        assert_eq!(result.tier, News2Tier::Low);
    }

    #[test]
    fn score_tier_boundary_medium_to_high() {
        // RR=22(2) SpO2=98(0) no-O2(0) temp=37.0(0) BP=91(2) HR=111(2) ALERT(0) = 6 → Medium
        let result = score(&vitals(22, 98, false, 37.0, 91, 111, "ALERT"));
        assert_eq!(result.score, 6);
        assert_eq!(result.tier, News2Tier::Medium);
    }
}