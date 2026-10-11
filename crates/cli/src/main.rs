use clap::{Parser, Subcommand};
use pfmt_context::{collect_repo_summary, extract_candidate_files};
use pfmt_core::{is_idempotent, no_llm_wrap, redact_secrets, Config, ExitCode, Profile};
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
    /// System diagnostics (keys, permissions, agent stores)
    Doctor,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Enhance { app, no_llm } => {
            handle_enhance(&app, no_llm);
        }
        Commands::Watch => {
            eprintln!("pfmt watch: background session watcher (phase 6)");
            exit(ExitCode::Ok.as_i32());
        }
        Commands::Review { .. } => {
            eprintln!("pfmt review: log review tool (phase 5)");
            exit(ExitCode::Ok.as_i32());
        }
        Commands::Replay { .. } => {
            eprintln!("pfmt replay: prompt replay tool (phase 5)");
            exit(ExitCode::Ok.as_i32());
        }
        Commands::Doctor => {
            eprintln!("pfmt doctor: system diagnostics (phase 4)");
            exit(ExitCode::Ok.as_i32());
        }
    }
}

fn handle_enhance(app: &str, force_no_llm: bool) {
    // 1. Read input from stdin
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("ERROR: failed to read input from stdin: {}", e);
        exit(ExitCode::Abort.as_i32());
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        eprintln!("empty input: nothing to do");
        exit(ExitCode::Abort.as_i32());
    }

    // 2. Idempotence check
    if is_idempotent(trimmed) {
        if input.ends_with('\n') {
            print!("{}", input);
        } else {
            println!("{}", input);
        }
        eprintln!("unchanged");
        exit(ExitCode::Ok.as_i32());
    }

    // 3. Load configuration and default profile
    let config = Config::load();
    let profile = Profile::load_default();

    // 4. Redaction stage
    let processed_prompt = if config.is_stage_enabled(app, "redact") {
        redact_secrets(trimmed)
    } else {
        trimmed.to_string()
    };

    // 5. Context collection stage
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (_repo_summary, _candidates) = if config.is_stage_enabled(app, "context") {
        let summary = collect_repo_summary(&cwd);
        let cand = extract_candidate_files(&processed_prompt, &cwd);
        (Some(summary), cand)
    } else {
        (None, Vec::new())
    };

    // 6. Offline / no-LLM path
    if force_no_llm || !config.is_stage_enabled(app, "writer") {
        let formatted = no_llm_wrap(&processed_prompt, &profile);
        println!("{}", formatted);
        eprintln!("OK (no-LLM skeleton, app={})", app);
        exit(ExitCode::Ok.as_i32());
    }

    // Default graceful wrap for current phase
    let formatted = no_llm_wrap(&processed_prompt, &profile);
    println!("{}", formatted);
    eprintln!("OK (app={})", app);
    exit(ExitCode::Ok.as_i32());
}
