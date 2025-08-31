use clap::Parser;
use tracing::{info, error};
use std::{fs, process};
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "myapp")]
#[command(about = "A Rust CLI application template", long_about = None)]
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
        repo: String,

        /// Workflow file name, e.g. "ci.yml"
        #[arg(long)]
        workflow: String,

        /// Branch or tag ref
        #[arg(long, default_value = "main")]
        r#ref: String,

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

#[tokio::main]
async fn main() {
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
            if let Err(e) = workflow_dispatch(repo, workflow, r#ref, token, args, mode).await {
                error!("Workflow dispatch failed: {}", e);
                exitcode::SOFTWARE
            } else {
                exitcode::OK
            }
        }

        None => {
            error!("No command provided");
            eprintln!("Error: No command provided. Try --help.");
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
            return Err(anyhow::anyhow!("Invalid arg format: {}", arg));
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
        println!(
            "curl -X POST \\
  -H 'Accept: application/vnd.github+json' \\
  -H 'Authorization: Bearer {}' \\
  -H 'X-GitHub-Api-Version: 2022-11-28' \\
  https://api.github.com/repos/{}/actions/workflows/{}/dispatches \\
  -d '{}'",
            token,
            repo,
            workflow,
            json_str.replace('\'', "\\'")
        );
    } else if mode == "make" {
        println!(
            "\tcurl -X POST \\\n\
        \t  -H 'Accept: application/vnd.github+json' \\\n\
        \t  -H 'Authorization: Bearer {}' \\\n\
        \t  -H 'X-GitHub-Api-Version: 2022-11-28' \\\n\
        \t  https://api.github.com/repos/{}/actions/workflows/{}/dispatches \\\n\
        \t  -d '{}'",
            token,
            repo,
            workflow,
            json_str.replace('\'', "\\'")
        );
    } else if mode == "call" {
        let client = reqwest::Client::new();
        let res = client
            .post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", token))
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&payload)
            .send()
            .await?;

        let response_status = res.status();
        if !response_status.is_success() {
            let text = res.text().await?;
            return Err(anyhow::anyhow!(
                "GitHub API error: {} - {}",
                response_status,
                text
            ));
        }

        info!("Workflow dispatch successful");
    } else {
        return Err(anyhow::anyhow!("Invalid mode: {}", mode));
    }

    Ok(())
}
