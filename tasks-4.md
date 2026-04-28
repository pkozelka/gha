# Task 4 - Artifact Download

Implement a new `gha artifacts` command to download artifacts from a completed workflow run.

## Requirements

1. **New `gha artifacts` command** (alias: `art`)
   ```
   gha artifacts [OPTIONS] --repo <owner/repo> <RUN_ID>
   ```

2. **Command flags**
   - `--repo <owner/repo>` — Required, repository
   - `--token <TOKEN>` — Optional, GitHub token
   - `--output-dir <PATH>` — Where to save artifacts (default: ./gha-artifacts)
   - `--filter <PATTERN>` — Optional, download only artifacts matching name pattern

3. **Behavior**
   - Fetch artifacts list from GitHub API: `GET /repos/{repo}/actions/runs/{run_id}/artifacts`
   - For each artifact, download the ZIP file
   - Extract to `--output-dir/<artifact_name>/`
   - Print summary of downloaded artifacts
   - Return exit code 0 if successful, non-zero otherwise

4. **Example usage**
   ```bash
   gha artifacts --repo myorg/myrepo 123456
   gha artifacts --repo myorg/myrepo 123456 --output-dir ./outputs
   gha artifacts --repo myorg/myrepo 123456 --filter "test-*"
   ```

## Implementation

Create new module `src/artifacts.rs`:
- `struct Artifact { name, download_url, size }`
- `async fn download_artifacts(repo, run_id, auth, output_dir, filter) -> Result<Vec<Artifact>>`
  - Fetch artifact list
  - Filter if provided
  - Download each via reqwest
  - Unzip to output directory
  - Return summary

Update `src/main.rs`:
- Add `Artifacts` command variant
- Add to Commands enum
- Add to command match block
- Call `artifacts::download_artifacts()`

## No tests required for Task 4 (real API calls)

## Exit Codes
- `0` — All artifacts downloaded successfully
- `65` — No artifacts found or download failed
- `70` — API error or auth failure

## Notes

This is a straightforward download-and-extract operation. Artifacts list is returned by GitHub API, but doesn't include direct download URLs in some endpoints - may need to query individual artifacts or use the "download" endpoint.

