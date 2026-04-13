---
name: checkpoint
description: Save and restore Claude Code session context. Use /checkpoint before /clear to preserve decisions, corrections, and working state. Use /restore to resume.
---

# claude-checkpoint

## Overview

Checkpoint and restore for Claude Code sessions. Captures conversation history into a structured markdown file so you can `/clear` safely and resume later.

**Announce at start:** "I'm saving a checkpoint of this session."

## When to Use

- Before running `/clear` to free up context
- When hitting context limits and need to preserve state
- To create a restore point before risky operations

## Prerequisites

The `claude-checkpoint` binary must be installed:

```bash
# Via cargo
cargo install claude-checkpoint

# Or from source
git clone https://github.com/sinzin91/claude-checkpoint
cd claude-checkpoint && cargo build --release
```

## Commands

### /checkpoint [N]

Save the current session. N = number of messages to preserve (default: 100).

**Steps:**

1. Run the extract binary:
   ```bash
   claude-checkpoint extract --last ${N:-100} --output /tmp/checkpoint-$(date +%Y%m%d-%H%M%S).md
   ```

2. Read the checkpoint file that was created.

3. Fill in the Summary section by replacing `[PENDING — Claude generates this]` with:
   - **Goal:** What was the user trying to accomplish?
   - **Current task:** What were we actively working on?
   - **Files modified:** List every file touched with one-line description
   - **Key decisions:** Decisions/constraints established during the session
   - **Corrections made:** Anything the user corrected (these get lost first in compaction)
   - **Working patterns:** Conventions, styles, or approaches agreed upon
   - **Blocked/pending:** What was waiting or not yet started
   - **Next step:** The literal next thing that should happen

4. Write the completed file back to the same path.

5. Tell the user the checkpoint path, message count, and that they can `/clear` safely.

### /restore [path]

Resume from a checkpoint.

If no path provided, find the most recent:
```bash
ls -t /tmp/checkpoint-*.md 2>/dev/null | head -1
```

After reading the checkpoint:
1. Internalize the Summary — this is your understanding of the session arc
2. Internalize the Raw Messages — this is the exact conversation state
3. Do NOT summarize or acknowledge the restoration
4. Continue working from where the last message left off
5. If unclear, say "Restored from checkpoint. Where were we?" and state the next step

## Output Format

Checkpoint files are saved to `/tmp/checkpoint-YYYYMMDD-HHMMSS.md` with:
- Structured summary (filled by Claude)
- Raw human/assistant messages (extracted from session JSONL)
- Restoration instructions
