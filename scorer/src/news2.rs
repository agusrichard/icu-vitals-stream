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

pub fn score_spo2(spo2: i32, supplemental_o2: bool) -> u8 {
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
        score_spo2(v.oxygen_saturation, v.supplemental_o2),
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
