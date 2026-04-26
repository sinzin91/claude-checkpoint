mod extract;
mod session;
mod types;

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use std::fs;
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
    /// Install /checkpoint and /restore slash commands into ~/.claude/commands
    Install,

    /// Remove slash commands and binary from ~/.claude
    Uninstall,

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
        Commands::Install => {
            let home = dirs_home()?;
            let commands_dir = home.join(".claude/commands");
            fs::create_dir_all(&commands_dir)?;

            fs::write(
                commands_dir.join("checkpoint.md"),
                include_str!("../commands/checkpoint.md"),
            )?;
            fs::write(
                commands_dir.join("restore.md"),
                include_str!("../commands/restore.md"),
            )?;

            eprintln!(
                "Installed /checkpoint and /restore to {}",
                commands_dir.display()
            );
            Ok(())
        }
        Commands::Uninstall => {
            let home = dirs_home()?;
            let claude_dir = home.join(".claude");
            let files = [
                claude_dir.join("commands/checkpoint.md"),
                claude_dir.join("commands/restore.md"),
            ];
            for f in &files {
                if f.exists() {
                    fs::remove_file(f)?;
                    eprintln!("Removed {}", f.display());
                }
            }
            let legacy_bin = claude_dir.join("bin/claude-checkpoint");
            if legacy_bin.exists() {
                fs::remove_file(&legacy_bin)?;
                eprintln!("Removed {}", legacy_bin.display());
            }
            eprintln!("Done. If installed via cargo, also run: cargo uninstall claude-checkpoint");
            Ok(())
        }
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
                    let cwd = std::env::current_dir()?;
                    match session::find_session_for_cwd(&session_dir, &cwd)? {
                        Some(p) => p,
                        None => {
                            eprintln!(
                                "# No sessions for {} — falling back to global most-recent",
                                cwd.display()
                            );
                            session::find_most_recent_session(&session_dir)?
                        }
                    }
                }
            };

            // Extract messages
            let (messages, stats) = extract::extract_messages(&session_path, last)?;

            // Print stats to stderr
            eprintln!("# Session: {}", stats.session_name);
            eprintln!("# Source: {}", session_path.display());
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
