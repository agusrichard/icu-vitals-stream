from pyspark.sql import functions as F
from pyspark.sql.functions import col, window, avg, min, max, count
from shared import create_spark_session, fetch_schema, read_kafka_avro_stream

spark = create_spark_session("vitals-scored-1hr-agg")
schema_str = fetch_schema("vitals.scored-value")
watermarked = (
    read_kafka_avro_stream(spark, "vitals.scored", schema_str)
    .withWatermark("event_time", "1 minutes")
)

agg = (
    watermarked
    .groupBy(col("patient_id"), window(col("event_time"), "2 minutes"))
    .agg(
        avg("heart_rate").alias("avg_heart_rate"),
        avg("respiration_rate").alias("avg_respiration_rate"),
        avg("oxygen_saturation").alias("avg_oxygen_saturation"),
        min("oxygen_saturation").alias("min_oxygen_saturation"),
        avg("systolic_bp").alias("avg_systolic_bp"),
        avg("temperature").alias("avg_temperature"),
        avg("news2_score").alias("avg_news2_score"),
        max("news2_score").alias("max_news2_score"),
        F.first("news2_tier").alias("news2_tier"),
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
        col("avg_news2_score"),
        col("max_news2_score"),
        col("news2_tier"),
        col("reading_count"),
        col("simulator_state"),
    )
)

query = (
    agg.writeStream
    .outputMode("append")
    .format("console")
    .option("truncate", False)
    .option("checkpointLocation", "/tmp/checkpoints/vitals_scored_1hr_agg")
    .trigger(processingTime="30 seconds")
    .start()
)

query.awaitTermination()
