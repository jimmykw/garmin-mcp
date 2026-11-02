//! Client for the Garmin Connect Personal API.
//!
//! Authentication (see <https://developer.garmin.com/gc-developer-program/personal/personal-api>):
//! a Personal API key, created at
//! <https://connect.garmin.com/app/settings/personalApiToken>, is exchanged for
//! a JWT bearer token valid for up to 24 hours. There are no refresh tokens;
//! the key is simply exchanged again when the token nears expiry.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use reqwest::StatusCode;
use serde_json::Value;
use tokio::sync::Mutex;

pub const DEFAULT_BASE_URL: &str = "https://personal.connectapi.garmin.com";
const TOKEN_PATH: &str = "/personalapitoken-service/personal-api/personal-api-key/token";
const API_KEY_ENV: &str = "GARMIN_PERSONAL_API_KEY";
const BASE_URL_ENV: &str = "GARMIN_PERSONAL_API_BASE_URL";
/// Same default location as Garmin's official Python client.
const API_KEY_FILE: &str = ".config/garmin/connect-personal-api-key";
/// Renew the bearer token when it has less than this long left.
const REFRESH_WINDOW_SECS: i64 = 60 * 60;

pub fn api_key_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(API_KEY_FILE)
}

/// Resolve the Personal API key from `GARMIN_PERSONAL_API_KEY`, falling back
/// to `~/.config/garmin/connect-personal-api-key`.
pub fn resolve_api_key() -> Result<String> {
    if let Ok(key) = std::env::var(API_KEY_ENV) {
        let key = key.trim().to_string();
        if key.is_empty() {
            bail!("{API_KEY_ENV} is set but empty");
        }
        return Ok(key);
    }
    let path = api_key_file();
    let key = std::fs::read_to_string(&path).map_err(|_| {
        anyhow!(
            "no Garmin Personal API key found. Create one at \
             https://connect.garmin.com/app/settings/personalApiToken, then set \
             {API_KEY_ENV} or save it to {}",
            path.display()
        )
    })?;
    let key = key.trim().to_string();
    if key.is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(key)
}

struct BearerToken {
    access_token: String,
    expires_at: i64,
}

/// Read the `exp` claim from a JWT without verifying it.
fn jwt_expiration(token: &str) -> Result<i64> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| anyhow!("token endpoint did not return a JWT"))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .context("decoding JWT payload")?;
    let claims: Value = serde_json::from_slice(&bytes).context("parsing JWT payload")?;
    claims["exp"]
        .as_f64()
        .map(|e| e as i64)
        .ok_or_else(|| anyhow!("JWT has no exp claim"))
}

pub struct GarminClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    token: Mutex<Option<BearerToken>>,
}

impl GarminClient {
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var(BASE_URL_ENV)
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("garmin-mcp/", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            base_url,
            api_key: resolve_api_key()?,
            token: Mutex::new(None),
        })
    }

    async fn bearer_token(&self, force_refresh: bool) -> Result<String> {
        let mut guard = self.token.lock().await;
        let now = chrono::Utc::now().timestamp();
        if let Some(t) = guard
            .as_ref()
            .filter(|t| !force_refresh && t.expires_at - now > REFRESH_WINDOW_SECS)
        {
            return Ok(t.access_token.clone());
        }

        let resp = self
            .http
            .post(format!("{}{TOKEN_PATH}", self.base_url))
            .header("accept", "*/*")
            .header("authorization", format!("ApiKey {}", self.api_key))
            .send()
            .await
            .context("requesting Garmin bearer token")?;
        let status = resp.status();
        let body = resp.text().await?;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            bail!(
                "Garmin rejected the Personal API key (HTTP {status}). It may be expired or \
                 deleted; create a new one at https://connect.garmin.com/app/settings/personalApiToken"
            );
        }
        if !status.is_success() {
            bail!(
                "token request failed with HTTP {status}: {}",
                snippet(&body)
            );
        }
        let access_token = body.trim().to_string();
        let expires_at = jwt_expiration(&access_token)?;
        *guard = Some(BearerToken {
            access_token: access_token.clone(),
            expires_at,
        });
        Ok(access_token)
    }

    async fn send(
        &self,
        path: &str,
        query: &[(&str, String)],
        accept: &str,
    ) -> Result<reqwest::Response> {
        if !path.starts_with('/') || path.starts_with("//") {
            bail!("API path must start with a single '/'");
        }
        let url = format!("{}{path}", self.base_url);
        let mut retried = false;
        loop {
            let token = self.bearer_token(retried).await?;
            let resp = self
                .http
                .get(&url)
                .query(query)
                .bearer_auth(token)
                .header("accept", accept)
                .send()
                .await
                .with_context(|| format!("GET {path}"))?;
            let status = resp.status();
            if status == StatusCode::UNAUTHORIZED && !retried {
                retried = true;
                continue;
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                bail!("GET {path} failed with HTTP {status}: {}", snippet(&body));
            }
            return Ok(resp);
        }
    }

    /// GET a Personal API path and decode the JSON body (`null` if empty).
    pub async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let body = self
            .send(path, query, "application/json")
            .await?
            .bytes()
            .await?;
        if body.iter().all(u8::is_ascii_whitespace) {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&body)
            .with_context(|| format!("GET {path} returned non-JSON response"))
    }

    /// GET a binary resource (e.g. an original activity file).
    pub async fn get_bytes(&self, path: &str, accept: &str) -> Result<Vec<u8>> {
        Ok(self.send(path, &[], accept).await?.bytes().await?.to_vec())
    }

    /// Exchange the key for a token without fetching data; used by `status`.
    pub async fn check(&self) -> Result<i64> {
        self.bearer_token(true).await?;
        Ok(self
            .token
            .lock()
            .await
            .as_ref()
            .map(|t| t.expires_at)
            .unwrap_or_default())
    }
}

fn snippet(body: &str) -> String {
    body.trim().chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_jwt_exp() {
        // {"alg":"none"} . {"sub":"x","exp":1767225600} . (no signature)
        let token = "eyJhbGciOiJub25lIn0.eyJzdWIiOiJ4IiwiZXhwIjoxNzY3MjI1NjAwfQ.";
        assert_eq!(jwt_expiration(token).unwrap(), 1_767_225_600);
    }

    #[test]
    fn rejects_non_jwt() {
        assert!(jwt_expiration("not-a-jwt").is_err());
    }
}
