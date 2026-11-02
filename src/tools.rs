//! MCP tool definitions and dispatch.
//!
//! Endpoints mirror Garmin's official Personal API client
//! (github.com/garmin/connect-personal-api-python).

use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use chrono::{Local, NaiveDate};
use serde_json::{Map, Value, json};

use crate::client::GarminClient;

fn date_only() -> Value {
    json!({
        "type": "object",
        "properties": { "date": { "type": "string", "description": "Calendar date (YYYY-MM-DD). Defaults to today." } }
    })
}

fn range() -> Value {
    json!({
        "type": "object",
        "properties": {
            "start_date": { "type": "string", "description": "Start date (YYYY-MM-DD), inclusive. Defaults to 6 days before end_date." },
            "end_date": { "type": "string", "description": "End date (YYYY-MM-DD), inclusive. Defaults to today." }
        }
    })
}

fn no_args() -> Value {
    json!({ "type": "object", "properties": {} })
}

fn activity_id() -> Value {
    json!({
        "type": "object",
        "properties": { "activity_id": { "type": ["integer", "string"], "description": "Garmin activity ID (from get_activities)." } },
        "required": ["activity_id"]
    })
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    // download_activity writes a file locally; everything else only reads.
    let read_only = name != "download_activity";
    json!({
        "name": name,
        "description": description,
        "inputSchema": schema,
        "annotations": { "readOnlyHint": read_only, "destructiveHint": false, "openWorldHint": true }
    })
}

pub fn list() -> Vec<Value> {
    vec![
        tool(
            "get_user_profile_settings",
            "Get the user's profile settings (units, birth date, height, weight, time zone, etc.).",
            no_args(),
        ),
        tool(
            "get_activities",
            "Search activities, newest first. Optionally filter by date range and activity type.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max activities to return (default 20, max 100).", "minimum": 1, "maximum": 100 },
                    "start": { "type": "integer", "description": "Offset for pagination (default 0).", "minimum": 0 },
                    "start_date": { "type": "string", "description": "Only activities on/after this date (YYYY-MM-DD)." },
                    "end_date": { "type": "string", "description": "Only activities on/before this date (YYYY-MM-DD)." },
                    "activity_type": { "type": "string", "description": "Activity type key, e.g. running, cycling, swimming, strength_training, hiking." }
                }
            }),
        ),
        tool(
            "get_activity",
            "Get the full summary and details of one activity.",
            activity_id(),
        ),
        tool(
            "get_activity_workouts",
            "Get the workouts associated with an activity.",
            activity_id(),
        ),
        tool(
            "download_activity",
            "Download an activity's original file (a ZIP, usually containing a .fit file) and save it locally. Returns the saved path.",
            json!({
                "type": "object",
                "properties": {
                    "activity_id": { "type": ["integer", "string"], "description": "Garmin activity ID." },
                    "output_dir": { "type": "string", "description": "Directory to save into. Defaults to ~/Downloads." }
                },
                "required": ["activity_id"]
            }),
        ),
        tool(
            "get_daily_summaries",
            "Daily summaries (steps, calories, distance, floors, intensity minutes, resting HR, stress, Body Battery) for a date range. Use the same start and end date for one day.",
            range(),
        ),
        tool(
            "get_wellness_activity_summaries",
            "Wellness activity summaries for a date range.",
            range(),
        ),
        tool(
            "get_sleep",
            "Detailed sleep data for one night: stages, sleep score, timings, SpO2/respiration/HRV during sleep.",
            date_only(),
        ),
        tool(
            "get_sleep_summaries",
            "Daily sleep summaries for a date range.",
            range(),
        ),
        tool(
            "get_heart_rate",
            "Heart-rate data for one day (resting, min, max and intraday samples).",
            date_only(),
        ),
        tool(
            "get_heart_rate_zones",
            "The user's configured heart-rate zones.",
            no_args(),
        ),
        tool(
            "get_hrv",
            "Heart-rate variability (HRV) status and overnight readings for one day.",
            date_only(),
        ),
        tool(
            "get_hrv_summaries",
            "Daily HRV summaries for a date range.",
            range(),
        ),
        tool(
            "get_stress",
            "Daily stress levels for a date range.",
            range(),
        ),
        tool(
            "get_body_battery",
            "Daily Body Battery reports (charged, drained, highs/lows) for a date range.",
            json!({
                "type": "object",
                "properties": {
                    "start_date": { "type": "string", "description": "Start date (YYYY-MM-DD), inclusive. Defaults to 6 days before end_date." },
                    "end_date": { "type": "string", "description": "End date (YYYY-MM-DD), inclusive. Defaults to today." },
                    "chunks": { "type": "integer", "description": "Optional number of chunks to split each day into.", "minimum": 1 }
                }
            }),
        ),
        tool(
            "get_spo2",
            "Pulse-ox (SpO2) and altitude acclimation data for one day.",
            date_only(),
        ),
        tool(
            "get_steps",
            "Daily step totals and goals for a date range.",
            range(),
        ),
        tool(
            "get_hydration",
            "Daily hydration intake and goals for a date range.",
            range(),
        ),
        tool(
            "get_fitness_age",
            "Fitness age and contributing factors as of a date.",
            date_only(),
        ),
        tool(
            "get_weight",
            "Weight and body-composition measurements for a date range.",
            range(),
        ),
        tool(
            "get_weight_goals",
            "Effective weight goals for a date range.",
            range(),
        ),
        tool(
            "get_food_logs",
            "Food/nutrition logs for one day.",
            date_only(),
        ),
        tool(
            "get_menstrual_cycle_summary",
            "Menstrual cycle summary as of a date.",
            date_only(),
        ),
        tool(
            "get_menstrual_cycle_day",
            "Menstrual cycle day view for a date.",
            date_only(),
        ),
        tool(
            "garmin_api_get",
            "Escape hatch: authenticated GET against any Garmin Personal API path (personal.connectapi.garmin.com). Use when no dedicated tool exists.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "API path starting with '/', e.g. /usersummary-service/stats/steps/daily/2026-10-01/2026-10-07" },
                    "query": { "type": "object", "description": "Optional query-string parameters.", "additionalProperties": { "type": ["string", "number", "boolean"] } }
                },
                "required": ["path"]
            }),
        ),
    ]
}

fn parse_date(args: &Map<String, Value>, key: &str) -> Result<Option<NaiveDate>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| anyhow!("`{key}` must be a date in YYYY-MM-DD format, got {s:?}")),
        Some(other) => bail!("`{key}` must be a string, got {other}"),
    }
}

fn date(args: &Map<String, Value>) -> Result<String> {
    Ok(parse_date(args, "date")?
        .unwrap_or_else(|| Local::now().date_naive())
        .to_string())
}

fn date_range(args: &Map<String, Value>) -> Result<(String, String)> {
    let end = parse_date(args, "end_date")?.unwrap_or_else(|| Local::now().date_naive());
    let start = parse_date(args, "start_date")?.unwrap_or(end - chrono::Days::new(6));
    if start > end {
        bail!("start_date must not be after end_date");
    }
    Ok((start.to_string(), end.to_string()))
}

fn int(args: &Map<String, Value>, key: &str) -> Result<Option<u64>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            .map(Some)
            .ok_or_else(|| anyhow!("`{key}` must be a non-negative integer")),
    }
}

fn activity(args: &Map<String, Value>) -> Result<u64> {
    match int(args, "activity_id")? {
        Some(id) if id > 0 => Ok(id),
        _ => bail!("`activity_id` must be a positive integer"),
    }
}

pub async fn call(client: &GarminClient, name: &str, args: &Map<String, Value>) -> Result<Value> {
    match name {
        "get_user_profile_settings" => {
            client
                .get("/userprofile-service/userprofile/settings", &[])
                .await
        }

        "get_activities" => {
            let limit = int(args, "limit")?.unwrap_or(20).clamp(1, 100);
            let start = int(args, "start")?.unwrap_or(0);
            let mut q = vec![("start", start.to_string()), ("limit", limit.to_string())];
            if let Some(d) = parse_date(args, "start_date")? {
                q.push(("startDate", d.to_string()));
            }
            if let Some(d) = parse_date(args, "end_date")? {
                q.push(("endDate", d.to_string()));
            }
            if let Some(t) = args
                .get("activity_type")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                q.push(("activityType", t.to_string()));
            }
            let raw = client
                .get("/activitylist-service/activities/search/activities", &q)
                .await?;
            Ok(summarize_activities(raw))
        }
        "get_activity" => {
            client
                .get(
                    &format!("/activity-service/activity/{}", activity(args)?),
                    &[],
                )
                .await
        }
        "get_activity_workouts" => {
            client
                .get(
                    &format!("/activity-service/activity/{}/workouts", activity(args)?),
                    &[],
                )
                .await
        }
        "download_activity" => {
            let id = activity(args)?;
            let dir = match args
                .get("output_dir")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                Some(d) => PathBuf::from(shellexpand_home(d)),
                None => dirs::download_dir()
                    .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
                    .ok_or_else(|| {
                        anyhow!("could not determine a download directory; pass output_dir")
                    })?,
            };
            let bytes = client
                .get_bytes(
                    &format!("/download-service/files/activity/{id}"),
                    "application/zip, application/x-zip-compressed, application/octet-stream",
                )
                .await?;
            if bytes.is_empty() {
                bail!("the original activity file response was empty");
            }
            tokio::fs::create_dir_all(&dir).await?;
            let path = dir.join(format!("garmin-activity-{id}.zip"));
            tokio::fs::write(&path, &bytes).await?;
            Ok(json!({ "path": path, "bytes": bytes.len() }))
        }

        "get_daily_summaries" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    "/usersummary-service/v2/dailysummaries",
                    &[("startDate", s), ("endDate", e)],
                )
                .await
        }
        "get_wellness_activity_summaries" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    &format!("/wellnessactivity-service/activity/summary/range/{s}/{e}"),
                    &[],
                )
                .await
        }
        "get_sleep" => {
            client
                .get("/sleep-service/sleep/dailySleep", &[("date", date(args)?)])
                .await
        }
        "get_sleep_summaries" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    "/sleep-service/sleep/dailySleepSummariesByDate",
                    &[("startDate", s), ("endDate", e)],
                )
                .await
        }
        "get_heart_rate" => {
            client
                .get(
                    "/wellness-service/wellness/dailyHeartRate",
                    &[("date", date(args)?)],
                )
                .await
        }
        "get_heart_rate_zones" => {
            client
                .get("/biometric-service/heartRateZones/v2", &[])
                .await
        }
        "get_hrv" => {
            client
                .get(&format!("/hrv-service/hrv/{}", date(args)?), &[])
                .await
        }
        "get_hrv_summaries" => {
            let (s, e) = date_range(args)?;
            client
                .get(&format!("/hrv-service/hrv/daily/{s}/{e}"), &[])
                .await
        }
        "get_stress" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    &format!("/usersummary-service/stats/stress/daily/{s}/{e}"),
                    &[],
                )
                .await
        }
        "get_body_battery" => {
            let (s, e) = date_range(args)?;
            let mut q = vec![("startDate", s), ("endDate", e)];
            if let Some(c) = int(args, "chunks")? {
                q.push(("chunks", c.to_string()));
            }
            client
                .get("/wellness-service/wellness/bodyBattery/reports/daily", &q)
                .await
        }
        "get_spo2" => {
            client
                .get(
                    &format!(
                        "/wellness-service/wellness/daily/spo2acclimation/{}",
                        date(args)?
                    ),
                    &[],
                )
                .await
        }
        "get_steps" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    &format!("/usersummary-service/stats/steps/daily/{s}/{e}"),
                    &[],
                )
                .await
        }
        "get_hydration" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    &format!("/usersummary-service/stats/hydration/daily/{s}/{e}"),
                    &[],
                )
                .await
        }
        "get_fitness_age" => {
            client
                .get(
                    &format!("/fitnessage-service/fitnessage/v2/{}", date(args)?),
                    &[],
                )
                .await
        }
        "get_weight" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    &format!("/weight-service/weight/range/{s}/{e}"),
                    &[("includeAll", "true".into())],
                )
                .await
        }
        "get_weight_goals" => {
            let (s, e) = date_range(args)?;
            client
                .get(
                    &format!("/goal-service/goal/user/effective/weightgoal/{s}/{e}"),
                    &[],
                )
                .await
        }
        "get_food_logs" => {
            client
                .get(
                    &format!("/nutrition-service/food/logs/{}", date(args)?),
                    &[],
                )
                .await
        }
        "get_menstrual_cycle_summary" => {
            client
                .get(
                    &format!(
                        "/periodichealth-service/menstrualcycle/summary/{}",
                        date(args)?
                    ),
                    &[],
                )
                .await
        }
        "get_menstrual_cycle_day" => {
            let d = date(args)?;
            let today = Local::now().date_naive().to_string();
            client
                .get(
                    &format!("/periodichealth-service/menstrualcycle/dayview/{d}"),
                    &[("todayDate", today)],
                )
                .await
        }

        "garmin_api_get" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("`path` is required"))?;
            let query: Vec<(&str, String)> = match args.get("query") {
                Some(Value::Object(m)) => m
                    .iter()
                    .map(|(k, v)| {
                        let v = match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        (k.as_str(), v)
                    })
                    .collect(),
                _ => vec![],
            };
            client.get(path, &query).await
        }

        other => bail!("unknown tool: {other}"),
    }
}

fn shellexpand_home(p: &str) -> String {
    match (p.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest).to_string_lossy().into_owned(),
        _ => p.to_string(),
    }
}

/// Activity search returns ~150 fields per activity; keep the useful ones so
/// results stay readable and fit in an assistant's context.
fn summarize_activities(raw: Value) -> Value {
    const KEEP: &[&str] = &[
        "activityId",
        "activityName",
        "startTimeLocal",
        "distance",
        "duration",
        "movingDuration",
        "elapsedDuration",
        "elevationGain",
        "elevationLoss",
        "averageSpeed",
        "maxSpeed",
        "averageHR",
        "maxHR",
        "calories",
        "averageRunningCadenceInStepsPerMinute",
        "avgPower",
        "normPower",
        "aerobicTrainingEffect",
        "anaerobicTrainingEffect",
        "trainingEffectLabel",
        "activityTrainingLoad",
        "vO2MaxValue",
        "steps",
        "locationName",
        "description",
    ];
    let Value::Array(items) = raw else { return raw };
    Value::Array(
        items
            .into_iter()
            .map(|a| {
                let mut out = Map::new();
                if let Some(t) = a.pointer("/activityType/typeKey") {
                    out.insert("activityType".into(), t.clone());
                }
                for k in KEEP {
                    if let Some(v) = a.get(*k).filter(|v| !v.is_null()) {
                        out.insert((*k).into(), v.clone());
                    }
                }
                Value::Object(out)
            })
            .collect(),
    )
}
