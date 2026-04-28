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
            tracing::debug!("Found ~/.netrc for authentication (requires curl/netrc support)");
            // Return a dummy token; the presence of .netrc will be used by curl
            return Ok(Self::from_token(String::new()));
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

#[cfg(test)]
mod tests {
    use super::*;

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
}


