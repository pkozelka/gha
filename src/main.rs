use clap::{CommandFactory, Parser};
use tracing::{info, error};
use std::{fs, process};
use std::path::PathBuf;
use serde::Serialize;

mod git_utils;
mod github_utils;

#[derive(Parser, Debug)]
#[command(name = "gha")]
#[command(about = "GitHub Action tool", long_about = None)]
struct Cli {
    /// Activate verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Subcommands
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Do something useful
    Run {
        #[arg(short, long, default_value = "world")]
        name: String,
    },

    /// Dispatch a GitHub Actions workflow
    WorkflowDispatch {
        /// GitHub repository in the form "owner/repo"
        #[arg(long)]
        repo: Option<String>,

        /// Workflow file name, e.g., "ci.yml" (default: auto-detect if only one workflow exists)
        #[arg(long)]
        workflow: Option<String>,

        /// Branch or tag ref
        #[arg(long)]
        r#ref: Option<String>,

        /// GitHub token (can also be provided via GITHUB_TOKEN env)
        #[arg(long, env = "GITHUB_TOKEN")]
        token: String,

        /// Input arguments in name=value or name=@file form
        #[arg(long = "arg")]
        args: Vec<String>,

        /// Mode: "curl" (print curl), "make" (Makefile syntax), or "call" (execute)
        #[arg(long, default_value = "curl")]
        mode: String,
    },
}

#[derive(Serialize)]
struct DispatchPayload {
    r#ref: String,
    inputs: serde_json::Map<String, serde_json::Value>,
}

/// Search upward from the current dir until HOME or root for `.env`.
/// Returns true if a file was loaded, false otherwise.
fn load_env_file() -> bool {
    let home_dir = dirs::home_dir();

    // Start from the current directory
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    loop {
        let candidate = dir.join(".env");
        if dotenvy::from_filename(&candidate).is_ok() {
            tracing::debug!("Loaded .env file from {}", candidate.display());
            break true;
        }

        // Stop if we reached home or root
        if Some(&dir) == home_dir.as_ref() || !dir.pop() {
            break false;
        }
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env from current dir or home
    load_env_file();

    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    let exit_code = match &cli.command {
        Some(Commands::Run { name }) => {
            println!("Hello, {}!", name);
            exitcode::OK
        }

        Some(Commands::WorkflowDispatch {
                 repo,
                 workflow,
                 r#ref,
                 token,
                 args,
                 mode,
             }) => {
            let repo = match repo {
                Some(repo) => repo.to_string(),
                None => {
                    match git_utils::default_repo_from_git() {
                        None => anyhow::bail!("Missing repo, and unable to find it locally"),
                        Some(repo) => {
                            tracing::debug!("Using default repo: {repo}");
                            repo.to_string()
                        }
                    }
                }
            };
            let repo_ref = match r#ref {
                Some(repo_ref) => repo_ref.to_string(),
                None => {
                    match git_utils::default_ref_from_git() {
                        None => anyhow::bail!("Missing ref, and unable to find it locally"),
                        Some(repo_ref) => {
                            tracing::debug!("Using default ref: {repo_ref}");
                            repo_ref.to_string()
                        }
                    }
                }
            };
            // resolve workflow
            let workflow = match workflow {
                Some(w) => w.clone(),
                None => match github_utils::default_workflow_from_dir() {
                    None => anyhow::bail!("Could not determine workflow automatically. Please use --workflow."),
                    Some(workflow) => {
                        tracing::debug!("Using single existing workflow as default: {workflow}");
                        workflow
                    },
                }
            };

            if let Err(e) = workflow_dispatch(&repo, &workflow, &repo_ref, token, args, mode).await {
                error!("Workflow dispatch failed: {e}");
                exitcode::SOFTWARE
            } else {
                exitcode::OK
            }
        }

        None => {
            error!("No command provided. Showing help:");

            // Print help to stdout
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!(); // newline after help

            exitcode::USAGE
        }
    };

    process::exit(exit_code);
}

async fn workflow_dispatch(
    repo: &str,
    workflow: &str,
    r#ref: &str,
    token: &str,
    args: &[String],
    mode: &str,
) -> anyhow::Result<()> {
    let mut inputs = serde_json::Map::new();

    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            let val = if value.starts_with('@') {
                let file_path = &value[1..];
                let contents = fs::read_to_string(file_path)?;
                serde_json::Value::String(contents)
            } else {
                serde_json::Value::String(value.to_string())
            };
            inputs.insert(key.to_string(), val);
        } else {
            return Err(anyhow::anyhow!("Invalid arg format: {arg}"));
        }
    }

    let payload = DispatchPayload {
        r#ref: r#ref.to_string(),
        inputs,
    };

    let url = format!(
        "https://api.github.com/repos/{}/actions/workflows/{}/dispatches",
        repo, workflow
    );

    let json_str = serde_json::to_string_pretty(&payload)?;

    if mode == "curl" {
        let escaped_json = json_str.replace('\'', "\\'");
        println!(
            "curl -X POST \\
  -H 'Accept: application/vnd.github+json' \\
  -H 'Authorization: Bearer {token}' \\
  -H 'X-GitHub-Api-Version: 2022-11-28' \\
  https://api.github.com/repos/{repo}/actions/workflows/{workflow}/dispatches \\
  -d '{escaped_json}'");
    } else if mode == "make" {
        let escaped_json = json_str.replace('\'', "\\'");
        println!(
            "\tcurl -X POST \\\n\
        \t  -H 'Accept: application/vnd.github+json' \\\n\
        \t  -H 'Authorization: Bearer {token}' \\\n\
        \t  -H 'X-GitHub-Api-Version: 2022-11-28' \\\n\
        \t  https://api.github.com/repos/{repo}/actions/workflows/{workflow}/dispatches \\\n\
        \t  -d '{escaped_json}'");
    } else if mode == "call" {
        let client = reqwest::Client::new();
        let res = client
            .post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {token}", ))
            .header("User-Agent", "gha")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&payload)
            .send()
            .await?;

        let response_status = res.status();
        if !response_status.is_success() {
            let text = res.text().await?;
            return Err(anyhow::anyhow!("GitHub API error: {response_status} - {text}"));
        }

        info!("Workflow dispatch successful");
    } else {
        return Err(anyhow::anyhow!("Invalid mode: {}", mode));
    }

    Ok(())
}
