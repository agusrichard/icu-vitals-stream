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

1. **Go simulator** generates vital signs for N virtual patients, each modeled as an independent goroutine with an underlying clinical state machine (see [Clinical States](#clinical-states)).
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

NEWS2 (National Early Warning Score 2) was published by the Royal College of Physicians in December 2017 and has received formal NHS England endorsement as the standard early warning system for identifying acutely ill patients, including those with sepsis. **No scoring threshold changes have been issued since the 2017 publication** — a 2022 update revised only the observation chart formatting.

The Rust consumer implements NEWS2 by scoring each of seven physiological parameters from 0–3 based on how far the measurement deviates from normal, then summing them. The maximum possible total is 20.

### Respiratory Rate (breaths/min)

| Range | Score |
|---|---|
| ≤ 8 | 3 |
| 9–11 | 1 |
| 12–20 | 0 |
| 21–24 | 2 |
| ≥ 25 | 3 |

### SpO₂

Scale 1 applies to all patients in this simulator. Scale 2 (for COPD / hypercapnic respiratory failure patients who target a lower SpO₂ of 88–92%) is out of scope — none of the six clinical states model chronic CO₂ retention.

| Range | Score |
|---|---|
| ≤ 91% | 3 |
| 92–93% | 2 |
| 94–95% | 1 |
| ≥ 96% | 0 |

### Supplemental Oxygen

| | Score |
|---|---|
| Room air | 0 |
| Any supplemental O₂ | 2 |

### Systolic Blood Pressure (mmHg)

| Range | Score |
|---|---|
| ≤ 90 | 3 |
| 91–100 | 2 |
| 101–110 | 1 |
| 111–219 | 0 |
| ≥ 220 | 3 |

### Heart Rate (bpm)

| Range | Score |
|---|---|
| ≤ 40 | 3 |
| 41–50 | 1 |
| 51–90 | 0 |
| 91–110 | 1 |
| 111–130 | 2 |
| ≥ 131 | 3 |

### Temperature (°C)

| Range | Score |
|---|---|
| ≤ 35.0 | 3 |
| 35.1–36.0 | 1 |
| 36.1–38.0 | 0 |
| 38.1–39.0 | 1 |
| ≥ 39.1 | 2 |

### Consciousness (ACVPU)

ACVPU is the standard clinical scale for assessing a patient's level of consciousness. New Confusion (C) was added in NEWS2 — absent from the original NEWS — because it is often the earliest sign of sepsis or hypoxia: a patient can be confused while still appearing physically well, and that single finding scores 3 points.

| Level | Meaning | Score |
|---|---|---|
| **A** — Alert | Fully awake and oriented; responds normally | 0 |
| **C** — New Confusion | Awake but disoriented, rambling, or not making sense | 3 |
| **V** — Voice | Eyes closed; opens them or moves only when spoken to | 3 |
| **P** — Pain | No response to voice; moves or grimaces only when pinched | 3 |
| **U** — Unresponsive | No reaction to voice or pain stimulus | 3 |

Any level below Alert scores 3 — a single consciousness finding overrides an otherwise low aggregate and triggers urgent escalation.

#### Consciousness in the simulator's 6 states

| State | Consciousness | Reasoning |
|---|---|---|
| Stable | Always Alert | Healthy baseline |
| Deteriorating — Sepsis | 70% Alert, 30% Voice | Sepsis can cloud cognition early |
| Deteriorating — Respiratory | 80% Alert, 20% Voice | Hypoxia gradually impairs cognition |
| Deteriorating — Cardiac | 80% Alert, 20% Voice | Low cardiac output reduces cerebral perfusion |
| Post-Op Recovering | Always Alert | Anaesthesia has worn off |
| Septic Shock | Voice / Pain / Unresponsive equally | Cardiovascular collapse severely impairs brain perfusion |

### Escalation Thresholds

| Score | Risk | Required Response |
|---|---|---|
| 0 | Low | Routine monitoring, minimum 12-hourly obs |
| 1–4 | Low | Minimum 12-hourly; nurse decides whether to escalate |
| Any single parameter = 3 | Low–Medium | Urgent assessment by a competent registered nurse |
| 5–6 | Medium | Urgent clinician review; minimum hourly obs |
| ≥ 7 | **High** | Emergency response team; consider HDU/ICU transfer; continuous monitoring |

Sepsis-specific rule: a score ≥ 5 in a patient with known or suspected infection should trigger the sepsis bundle (lactate, blood cultures, antibiotics).

### How the Score and Clinical State Are Used Across the Pipeline

The Rust scorer does two things with each reading:

1. **Computes the NEWS2 aggregate score** and per-parameter subscores, firing an alert when thresholds are crossed.
2. **Classifies the current clinical state** using rule-based pattern matching on the current snapshot — e.g. high temp + low BP + high HR → sepsis; very low SpO₂ + very high RR + supplemental O₂ → respiratory failure. This is deterministic, explainable, and fast: exactly what a real-time clinical alerter needs to be.

The PySpark ML model does something the Rust scorer fundamentally cannot: it sees a **time series** of parameter values across a rolling window and detects deteriorating trends *before thresholds are crossed*. A patient whose heart rate has drifted 75 → 85 → 95 → 108 over 10 minutes may still have a NEWS2 score of 2, but the trajectory already matches a pre-sepsis pattern the model has learned.

The key distinction is **snapshot vs trajectory**. The Rust scorer is always right about *now*. The ML model is trying to be right about *what is coming* — predicting the clinical condition earlier than any threshold-based rule can, by learning from `simulator_state` ground truth labels attached to every reading.

---

## Clinical States

Each simulated patient holds one of six clinical states and transitions between them probabilistically on every tick. The `simulator_state` field is included in every emitted message as ground truth for the PySpark ML model — it is never read by the Rust NEWS2 scorer.

### Stable

Normal physiology. All seven NEWS2 parameters are within healthy ranges. This is the baseline state most patients start in and the target of any recovery trajectory. NEWS2 score is typically 0–2.

| Parameter | Range |
|---|---|
| Respiratory rate | 12–20 breaths/min |
| SpO₂ | 95–99% |
| Supplemental O₂ | No |
| Temperature | 36.1–37.2 °C |
| Systolic BP | 110–130 mmHg |
| Heart rate | 60–80 bpm |
| Consciousness | Alert |

---

### Deteriorating — Sepsis

Infection-driven organ dysfunction. Sepsis is defined clinically by at least two of: respiratory rate ≥ 22, systolic BP ≤ 100 mmHg, or altered consciousness. Presents with high fever, elevated heart rate, low blood pressure, and elevated respiratory rate as the body mounts a systemic inflammatory response. NEWS2 score typically 4–7.

| Parameter | Range |
|---|---|
| Respiratory rate | 20–30 breaths/min |
| SpO₂ | 93–97% |
| Supplemental O₂ | ~20% chance |
| Temperature | 38.5–40.0 °C |
| Systolic BP | 85–105 mmHg |
| Heart rate | 100–140 bpm |
| Consciousness | Alert or Voice |

---

### Deteriorating — Respiratory

Acute respiratory failure. Respiratory rate is often the first and most sensitive sign of clinical decline. Presents with very high breathing effort, low oxygen saturation, and mandatory supplemental oxygen. May co-occur with cardiac or septic presentations. NEWS2 score typically 5–8.

| Parameter | Range |
|---|---|
| Respiratory rate | 25–35 breaths/min |
| SpO₂ | 88–93% |
| Supplemental O₂ | Always |
| Temperature | 36.5–37.5 °C |
| Systolic BP | 105–125 mmHg |
| Heart rate | 90–120 bpm |
| Consciousness | Alert or Voice |

---

### Deteriorating — Cardiac

Cardiovascular instability — low cardiac output, arrhythmia, or early cardiogenic shock. Distinguishable from sepsis by the absence of fever: blood pressure and heart rate are abnormal but temperature is near normal. NEWS2 score typically 4–7.

| Parameter | Range |
|---|---|
| Respiratory rate | 18–26 breaths/min |
| SpO₂ | 92–96% |
| Supplemental O₂ | ~40% chance |
| Temperature | 36.0–37.0 °C |
| Systolic BP | 80–100 mmHg |
| Heart rate | 100–150 bpm |
| Consciousness | Alert or Voice |

---

### Post-Op Recovering

Post-surgical state. The patient is through the acute phase but the body is still under systemic stress from anaesthesia and tissue trauma. Vitals are mildly abnormal — slightly elevated heart rate, slightly low blood pressure, moderate respiratory rate — but trending toward stable. NEWS2 score typically 1–4.

| Parameter | Range |
|---|---|
| Respiratory rate | 14–22 breaths/min |
| SpO₂ | 93–97% |
| Supplemental O₂ | ~30% chance |
| Temperature | 36.8–37.8 °C |
| Systolic BP | 100–120 mmHg |
| Heart rate | 75–95 bpm |
| Consciousness | Alert |

---

### Septic Shock

End-stage sepsis with cardiovascular collapse. The infection is now refractory to fluid resuscitation: blood pressure is critically low, heart rate is very high, and the patient is in altered consciousness. Carries a high mortality risk. NEWS2 score typically ≥ 7 (emergency threshold).

| Parameter | Range |
|---|---|
| Respiratory rate | 25–38 breaths/min |
| SpO₂ | 85–91% |
| Supplemental O₂ | Always |
| Temperature | 38.5–41.0 °C |
| Systolic BP | 60–85 mmHg |
| Heart rate | 120–160 bpm |
| Consciousness | Voice, Pain, or Unresponsive |

---

## Tech Stack

### Data Generation — Go

- **Language**: Go 1.26.2
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
- **Snapshot vs trajectory.** The Rust scorer classifies the current clinical state from a single reading using rule-based pattern matching — fast, deterministic, explainable. The PySpark ML model learns from rolling windows of parameter trajectories to predict deterioration *before* thresholds are crossed. The two layers are complementary, not redundant.
- **Simulator ground truth.** Every emitted reading carries a hidden `simulator_state` field, used as ground truth for evaluating the ML model. This field is never read by the Rust scorer.

---

## Disclaimer

This project uses entirely synthetic data and is intended for educational and portfolio purposes only. It is **not** a medical device, has not been clinically validated, and must not be used for any real patient care decisions. The NEWS2 algorithm is implemented as described in published clinical literature, but its application here is purely illustrative.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.