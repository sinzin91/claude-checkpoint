mod extract;
mod session;
mod types;

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "claude-checkpoint",
    version,
    about = "Session checkpoint and restore for Claude Code"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract messages from a Claude Code session into a checkpoint file
    Extract {
        /// Number of messages to preserve
        #[arg(long, default_value = "100")]
        last: usize,

        /// Output file path (default: /tmp/checkpoint-YYYYMMDD-HHMMSS.md)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Path to session JSONL file (default: most recent in ~/.claude/projects/)
        #[arg(long)]
        session: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            last,
            output,
            session,
        } => {
            // Find session file
            let session_path = match session {
                Some(p) => p,
                None => {
                    let session_dir = dirs_home()?.join(".claude/projects");
                    session::find_most_recent_session(&session_dir)?
                }
            };

            // Extract messages
            let (messages, stats) = extract::extract_messages(&session_path, last)?;

            // Print stats to stderr
            eprintln!("# Session: {}", stats.session_name);
            eprintln!("# Source: {}", stats.source_path);
            eprintln!("# Size: {}", format_size(stats.file_size));
            eprintln!(
                "# Total messages: {} user + {} assistant",
                stats.total_user, stats.total_assistant
            );
            eprintln!(
                "# Extracted: {} messages with text content",
                stats.extracted
            );

            // Determine output path
            let output_path = output.unwrap_or_else(|| {
                let ts = Local::now().format("%Y%m%d-%H%M%S");
                PathBuf::from(format!("/tmp/checkpoint-{ts}.md"))
            });

            // Render and write
            let content = extract::render_checkpoint(&messages, &stats, last);
            extract::write_checkpoint(&content, &output_path)?;

            // Print output path to stdout (for scripts to capture)
            println!("{}", output_path.display());

            Ok(())
        }
    }
}

fn dirs_home() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0}K", bytes as f64 / 1024.0)
    } else {
        format!("{bytes}B")
    }
}
