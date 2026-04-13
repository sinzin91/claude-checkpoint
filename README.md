# claude-checkpoint

<p align="center">
  <img src="assets/header.jpg" alt="claude-checkpoint" width="600">
</p>

[![CI](https://github.com/sinzin91/claude-checkpoint/actions/workflows/ci.yml/badge.svg)](https://github.com/sinzin91/claude-checkpoint/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/claude-checkpoint.svg)](https://crates.io/crates/claude-checkpoint)
[![GitHub Release](https://img.shields.io/github/v/release/sinzin91/claude-checkpoint)](https://github.com/sinzin91/claude-checkpoint/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Save your Claude Code session before `/clear`. Restore it after.

## Why

`/clear` nukes everything. All the decisions, corrections, file context, and working state — gone. You need to clear because you hit context limits, but then you spend the next ten minutes re-explaining what you were doing.

`claude-checkpoint` fixes this. Before you clear, run `/checkpoint`. It extracts your conversation into a markdown file and has Claude write a structured summary. After clearing, run `/restore` and Claude picks up exactly where you left off.

## Quick start

```
/checkpoint        # saves last 100 messages
/clear             # wipe context as usual
/restore           # Claude continues like nothing happened
```

That's it. Three commands.

## Install

**For Claude Code users** — paste this into any session:

```
Install https://github.com/sinzin91/claude-checkpoint and set up the /checkpoint and /restore commands.
```

Claude will handle the rest.

### Manual install

```bash
cargo install claude-checkpoint
claude-checkpoint install
```

Or as a Claude Code plugin:

```bash
claude plugin marketplace add sinzin91/claude-checkpoint
claude plugin install claude-checkpoint
```

### From source

```bash
git clone https://github.com/sinzin91/claude-checkpoint.git
cd claude-checkpoint
bash install.sh
```

### Pre-built binaries

Grab a tarball from [GitHub Releases](https://github.com/sinzin91/claude-checkpoint/releases) — Linux and macOS, x86_64 and ARM.

### Uninstall

```bash
claude-checkpoint uninstall        # removes slash commands from ~/.claude
cargo uninstall claude-checkpoint  # removes the binary (if installed via cargo)
```

## Usage

```
/checkpoint        # save last 100 messages (default)
/checkpoint 50     # or pick a number
```

Claude extracts the raw messages, writes a summary (goal, decisions, files touched, corrections, next step), and saves it all to `/tmp/checkpoint-YYYYMMDD-HHMMSS.md`.

```
/restore                                     # most recent checkpoint
/restore /tmp/checkpoint-20260403-161507.md   # specific file
```

Claude reads it back and continues working. No preamble, no "welcome back", just picks up the thread.

### Standalone CLI

You can also run the binary directly outside of Claude Code:

```bash
claude-checkpoint extract --last 100 --output /tmp/checkpoint.md
claude-checkpoint extract --session ~/.claude/projects/-Users-me/abc123.jsonl
```

## What gets saved

Two sections in the checkpoint file:

**Summary** — Claude generates this during `/checkpoint`: the goal, current task, files modified, key decisions, corrections you made, working patterns, blockers, and the literal next step. These are the things that get lost first when context compacts.

**Raw messages** — the actual human/assistant exchanges pulled from the session JSONL. Tool calls and thinking blocks are stripped out to keep it lean.

## How it finds the session

The binary walks `~/.claude/projects/` looking for the most recently modified `.jsonl` file (max depth 3, skips subagent sessions). Pass `--session` to point it at a specific file instead.

## Requirements

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- macOS or Linux
- [Rust](https://rustup.rs) (only if building from source)

## License

MIT
