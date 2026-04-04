#!/usr/bin/env bash
# Install claude-checkpoint into ~/.claude
# Usage: bash install.sh [--uninstall]

set -euo pipefail

CLAUDE_DIR="$HOME/.claude"
COMMANDS_DIR="$CLAUDE_DIR/commands"
SCRIPTS_DIR="$CLAUDE_DIR/scripts"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ "${1:-}" == "--uninstall" ]]; then
  echo "Removing claude-checkpoint..."
  rm -f "$COMMANDS_DIR/checkpoint.md"
  rm -f "$COMMANDS_DIR/restore.md"
  rm -f "$SCRIPTS_DIR/extract-messages.sh"
  echo "Done. Removed /checkpoint and /restore commands."
  exit 0
fi

# Ensure directories exist
mkdir -p "$COMMANDS_DIR" "$SCRIPTS_DIR"

# Copy files
cp "$SCRIPT_DIR/commands/checkpoint.md" "$COMMANDS_DIR/checkpoint.md"
cp "$SCRIPT_DIR/commands/restore.md" "$COMMANDS_DIR/restore.md"
cp "$SCRIPT_DIR/scripts/extract-messages.sh" "$SCRIPTS_DIR/extract-messages.sh"
chmod +x "$SCRIPTS_DIR/extract-messages.sh"

echo "Installed claude-checkpoint:"
echo "  /checkpoint [N]  — save last N exchanges (default: 100) before /clear"
echo "  /restore [path]  — resume from a checkpoint file"
echo ""
echo "Usage:"
echo "  1. /checkpoint        — saves context to /tmp/checkpoint-*.md"
echo "  2. /clear             — wipe context window"
echo "  3. /restore           — pick up where you left off"
