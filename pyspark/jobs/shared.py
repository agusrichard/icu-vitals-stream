import json
from urllib.request import urlopen
from pyspark.sql import SparkSession, DataFrame, functions as F
from pyspark.sql.avro.functions import from_avro

SCHEMA_REGISTRY_URL = "http://schema-registry:8081"


def create_spark_session(app_name: str) -> SparkSession:
    spark = (
        SparkSession.builder
        .appName(app_name)
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
    return spark


def fetch_schema(subject: str) -> str:
    with urlopen(f"{SCHEMA_REGISTRY_URL}/subjects/{subject}/versions/latest") as resp:
        return json.dumps(json.loads(json.loads(resp.read())["schema"]))


def read_kafka_avro_stream(spark: SparkSession, topic: str, schema_str: str) -> DataFrame:
    raw_stream = (
        spark.readStream
        .format("kafka")
        .option("kafka.bootstrap.servers", "kafka:9092")
        .option("subscribe", topic)
        .option("startingOffsets", "latest")
        .option("failOnDataLoss", "false")
        .load()
    )
    payload = raw_stream.select(
        F.expr("substring(value, 6, length(value) - 5)").alias("avro_payload")
    )
    return (
        payload
        .select(from_avro(F.col("avro_payload"), schema_str).alias("d"))
        .select("d.*")
        .withColumnRenamed("timestamp", "event_time")
    )
