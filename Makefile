COMPOSE = docker compose -f infra/docker-compose.yml
PYSPARK_PACKAGES = org.apache.spark:spark-sql-kafka-0-10_2.13:4.1.1,org.apache.spark:spark-avro_2.13:4.1.1,io.delta:delta-spark_4.1_2.13:4.1.0,org.apache.hadoop:hadoop-aws:3.4.1
PYSPARK_JOBS_DIR = /opt/spark/jobs

.PHONY: infra simulator all pyspark-submit-raw pyspark-submit-scored pyspark-shell pyspark-stop

infra:
	$(COMPOSE) up --build schema-registry timescaledb pgweb schema-registry-init kafka-init

simulator:
	$(COMPOSE) up --build simulator scorer

all:
	$(COMPOSE) up --build

down:
	$(COMPOSE) down -v

pyspark-submit-raw:
	$(COMPOSE) up -d pyspark-raw

pyspark-submit-scored:
	$(COMPOSE) up -d pyspark-scored

pyspark-shell:
	docker exec -it pyspark /opt/spark/bin/pyspark \
		--master 'local[*]' \
		--conf 'spark.jars.ivy=/opt/.ivy2' \
		--packages '$(PYSPARK_PACKAGES)' \
		--conf 'spark.sql.extensions=io.delta.sql.DeltaSparkSessionExtension' \
		--conf 'spark.sql.catalog.spark_catalog=org.apache.spark.sql.delta.catalog.DeltaCatalog' \
		--conf 'spark.hadoop.fs.s3a.endpoint=http://minio:9000' \
		--conf 'spark.hadoop.fs.s3a.access.key=minioadmin' \
		--conf 'spark.hadoop.fs.s3a.secret.key=minioadmin' \
		--conf 'spark.hadoop.fs.s3a.path.style.access=true' \
		--conf 'spark.hadoop.fs.s3a.impl=org.apache.hadoop.fs.s3a.S3AFileSystem'

pyspark-stop:
	$(COMPOSE) stop pyspark-raw pyspark-scored