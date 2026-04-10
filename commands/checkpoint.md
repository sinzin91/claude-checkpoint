# Checkpoint — Save context before /clear

You are creating a session checkpoint so the user can /clear and restore context.

## Steps

1. **Run the extract script** to capture raw messages:
   ```bash
   ~/.claude/bin/claude-checkpoint extract --last $ARGUMENTS --output /tmp/checkpoint-$(date +%Y%m%d-%H%M%S).md
   ```
   If no argument provided, default to 100 messages.

2. **Read the checkpoint file** that was just created.

3. **Fill in the Summary section** by replacing `[PENDING — Claude generates this]` with a structured summary covering:

   ### Session Summary
   - **Goal:** What was the user trying to accomplish?
   - **Current task:** What were we actively working on when checkpointed?
   - **Files modified:** List every file touched with one-line description of changes
   - **Key decisions:** Bullet list of decisions/constraints established during the session
   - **Corrections made:** Anything the user corrected or refined (these get lost first in compaction)
   - **Working patterns:** Any conventions, styles, or approaches agreed upon
   - **Blocked/pending:** What was waiting on something or not yet started
   - **Next step:** The literal next thing that should happen

4. **Write the completed file** back to the same path.

5. **Tell the user:**
   - The checkpoint path
   - How many messages were captured
   - That they can now `/clear` safely
   - To restore: `/restore <path>` or just say `read <path> and continue where we left off`
