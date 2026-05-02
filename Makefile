COMPOSE = docker compose -f infra/docker-compose.yml

.PHONY: infra simulator

infra:
	$(COMPOSE) up kafka schema-registry timescaledb pgweb schema-registry-init kafka-init

simulator:
	$(COMPOSE) up simulator
