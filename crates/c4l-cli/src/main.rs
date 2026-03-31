use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claw4love", version, about = "Claude Code CLI — reimagined in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Verbose output (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current configuration
    Config,
    /// Show version and build info
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "c4l=info".into()),
        )
        .init();

    let cli = Cli::parse();

    // Load config
    let cwd = std::env::current_dir().ok();
    let config = c4l_config::C4lConfig::load(cwd.as_deref())?;

    match cli.command {
        Some(Commands::Config) => {
            let toml_str = toml::to_string_pretty(&config)?;
            println!("{toml_str}");
        }
        Some(Commands::Version) => {
            println!("claw4love {}", env!("CARGO_PKG_VERSION"));
            println!("model: {}", config.model.default_model);
        }
        None => {
            // Default: interactive REPL (Phase 4)
            println!("claw4love {} — interactive mode coming in Phase 4", env!("CARGO_PKG_VERSION"));
            println!("model: {}", config.model.default_model);
            println!("\nTry: claw4love config | claw4love version");
        }
    }

    Ok(())
}
