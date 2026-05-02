pub mod vitals;
pub mod news2;
pub mod state;

use std::sync::Arc;
use apache_avro::from_value;
use dashmap::DashMap;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::message::Message;
use vitals::VitalSigns;
use crate::state::StateStore;

async fn fetch_schema(sr_url: &str, subject: &str) -> anyhow::Result<apache_avro::Schema> {
    let resp: serde_json::Value =
        reqwest::get(format!("{}/subjects/{}/versions/latest", sr_url, subject))
            .await?
            .json()
            .await?;
    let schema_str = resp["schema"].as_str().ok_or_else(|| anyhow::anyhow!("missing schema field"))?;
    Ok(apache_avro::Schema::parse_str(schema_str)?)
}

async fn process_patient(vitals: VitalSigns, state: &Arc<StateStore>) {
    tracing::info!(patient_id = %vitals.patient_id, "received vitals");
    let result = news2::score(&vitals);
    let mut entry = state.entry(vitals.patient_id.clone()).or_default();
    let _prev_tier = entry.last_tier.clone();
    entry.last_tier = Some(result.tier.clone());
    drop(entry);
}

async fn run_consumer(brokers: &str, group_id: &str, schema_registry_url: &str, state: &Arc<StateStore>) -> anyhow::Result<()> {
    let schema = fetch_schema(schema_registry_url, "vitals.raw-value").await?;

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("auto.offset.reset", "earliest")
        .create()?;

    consumer.subscribe(&["vitals.raw"])?;

    loop {
        match consumer.recv().await {
            Ok(msg) => {
                let payload = msg.payload().unwrap_or_default();
                // Strip Confluent wire format header: 0x00 magic byte + 4-byte schema ID
                if payload.len() > 5 && payload[0] == 0x00 {
                    let avro_bytes = &payload[5..];
                    match apache_avro::from_avro_datum(&schema, &mut std::io::Cursor::new(avro_bytes), None)
                        .and_then(|v| from_value::<VitalSigns>(&v))
                    {
                        Ok(vitals) => process_patient(vitals, &state).await,
                        Err(e) => tracing::warn!("decode error: {}", e),
                    }
                }
                consumer.commit_message(&msg, CommitMode::Async)?;
            }
            Err(e) => tracing::warn!("consumer error: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let brokers = std::env::var("BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
    let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "scorer".to_string());
    let schema_registry_url = std::env::var("SCHEMA_REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let state: Arc<StateStore> = Arc::new(DashMap::new());

    run_consumer(&brokers, &group_id, &schema_registry_url, &state).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::news2::News2Tier;

    fn normal_vitals(patient_id: &str) -> VitalSigns {
        VitalSigns {
            patient_id: patient_id.to_string(),
            timestamp: 0,
            simulator_state: "NORMAL".to_string(),
            respiration_rate: 15,
            oxygen_saturation: 97,
            supplemental_o2: false,
            temperature: 37.0,
            systolic_bp: 120,
            heart_rate: 75,
            consciousness_level: "ALERT".to_string(),
        }
    }

    fn critical_vitals(patient_id: &str) -> VitalSigns {
        VitalSigns {
            patient_id: patient_id.to_string(),
            timestamp: 1,
            simulator_state: "CRITICAL".to_string(),
            respiration_rate: 26,
            oxygen_saturation: 90,
            supplemental_o2: true,
            temperature: 39.5,
            systolic_bp: 85,
            heart_rate: 135,
            consciousness_level: "VOICE".to_string(),
        }
    }

    #[tokio::test]
    async fn process_patient_sets_initial_state() {
        let state: Arc<StateStore> = Arc::new(DashMap::new());
        process_patient(normal_vitals("p1"), &state).await;
        assert_eq!(state.get("p1").unwrap().last_tier, Some(News2Tier::Low));
    }

    #[tokio::test]
    async fn process_patient_updates_state_between_readings() {
        let state: Arc<StateStore> = Arc::new(DashMap::new());

        process_patient(normal_vitals("p1"), &state).await;
        assert_eq!(state.get("p1").unwrap().last_tier, Some(News2Tier::Low));

        process_patient(critical_vitals("p1"), &state).await;
        assert_eq!(state.get("p1").unwrap().last_tier, Some(News2Tier::High));
    }

    #[tokio::test]
    async fn process_patient_isolates_state_per_patient() {
        let state: Arc<StateStore> = Arc::new(DashMap::new());

        process_patient(normal_vitals("p1"), &state).await;
        process_patient(critical_vitals("p2"), &state).await;

        assert_eq!(state.get("p1").unwrap().last_tier, Some(News2Tier::Low));
        assert_eq!(state.get("p2").unwrap().last_tier, Some(News2Tier::High));
    }
}
