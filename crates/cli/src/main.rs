use clap::{Parser, Subcommand};
use pfmt_cli::doctor;
use pfmt_cli::pipeline::execute_enhance_pipeline;
use pfmt_core::{Config, ExitCode, Profile};
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command(
    name = "pfmt",
    about = "Prompt Formatter: AI prompt enhancer and agent watcher",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Enhance a raw prompt from stdin and print formatted XML to stdout
    Enhance {
        /// Name of the frontmost application (e.g. Antigravity, OpenCode, Terminal)
        #[arg(long, default_value = "unknown")]
        app: String,

        /// Force offline no-LLM path
        #[arg(long)]
        no_llm: bool,
    },
    /// Run the session watcher daemon
    Watch,
    /// Review logged prompt metrics and degradations
    Review {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
    },
    /// Replay a logged run for regression testing
    Replay { log_file: PathBuf },
    /// System diagnostics (keys, permissions, agent stores, model endpoints)
    Doctor,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Enhance { app, no_llm } => {
            handle_enhance(&app, no_llm).await;
        }
        Commands::Watch => {
            eprintln!("pfmt watch: background session watcher (phase 6)");
            exit(ExitCode::Ok.as_i32());
        }
        Commands::Review { from, to } => {
            let log_dir = PathBuf::from("daily_logs");
            match pfmt_cli::review::run_review(&log_dir, from.as_deref(), to.as_deref()) {
                Ok(report) => {
                    pfmt_cli::review::print_review_report(&report, from.as_deref(), to.as_deref());
                    exit(ExitCode::Ok.as_i32());
                }
                Err(e) => {
                    eprintln!("Error reading review logs: {}", e);
                    exit(ExitCode::Abort.as_i32());
                }
            }
        }
        Commands::Replay { log_file } => {
            let config = Config::load();
            let profile = Profile::load_default();
            match pfmt_cli::replay::run_replay(&log_file, &config, &profile).await {
                Ok(()) => exit(ExitCode::Ok.as_i32()),
                Err(e) => {
                    eprintln!("Error replaying log: {}", e);
                    exit(ExitCode::Abort.as_i32());
                }
            }
        }
        Commands::Doctor => {
            let config = Config::load();
            let ok = doctor::run_doctor(&config).await;
            if ok {
                exit(ExitCode::Ok.as_i32());
            } else {
                exit(ExitCode::Abort.as_i32());
            }
        }
    }
}

async fn handle_enhance(app: &str, force_no_llm: bool) {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("ERROR: failed to read input from stdin: {}", e);
        exit(ExitCode::Abort.as_i32());
    }

    let config = Config::load();
    let profile = Profile::load_default();
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let result =
        execute_enhance_pipeline(&input, app, force_no_llm, &cwd, &config, &profile, None).await;

    if !result.stdout_content.is_empty() {
        if result.stdout_content.ends_with('\n') {
            print!("{}", result.stdout_content);
        } else {
            println!("{}", result.stdout_content);
        }
    }

    if !result.stderr_status.is_empty() {
        eprintln!("{}", result.stderr_status);
    }

    exit(result.exit_code.as_i32());
}
