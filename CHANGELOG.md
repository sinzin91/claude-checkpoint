# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Underscores in project paths broke exact session lookup**: `mangle_cwd` replaced `/` and `.` with `-` but not `_`, while Claude Code replaces all three. For any project under a path containing an underscore (e.g. `~/3_Resources/wiki`, `~/1_Projects/app`) the computed directory never exists, so `--session-id` missed **100% of the time** — not intermittently — and fell through to the mtime-based fallbacks. This defeated the guarantee 0.2.1 was released for.
- **`--session-id` no longer degrades into a different session**: a valid session ID that cannot be resolved is now a hard error (exit 1) instead of a silent fallback to CWD-scoped or global most-recent. Checkpointing the wrong session produces a plausible-looking file containing an unrelated conversation, discovered only on restore — strictly worse than producing nothing.
- **Lookup no longer depends on the mangling being right**: if the ancestor walk misses, `find_session_by_id` scans the project directories for `<id>.jsonl` directly. Session IDs are globally unique, so this is exact rather than heuristic, and it keeps resolution working if Claude Code's naming convention changes again.

## [0.2.1] - 2026-04-27

### Fixed

- **Deterministic session resolution via `${CLAUDE_SESSION_ID}`**: `/checkpoint` now passes the calling Claude Code instance's session ID through to the binary, which looks up `<projects>/<mangled-cwd>/<id>.jsonl` exactly. This eliminates the race condition when multiple Claude Code instances run in the same project dir — each one now extracts *its own* session, regardless of which `.jsonl` was most recently touched ([#6](https://github.com/sinzin91/claude-checkpoint/pull/6))
- **CWD-scoped fallback**: When no session ID is provided, `extract` resolves to the project dir for the current working directory via Claude Code's mangling convention (`/` and `.` → `-`) — preventing a brief session in another project from silently shadowing your working session ([#4](https://github.com/sinzin91/claude-checkpoint/pull/4))
- **Subdirectory execution**: Running from a subdir of a project (e.g. `~/Projects/foo/src`) walks up the parent chain to find the matching project dir instead of falling back to global lookup
- **Visible source path**: Stderr now prints the resolved session path so wrong-session captures are immediately obvious

### Added

- `--session-id <id>` flag on `claude-checkpoint extract` for explicit session pinning. Validates UUID-shape input (hex + dashes), rejecting path-traversal attempts and unsubstituted shell placeholders.

### Upgrade notes

- **Re-run `claude-checkpoint install`** after upgrading to refresh the `/checkpoint` slash command. The fix only takes effect once the new template (which passes `${CLAUDE_SESSION_ID}` through to the binary) is in `~/.claude/commands/checkpoint.md`. Binary-only upgrades will keep the old command file and won't get the deterministic resolution.

## [0.2.0] - 2026-04-09

### Added

- **Rust extraction binary**: Replaces bash+jq dependency with a native Rust binary for reliability and performance
- **Session discovery**: Automatic detection of the most recent JSONL session file
- **Subagent filtering**: Excludes subagent sessions from discovery
- **Structured checkpoints**: Markdown output with summary template and raw messages
- **CI/CD**: GitHub Actions for testing, linting, release builds, and secret scanning
- **Cross-platform releases**: Pre-built binaries for Linux (x86_64, aarch64) and macOS (x86_64, aarch64)

## [0.1.0] - 2026-04-03

### Added

- **Checkpoint command**: `/checkpoint` slash command for saving session context
- **Restore command**: `/restore` slash command for resuming from checkpoints
- **Bash extraction**: Initial implementation using bash and jq
- **Install script**: One-command setup with `install.sh`

[0.2.1]: https://github.com/sinzin91/claude-checkpoint/releases/tag/v0.2.1
[0.2.0]: https://github.com/sinzin91/claude-checkpoint/releases/tag/v0.2.0
[0.1.0]: https://github.com/sinzin91/claude-checkpoint/releases/tag/v0.1.0
