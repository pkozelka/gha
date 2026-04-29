use anyhow::Result;

/// Authentication token for GitHub API
#[derive(Clone, Debug)]
pub struct GithubAuth {
    pub token: String,
}

impl GithubAuth {
    /// Create auth from explicit token
    pub fn from_token(token: String) -> Self {
        Self { token }
    }

    /// Resolve authentication from: CLI arg > GITHUB_TOKEN env > ~/.netrc
    pub fn resolve(cli_token: Option<String>) -> Result<Self> {
        // Priority 1: explicit CLI argument
        if let Some(token) = cli_token {
            tracing::debug!("Using token from CLI argument");
            return Ok(Self::from_token(token));
        }

        // Priority 2: GITHUB_TOKEN environment variable
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                tracing::debug!("Using token from GITHUB_TOKEN environment variable");
                return Ok(Self::from_token(token));
            }
        }

        // Priority 3: ~/.netrc (implicit via curl/reqwest)
        // For now, we just note if it might exist
        let netrc_path = dirs::home_dir()
            .map(|h| h.join(".netrc"))
            .unwrap_or_default();

        if netrc_path.exists() {
            if let Some(token) = parse_netrc_token(&netrc_path)? {
                tracing::debug!("Using token from ~/.netrc");
                return Ok(Self::from_token(token));
            }
            tracing::debug!("~/.netrc exists but no api.github.com/github.com machine entry with password found");
        }

        anyhow::bail!(
            "No GitHub authentication found. \
             Provide --token, set GITHUB_TOKEN env, or configure ~/.netrc"
        )
    }

    /// Check if token is valid (non-empty)
    pub fn is_valid(&self) -> bool {
        !self.token.is_empty()
    }
}

fn parse_netrc_token(path: &std::path::Path) -> Result<Option<String>> {
    let contents = std::fs::read_to_string(path)?;
    let tokens = contents.split_whitespace().collect::<Vec<_>>();

    let mut i = 0;
    let mut current_machine: Option<&str> = None;
    while i < tokens.len() {
        match tokens[i] {
            "machine" if i + 1 < tokens.len() => {
                current_machine = Some(tokens[i + 1]);
                i += 2;
            }
            "password" if i + 1 < tokens.len() => {
                if matches!(current_machine, Some("api.github.com") | Some("github.com")) {
                    let token = tokens[i + 1].to_string();
                    if !token.is_empty() {
                        return Ok(Some(token));
                    }
                }
                i += 2;
            }
            _ => i += 1,
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::fixture::FileWriteStr;
    use assert_fs::fixture::PathChild;

    #[test]
    fn cli_token_takes_precedence() {
        let auth = GithubAuth::resolve(Some("cli-token".to_string())).unwrap();
        assert_eq!(auth.token, "cli-token");
    }

    #[test]
    fn empty_cli_token_is_treated_as_some() {
        let auth = GithubAuth::resolve(Some(String::new()));
        assert!(auth.is_ok());
    }

    #[test]
    fn parse_netrc_extracts_api_github_password() {
        let dir = assert_fs::TempDir::new().unwrap();
        let netrc = dir.child(".netrc");
        netrc
            .write_str("machine api.github.com login me password ghp_netrc_token")
            .unwrap();

        let token = parse_netrc_token(netrc.path()).unwrap();
        assert_eq!(token.as_deref(), Some("ghp_netrc_token"));
    }
}


