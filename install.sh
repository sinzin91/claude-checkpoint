#!/usr/bin/env bash
# Install claude-checkpoint into ~/.claude
# Usage: bash install.sh [--uninstall]

set -euo pipefail

CLAUDE_DIR="$HOME/.claude"
COMMANDS_DIR="$CLAUDE_DIR/commands"
BIN_DIR="$CLAUDE_DIR/bin"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ "${1:-}" == "--uninstall" ]]; then
  echo "Removing claude-checkpoint..."
  rm -f "$COMMANDS_DIR/checkpoint.md"
  rm -f "$COMMANDS_DIR/restore.md"
  rm -f "$BIN_DIR/claude-checkpoint"
  rm -f "$CLAUDE_DIR/scripts/extract-messages.sh"  # legacy cleanup
  echo "Done. Removed /checkpoint and /restore commands."
  exit 0
fi

mkdir -p "$COMMANDS_DIR" "$BIN_DIR"

# Build binary (always rebuild if cargo available to avoid stale binaries)
if command -v cargo &>/dev/null; then
  echo "Building claude-checkpoint..."
  (cd "$SCRIPT_DIR" && cargo build --release)
  cp "$SCRIPT_DIR/target/release/claude-checkpoint" "$BIN_DIR/"
elif [[ -f "$SCRIPT_DIR/target/release/claude-checkpoint" ]]; then
  echo "Using pre-built binary (cargo not found, skipping rebuild)..."
  cp "$SCRIPT_DIR/target/release/claude-checkpoint" "$BIN_DIR/"
else
  echo "ERROR: cargo not found and no pre-built binary available." >&2
  echo "" >&2
  echo "Options:" >&2
  echo "  1. Install Rust: https://rustup.rs" >&2
  echo "  2. cargo install --git https://github.com/sinzin91/claude-checkpoint" >&2
  exit 1
fi

chmod +x "$BIN_DIR/claude-checkpoint"

# Copy slash commands
cp "$SCRIPT_DIR/commands/checkpoint.md" "$COMMANDS_DIR/"
cp "$SCRIPT_DIR/commands/restore.md" "$COMMANDS_DIR/"

# Clean up legacy bash script if present
rm -f "$CLAUDE_DIR/scripts/extract-messages.sh"

echo ""
echo "Installed claude-checkpoint:"
echo "  /checkpoint [N]  — save last N messages (default: 100) before /clear"
echo "  /restore [path]  — resume from a checkpoint file"
echo ""
echo "Note: ensure ~/.claude/bin is on your PATH, or use 'cargo install claude-checkpoint' instead."
echo ""
echo "Usage:"
echo "  1. /checkpoint        — saves context to /tmp/checkpoint-*.md"
echo "  2. /clear             — wipe context window"
echo "  3. /restore           — pick up where you left off"
