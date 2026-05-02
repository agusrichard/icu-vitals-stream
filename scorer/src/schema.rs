pub struct RegisteredSchema {
    pub schema: apache_avro::Schema,
    pub id: u32,
}

impl RegisteredSchema {
    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            schema: apache_avro::Schema::parse_str(r#"{"type":"null"}"#).unwrap(),
            id: 0,
        }
    }

    pub async fn new(sr_url: &str, subject: &str) -> anyhow::Result<Self> {
        let resp: serde_json::Value =
            reqwest::get(format!("{}/subjects/{}/versions/latest", sr_url, subject))
                .await?
                .json()
                .await?;
        let id = resp["id"].as_u64().ok_or_else(|| anyhow::anyhow!("missing id field"))? as u32;
        let schema_str = resp["schema"].as_str().ok_or_else(|| anyhow::anyhow!("missing schema field"))?;
        Ok(Self {
            schema: apache_avro::Schema::parse_str(schema_str)?,
            id,
        })
    }
}
