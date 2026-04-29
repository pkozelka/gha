use clap::{CommandFactory, Parser, ValueEnum};
use serde_json::json;
use tracing::{info, error};
use std::path::PathBuf;
use std::process;
mod git_utils;
mod gen_client;
mod spawn;
mod auth;
mod completion;
mod awaiting;
mod artifacts;
mod webhook;
mod logs;

#[derive(Clone, Debug, ValueEnum)]
enum OutputArg {
    Human,
    Json,
}

impl From<OutputArg> for awaiting::OutputFormat {
    fn from(value: OutputArg) -> Self {
        match value {
            OutputArg::Human => awaiting::OutputFormat::Human,
            OutputArg::Json => awaiting::OutputFormat::Json,
        }
    }
}

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
    /// Spawn a GitHub Actions workflow directly
    #[clap(alias = "s")]
    Spawn {
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

        /// Await workflow completion before exiting
        #[arg(long = "await")]
        r#await: bool,

        /// Maximum time to await in seconds (default: 3600 = 1 hour)
        #[arg(long, requires = "await")]
        timeout: Option<u64>,

        /// Polling interval in milliseconds while awaiting (default: 500)
        #[arg(long, requires = "await")]
        poll_interval: Option<u64>,

        /// Output format: "human" or "json" (default: human)
        #[arg(long, value_enum)]
        output: Option<OutputArg>,

        /// Await completion via webhook listener instead of API polling
        #[arg(long, requires = "await")]
        webhook: bool,

        /// Local port for webhook listener (used with --webhook)
        #[arg(long, default_value_t = 3456, requires = "await", requires = "webhook")]
        webhook_port: u16,

        /// Shared secret used to validate webhook signatures
        #[arg(long, env = "GITHUB_WEBHOOK_SECRET", requires = "await", requires = "webhook")]
        webhook_secret: Option<String>,

        /// Stream logs while awaiting completion (requires --await)
        #[arg(long, requires = "await")]
        follow_logs: bool,
    },

    /// Await a workflow run to complete
    #[clap(alias = "a")]
    Await {
        /// GitHub repository in the form "owner/repo" (auto-detected if omitted)
        #[arg(long)]
        repo: Option<String>,

        /// Base directory for default repo detection
        #[arg(long, default_value = ".")]
        base_dir: PathBuf,

        /// Workflow run ID
        #[arg(value_name = "RUN_ID")]
        run_id: u64,

        /// GitHub authentication token (overrides GITHUB_TOKEN env and ~/.netrc)
        #[arg(long)]
        token: Option<String>,

        /// Maximum time to await in seconds (default: 3600 = 1 hour)
        #[arg(long)]
        timeout: Option<u64>,

        /// Polling interval in milliseconds while awaiting (default: 500)
        #[arg(long)]
        poll_interval: Option<u64>,

        /// Output format: "human" or "json" (default: human)
        #[arg(long, value_enum)]
        output: Option<OutputArg>,

        /// Await completion via webhook listener instead of API polling
        #[arg(long)]
        webhook: bool,

        /// Local port for webhook listener (used with --webhook)
        #[arg(long, default_value_t = 3456)]
        webhook_port: u16,

        /// Shared secret used to validate webhook signatures
        #[arg(long, env = "GITHUB_WEBHOOK_SECRET")]
        webhook_secret: Option<String>,

        /// Stream logs while awaiting completion
        #[arg(long)]
        follow_logs: bool,
    },

    /// Generate shell completions
    #[clap(alias = "comp")]
    Completion {
        /// Shell type for completion
        #[arg(value_enum)]
        shell: completion::Shell,
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
    /// Download artifacts from a workflow run
    #[clap(alias = "art")]
    Artifacts {
        /// GitHub repository in the form "owner/repo"
        #[arg(long)]
        repo: String,

        /// Workflow run ID
        #[arg(value_name = "RUN_ID")]
        run_id: u64,

        /// Directory to save artifacts (default: ./gha-artifacts)
        #[arg(long, default_value = "./gha-artifacts")]
        output_dir: PathBuf,

        /// Filter artifacts by name pattern
        #[arg(long)]
        filter: Option<String>,

        /// GitHub authentication token
        #[arg(long)]
        token: Option<String>,
    },
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

fn build_await_options(
    timeout: &Option<u64>,
    poll_interval: &Option<u64>,
    output: &Option<OutputArg>,
    webhook: bool,
    webhook_port: u16,
    webhook_secret: Option<String>,
    follow_logs: bool,
) -> awaiting::AwaitOptions {
    let mut opts = awaiting::AwaitOptions::default();
    if let Some(t) = timeout {
        opts.timeout_secs = *t;
    }
    if let Some(p) = poll_interval {
        opts.poll_interval_ms = *p;
    }
    if let Some(fmt) = output {
        opts.output_format = fmt.clone().into();
    }
    opts.use_webhook = webhook;
    opts.webhook_port = webhook_port;
    opts.webhook_secret = webhook_secret;
    opts.follow_logs = follow_logs;
    opts
}

fn print_final_run_output(
    run: &awaiting::WorkflowRun,
    output_format: awaiting::OutputFormat,
) -> awaiting::WorkflowRunConclusion {
    let conclusion = run
        .conclusion
        .as_ref()
        .cloned()
        .unwrap_or(awaiting::WorkflowRunConclusion::Neutral);
    match output_format {
        awaiting::OutputFormat::Human => {
            info!("Workflow run completed: {} — {}", conclusion.display(), run.html_url);
        }
        awaiting::OutputFormat::Json => {
            if let Ok(json) = run.to_json() {
                println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
            }
        }
    }
    conclusion
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

        Some(Commands::Spawn {
            repo,
            workflow,
            r#ref,
            token,
            base_dir,
            args,
            r#await: should_await,
            timeout,
            poll_interval,
            output,
            webhook,
            webhook_port,
            webhook_secret,
            follow_logs,
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
            let inputs = match spawn::parse_input_args(args) {
                Err(e) => {
                    error!("Invalid input arguments: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(inputs) => inputs,
            };

            // Spawn the workflow
            if let Err(e) = spawn::spawn_workflow(&repo, workflow, &repo_ref, &auth, &inputs).await {
                error!("Workflow execution failed: {e}");
                process::exit(exitcode::SOFTWARE);
            }

            let run_id = match awaiting::get_run_id_after_dispatch(&repo, workflow, &repo_ref, &auth.token).await {
                Err(e) => {
                    error!("Failed to get workflow run ID after spawn: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(run_id) => run_id,
            };

            let run_url = format!("https://github.com/{repo}/actions/runs/{run_id}");

            // If --await, fetch the run ID and await completion
            if *should_await {
                {
                    info!("Awaiting workflow run {} to complete...", run_id);

                    // Build await options from CLI args
                    let opts = build_await_options(
                        timeout,
                        poll_interval,
                        output,
                        *webhook,
                        *webhook_port,
                        webhook_secret.clone(),
                        *follow_logs,
                    );

                    if opts.use_webhook && opts.webhook_secret.is_none() {
                        error!("--webhook requires --webhook-secret (or GITHUB_WEBHOOK_SECRET)");
                        process::exit(exitcode::USAGE);
                    }

                    let log_task = if opts.follow_logs {
                        let repo_for_logs = repo.clone();
                        let token_for_logs = auth.token.clone();
                        let interval_ms = opts.poll_interval_ms;
                        Some(tokio::spawn(async move {
                            logs::stream_logs(
                                &repo_for_logs,
                                run_id,
                                &token_for_logs,
                                interval_ms,
                            )
                            .await
                        }))
                    } else {
                        None
                    };

                    match awaiting::await_run(&repo, run_id, &auth.token, &opts).await {
                        Err(e) => {
                            if let Some(handle) = log_task {
                                handle.abort();
                            }
                            error!("Failed to await run: {e}");
                            process::exit(exitcode::SOFTWARE);
                        }
                        Ok(run) => {
                            if let Some(handle) = log_task {
                                handle.abort();
                            }
                            let conclusion = print_final_run_output(&run, opts.output_format);
                            process::exit(conclusion.exit_code());
                        }
                    }
                }
            } else {
                if matches!(output, Some(OutputArg::Json)) {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "id": run_id,
                            "repo": repo,
                            "workflow": workflow,
                            "ref": repo_ref,
                            "url": run_url,
                        }))
                        .unwrap_or_default()
                    );
                } else {
                    println!("Spawned workflow run: {} — {}", run_id, run_url);
                }
                exitcode::OK
            }
        }

        Some(Commands::Await { repo, base_dir, run_id, token, timeout, poll_interval, output, webhook, webhook_port, webhook_secret, follow_logs }) => {
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

            // Resolve authentication
            let auth = match auth::GithubAuth::resolve(token.clone()) {
                Err(e) => {
                    error!("Authentication failed: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(auth) => auth,
            };

            // Build await options from CLI args
            let opts = build_await_options(
                timeout,
                poll_interval,
                output,
                *webhook,
                *webhook_port,
                webhook_secret.clone(),
                *follow_logs,
            );

            if opts.use_webhook && opts.webhook_secret.is_none() {
                error!("--webhook requires --webhook-secret (or GITHUB_WEBHOOK_SECRET)");
                process::exit(exitcode::USAGE);
            }

            let log_task = if opts.follow_logs {
                let repo_for_logs = repo.clone();
                let token_for_logs = auth.token.clone();
                let run_id_for_logs = *run_id;
                let interval_ms = opts.poll_interval_ms;
                Some(tokio::spawn(async move {
                    logs::stream_logs(
                        &repo_for_logs,
                        run_id_for_logs,
                        &token_for_logs,
                        interval_ms,
                    )
                    .await
                }))
            } else {
                None
            };

            // Await the run
            match awaiting::await_run(&repo, *run_id, &auth.token, &opts).await {
                Err(e) => {
                    if let Some(handle) = log_task {
                        handle.abort();
                    }
                    error!("Failed to await run: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(run) => {
                    if let Some(handle) = log_task {
                        handle.abort();
                    }
                    let conclusion = print_final_run_output(&run, opts.output_format);
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

        Some(Commands::Artifacts { repo, run_id, output_dir, filter, token }) => {
            let auth = match auth::GithubAuth::resolve(token.clone()) {
                Err(e) => {
                    error!("Authentication failed: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(auth) => auth,
            };

            match artifacts::download_artifacts(
                repo,
                *run_id,
                &auth.token,
                output_dir,
                filter.as_deref(),
            ).await {
                Err(artifacts::ArtifactError::NoArtifacts(_)) => {
                    error!("No artifacts matched the request");
                    process::exit(exitcode::DATAERR);
                }
                Err(artifacts::ArtifactError::Download(e)) => {
                    error!("Failed to download artifacts: {e}");
                    process::exit(exitcode::DATAERR);
                }
                Err(artifacts::ArtifactError::Api(e)) => {
                    error!("Failed to download artifacts: {e}");
                    process::exit(exitcode::SOFTWARE);
                }
                Ok(summary) => {
                    info!("Downloaded {} artifact(s)", summary.downloaded);
                    exitcode::OK
                }
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

