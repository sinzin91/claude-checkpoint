use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Find the most recently modified `.jsonl` session file in the Claude projects directory.
/// Excludes subagent session files.
pub fn find_most_recent_session(session_dir: &Path) -> Result<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;

    for entry in WalkDir::new(session_dir)
        .max_depth(3)
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
        .ok_or_else(|| anyhow!("No session files found in {}", session_dir.display()))
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
}
