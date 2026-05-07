COMPOSE = docker compose -f infra/docker-compose.yml
PYSPARK_PACKAGES = org.apache.spark:spark-sql-kafka-0-10_2.13:4.1.1,org.apache.spark:spark-avro_2.13:4.1.1,io.delta:delta-spark_4.1_2.13:4.1.0,org.apache.hadoop:hadoop-aws:3.4.1

.PHONY: infra simulator all pyspark-submit pyspark-stop

infra:
	$(COMPOSE) up --build schema-registry timescaledb pgweb schema-registry-init kafka-init

simulator:
	$(COMPOSE) up --build simulator scorer

all:
	$(COMPOSE) up --build

down:
	$(COMPOSE) down -v

pyspark-submit:
	docker exec pyspark /opt/spark/bin/spark-submit \
		--master 'local[*]' \
		--conf 'spark.jars.ivy=/tmp/.ivy2' \
		--packages '$(PYSPARK_PACKAGES)' \
		/opt/spark/jobs/vitals_raw_5min_agg.py

pyspark-stop:
	pkill -f vitals_raw_5min_agg || true