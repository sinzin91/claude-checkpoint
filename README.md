# claude-checkpoint

[![CI](https://github.com/sinzin91/claude-checkpoint/actions/workflows/ci.yml/badge.svg)](https://github.com/sinzin91/claude-checkpoint/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/claude-checkpoint.svg)](https://crates.io/crates/claude-checkpoint)
[![GitHub Release](https://img.shields.io/github/v/release/sinzin91/claude-checkpoint)](https://github.com/sinzin91/claude-checkpoint/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Session checkpoint and restore for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Save your conversation context before `/clear`, then seamlessly resume where you left off.

## The problem

Claude Code's `/clear` command wipes the entire conversation context. This is necessary when you hit context limits, but you lose all the decisions, corrections, and working state from the session.

`claude-checkpoint` solves this by extracting conversation history into a structured checkpoint file, having Claude generate a summary of the session arc, and then restoring both the summary and raw messages into a new context window.

## How it works

1. **`/checkpoint`** runs a native Rust binary that extracts the last N messages from your session's JSONL file, writes them to a markdown file, and asks Claude to fill in a structured summary (goal, files modified, key decisions, corrections, blockers, next step).

2. **`/clear`** wipes the context as usual.

3. **`/restore`** reads the checkpoint file back in. Claude internalizes the summary and raw messages, then continues from where the conversation left off — no "welcome back" preamble, just picks up the thread.

## Install

### Quick install (recommended)

```bash
cargo install claude-checkpoint
```

Then install the slash commands:

```bash
git clone https://github.com/sinzin91/claude-checkpoint.git /tmp/claude-checkpoint
mkdir -p ~/.claude/commands
cp /tmp/claude-checkpoint/commands/checkpoint.md ~/.claude/commands/
cp /tmp/claude-checkpoint/commands/restore.md ~/.claude/commands/
rm -rf /tmp/claude-checkpoint
```

### From source

```bash
git clone https://github.com/sinzin91/claude-checkpoint.git
cd claude-checkpoint
bash install.sh
```

This builds the binary, copies it to `~/.claude/bin/`, and installs the slash commands.

### Pre-built binaries

Download from [GitHub Releases](https://github.com/sinzin91/claude-checkpoint/releases) for Linux (x86_64, aarch64) and macOS (x86_64, aarch64).

### Uninstall

```bash
bash install.sh --uninstall
```

## Usage

### Save a checkpoint

```
/checkpoint        # save last 100 messages (default)
/checkpoint 50     # save last 50 messages
```

Claude will:
- Extract raw messages from the current session
- Generate a structured summary covering goals, decisions, files, corrections, and next steps
- Save everything to `/tmp/checkpoint-YYYYMMDD-HHMMSS.md`
- Tell you the file path

### Clear and restore

```
/clear                           # wipe context
/restore                         # restore most recent checkpoint
/restore /tmp/checkpoint-20260403-161507.md  # restore specific checkpoint
```

Claude will read the checkpoint and continue from where you left off.

### CLI usage (standalone)

```bash
# Extract from most recent session
claude-checkpoint extract --last 100 --output /tmp/checkpoint.md

# Extract from a specific session
claude-checkpoint extract --session ~/.claude/projects/-Users-me/abc123.jsonl
```

## What gets saved

The checkpoint file has two sections:

**Summary** — a structured overview Claude generates:
- Goal, current task, files modified
- Key decisions and corrections (these are lost first during compaction)
- Working patterns, blocked items, next step

**Raw Messages** — verbatim human/assistant exchanges extracted from the session JSONL. Tool-only messages (no text content) are filtered out to save space. Thinking blocks are excluded.

## Requirements

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) CLI
- [Rust](https://rustup.rs) toolchain (for building from source)
- macOS or Linux

## How it finds the session

The binary looks in `~/.claude/projects/` for the most recently modified `.jsonl` file, walking up to 3 directories deep and excluding subagent sessions. You can also pass `--session /path/to/session.jsonl` explicitly.

## License

MIT
