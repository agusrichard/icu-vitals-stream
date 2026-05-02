use std::sync::Arc;
use crate::alert::{emit_alert, AlertEvent};
use crate::news2;
use crate::vitals::VitalSigns;
use crate::ScorerContext;

pub async fn process_patient(vitals: VitalSigns, ctx: Arc<ScorerContext>) {
    tracing::info!(patient_id = %vitals.patient_id, "received vitals");
    let result = news2::score(&vitals);
    let mut entry = ctx.state.entry(vitals.patient_id.clone()).or_default();
    let prev_tier = entry.last_tier.clone();
    entry.last_tier = Some(result.tier.clone());
    drop(entry);

    if prev_tier.as_ref() != Some(&result.tier)
        && let Some(prev) = prev_tier
    {
        let alert = AlertEvent {
            patient_id: vitals.patient_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            previous_tier: format!("{:?}", prev),
            new_tier: format!("{:?}", result.tier),
            news2_score: result.score as i32,
        };
        if let Err(e) = emit_alert(&ctx.alert_producer, &ctx.alert_schema, &alert).await {
            tracing::warn!("alert emit failed: {}", e);
        }
    }
}
