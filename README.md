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
┌─────────────────────┐
│  Patient Admin API  │────────────────────────────────────┐
│  (Go REST API)      │                                    │
└─────────────────────┘                                    │
           │                                               ▼
           │ patient registry             ┌──────────────────────┐
           ▼                              │     TimescaleDB      │
┌──────────────────┐     ┌─────────┐      │  (state + registry)  │
│  Go              │────▶│  Kafka  │────▶ │                      │
│  Simulator       │     │         │      └──────────────────────┘
│  (N patients)    │     │ topics  │      ┌──────────────┐
└──────────────────┘     │         │      │   Grafana    │
                         │         │      │  dashboards  │
                         │         │      └──────────────┘
                         │         │     ┌────────────────┐
                         │         │────▶│  PySpark       │────▶┌──────────────┐
                         │         │     │  Structured    │     │   Delta /    │
                         │         │     │  Streaming     │     │   Parquet    │
                         └─────────┘     └────────────────┘     └──────────────┘
```

### Data Flow

1. **Patient Admin API** (Go REST API) provides the patient registry — the simulator queries it on startup to load registered patients and their initial clinical state. Admission and discharge events will eventually be published to the `patients.admin` Kafka topic (deferred to Layer 2 implementation).
2. **Go simulator** generates vital signs for N virtual patients, each modeled as an independent goroutine with an underlying clinical state machine (see [Clinical States](#clinical-states)).
3. **Kafka** serves as the durable event backbone, with topics keyed by `patient_id` to preserve per-patient ordering.
4. **Rust consumer** maintains per-patient rolling windows in memory, computes NEWS2 in real time, deduplicates and emits alerts on score transitions, and snapshots state to TimescaleDB for the live dashboard.
5. **PySpark** runs both Structured Streaming jobs (5-minute and 1-hour aggregates) and nightly batch jobs (ward KPIs, ML training for deterioration prediction).
6. **Grafana** visualizes live patient state and ward-level metrics from TimescaleDB.

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

## Simulator State Transitions

The state machine transitions states probabilistically each tick (see `simulator/internal/state.go`). Without additional handling, each transition would be an abrupt jump — one tick the patient looks Stable, the next they look fully septic. That produces data with no pre-deterioration signal for the ML model to learn from.

### Clinical transition constraints

Not all transitions are valid. `PostOpRecovering` represents surgical recovery — a state a patient enters from the operating room, not from an acute deterioration episode. As a result:

- **Stable** can transition to any deteriorating state but not to `PostOpRecovering` (surgery is an external event the simulator does not model).
- **Deteriorating and SepticShock states** can recover to Stable or escalate to other deteriorating states, but not to `PostOpRecovering`.
- **PostOpRecovering** transitions only outward: to Stable (recovery) or to any deteriorating state (post-op complication).

To fix this, the simulator uses a **drift mechanism**: when a state transition occurs, vital sign parameters do not jump immediately to the new state's target ranges. Instead, on each tick, every continuous parameter moves a fraction of the remaining gap toward the new state's centre value, with a small noise term added:

```
new_value = prev_value + rate × (target_centre − prev_value) + noise
```

The drift rate is fixed per destination state rather than per transition pair, because it is the target physiology — not the origin — that determines how fast the body moves toward a new equilibrium:

| Destination state | Rate | Rationale |
|---|---|---|
| Stable | 0.05 | Recovery is gradual — vitals normalise over several minutes even with treatment |
| Deteriorating Sepsis / Respiratory / Cardiac | 0.05 | Slow enough to produce a multi-minute pre-deterioration ramp the ML model can learn from |
| Post-Op Recovering | 0.07 | Slightly faster — post-op stress is bounded and managed |
| Septic Shock | 0.10 | Cardiovascular collapse accelerates faster than early deterioration |

With a 5-second tick interval and a rate of 0.05, a patient reaches approximately 23% of the target state after 5 ticks (25 seconds), 64% after 20 ticks (100 seconds), and 78% after 30 ticks (150 seconds — 2.5 minutes). A 5-minute rolling window therefore captures most of the drift trajectory, giving the ML model a genuine pre-deterioration signal — rising heart rate and falling blood pressure while absolute values are still within low-NEWS2 thresholds.

All 30 non-self transitions are covered by the same formula. The source state is irrelevant; only the destination determines the target centre and drift rate. This means recovery trajectories (DeterioratingSepsis → Stable) and cross-deterioration transitions (DeterioratingRespiratory → DeterioratingSepsis) are all handled without per-transition logic.

---

## Analytics Pipeline

### 5-Minute Streaming Aggregates

**Source:** `vitals.raw`
**Granularity:** one row per patient per 5-minute tumbling window
**Sink:** Delta Lake on MinIO (ML feature store)

| Field | Description |
|---|---|
| `patient_id` | Patient identifier |
| `window_start`, `window_end` | Window boundaries |
| `hr_mean`, `hr_min`, `hr_max`, `hr_stddev` | Heart rate distribution across the window |
| `hr_slope` | Heart rate trend (bpm/min) — rising slope is a pre-sepsis signal |
| `rr_mean`, `rr_min`, `rr_max`, `rr_stddev` | Respiratory rate distribution |
| `rr_slope` | Respiratory rate trend — earliest parameter to rise in deterioration |
| `spo2_mean`, `spo2_min`, `spo2_stddev` | SpO₂ distribution (min captures the dangerous extreme) |
| `spo2_slope` | SpO₂ trend — negative slope indicates worsening oxygenation |
| `sbp_mean`, `sbp_min`, `sbp_stddev` | Systolic BP distribution |
| `sbp_slope` | Systolic BP trend — falling slope indicates haemodynamic compromise |
| `temp_mean`, `temp_stddev` | Temperature distribution (distinguishes sepsis from cardiac deterioration) |
| `on_o2_fraction` | Fraction of readings in the window with supplemental O₂ active |
| `dominant_consciousness` | Mode of consciousness level across the window |
| `reading_count` | Number of readings in the window (data completeness check) |
| `simulator_state` | Clinical state label from the most recent reading — ground truth for ML |

Slope is the critical feature class. It is computed as the linear regression coefficient across all readings in the window (or equivalently `(last_value − first_value) / window_duration_minutes`). Because the simulator uses drift, slope rises smoothly over several ticks before absolute values cross any NEWS2 threshold — that directional signal is what enables early prediction.

PySpark Structured Streaming reads from `vitals.raw` with a 10-second watermark for late data and writes each completed window to Delta Lake using a 5-minute tumbling window keyed by `patient_id`.

---

### 1-Hour Streaming Aggregates

**Source:** `vitals.scored`
**Granularity:** one row per ward per 1-hour tumbling window
**Sink:** Delta Lake on MinIO (Grafana ward KPIs dashboard)

| Field | Description |
|---|---|
| `window_start`, `window_end` | Window boundaries |
| `patients_low` | Count of patients with NEWS2 score 0–4 during the hour |
| `patients_medium` | Count of patients with NEWS2 score 5–6 during the hour |
| `patients_high` | Count of patients with NEWS2 score ≥ 7 during the hour |
| `alerts_fired` | Total deterioration alerts fired across the ward in the hour |
| `avg_news2_score` | Ward-average NEWS2 score |
| `max_news2_score` | Highest NEWS2 score observed across any patient in the hour |
| `dominant_state` | Most common clinical state across all patients and readings |

This is ward-level, not per-patient. The consumer is the Grafana ward KPIs dashboard — a charge nurse view rather than a bedside view. The 1-hour granularity smooths transient spikes and surfaces sustained trends: is the ward getting sicker overall, or are high-score patients recovering? The `patients_high` count and `alerts_fired` are the primary operational signals.

---

### ML Deterioration Prediction Model

**Goal:** predict a patient's clinical state from a 5-minute vital sign trajectory, earlier than NEWS2 thresholds would fire.

**Training data:** 5-minute aggregate windows from Delta Lake, one row per patient per window.

**Features:** all aggregate and slope fields from the 5-minute window schema — `hr_mean`, `hr_slope`, `rr_mean`, `rr_slope`, `spo2_min`, `spo2_slope`, `sbp_mean`, `sbp_min`, `sbp_slope`, `temp_mean`, `on_o2_fraction`, and `dominant_consciousness`.

**Label:** `simulator_state` of the most recent reading in the window (6-class: Stable, DeterioratingSepsis, DeterioratingRespiratory, DeterioratingCardiac, PostOpRecovering, SepticShock).

**Algorithm:** Gradient Boosted Trees (GBT) via Spark MLlib.

GBT was chosen because it captures non-linear interactions between features — rising HR combined with falling BP and rising RR together carry more information than each parameter in isolation, and GBT learns those compound patterns without requiring feature normalisation. It also produces feature importances, which makes it possible to verify the model is learning clinically meaningful signals (`hr_slope` and `sbp_slope` should rank highly for sepsis, `spo2_slope` for respiratory deterioration).

**Training loop:** a nightly batch job reads the last N days of 5-minute aggregate windows from Delta Lake, encodes the label as an integer, trains GBT, and saves the model artefact back to Delta Lake / MinIO. A separate inference job loads the saved model and scores live windows from the feature store.

**Why early detection is achievable:** during a Stable→Sepsis transition with drift rate 0.05, a 5-minute window spanning the transition will contain readings from both sides. The absolute values may still be borderline-normal, but the slope features will be clearly directional — heart rate rising steadily, blood pressure drifting down — across 20–30 readings before any NEWS2 threshold is crossed. The model is trained on thousands of such windows and learns: "this slope signature, even with borderline absolute values, predicts DETERIORATING_SEPSIS." That is the signal NEWS2 cannot produce from a single snapshot.

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

The infrastructure is organized in four layers, each building on the one below it.

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

#### Layer 2 — Patient Administration API

A Go REST API that manages the patient registry — registering patients, setting initial clinical conditions, and providing the simulator with the patient roster on startup. Admission and discharge events are published to the `patients.admin` Kafka topic, allowing the scorer and analytics layer to react to ward-level changes.

| Component | Purpose |
|---|---|
| **Patient Admin API** | Go REST API for registering patients and setting initial clinical state |
| **`patients.admin` topic** | Kafka topic for admit / discharge events consumed by the scorer and PySpark |

#### Layer 3 — Analytics & Storage

Handles workloads the Rust scorer intentionally avoids: long-window aggregations, cross-topic joins, and ML training over historical data.

| Component | Purpose |
|---|---|
| **PySpark** | Structured Streaming jobs (5-min, 1-hour aggregates) and nightly batch ML |
| **Delta Lake** | ACID lakehouse storage for analytics and ML training data |
| **MinIO** | S3-compatible object storage; local stand-in for AWS S3 during development |

#### Layer 4 — Observability

Sits on top of all layers and provides visibility into clinical state and system health.

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
- Rust (stable, via rustup) — plus `cmake` for the `rdkafka` C build (`brew install cmake` on macOS)
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
- **Compacted admin topic.** `patients.admin` is log-compacted so the scorer can rebuild patient demographics on restart without replaying the full event log. This topic is produced by the Patient Admin API (Layer 2) and is deferred until that layer is implemented.
- **Rust over Go for the scorer.** Both could do this job. Rust was chosen specifically to demonstrate when predictable latency and zero GC pauses earn their complexity — a real-time clinical alerter is a clean justification.
- **Snapshot vs trajectory.** The Rust scorer classifies the current clinical state from a single reading using rule-based pattern matching — fast, deterministic, explainable. The PySpark ML model learns from rolling windows of parameter trajectories to predict deterioration *before* thresholds are crossed. The two layers are complementary, not redundant.
- **Simulator ground truth.** Every emitted reading carries a hidden `simulator_state` field, used as ground truth for evaluating the ML model. This field is never read by the Rust scorer.

---

## Disclaimer

This project uses entirely synthetic data and is intended for educational and portfolio purposes only. It is **not** a medical device, has not been clinically validated, and must not be used for any real patient care decisions. The NEWS2 algorithm is implemented as described in published clinical literature, but its application here is purely illustrative.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.