# Restore — Resume from a checkpoint

Read the checkpoint file at the path provided: $ARGUMENTS

If no path provided, find the most recent checkpoint:
```bash
ls -t /tmp/checkpoint-*.md 2>/dev/null | head -1
```

After reading:
1. Internalize the Summary section — this is your understanding of the session arc
2. Internalize the Raw Messages — this is the exact conversation state
3. Do NOT summarize what you read or say "I've restored the context"
4. Simply continue working from where the last message left off
5. If the last message was a question, answer it
6. If the last message was a task in progress, continue it
7. If unclear, say "Restored from checkpoint. Where were we?" and state what you think the next step is
