import json
from urllib.request import urlopen
from pyspark.sql import SparkSession, functions as F
from pyspark.sql.avro.functions import from_avro
from pyspark.sql.functions import col, to_timestamp, window, avg, min, max, count


spark = (
    SparkSession.builder
    .appName("vitals-raw-5min-agg")
    .config("spark.sql.extensions", "io.delta.sql.DeltaSparkSessionExtension")
    .config("spark.sql.catalog.spark_catalog", "org.apache.spark.sql.delta.catalog.DeltaCatalog")
    .config("spark.hadoop.fs.s3a.endpoint", "http://minio:9000")
    .config("spark.hadoop.fs.s3a.access.key", "minioadmin")
    .config("spark.hadoop.fs.s3a.secret.key", "minioadmin")
    .config("spark.hadoop.fs.s3a.path.style.access", "true")
    .config("spark.hadoop.fs.s3a.impl", "org.apache.hadoop.fs.s3a.S3AFileSystem")
    .getOrCreate()
)
spark.sparkContext.setLogLevel("WARN")

def fetch_schema(sr_url: str, subject: str) -> str:
    with urlopen(f"{sr_url}/subjects/{subject}/versions/latest") as resp:
        return json.loads(json.loads(resp.read())["schema"])

SCHEMA_REGISTRY_URL = "http://schema-registry:8081"
vitals_schema_json = fetch_schema(SCHEMA_REGISTRY_URL, "vitals.raw-value")
vitals_schema_str = json.dumps(vitals_schema_json)

raw_stream = (
    spark.readStream
    .format("kafka")
    .option("kafka.bootstrap.servers", "kafka:9092")
    .option("subscribe", "vitals.raw")
    .option("startingOffsets", "latest")
    .option("failOnDataLoss", "false")
    .load()
)

payload = raw_stream.select(
    F.expr("substring(value, 6, length(value) - 5)").alias("avro_payload")
)

vitals = payload.select(
    from_avro(F.col("avro_payload"), vitals_schema_str).alias("v")
).select("v.*")


vitals_ts = vitals.withColumnRenamed("timestamp", "event_time")

watermarked = vitals_ts.withWatermark("event_time", "2 minutes")

agg = (
    watermarked
    .groupBy(
        col("patient_id"),
        window(col("event_time"), "1 minutes")
    )
    .agg(
        avg("heart_rate").alias("avg_heart_rate"),
        avg("respiration_rate").alias("avg_respiration_rate"),
        avg("oxygen_saturation").alias("avg_oxygen_saturation"),
        min("oxygen_saturation").alias("min_oxygen_saturation"),
        avg("systolic_bp").alias("avg_systolic_bp"),
        avg("temperature").alias("avg_temperature"),
        count("*").alias("reading_count"),
        F.first("simulator_state").alias("simulator_state")
    )
    .select(
        col("patient_id"),
        col("window.start").alias("window_start"),
        col("window.end").alias("window_end"),
        col("avg_heart_rate"),
        col("avg_respiration_rate"),
        col("avg_oxygen_saturation"),
        col("min_oxygen_saturation"),
        col("avg_systolic_bp"),
        col("avg_temperature"),
        col("reading_count"),
        col("simulator_state"),
    )
)

query = (
    agg.writeStream
    .outputMode("append")
    .format("console")
    .option("truncate", False)
    .option("checkpointLocation", "/tmp/checkpoints/vitals_raw_5min_agg")
    .trigger(processingTime="30 seconds")
    .start()
)

query.awaitTermination()