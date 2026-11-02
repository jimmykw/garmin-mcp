mod client;
mod mcp;
mod tools;

use anyhow::Result;

const USAGE: &str = "\
garmin-mcp — MCP server for the Garmin Connect Personal API

USAGE:
    garmin-mcp [serve]   Run the MCP server on stdio (default)
    garmin-mcp status    Check that the Personal API key works

SETUP:
    Create a Personal API key at
    https://connect.garmin.com/app/settings/personalApiToken
    then either set GARMIN_PERSONAL_API_KEY or save the key to
    ~/.config/garmin/connect-personal-api-key (chmod 600).

ENVIRONMENT:
    GARMIN_PERSONAL_API_KEY        Personal API key
    GARMIN_PERSONAL_API_BASE_URL   Override the API host
                                   (default: https://personal.connectapi.garmin.com)
";

async fn status() -> Result<()> {
    let client = client::GarminClient::from_env()?;
    let expires_at = client.check().await?;
    let expires = chrono::DateTime::from_timestamp(expires_at, 0)
        .map(|t| t.with_timezone(&chrono::Local).to_rfc2822())
        .unwrap_or_else(|| expires_at.to_string());
    println!("Personal API key OK. Bearer token valid until {expires}.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("serve") => mcp::serve().await,
        Some("status") => status().await,
        Some("-h" | "--help" | "help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => {
            eprint!("unknown command: {other}\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}
