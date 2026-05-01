FROM golang:1.26-alpine AS builder
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go build -ldflags="-s -w" -o /simulator ./cmd/simulator

FROM golang:1.26-alpine AS dev
WORKDIR /src
RUN go install github.com/air-verse/air@latest
COPY go.mod go.sum ./
RUN go mod download
CMD ["air", "-c", ".air.toml"]

FROM alpine:3.21 AS runtime
COPY --from=builder /simulator /simulator
CMD /simulator \
  -patients=${PATIENTS:-1} \
  -brokers=${BROKERS:-localhost:9092} \
  -no-kafka=${NO_KAFKA:-false} \
  -schema-registry=${SCHEMA_REGISTRY_URL:-http://localhost:8081}
