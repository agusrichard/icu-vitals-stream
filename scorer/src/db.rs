use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use crate::alert::AlertEvent;
use crate::news2::News2Result;
use crate::vitals::VitalSigns;

pub async fn insert_scored_reading(pool: &PgPool, v: &VitalSigns, n: &News2Result) -> anyhow::Result<()> {
    let time: DateTime<Utc> = Utc.timestamp_millis_opt(v.timestamp).single()
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp: {}", v.timestamp))?;
    let tier = format!("{:?}", n.tier);
    sqlx::query!(
        r#"
        INSERT INTO scored_readings
          (time, patient_id, simulator_state, respiration_rate, oxygen_saturation,
           supplemental_o2, temperature, systolic_bp, heart_rate, consciousness_level,
           news2_score, news2_tier)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        "#,
        time, v.patient_id, v.simulator_state,
        v.respiration_rate, v.oxygen_saturation, v.supplemental_o2,
        v.temperature, v.systolic_bp, v.heart_rate, v.consciousness_level,
        n.score as i32, tier
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_alert(pool: &PgPool, alert: &AlertEvent) -> anyhow::Result<()> {
    let time: DateTime<Utc> = Utc.timestamp_millis_opt(alert.timestamp).single()
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp: {}", alert.timestamp))?;
    sqlx::query!(
        "INSERT INTO alerts (time, patient_id, previous_tier, new_tier, news2_score)
         VALUES ($1,$2,$3,$4,$5)",
        time, alert.patient_id, alert.previous_tier,
        alert.new_tier, alert.news2_score
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    #[test]
    fn zero_timestamp_is_unix_epoch() {
        let dt = Utc.timestamp_millis_opt(0).single();
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().timestamp_millis(), 0);
    }

    #[test]
    fn valid_recent_timestamp_converts() {
        let ts = 1_700_000_000_000i64; // ~Nov 2023
        let dt = Utc.timestamp_millis_opt(ts).single();
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().timestamp_millis(), ts);
    }

    #[test]
    fn overflow_timestamp_returns_none() {
        // i64::MAX milliseconds exceeds chrono's representable range
        let dt = Utc.timestamp_millis_opt(i64::MAX).single();
        assert!(dt.is_none());
    }
}
