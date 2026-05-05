COMPOSE = docker compose -f infra/docker-compose.yml

.PHONY: infra simulator

infra:
	$(COMPOSE) up --build schema-registry timescaledb pgweb schema-registry-init kafka-init

simulator:
	$(COMPOSE) up --build simulator scorer
