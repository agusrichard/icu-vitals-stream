use rdkafka::producer::{FutureProducer, FutureRecord};
use crate::schema::RegisteredSchema;

#[derive(serde::Serialize)]
pub struct ScoredReading {
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
    pub news2_score: i32,
    pub news2_tier: String,
}

pub async fn emit_scored(
    producer: &FutureProducer,
    registered: &RegisteredSchema,
    reading: &ScoredReading,
) -> anyhow::Result<()> {
    let value = apache_avro::to_value(reading)?;
    let avro_bytes = apache_avro::to_avro_datum(&registered.schema, value)?;

    let mut msg = Vec::with_capacity(5 + avro_bytes.len());
    msg.push(0x00);
    msg.extend_from_slice(&registered.id.to_be_bytes());
    msg.extend_from_slice(&avro_bytes);

    let record = FutureRecord::to("vitals.scored")
        .key(&reading.patient_id)
        .payload(&msg);
    producer.send(record, rdkafka::util::Timeout::Never).await
        .map_err(|(e, _)| anyhow::anyhow!(e))?;
    Ok(())
}
