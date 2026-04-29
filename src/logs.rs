use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

const COLOR_PALETTE: &[&str] = &[
    colors::CYAN,
    colors::YELLOW,
    colors::GREEN,
    colors::MAGENTA,
    colors::BLUE,
];

/// Stream logs from a workflow run to stdout
///
/// Periodically fetches logs from GitHub API and prints them to stdout
/// with timestamps and color coding.
pub async fn stream_logs(
    repo: &str,
    run_id: u64,
    auth_token: &str,
    poll_interval_ms: u64,
) -> Result<()> {
    let client = reqwest::Client::new();
    let logs_url = format!(
        "https://api.github.com/repos/{}/actions/runs/{}/logs",
        repo, run_id
    );
    let run_url = format!(
        "https://api.github.com/repos/{}/actions/runs/{}",
        repo, run_id
    );

    let mut line_offsets: BTreeMap<String, usize> = BTreeMap::new();
    let sleep_ms = poll_interval_ms.max(250);

    info!("Streaming logs for run {}...", run_id);

    loop {
        let run_json = fetch_json(&client, &run_url, auth_token).await?;
        let status = run_json["status"].as_str().unwrap_or("unknown");

        if let Ok(log_files) = fetch_logs_archive(&client, &logs_url, auth_token).await {
            for (idx, (name, contents)) in log_files.iter().enumerate() {
                let seen = line_offsets.get(name).copied().unwrap_or(0);
                let lines: Vec<&str> = contents.lines().collect();
                if lines.len() <= seen {
                    continue;
                }

                let color = COLOR_PALETTE[idx % COLOR_PALETTE.len()];
                for line in &lines[seen..] {
                    println!(
                        "[{}] {}{}{} {}",
                        unix_timestamp(),
                        color,
                        name,
                        colors::RESET,
                        line
                    );
                }
                line_offsets.insert(name.clone(), lines.len());
            }
        }

        if status == "completed" {
            info!("Run {} reached completed state; stopping log stream", run_id);
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
    }

    Ok(())
}

async fn fetch_json(client: &reqwest::Client, url: &str, auth_token: &str) -> Result<Value> {
    let mut builder = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gha")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if !auth_token.is_empty() {
        builder = builder.header("Authorization", format!("Bearer {}", auth_token));
    }

    let response = builder.send().await?;
    if !response.status().is_success() {
        anyhow::bail!("failed to fetch JSON from {}: {}", url, response.status());
    }
    Ok(response.json::<Value>().await?)
}

async fn fetch_logs_archive(
    client: &reqwest::Client,
    url: &str,
    auth_token: &str,
) -> Result<BTreeMap<String, String>> {
    let mut builder = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gha")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if !auth_token.is_empty() {
        builder = builder.header("Authorization", format!("Bearer {}", auth_token));
    }

    let response = builder.send().await?;
    if !response.status().is_success() {
        anyhow::bail!("failed to fetch logs archive: {}", response.status());
    }

    let bytes = response.bytes().await?;
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)?;
    let mut files = BTreeMap::new();

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name().ends_with('/') {
            continue;
        }
        let mut text = String::new();
        if file.read_to_string(&mut text).is_ok() {
            files.insert(file.name().to_string(), text);
        }
    }

    debug!("Fetched {} log files", files.len());
    Ok(files)
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Colors for terminal output
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const CYAN: &str = "\x1b[36m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const GREEN: &str = "\x1b[32m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const BLUE: &str = "\x1b[34m";
}

