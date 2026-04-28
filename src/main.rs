use clap::{CommandFactory, Parser};
use tracing::{info, error};
use std::path::PathBuf;
use serde::Serialize;
use std::process;
use std::fs;mod git_utils;
mod github_utils;
mod gen_client;
mod run;
mod auth;
mod completion;
mod wait;

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
    /// Run a GitHub Actions workflow directly
    #[clap(alias = "r")]
    Run {
        /// Workflow file name or ID, e.g., "ci.yml"
        #[arg(value_name = "WORKFLOW")]
        workflow: String,

        /// GitHub repository in the form "owner/repo"
        #[arg(long)]
        repo: Option<String>,

        /// Branch or tag ref
        #[arg(long = "ref", short = 'b')]
        r#ref: Option<String>,

        /// GitHub authentication token (overrides GITHUB_TOKEN env and ~/.netrc)
        #[arg(long)]
        token: Option<String>,

        /// Base directory for default repo and ref
        #[arg(long, default_value = ".")]
        base_dir: PathBuf,

        /// Input arguments in name=value or name=@file form
        #[arg(value_name = "ARG", trailing_var_arg = true)]
        args: Vec<String>,

        /// Wait for the workflow run to complete before exiting
        #[arg(long)]
        wait: bool,

        /// Maximum time to wait in seconds (default: 3600 = 1 hour)
        #[arg(long)]
        timeout: Option<u64>,

        /// Polling interval in milliseconds (default: 500)
        #[arg(long)]
        poll_interval: Option<u64>,

        /// Output format: "human" or "json" (default: human)
        #[arg(long)]
        output: Option<String>,
    },

    /// Wait for a workflow run to complete
    #[clap(alias = "w")]
    Wait {
        /// GitHub repository in the form "owner/repo"
        #[arg(long)]
        repo: String,

        /// Workflow run ID
        #[arg(value_name = "RUN_ID")]
        run_id: u64,

        /// GitHub authentication token (overrides GITHUB_TOKEN env and ~/.netrc)
        #[arg(long)]
        token: Option<String>,

        /// Maximum time to wait in seconds (default: 3600 = 1 hour)
        #[arg(long)]
        timeout: Option<u64>,

        /// Polling interval in milliseconds (default: 500)
        #[arg(long)]
        poll_interval: Option<u64>,

        /// Output format: "human" or "json" (default: human)
        #[arg(long)]
        output: Option<String>,
    },

    /// Generate shell completions
    #[clap(alias = "comp")]
    Completion {
        /// Shell type for completion
        #[arg(value_enum)]
        shell: completion::Shell,
    },

    /// Dispatch a GitHub Actions workflow
    #[clap(alias = "wd")]
    WorkflowDispatch {
        /// Base directory for default repo and ref
        #[arg(long, default_value = ".")]
        base_dir: PathBuf,
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
    /// Generate Makefile clients for workflow_dispatch workflows
    #[clap(alias = "gen")]
    GenWorkflowClient {
        /// Directory containing the workflow yml files
        #[arg(short='d',long, default_value = ".github/workflows")]
        workflows_dir: PathBuf,
        /// Path to write the generated Makefile
        #[arg(short,long, default_value = "workflow_dispatch.Makefile")]
        output_file: PathBuf,
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
        .with_writer(std::io::stderr)
        .with_env_filter(log_level)
        // Use compact formatting for cleaner output
        .compact()
        .with_target(false)
        .init();

    let exit_code = match &cli.command {
        Some(Commands::Completion { shell }) => {
            if let Err(e) = completion::generate_completion(shell.clone()) {
                error!("Failed to generate completions: {e}");
                process::exit(exitcode::SOFTWARE);
            }
            exitcode::OK
        }

        Some(Commands::Run {
            repo,
            workflow,
            r#ref,
            token,
            base_dir,
            args,
            wait: should_wait,
            timeout,
            poll_interval,
            output,
        }) => {
            // Resolve repo
            let repo = match repo {
                Some(repo) => repo.to_string(),
                None => {
                    match git_utils::default_repo_from_git(base_dir.as_path()) {
                        None => {
                            error!("Missing repo, and unable to find it locally");
                            process::exit(exitcode::SOFTWARE);
                        }
                        Some(repo) => {
                            tracing::debug!("Using default repo: {repo}");
                            repo.to_string()
                        }
                    }
                }
            };

            // Resolve ref
            let repo_ref = match r#ref {
                Some(repo_ref) => repo_ref.to_string(),
                None => {
                    match git_utils::default_ref_from_git(base_dir.as_path()) {
                        None => {
                            error!("Missing ref, and unable to find it locally");
                            process::exit(exitcode::SOFTWARE);
                        }
                        Some(repo_ref) => {
                            tracing::debug!("Using default ref: {repo_ref}");
                            repo_ref.to_string()
                        }
                    }
                }
            };

            // Resolve authentication
            let auth = match auth::GithubAuth::resolve(token.clone()) {
                Err(e) => {
                    error!("Authentication failed: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(auth) => auth,
            };

            // Parse input arguments
            let inputs = match run::parse_input_args(args) {
                Err(e) => {
                    error!("Invalid input arguments: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(inputs) => inputs,
            };

            // Run the workflow
            if let Err(e) = run::run_workflow(&repo, workflow, &repo_ref, &auth, &inputs).await {
                error!("Workflow execution failed: {e}");
                process::exit(exitcode::SOFTWARE);
            }

            // If --wait, fetch the run ID and wait for completion
            if *should_wait {
                match wait::get_run_id_after_dispatch(&repo, workflow, &repo_ref, &auth.token).await {
                    Err(e) => {
                        error!("Failed to get workflow run ID: {e}");
                        process::exit(exitcode::SOFTWARE);
                    }
                    Ok(run_id) => {
                        info!("Waiting for workflow run {} to complete...", run_id);

                        // Build wait options from CLI args
                        let mut opts = wait::WaitOptions::default();
                        if let Some(t) = timeout { opts.timeout_secs = *t; }
                        if let Some(p) = poll_interval { opts.poll_interval_ms = *p; }
                        if let Some(fmt) = output {
                            opts.output_format = match wait::OutputFormat::from_str(fmt) {
                                Ok(f) => f,
                                Err(e) => {
                                    error!("Invalid output format: {e}");
                                    process::exit(exitcode::USAGE);
                                }
                            };
                        }

                        match wait::wait_for_run(&repo, run_id, &auth.token, &opts).await {
                            Err(e) => {
                                error!("Failed to wait for run: {e}");
                                process::exit(exitcode::SOFTWARE);
                            }
                            Ok(run) => {
                                let conclusion = run.conclusion.as_ref().map(|c| c.clone()).unwrap_or(wait::WorkflowRunConclusion::Neutral);
                                match opts.output_format {
                                    wait::OutputFormat::Human => {
                                        info!("Workflow run completed: {} — {}", conclusion.display(), run.html_url);
                                    }
                                    wait::OutputFormat::Json => {
                                        if let Ok(json) = run.to_json() {
                                            println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                                        }
                                    }
                                }
                                process::exit(conclusion.exit_code());
                            }
                        }
                    }
                }
            } else {
                exitcode::OK
            }
        }

        Some(Commands::Wait { repo, run_id, token, timeout, poll_interval, output }) => {
            // Resolve authentication
            let auth = match auth::GithubAuth::resolve(token.clone()) {
                Err(e) => {
                    error!("Authentication failed: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(auth) => auth,
            };

            // Build wait options from CLI args
            let mut opts = wait::WaitOptions::default();
            if let Some(t) = timeout { opts.timeout_secs = *t; }
            if let Some(p) = poll_interval { opts.poll_interval_ms = *p; }
            if let Some(fmt) = output {
                opts.output_format = match wait::OutputFormat::from_str(fmt) {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Invalid output format: {e}");
                        process::exit(exitcode::USAGE);
                    }
                };
            }

            // Wait for the run
            match wait::wait_for_run(repo, *run_id, &auth.token, &opts).await {
                Err(e) => {
                    error!("Failed to wait for run: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(run) => {
                    let conclusion = run.conclusion.as_ref().map(|c| c.clone()).unwrap_or(wait::WorkflowRunConclusion::Neutral);
                    match opts.output_format {
                        wait::OutputFormat::Human => {
                            info!("Workflow run completed: {} — {}", conclusion.display(), run.html_url);
                        }
                        wait::OutputFormat::Json => {
                            if let Ok(json) = run.to_json() {
                                println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                            }
                        }
                    }
                    process::exit(conclusion.exit_code());
                }
            }
        }

        Some(Commands::GenWorkflowClient { workflows_dir, output_file }) => {
            if let Err(e) = gen_client::generate_makefile(workflows_dir, output_file) {
                error!("Failed to generate workflow client: {e:?}");
                process::exit(exitcode::SOFTWARE);
            }
            exitcode::OK
        }

        Some(Commands::WorkflowDispatch {
                 base_dir,
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
                    match git_utils::default_repo_from_git(base_dir.as_path()) {
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
                    match git_utils::default_ref_from_git(base_dir.as_path()) {
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
                None => match github_utils::default_workflow_from_dir(base_dir) {
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
            let mut cmd = Cli::command();
            let mut buf = Vec::new();
            cmd.write_help(&mut buf).unwrap();
            let help_text = String::from_utf8_lossy(&buf);
            
            error!("No command provided. Showing help:\n{}", help_text);

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
