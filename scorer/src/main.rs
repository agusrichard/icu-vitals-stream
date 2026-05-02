pub mod vitals;
pub mod news2;
pub mod state;
pub mod alert;
pub mod patient;
pub mod schema;

use std::sync::Arc;
use apache_avro::from_value;
use dashmap::DashMap;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::message::Message;
use rdkafka::producer::FutureProducer;
use vitals::VitalSigns;
use state::StateStore;
use patient::process_patient;
use schema::RegisteredSchema;

pub struct ScorerContext {
    pub alert_producer: FutureProducer,
    pub alert_schema: RegisteredSchema,
    pub state: StateStore,
}

async fn run_consumer(brokers: &str, group_id: &str, schema_registry_url: &str, ctx: Arc<ScorerContext>) -> anyhow::Result<()> {
    let vitals_schema = RegisteredSchema::new(schema_registry_url, "vitals.raw-value").await?;

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
                    match apache_avro::from_avro_datum(&vitals_schema.schema, &mut std::io::Cursor::new(avro_bytes), None)
                        .and_then(|v| from_value::<VitalSigns>(&v))
                    {
                        Ok(vitals) => process_patient(vitals, Arc::clone(&ctx)).await,
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

    let ctx = Arc::new(ScorerContext {
        alert_producer: ClientConfig::new()
            .set("bootstrap.servers", &brokers)
            .create()?,
        alert_schema: RegisteredSchema::new(&schema_registry_url, "vitals.alerts-value").await?,
        state: DashMap::new(),
    });

    run_consumer(&brokers, &group_id, &schema_registry_url, ctx).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::news2::News2Tier;

    fn make_ctx() -> Arc<ScorerContext> {
        Arc::new(ScorerContext {
            alert_producer: ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .unwrap(),
            alert_schema: RegisteredSchema::dummy(),
            state: DashMap::new(),
        })
    }

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
        let ctx = make_ctx();
        process_patient(normal_vitals("p1"), Arc::clone(&ctx)).await;
        assert_eq!(ctx.state.get("p1").unwrap().last_tier, Some(News2Tier::Low));
    }

    #[tokio::test]
    async fn process_patient_updates_state_between_readings() {
        let ctx = make_ctx();

        process_patient(normal_vitals("p1"), Arc::clone(&ctx)).await;
        assert_eq!(ctx.state.get("p1").unwrap().last_tier, Some(News2Tier::Low));

        process_patient(critical_vitals("p1"), Arc::clone(&ctx)).await;
        assert_eq!(ctx.state.get("p1").unwrap().last_tier, Some(News2Tier::High));
    }

    #[tokio::test]
    async fn process_patient_isolates_state_per_patient() {
        let ctx = make_ctx();

        process_patient(normal_vitals("p1"), Arc::clone(&ctx)).await;
        process_patient(critical_vitals("p2"), Arc::clone(&ctx)).await;

        assert_eq!(ctx.state.get("p1").unwrap().last_tier, Some(News2Tier::Low));
        assert_eq!(ctx.state.get("p2").unwrap().last_tier, Some(News2Tier::High));
    }
}
