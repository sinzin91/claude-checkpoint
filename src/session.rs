use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Find the most recently modified `.jsonl` session file in the Claude projects directory.
/// Excludes subagent session files.
pub fn find_most_recent_session(session_dir: &Path) -> Result<PathBuf> {
    most_recent_jsonl(session_dir, 3)
        .ok_or_else(|| anyhow!("No session files found in {}", session_dir.display()))
}

/// Mangle an absolute path into the directory name Claude Code uses under
/// `~/.claude/projects/`. The convention replaces `/` and `.` with `-`.
///
/// Example: `/Users/tz/Projects/foo` → `-Users-tz-Projects-foo`
pub fn mangle_cwd(cwd: &Path) -> String {
    let s = cwd.to_string_lossy();
    s.chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}

/// Find the most recent session for the project corresponding to `cwd`.
/// Returns `Ok(None)` if the project dir does not exist or has no sessions —
/// callers should fall back to the global lookup. Returns `Err` only on
/// unexpected I/O failures.
pub fn find_session_for_cwd(session_dir: &Path, cwd: &Path) -> Result<Option<PathBuf>> {
    let project_dir = session_dir.join(mangle_cwd(cwd));
    if !project_dir.is_dir() {
        return Ok(None);
    }
    Ok(most_recent_jsonl(&project_dir, 1))
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
    fn test_find_session_for_cwd_returns_none_when_project_dir_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = Path::new("/Users/tz/Projects/empty");
        fs::create_dir_all(dir.path().join(mangle_cwd(cwd))).unwrap();
        let result = find_session_for_cwd(dir.path(), cwd).unwrap();
        assert!(result.is_none());
    }
}
