//! Model Context Protocol server over stdio (newline-delimited JSON-RPC 2.0).

use std::sync::Arc;

use anyhow::Result;
use serde_json::{Map, Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{OnceCell, mpsc};

use crate::{client::GarminClient, tools};

const SUPPORTED_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

struct Server {
    // Initialised lazily so the server can start (and list tools) before an
    // API key is configured; a missing key surfaces as a tool error instead.
    client: OnceCell<Arc<GarminClient>>,
}

impl Server {
    async fn client(&self) -> Result<Arc<GarminClient>> {
        self.client
            .get_or_try_init(|| async { GarminClient::from_env().map(Arc::new) })
            .await
            .cloned()
    }

    async fn handle(&self, method: &str, params: Value) -> Result<Value, (i64, String)> {
        match method {
            "initialize" => {
                let requested = params["protocolVersion"].as_str().unwrap_or_default();
                let version = SUPPORTED_VERSIONS
                    .iter()
                    .find(|v| **v == requested)
                    .unwrap_or(&SUPPORTED_VERSIONS[0]);
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": {
                        "name": "garmin-mcp",
                        "title": "Garmin Connect",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "instructions": "Read-only access to the user's own Garmin Connect data through the official Garmin Personal API: activities, daily summaries, sleep, heart rate, HRV, stress, Body Battery, SpO2, steps, hydration, weight, nutrition and more. Dates are YYYY-MM-DD; single dates default to today and ranges to the last 7 days. Distances are in meters, durations in seconds, speeds in m/s."
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools::list() })),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or_default().to_string();
                let args = match params.get("arguments") {
                    Some(Value::Object(m)) => m.clone(),
                    _ => Map::new(),
                };
                let result = match self.client().await {
                    Ok(client) => tools::call(&client, &name, &args).await,
                    Err(e) => Err(e),
                };
                Ok(match result {
                    Ok(value) => json!({
                        "content": [{ "type": "text", "text": serde_json::to_string(&value).unwrap_or_default() }],
                        "isError": false
                    }),
                    Err(e) => json!({
                        "content": [{ "type": "text", "text": format!("Error: {e:#}") }],
                        "isError": true
                    }),
                })
            }
            _ => Err((-32601, format!("method not found: {method}"))),
        }
    }
}

pub async fn serve() -> Result<()> {
    let server = Arc::new(Server {
        client: OnceCell::new(),
    });
    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();

    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(msg) = rx.recv().await {
            let mut line = serde_json::to_vec(&msg).unwrap_or_default();
            line.push(b'\n');
            if stdout.write_all(&line).await.is_err() || stdout.flush().await.is_err() {
                break;
            }
        }
    });

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(json!({
                    "jsonrpc": "2.0", "id": null,
                    "error": { "code": -32700, "message": format!("parse error: {e}") }
                }));
                continue;
            }
        };
        // Notifications and responses carry no id (or no method); nothing to reply.
        let (Some(id), Some(method)) = (
            msg.get("id").cloned(),
            msg["method"].as_str().map(str::to_owned),
        ) else {
            continue;
        };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let server = server.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let reply = match server.handle(&method, params).await {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err((code, message)) => {
                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
                }
            };
            let _ = tx.send(reply);
        });
    }

    drop(tx);
    let _ = writer.await;
    Ok(())
}
