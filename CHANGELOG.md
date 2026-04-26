# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-04-27

### Fixed

- **CWD-scoped session lookup**: `extract` (and therefore `/checkpoint`) no longer picks the globally most-recent `.jsonl` across all projects. It now resolves to the project dir for the current working directory via Claude Code's mangling convention (`/` and `.` → `-`), so a brief session in another project can no longer silently shadow your active working session ([#4](https://github.com/sinzin91/claude-checkpoint/pull/4))
- **Subdirectory execution**: Running from a subdir of a project (e.g. `~/Projects/foo/src`) walks up the parent chain to find the matching project dir instead of falling back to global lookup
- **Visible source path**: Stderr now prints the resolved session path so wrong-session captures are immediately obvious

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
