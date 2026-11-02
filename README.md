# garmin-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server that gives AI assistants read-only access to your own Garmin Connect data through the official [Garmin Connect Personal API](https://developer.garmin.com/gc-developer-program/personal/personal-api). It covers activities, daily summaries, sleep, heart rate, HRV, stress, Body Battery, SpO2, steps, hydration, weight, nutrition and more.

Written in Rust. It runs as a single binary and speaks MCP over stdio.

## Setup

1. Create a Personal API key at <https://connect.garmin.com/app/settings/personalApiToken>. Garmin shows the key only once.
2. Give the key to the server in either of these ways:
   ```sh
   # Option A: environment variable
   export GARMIN_PERSONAL_API_KEY=...

   # Option B: key file (the same location Garmin's official Python client uses)
   mkdir -p ~/.config/garmin
   pbpaste > ~/.config/garmin/connect-personal-api-key
   chmod 600 ~/.config/garmin/connect-personal-api-key
   ```
3. Build the server and check that the key works:
   ```sh
   cargo install --path .
   garmin-mcp status
   ```

The server exchanges the key for a bearer token that lasts up to 24 hours. It keeps the token in memory and gets a new one automatically. Keys expire after 30 days to 1 year, depending on what you chose, and Garmin emails you before that happens.

## Use with Claude Code

```sh
claude mcp add garmin -- garmin-mcp
# or pass the key explicitly:
claude mcp add garmin -e GARMIN_PERSONAL_API_KEY=... -- garmin-mcp
```

## Use with Claude Desktop and other clients

```json
{
  "mcpServers": {
    "garmin": { "command": "/absolute/path/to/garmin-mcp" }
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `get_user_profile_settings` | Units, birth date, height, weight, time zone |
| `get_activities` | Search activities (paging, date range and type filters) |
| `get_activity`, `get_activity_workouts` | Details for one activity |
| `download_activity` | Save an activity's original file (ZIP/FIT) to disk |
| `get_daily_summaries`, `get_wellness_activity_summaries` | Daily totals over a date range |
| `get_sleep`, `get_sleep_summaries` | Sleep detail for one night, or summaries over a range |
| `get_heart_rate`, `get_heart_rate_zones` | Daily heart rate and configured zones |
| `get_hrv`, `get_hrv_summaries` | Heart-rate variability |
| `get_stress`, `get_body_battery`, `get_spo2` | Stress, Body Battery, pulse ox |
| `get_steps`, `get_hydration` | Daily steps and hydration |
| `get_fitness_age` | Fitness age |
| `get_weight`, `get_weight_goals` | Weight and body composition |
| `get_food_logs` | Nutrition logs |
| `get_menstrual_cycle_summary`, `get_menstrual_cycle_day` | Cycle tracking |
| `garmin_api_get` | Authenticated GET against any Personal API path |

Dates use `YYYY-MM-DD`. Single dates default to today and ranges default to the last 7 days. Values use Garmin's native units: meters, seconds and m/s.

The endpoints match Garmin's official client, [garmin/connect-personal-api-python](https://github.com/garmin/connect-personal-api-python).
