pub mod vitals;
pub mod news2;

use apache_avro::from_value;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::message::Message;
use vitals::VitalSigns;

async fn fetch_schema(sr_url: &str, subject: &str) -> anyhow::Result<apache_avro::Schema> {
    let resp: serde_json::Value =
        reqwest::get(format!("{}/subjects/{}/versions/latest", sr_url, subject))
            .await?
            .json()
            .await?;
    let schema_str = resp["schema"].as_str().ok_or_else(|| anyhow::anyhow!("missing schema field"))?;
    Ok(apache_avro::Schema::parse_str(schema_str)?)
}

async fn process_patient(vitals: VitalSigns) {
    tracing::info!(patient_id = %vitals.patient_id, "received vitals");
}

async fn run_consumer(brokers: &str, group_id: &str, schema_registry_url: &str) -> anyhow::Result<()> {
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
                        Ok(vitals) => process_patient(vitals).await,
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

    run_consumer(&brokers, &group_id, &schema_registry_url).await
}
