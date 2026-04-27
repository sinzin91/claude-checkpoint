use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Walk depth for the global fallback lookup — covers `<projects>/<project>/<file>.jsonl`
/// with headroom for legacy nesting.
const GLOBAL_SEARCH_DEPTH: usize = 3;

/// Walk depth for a single project dir — sessions live directly inside.
const PROJECT_SEARCH_DEPTH: usize = 1;

/// Find the most recently modified `.jsonl` session file in the Claude projects directory.
/// Excludes subagent session files.
pub fn find_most_recent_session(session_dir: &Path) -> Result<PathBuf> {
    most_recent_jsonl(session_dir, GLOBAL_SEARCH_DEPTH)
        .ok_or_else(|| anyhow!("No session files found in {}", session_dir.display()))
}

/// Mangle an absolute path into the directory name Claude Code uses under
/// `~/.claude/projects/`. The convention replaces `/` and `.` with `-`.
///
/// Example: `/Users/tz/Projects/foo` → `-Users-tz-Projects-foo`
///
/// Unix-only: assumes `/`-separated paths. On Windows this will not match
/// Claude Code's mangling and `find_session_for_cwd` will fall back.
pub fn mangle_cwd(cwd: &Path) -> String {
    let s = cwd.to_string_lossy();
    s.chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}

/// Returns true if the string looks like a Claude Code session ID — a UUID-style
/// token of hex digits and dashes. Rejects empty strings, path separators
/// (`/`, `\`), `..`, and unsubstituted shell placeholders like
/// `${CLAUDE_SESSION_ID}`. Used to fail closed on malformed or hostile inputs
/// before they reach the filesystem.
fn is_valid_session_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Find a session by exact ID, walking up from `cwd` through parent dirs.
///
/// Each Claude Code session writes to `<projects>/<mangled-cwd>/<session-id>.jsonl`.
/// This is the only deterministic way to identify the *current* session when
/// multiple Claude Code instances run in the same project — mtime ordering
/// is racy in that scenario.
///
/// Returns `Ok(None)` if `session_id` is empty, fails validation
/// (e.g. unsubstituted `${CLAUDE_SESSION_ID}` placeholder, path separators),
/// or no `<id>.jsonl` exists under any ancestor's project dir — callers
/// can fall back to mtime-based lookup.
pub fn find_session_by_id(
    session_dir: &Path,
    cwd: &Path,
    session_id: &str,
) -> Result<Option<PathBuf>> {
    if !is_valid_session_id(session_id) {
        return Ok(None);
    }
    let filename = format!("{session_id}.jsonl");
    let mut current = Some(cwd);
    while let Some(dir) = current {
        let candidate = session_dir.join(mangle_cwd(dir)).join(&filename);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
        current = dir.parent();
    }
    Ok(None)
}

/// Find the most recent session for the project corresponding to `cwd`.
///
/// Walks up the directory tree from `cwd` so running from a subdirectory
/// (e.g. `~/Projects/foo/src`) still resolves to the project session in
/// `-Users-tz-Projects-foo/`. Returns `Ok(None)` if no ancestor maps to an
/// existing project dir with sessions — callers should fall back to the
/// global lookup.
///
/// Best-effort: I/O failures surfaced by walkdir during traversal are
/// swallowed and treated as "no session here," consistent with the
/// fallback contract.
pub fn find_session_for_cwd(session_dir: &Path, cwd: &Path) -> Result<Option<PathBuf>> {
    let mut current = Some(cwd);
    while let Some(dir) = current {
        let project_dir = session_dir.join(mangle_cwd(dir));
        if project_dir.is_dir() {
            if let Some(session) = most_recent_jsonl(&project_dir, PROJECT_SEARCH_DEPTH) {
                return Ok(Some(session));
            }
        }
        current = dir.parent();
    }
    Ok(None)
}

fn most_recent_jsonl(root: &Path, max_depth: usize) -> Option<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;

    for entry in WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip subagent sessions
        if path.components().any(|c| c.as_os_str() == "subagents") {
            continue;
        }

        if path.extension().is_some_and(|e| e == "jsonl") && path.is_file() {
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    if best.as_ref().is_none_or(|(t, _)| modified > *t) {
                        best = Some((modified, path.to_path_buf()));
                    }
                }
            }
        }
    }

    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_finds_most_recent_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        fs::create_dir_all(&proj).unwrap();

        // Create older file
        let old = proj.join("old.jsonl");
        fs::write(&old, "{}").unwrap();

        thread::sleep(Duration::from_millis(50));

        // Create newer file
        let new = proj.join("new.jsonl");
        fs::write(&new, "{}").unwrap();

        let result = find_most_recent_session(dir.path()).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_skips_subagent_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("proj/subagents");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("agent.jsonl"), "{}").unwrap();

        let proj = dir.path().join("proj");
        let main = proj.join("main.jsonl");
        fs::write(&main, "{}").unwrap();

        let result = find_most_recent_session(dir.path()).unwrap();
        assert_eq!(result, main);
    }

    #[test]
    fn test_no_sessions_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_most_recent_session(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_mangle_cwd_replaces_slashes_and_dots() {
        assert_eq!(
            mangle_cwd(Path::new("/Users/tz/Projects/foo")),
            "-Users-tz-Projects-foo"
        );
        assert_eq!(
            mangle_cwd(Path::new("/Users/tz/.claude")),
            "-Users-tz--claude"
        );
        assert_eq!(
            mangle_cwd(Path::new("/Users/tz/Projects/claude-checkpoint")),
            "-Users-tz-Projects-claude-checkpoint"
        );
    }

    #[test]
    fn test_find_session_for_cwd_picks_local_over_global() {
        // Repro the bug: project A has older session, project B has newer one.
        // CWD-scoped lookup must return A's session, not B's.
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path();

        let cwd_a = Path::new("/Users/tz/Projects/foo");
        let project_a = projects.join(mangle_cwd(cwd_a));
        fs::create_dir_all(&project_a).unwrap();
        let session_a = project_a.join("a.jsonl");
        fs::write(&session_a, "{}").unwrap();

        thread::sleep(Duration::from_millis(50));

        let project_b = projects.join("-Users-tz-Projects-bar");
        fs::create_dir_all(&project_b).unwrap();
        fs::write(project_b.join("b.jsonl"), "{}").unwrap();

        // Global lookup picks B (most recent globally).
        assert_eq!(
            find_most_recent_session(projects).unwrap(),
            project_b.join("b.jsonl")
        );

        // CWD-scoped lookup picks A (most recent in A's project dir).
        assert_eq!(
            find_session_for_cwd(projects, cwd_a).unwrap(),
            Some(session_a)
        );
    }

    #[test]
    fn test_find_session_for_cwd_returns_none_for_unknown_project() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = Path::new("/Users/tz/Projects/nonexistent");
        let result = find_session_for_cwd(dir.path(), cwd).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_session_by_id_picks_exact_match_over_mtime() {
        // Two sessions in the same project dir — older one is the "current"
        // session per its ID. Mtime would pick the newer; ID lookup must pick
        // the older one.
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path();
        let cwd = Path::new("/Users/tz/Projects/foo");
        let project_dir = projects.join(mangle_cwd(cwd));
        fs::create_dir_all(&project_dir).unwrap();

        let target_id = "9cce9c9f-b5bc-4a3c-a5ad-48926e45eccb";
        let target = project_dir.join(format!("{target_id}.jsonl"));
        fs::write(&target, "{}").unwrap();

        thread::sleep(Duration::from_millis(50));

        // Newer session that mtime would prefer.
        let newer = project_dir.join("0582224a-aaaa-aaaa-aaaa-aaaaaaaaaaaa.jsonl");
        fs::write(&newer, "{}").unwrap();

        // Mtime path picks newer.
        assert_eq!(find_session_for_cwd(projects, cwd).unwrap(), Some(newer));

        // ID path picks the exact match.
        assert_eq!(
            find_session_by_id(projects, cwd, target_id).unwrap(),
            Some(target)
        );
    }

    #[test]
    fn test_find_session_by_id_walks_up_from_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path();
        let project_root = Path::new("/Users/tz/Projects/foo");
        let project_dir = projects.join(mangle_cwd(project_root));
        fs::create_dir_all(&project_dir).unwrap();

        let id = "deadbeef-dead-beef-dead-beefdeadbeef";
        let session = project_dir.join(format!("{id}.jsonl"));
        fs::write(&session, "{}").unwrap();

        let nested = Path::new("/Users/tz/Projects/foo/src/nested");
        assert_eq!(
            find_session_by_id(projects, nested, id).unwrap(),
            Some(session)
        );
    }

    #[test]
    fn test_find_session_by_id_returns_none_when_id_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path();
        let cwd = Path::new("/Users/tz/Projects/foo");
        let project_dir = projects.join(mangle_cwd(cwd));
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join("real.jsonl"), "{}").unwrap();

        let result = find_session_by_id(projects, cwd, "nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_session_by_id_empty_id_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = Path::new("/Users/tz/Projects/foo");
        let result = find_session_by_id(dir.path(), cwd, "").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_session_by_id_rejects_unsubstituted_placeholder() {
        // If Claude Code didn't expand the placeholder, the literal string
        // arrives here. Must be treated as absent, not crashed on or used as
        // a filename.
        let dir = tempfile::tempdir().unwrap();
        let cwd = Path::new("/Users/tz/Projects/foo");
        let result = find_session_by_id(dir.path(), cwd, "${CLAUDE_SESSION_ID}").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_session_by_id_rejects_path_traversal() {
        // A hostile or malformed ID containing `..` or `/` must not be
        // turned into a path component. Returns None so the caller falls back.
        let dir = tempfile::tempdir().unwrap();
        let cwd = Path::new("/Users/tz/Projects/foo");
        for bad in ["../etc/passwd", "..", "foo/bar", "a/b", "weird name"] {
            let result = find_session_by_id(dir.path(), cwd, bad).unwrap();
            assert!(result.is_none(), "expected None for {bad:?}");
        }
    }

    #[test]
    fn test_is_valid_session_id_accepts_uuid_shape_only() {
        // Real Claude Code session IDs.
        assert!(is_valid_session_id("9cce9c9f-b5bc-4a3c-a5ad-48926e45eccb"));
        assert!(is_valid_session_id("deadbeef"));
        // Reject everything else.
        assert!(!is_valid_session_id(""));
        assert!(!is_valid_session_id("${CLAUDE_SESSION_ID}"));
        assert!(!is_valid_session_id("../etc"));
        assert!(!is_valid_session_id("session_with_underscore"));
        assert!(!is_valid_session_id("not a uuid"));
    }

    #[test]
    fn test_find_session_for_cwd_walks_up_from_subdirectory() {
        // Running from ~/Projects/foo/src should still find the session
        // for ~/Projects/foo, not fall back to global.
        let dir = tempfile::tempdir().unwrap();
        let projects = dir.path();

        let project_root = Path::new("/Users/tz/Projects/foo");
        let project_dir = projects.join(mangle_cwd(project_root));
        fs::create_dir_all(&project_dir).unwrap();
        let session = project_dir.join("foo.jsonl");
        fs::write(&session, "{}").unwrap();

        // Caller is two levels deep into the project.
        let nested_cwd = Path::new("/Users/tz/Projects/foo/src/nested");
        assert_eq!(
            find_session_for_cwd(projects, nested_cwd).unwrap(),
            Some(session)
        );
    }

    #[test]
    fn test_find_session_for_cwd_returns_none_when_project_dir_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = Path::new("/Users/tz/Projects/empty");
        fs::create_dir_all(dir.path().join(mangle_cwd(cwd))).unwrap();
        let result = find_session_for_cwd(dir.path(), cwd).unwrap();
        assert!(result.is_none());
    }
}
