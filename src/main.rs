use clap::Parser;
use tracing::{info, error};
use std::process;

/// Command-line application template
#[derive(Parser, Debug)]
#[command(name = "gha")]
#[command(about = "GitHub Actions tool", long_about = None)]
struct Cli {
    /// Activate verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Example subcommand
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
}

fn main() {
    // Parse CLI args
    let cli = Cli::parse();

    // Setup logging
    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    info!("Starting application...");

    let exit_code = match &cli.command {
        Some(Commands::Run { name }) => {
            info!("Running with name: {}", name);
            println!("Hello, {}!", name);
            exitcode::OK
        }
        None => {
            error!("No command provided");
            eprintln!("Error: No command provided. Try --help.");
            exitcode::USAGE
        }
    };

    process::exit(exit_code);
}
