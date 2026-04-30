#!/bin/sh
set -e

apk add --no-cache curl jq > /dev/null 2>&1

REGISTRY=${SCHEMA_REGISTRY_URL:-http://schema-registry:8081}

register() {
  topic=$1
  file=$2
  echo "Registering $topic..."
  curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$REGISTRY/subjects/$topic-value/versions" \
    -H "Content-Type: application/vnd.schemaregistry.v1+json" \
    -d "{\"schema\": $(jq -Rs . < "$file")}"
  echo " done"
}

register vitals.raw     /schemas/vitals.raw.avsc
register vitals.scored  /schemas/vitals.scored.avsc
register vitals.alerts  /schemas/vitals.alerts.avsc
