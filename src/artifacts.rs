use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct ArtifactSummary {
    pub downloaded: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("GitHub API error: {0}")]
    Api(String),
    #[error("no artifacts found for run {0}")]
    NoArtifacts(u64),
    #[error("artifact download failed: {0}")]
    Download(String),
}

/// Download artifacts from a workflow run
pub async fn download_artifacts(
    repo: &str,
    run_id: u64,
    auth_token: &str,
    output_dir: &Path,
    filter: Option<&str>,
) -> Result<ArtifactSummary, ArtifactError> {
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

    let response = builder
        .send()
        .await
        .map_err(|e| ArtifactError::Api(e.to_string()))?;
    let status_code = response.status();
    if !status_code.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(ArtifactError::Api(format!(
            "Failed to fetch artifacts: {} - {}",
            status_code, text
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| ArtifactError::Api(e.to_string()))?;
    let json: Value =
        serde_json::from_str(&body).map_err(|e| ArtifactError::Api(e.to_string()))?;
    let artifacts = json["artifacts"]
        .as_array()
        .ok_or_else(|| ArtifactError::Api("No artifacts array in response".to_string()))?;

    if artifacts.is_empty() {
        return Err(ArtifactError::NoArtifacts(run_id));
    }

    fs::create_dir_all(output_dir).map_err(|e| ArtifactError::Download(e.to_string()))?;
    let mut downloaded = 0;
    let glob_filter = filter.and_then(|p| glob::Pattern::new(p).ok());

    for artifact in artifacts {
        let name = artifact["name"]
            .as_str()
            .ok_or_else(|| ArtifactError::Api("Missing artifact name".to_string()))?;
        let download_url = artifact["archive_download_url"]
            .as_str()
            .ok_or_else(|| ArtifactError::Api("Missing download URL".to_string()))?;

        // Check filter
        if let Some(pattern) = &glob_filter {
            if !pattern.matches(name) {
                debug!("Skipping artifact {} (filter glob)", name);
                continue;
            }
        }

        info!("Downloading artifact: {}", name);

        // Download artifact ZIP
        let artifact_dir = output_dir.join(name);
        fs::create_dir_all(&artifact_dir)
            .map_err(|e| ArtifactError::Download(e.to_string()))?;

        let mut builder = client.get(download_url);
        builder = builder
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "gha")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if !auth_token.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", auth_token));
        }

        let response = builder
            .send()
            .await
            .map_err(|e| ArtifactError::Api(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ArtifactError::Download(format!(
                "Failed to download artifact {}: {}",
                name,
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ArtifactError::Download(e.to_string()))?;
        let zip_path = artifact_dir.join("archive.zip");
        fs::write(&zip_path, &bytes).map_err(|e| ArtifactError::Download(e.to_string()))?;

        // Extract ZIP
        let zip_file = std::fs::File::open(&zip_path)
            .map_err(|e| ArtifactError::Download(e.to_string()))?;
        let mut archive =
            zip::ZipArchive::new(zip_file).map_err(|e| ArtifactError::Download(e.to_string()))?;
        archive
            .extract(&artifact_dir)
            .map_err(|e| ArtifactError::Download(e.to_string()))?;
        fs::remove_file(&zip_path).map_err(|e| ArtifactError::Download(e.to_string()))?;

        downloaded += 1;
    }

    if downloaded == 0 {
        return Err(ArtifactError::NoArtifacts(run_id));
    }

    info!("Downloaded {} artifact(s) to {}", downloaded, output_dir.display());
    Ok(ArtifactSummary { downloaded })
}


