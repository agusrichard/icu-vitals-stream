CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE TABLE scored_readings
(
    time                TIMESTAMPTZ NOT NULL,
    patient_id          TEXT        NOT NULL,
    simulator_state     TEXT        NOT NULL,
    respiration_rate    INT,
    oxygen_saturation   INT,
    supplemental_o2     BOOLEAN,
    temperature         DOUBLE PRECISION,
    systolic_bp         INT,
    heart_rate          INT,
    consciousness_level TEXT,
    news2_score         INT         NOT NULL,
    news2_tier          TEXT        NOT NULL
);

SELECT create_hypertable('scored_readings', by_range('time'));
CREATE INDEX ON scored_readings (patient_id, time DESC);

CREATE TABLE alerts
(
    time          TIMESTAMPTZ NOT NULL,
    patient_id    TEXT        NOT NULL,
    previous_tier TEXT        NOT NULL,
    new_tier      TEXT        NOT NULL,
    news2_score   INT         NOT NULL
);

SELECT create_hypertable('alerts', by_range('time'));
CREATE INDEX ON alerts (patient_id, time DESC);
