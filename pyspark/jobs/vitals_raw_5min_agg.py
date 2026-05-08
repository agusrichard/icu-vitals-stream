from pyspark.sql import functions as F
from pyspark.sql.functions import col, window, avg, min, max, count, to_date
from shared import create_spark_session, fetch_schema, read_kafka_avro_stream

spark = create_spark_session("vitals-raw-5min-agg")
schema_str = fetch_schema("vitals.raw-value")
watermarked = (
    read_kafka_avro_stream(spark, "vitals.raw", schema_str)
    .withWatermark("event_time", "10 minutes")
)

agg = (
    watermarked
    .groupBy(col("patient_id"), window(col("event_time"), "5 minutes"))
    .agg(
        avg("heart_rate").alias("avg_heart_rate"),
        avg("respiration_rate").alias("avg_respiration_rate"),
        avg("oxygen_saturation").alias("avg_oxygen_saturation"),
        min("oxygen_saturation").alias("min_oxygen_saturation"),
        avg("systolic_bp").alias("avg_systolic_bp"),
        avg("temperature").alias("avg_temperature"),
        count("*").alias("reading_count"),
        F.first("simulator_state").alias("simulator_state"),
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

agg_partitioned = agg.withColumn("window_date", to_date(col("window_start")))

query = (
    agg_partitioned.writeStream
    .outputMode("append")
    .format("delta")
    .option("checkpointLocation", "/tmp/checkpoints/vitals_raw_5min_agg")
    .partitionBy("window_date")
    .trigger(processingTime="30 seconds")
    .start("s3a://delta-lake/vitals_raw_5min_agg")
)

query.awaitTermination()
