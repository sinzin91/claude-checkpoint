#!/usr/bin/env bash
# Extract the last N human/assistant messages from the current Claude Code session.
# Usage: extract-messages.sh [--last N] [--output PATH] [--session PATH]

set -euo pipefail

LAST=${LAST:-100}
OUTPUT=""
SESSION_FILE=""
SESSION_DIR="$HOME/.claude/projects"

# Parse args
while [[ $# -gt 0 ]]; do
  case $1 in
    --last) LAST="$2"; shift 2 ;;
    --output) OUTPUT="$2"; shift 2 ;;
    --session) SESSION_FILE="$2"; shift 2 ;;
    *) shift ;;
  esac
done

# Find session file if not provided
if [[ -z "$SESSION_FILE" ]]; then
  # macOS uses -f '%m %N', Linux uses -c '%Y %n' for stat format
  if stat -f '%m' /dev/null &>/dev/null; then
    STAT_FMT=(-f '%m %N')
  else
    STAT_FMT=(-c '%Y %n')
  fi

  SESSION_FILE=$(find "$SESSION_DIR" -maxdepth 3 -name "*.jsonl" -not -path "*/subagents/*" -type f -print0 2>/dev/null \
    | xargs -0 stat "${STAT_FMT[@]}" 2>/dev/null \
    | sort -rn | head -1 | cut -d' ' -f2-)
fi

if [[ -z "$SESSION_FILE" ]] || [[ ! -f "$SESSION_FILE" ]]; then
  echo "ERROR: No session files found in $SESSION_DIR" >&2
  exit 1
fi

echo "# Session: $(basename "$SESSION_FILE" .jsonl)" >&2
echo "# Source: $SESSION_FILE" >&2
echo "# Size: $(du -h "$SESSION_FILE" | cut -f1)" >&2

# Count total messages
TOTAL_USER=$(jq -c 'select(.type == "user")' "$SESSION_FILE" | wc -l | tr -d ' ')
TOTAL_ASSISTANT=$(jq -c 'select(.type == "assistant")' "$SESSION_FILE" | wc -l | tr -d ' ')
echo "# Total messages: $TOTAL_USER user + $TOTAL_ASSISTANT assistant" >&2

# Extract last N user+assistant messages that contain actual text content.
# Skip assistant messages that are purely tool calls (no text) — they waste
# message budget without carrying recoverable context.
MESSAGES=$(jq -r '
  select(.type == "user" or .type == "assistant")
  | .type as $type
  | (
      [.message.content[] | select(.type == "text") | .text]
    ) as $texts
  | if ($texts | length) == 0 then empty
    elif $type == "user" then
      "---\n\n## Human\n\n" + (
        [.message.content[]
         | if .type == "text" then .text
           elif .type == "image" then "[Image attached]"
           else empty
           end
        ] | join("\n")
      )
    elif $type == "assistant" then
      "---\n\n## Assistant\n\n" + ($texts | join("\n"))
    else empty
    end
' "$SESSION_FILE" 2>/dev/null | tail -20000)

# Generate output
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
OUT_FILE="${OUTPUT:-/tmp/checkpoint-$(date +%Y%m%d-%H%M%S).md}"

cat > "$OUT_FILE" << HEADER
# Context Checkpoint
- **Created:** $TIMESTAMP
- **Session:** $(basename "$SESSION_FILE" .jsonl)
- **Messages preserved:** last ~$LAST exchanges
- **Source:** $SESSION_FILE

> The SUMMARY section is a structured overview. The RAW MESSAGES section
> contains verbatim conversation history. Both are needed for full restoration.

---

## Summary

<!-- Claude will fill this section during /checkpoint -->
[PENDING — Claude generates this]

---

## Raw Messages (Last $LAST)

$MESSAGES

---

## Restoration Instructions

When restoring from this checkpoint:
1. Read this entire file
2. The SUMMARY gives you the arc — decisions, files, patterns, what was being worked on
3. The RAW MESSAGES give you the exact state — continue from the last message as if the conversation never stopped
4. Do NOT re-summarize or acknowledge the restoration. Just pick up where we left off.
HEADER

echo "$OUT_FILE"
