use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Download artifacts from a workflow run
pub async fn download_artifacts(
    repo: &str,
    run_id: u64,
    auth_token: &str,
    output_dir: &Path,
    filter: Option<&str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.github.com/repos/{}/actions/runs/{}/artifacts",
        repo, run_id
    );

    // Fetch artifacts list
    debug!("Fetching artifacts for run {}", run_id);
    let mut builder = client.get(&url);
    builder = builder
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gha")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if !auth_token.is_empty() {
        builder = builder.header("Authorization", format!("Bearer {}", auth_token));
    }

    let response = builder.send().await?;
    let status_code = response.status();
    if !status_code.is_success() {
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to fetch artifacts: {} - {}", status_code, text);
    }

    let body = response.text().await?;
    let json: Value = serde_json::from_str(&body)?;
    let artifacts = json["artifacts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No artifacts array in response"))?;

    if artifacts.is_empty() {
        info!("No artifacts found for run {}", run_id);
        return Ok(());
    }

    fs::create_dir_all(output_dir)?;
    let mut downloaded = 0;

    for artifact in artifacts {
        let name = artifact["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing artifact name"))?;
        let download_url = artifact["archive_download_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing download URL"))?;

        // Check filter
        if let Some(pattern) = filter {
            if !name.contains(pattern) {
                debug!("Skipping artifact {} (filter: {})", name, pattern);
                continue;
            }
        }

        info!("Downloading artifact: {}", name);

        // Download artifact ZIP
        let artifact_dir = output_dir.join(name);
        fs::create_dir_all(&artifact_dir)?;

        let mut builder = client.get(download_url);
        builder = builder
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gha")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if !auth_token.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", auth_token));
        }

        let response = builder.send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to download artifact {}: {}", name, response.status());
        }

        let bytes = response.bytes().await?;
        let zip_path = artifact_dir.join("archive.zip");
        fs::write(&zip_path, &bytes)?;

        // Extract ZIP
        let zip_file = std::fs::File::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(zip_file)?;
        archive.extract(&artifact_dir)?;
        fs::remove_file(&zip_path)?;

        downloaded += 1;
    }

    info!("Downloaded {} artifact(s) to {}", downloaded, output_dir.display());
    Ok(())
}


