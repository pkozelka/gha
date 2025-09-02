use std::fmt::Display;
use std::process::Command;

/// Try to get default "owner/repo" from git remote origin
#[derive(Debug, Clone)]
pub(crate) struct RepoInfo {
    pub(crate) owner: String,
    pub(crate) repo: String,
}

impl Display for RepoInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

pub(crate) fn default_repo_from_git() -> Option<RepoInfo> {
    let output = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Examples:
    //   https://github.com/owner/repo.git
    //   git@github.com:owner/repo.git
    if url.contains("github.com") {
        if let Some(pos) = url.find("github.com") {
            let mut path = &url[pos + "github.com".len()..];

            // strip leading ':' or '/'
            if path.starts_with(':') || path.starts_with('/') {
                path = &path[1..];
            }

            // strip trailing ".git"
            let path = path.strip_suffix(".git").unwrap_or(path);

            // split into owner/repo
            let mut parts = path.splitn(2, '/');
            let owner = parts.next()?.to_string();
            let repo = parts.next()?.to_string();

            return Some(RepoInfo { owner, repo });
        }
    }

    None
}

#[derive(Debug, Clone)]
pub struct RefInfo {
    r#ref: String,
}

impl RefInfo {
    pub fn new(r#ref: String) -> Self {
        Self { r#ref }
    }
}

impl Display for RefInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.r#ref)
    }
}

pub fn default_ref_from_git() -> Option<RefInfo> {
    // Try to get branch name
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Some(RefInfo { r#ref: branch });
        }
    }

    // If not on a branch (detached HEAD), fall back to commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !sha.is_empty() {
            return Some(RefInfo { r#ref: sha });
        }
    }

    None
}
