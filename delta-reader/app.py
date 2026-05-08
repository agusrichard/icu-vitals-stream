import os
from datetime import datetime, timezone, timedelta
import pandas as pd
from deltalake import DeltaTable
from flask import Flask, jsonify

app = Flask(__name__)

STORAGE_OPTIONS = {
    "AWS_ENDPOINT_URL": os.getenv("MINIO_ENDPOINT", "http://minio:9000"),
    "AWS_ACCESS_KEY_ID": os.getenv("MINIO_ACCESS_KEY", "minioadmin"),
    "AWS_SECRET_ACCESS_KEY": os.getenv("MINIO_SECRET_KEY", "minioadmin"),
    "AWS_REGION": "us-east-1",
    "AWS_ALLOW_HTTP": "true",
}
SCORED_1HR_TABLE = os.getenv("SCORED_1HR_TABLE", "s3://delta-lake/vitals_scored_1hr_agg")


def read_scored_1hr() -> pd.DataFrame:
    dt = DeltaTable(SCORED_1HR_TABLE, storage_options=STORAGE_OPTIONS)
    return dt.to_pandas()


def latest_window_per_patient(df: pd.DataFrame) -> pd.DataFrame:
    idx = df.groupby("patient_id")["window_start"].idxmax()
    return df.loc[idx]


@app.route("/api/ward/summary")
def ward_summary():
    df = read_scored_1hr()
    latest = latest_window_per_patient(df)
    total = len(latest)
    high = int((latest["news2_tier"] == "High").sum())
    medium = int((latest["news2_tier"] == "Medium").sum())
    low = int((latest["news2_tier"] == "Low").sum())
    avg_news2 = round(float(latest["avg_news2_score"].mean()), 2) if total > 0 else 0.0
    return jsonify([{
        "total_patients": total,
        "high_count": high,
        "medium_count": medium,
        "low_count": low,
        "avg_news2_score": avg_news2,
    }])


@app.route("/api/ward/news2-trend")
def news2_trend():
    df = read_scored_1hr()
    cutoff = datetime.now(tz=timezone.utc) - timedelta(hours=24)
    df["window_start"] = pd.to_datetime(df["window_start"], utc=True)
    recent = df[df["window_start"] >= cutoff]
    trend = (
        recent.groupby("window_start")["avg_news2_score"]
        .mean()
        .reset_index()
        .sort_values("window_start")
    )
    trend["window_start"] = trend["window_start"].dt.strftime("%Y-%m-%dT%H:%M:%S+00:00")
    trend["avg_news2_score"] = trend["avg_news2_score"].round(2)
    return jsonify(trend.to_dict(orient="records"))


@app.route("/api/ward/patient-ranks")
def patient_ranks():
    df = read_scored_1hr()
    latest = latest_window_per_patient(df)
    ranked = latest[["patient_id", "max_news2_score", "news2_tier", "simulator_state"]].copy()
    ranked = ranked.sort_values("max_news2_score", ascending=False)
    return jsonify(ranked.to_dict(orient="records"))


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=5000, debug=False)
