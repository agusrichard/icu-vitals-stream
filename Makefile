COMPOSE = docker compose -f infra/docker-compose.yml
PYSPARK_PACKAGES = org.apache.spark:spark-sql-kafka-0-10_2.13:4.1.1,org.apache.spark:spark-avro_2.13:4.1.1,io.delta:delta-spark_4.1_2.13:4.1.0,org.apache.hadoop:hadoop-aws:3.4.1
PYSPARK_JOBS_DIR = /opt/spark/jobs

.PHONY: infra simulator all pyspark-submit-raw pyspark-submit-scored pyspark-stop

infra:
	$(COMPOSE) up --build schema-registry timescaledb pgweb schema-registry-init kafka-init

simulator:
	$(COMPOSE) up --build simulator scorer

all:
	$(COMPOSE) up --build

down:
	$(COMPOSE) down -v

pyspark-submit-raw:
	docker exec pyspark /opt/spark/bin/spark-submit \
		--master 'local[*]' \
		--conf 'spark.jars.ivy=/tmp/.ivy2' \
		--packages '$(PYSPARK_PACKAGES)' \
		--py-files '$(PYSPARK_JOBS_DIR)/shared.py' \
		$(PYSPARK_JOBS_DIR)/vitals_raw_5min_agg.py

pyspark-submit-scored:
	docker exec pyspark /opt/spark/bin/spark-submit \
		--master 'local[*]' \
		--conf 'spark.jars.ivy=/tmp/.ivy2' \
		--packages '$(PYSPARK_PACKAGES)' \
		--py-files '$(PYSPARK_JOBS_DIR)/shared.py' \
		$(PYSPARK_JOBS_DIR)/vitals_scored_1hr_agg.py

pyspark-stop:
	pkill -f 'vitals_raw_5min_agg\|vitals_scored_1hr_agg' || true