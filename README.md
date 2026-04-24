# icu-vitals-stream

> Real-time ICU vital signs pipeline with Go producers, a Rust scorer, Kafka, and PySpark analytics. Detects patient deterioration using the NEWS2 early warning score.

[![Status](https://img.shields.io/badge/status-in%20development-yellow)]()
[![License](https://img.shields.io/badge/license-Apache%20License%202.0-blue)]()

---

## Overview

`icu-vitals-streaming` is an end-to-end data streaming portfolio project that simulates an ICU ward of virtual patients, ingests their vital signs in real time, computes the [NEWS2 (National Early Warning Score 2)](https://www.rcp.ac.uk/improving-care/national-early-warning-score-news/) at the bedside, fires deterioration alerts, and runs longer-window analytics and ML over the historical stream.

The goal is to demonstrate a realistic streaming architecture end-to-end: high-frequency producers, low-latency stateful consumers, durable event storage, and heavyweight analytics — all justified by a defensible clinical use case.

> **Note:** All data is synthetically generated. This project is for educational and portfolio purposes only and is **not** a medical device.

---

## Why This Project

In real ICUs, vital signs are charted every 1–4 hours, and clinical deterioration between charting intervals is a leading cause of "failure to rescue." Continuous monitoring with automated early warning scoring catches deterioration earlier. This project simulates that pipeline end-to-end, exercising:

- **High-throughput streaming ingestion** (Kafka)
- **Stateful, low-latency stream processing** (Rust)
- **Distributed batch and streaming analytics** (PySpark)
- **Concurrent data generation** (Go)
- **A real, published clinical algorithm** (NEWS2)

---

## Architecture

```
┌──────────────┐     ┌─────────┐     ┌────────────────┐     ┌──────────────┐
│  Go          │────▶│  Kafka  │────▶│  Rust          │────▶│ TimescaleDB  │
│  Simulator   │     │         │     │  Scorer/Alert  │     │  (state)     │
│  (N patients)│     │ topics  │     │                │     └──────────────┘
└──────────────┘     │         │     └────────────────┘     ┌──────────────┐
                     │         │              │             │   Grafana    │
                     │         │              ▼             │  dashboards  │
                     │         │     ┌────────────────┐     └──────────────┘
                     │         │────▶│  PySpark       │────▶┌──────────────┐
                     │         │     │  Structured    │     │   Delta /    │
                     │         │     │  Streaming     │     │   Parquet    │
                     └─────────┘     └────────────────┘     └──────────────┘
```

### Data Flow

1. **Go simulator** generates vital signs for N virtual patients, each modeled as an independent goroutine with an underlying clinical state machine (stable, deteriorating-sepsis, deteriorating-respiratory, post-op recovering, etc.).
2. **Kafka** serves as the durable event backbone, with topics keyed by `patient_id` to preserve per-patient ordering.
3. **Rust consumer** maintains per-patient rolling windows in memory, computes NEWS2 in real time, deduplicates and emits alerts on score transitions, and snapshots state to TimescaleDB for the live dashboard.
4. **PySpark** runs both Structured Streaming jobs (5-minute and 1-hour aggregates) and nightly batch jobs (ward KPIs, ML training for deterioration prediction).
5. **Grafana** visualizes live patient state and ward-level metrics from TimescaleDB.

### Kafka Topic Design

| Topic | Key | Partitions | Retention | Purpose |
|---|---|---|---|---|
| `vitals.raw` | `patient_id` | 12 | 24h | High-frequency raw readings |
| `vitals.scored` | `patient_id` | 12 | 7d | Per-window NEWS2 scores |
| `vitals.alerts` | `patient_id` | 6 | 30d | Deterioration alerts |
| `patients.admin` | `patient_id` | 3 | compacted | Admit / discharge / demographics |

Schemas are managed via a Schema Registry (Avro).

---

## The NEWS2 Scoring Algorithm

The Rust consumer implements the NEWS2 score, which assigns 0–3 points to each of seven physiological parameters:

| Parameter | Measured |
|---|---|
| Respiratory rate | breaths/min |
| SpO₂ | oxygen saturation % |
| Supplemental oxygen | yes / no |
| Temperature | °C |
| Systolic blood pressure | mmHg |
| Heart rate | bpm |
| Consciousness (ACVPU) | Alert / Confusion / Voice / Pain / Unresponsive |

Aggregate scores trigger escalation tiers:

- **0–4**: routine monitoring
- **5–6** (or any single parameter scoring 3): urgent clinical review
- **≥ 7**: emergency response

---

## Tech Stack

### Data Generation — Go

- **Language**: Go 1.22+
- **Kafka client**: [`segmentio/kafka-go`](https://github.com/segmentio/kafka-go)
- **Concurrency model**: One goroutine per simulated patient, sharing a producer pool via channels
- **Control plane**: Small HTTP API for admit / discharge / scenario triggering

Go was chosen because each virtual patient maps cleanly onto a goroutine, and the standard library's concurrency primitives make it straightforward to scale from 10 to 1,000+ simultaneous emitters.

### Stream Processing — Rust

- **Language**: Rust (stable)
- **Async runtime**: [`tokio`](https://tokio.rs/)
- **Kafka client**: [`rdkafka`](https://github.com/fede1024/rust-rdkafka) (librdkafka bindings)
- **Serialization**: [`apache-avro`](https://crates.io/crates/apache-avro)
- **In-memory state**: [`dashmap`](https://crates.io/crates/dashmap) for per-patient sharded state
- **Persistence**: [`sqlx`](https://crates.io/crates/sqlx) → TimescaleDB
- **Metrics**: [`prometheus`](https://crates.io/crates/prometheus) crate

Rust was chosen for the hot path because the scorer needs predictable latency, no GC pauses when emitting alerts, and per-patient stateful processing at scale. Zero-cost abstractions and the actor-style consumer model fit the per-key stream processing pattern naturally.

### Analytics — PySpark

- **Engine**: Apache Spark 3.5+ (PySpark)
- **Streaming**: Structured Streaming with Kafka source
- **Storage**: Delta Lake on MinIO (or S3)
- **ML**: Spark MLlib and / or XGBoost for deterioration prediction

PySpark handles workloads that the Rust scorer intentionally avoids: long-window aggregations, joins across topics, and ML training over historical data.

### Supporting Infrastructure

The infrastructure is organized in three layers, each building on the one below it.

#### Layer 1 — Core Pipeline (Critical Path)

This is the heart of the project. Every other layer depends on this being correct and low-latency.

| Component | Purpose |
|---|---|
| **Docker Compose** | Orchestrates Kafka (KRaft), Schema Registry, and TimescaleDB for local development |
| **Go Simulator** | Generates concurrent vital-sign streams; one goroutine per patient |
| **Apache Kafka** | Durable, ordered event backbone keyed by `patient_id` |
| **Schema Registry** | Enforces Avro contracts between producers and consumers |
| **Rust Scorer** | Computes NEWS2 in real time with predictable latency and no GC pauses |
| **TimescaleDB** | Hot state store written by the Rust scorer for live dashboard queries |

#### Layer 2 — Analytics & Storage

Handles workloads the Rust scorer intentionally avoids: long-window aggregations, cross-topic joins, and ML training over historical data.

| Component | Purpose |
|---|---|
| **PySpark** | Structured Streaming jobs (5-min, 1-hour aggregates) and nightly batch ML |
| **Delta Lake** | ACID lakehouse storage for analytics and ML training data |
| **MinIO** | S3-compatible object storage; local stand-in for AWS S3 during development |

#### Layer 3 — Observability

Sits on top of both layers and provides visibility into clinical state and system health.

| Component | Purpose |
|---|---|
| **Grafana** | Real-time dashboards over TimescaleDB (live vitals) and Delta Lake (ward KPIs) |
| **Prometheus** | Scrapes operational metrics from the Rust scorer (throughput, latency, alert counts) |

---

## Repository Layout

```
icu-vitals-streaming/
├── simulator/              # Go simulator
│   ├── cmd/
│   ├── internal/
│   │   ├── patient/        # Patient state machine and trajectories
│   │   ├── producer/       # Kafka producer pool
│   │   └── scenarios/      # Demo scenarios (sepsis-outbreak, etc.)
│   └── go.mod
├── scorer/                 # Rust real-time scorer
│   ├── src/
│   │   ├── consumer.rs
│   │   ├── news2.rs        # NEWS2 scoring logic
│   │   ├── state.rs        # Per-patient state management
│   │   └── alerter.rs
│   └── Cargo.toml
├── analytics/              # PySpark jobs
│   ├── streaming/          # Structured Streaming jobs
│   ├── batch/              # Nightly batch jobs
│   └── ml/                 # Deterioration prediction model
├── schemas/                # Avro schemas
├── infra/
│   ├── docker-compose.yml
│   ├── grafana/
│   └── prometheus/
└── docs/
    ├── architecture.md
    ├── news2.md
    └── simulator.md
```

---

## Getting Started

> Detailed setup instructions are coming as the project develops.

### Prerequisites

- Docker and Docker Compose
- Go 1.22+
- Rust (stable, via rustup)
- Python 3.11+ with PySpark 3.5+

### Quick Start (Planned)

```bash
# Spin up Kafka (KRaft), Schema Registry, TimescaleDB, MinIO, Grafana, etc.
cd infra && docker compose up -d

# Start the simulator with 50 patients
cd simulator && go run ./cmd/simulator --patients 50

# Run the Rust scorer
cd scorer && cargo run --release

# Submit the PySpark streaming job
cd analytics && spark-submit streaming/vitals_aggregator.py
```

---

## Design Decisions

A few choices worth calling out for anyone reading the code:

- **Per-patient keying.** All topics are keyed by `patient_id` to preserve ordering within a patient's stream. Cross-patient ordering doesn't matter clinically.
- **Compacted admin topic.** `patients.admin` is log-compacted so the scorer can rebuild patient demographics on restart without replaying the full event log.
- **Rust over Go for the scorer.** Both could do this job. Rust was chosen specifically to demonstrate when predictable latency and zero GC pauses earn their complexity — a real-time clinical alerter is a clean justification.
- **NEWS2 as a baseline, not the goal.** The PySpark ML pipeline is explicitly framed as "can we beat NEWS2?" rather than "can we replace it?" Interpretability matters in clinical contexts.
- **Simulator ground truth.** Every emitted reading carries a hidden `simulator_state` field, used as ground truth for evaluating the ML model. This field is never read by the Rust scorer.

---

## Disclaimer

This project uses entirely synthetic data and is intended for educational and portfolio purposes only. It is **not** a medical device, has not been clinically validated, and must not be used for any real patient care decisions. The NEWS2 algorithm is implemented as described in published clinical literature, but its application here is purely illustrative.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.