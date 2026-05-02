use rdkafka::producer::{FutureProducer, FutureRecord};
use crate::schema::RegisteredSchema;

#[derive(serde::Serialize)]
pub struct AlertEvent {
    pub patient_id: String,
    pub timestamp: i64,
    pub previous_tier: String,
    pub new_tier: String,
    pub news2_score: i32,
}

pub async fn emit_alert(
    producer: &FutureProducer,
    registered: &RegisteredSchema,
    alert: &AlertEvent,
) -> anyhow::Result<()> {
    let value = apache_avro::to_value(alert)?;
    let avro_bytes = apache_avro::to_avro_datum(&registered.schema, value)?;

    let mut msg = Vec::with_capacity(5 + avro_bytes.len());
    msg.push(0x00);
    msg.extend_from_slice(&registered.id.to_be_bytes());
    msg.extend_from_slice(&avro_bytes);

    let record = FutureRecord::to("vitals.alerts")
        .key(&alert.patient_id)
        .payload(&msg);
    producer.send(record, rdkafka::util::Timeout::Never).await
        .map_err(|(e, _)| anyhow::anyhow!(e))?;
    Ok(())
}
