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

#[cfg(test)]
mod tests {
    use super::*;

    const ALERTS_AVSC: &str = r#"{
        "type": "record",
        "name": "VitalAlert",
        "namespace": "com.icu.vitals",
        "fields": [
            {"name": "patient_id",    "type": "string"},
            {"name": "timestamp",     "type": {"type": "long", "logicalType": "timestamp-millis"}},
            {"name": "previous_tier", "type": "string"},
            {"name": "new_tier",      "type": "string"},
            {"name": "news2_score",   "type": "int"}
        ]
    }"#;

    fn registered_schema() -> RegisteredSchema {
        RegisteredSchema {
            schema: apache_avro::Schema::parse_str(ALERTS_AVSC).unwrap(),
            id: 1,
        }
    }

    fn sample_alert() -> AlertEvent {
        AlertEvent {
            patient_id: "p1".to_string(),
            timestamp: 1_000_000,
            previous_tier: "Low".to_string(),
            new_tier: "High".to_string(),
            news2_score: 9,
        }
    }

    #[test]
    fn alert_event_encodes_to_avro_without_error() {
        let registered = registered_schema();
        let alert = sample_alert();
        let value = apache_avro::to_value(&alert).unwrap();
        let result = apache_avro::to_avro_datum(&registered.schema, value);
        assert!(result.is_ok(), "encoding failed: {:?}", result.err());
    }

    #[test]
    fn confluent_wire_frame_starts_with_magic_byte() {
        let registered = registered_schema();
        let alert = sample_alert();
        let value = apache_avro::to_value(&alert).unwrap();
        let avro_bytes = apache_avro::to_avro_datum(&registered.schema, value).unwrap();

        let mut msg = Vec::with_capacity(5 + avro_bytes.len());
        msg.push(0x00);
        msg.extend_from_slice(&registered.id.to_be_bytes());
        msg.extend_from_slice(&avro_bytes);

        assert_eq!(msg[0], 0x00);
    }

    #[test]
    fn confluent_wire_frame_encodes_schema_id_big_endian() {
        let registered = registered_schema(); // id = 1
        let alert = sample_alert();
        let value = apache_avro::to_value(&alert).unwrap();
        let avro_bytes = apache_avro::to_avro_datum(&registered.schema, value).unwrap();

        let mut msg = Vec::with_capacity(5 + avro_bytes.len());
        msg.push(0x00);
        msg.extend_from_slice(&registered.id.to_be_bytes());
        msg.extend_from_slice(&avro_bytes);

        assert_eq!(&msg[1..5], &1u32.to_be_bytes());
    }

    #[test]
    fn avro_payload_is_decodable_after_stripping_header() {
        let registered = registered_schema();
        let alert = sample_alert();
        let value = apache_avro::to_value(&alert).unwrap();
        let avro_bytes = apache_avro::to_avro_datum(&registered.schema, value).unwrap();

        let mut msg = Vec::with_capacity(5 + avro_bytes.len());
        msg.push(0x00);
        msg.extend_from_slice(&registered.id.to_be_bytes());
        msg.extend_from_slice(&avro_bytes);

        assert!(msg.len() > 5 && msg[0] == 0x00);
        let decoded = apache_avro::from_avro_datum(
            &registered.schema,
            &mut std::io::Cursor::new(&msg[5..]),
            None,
        );
        assert!(decoded.is_ok(), "decode failed: {:?}", decoded.err());
    }
}
