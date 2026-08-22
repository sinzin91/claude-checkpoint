//! End-to-end tests for `extract` session resolution.
//!
//! These cover the CLI contract that unit tests cannot reach: the exit code and
//! whether an output file is produced. The `--session-id` hard-error path lives
//! entirely in `main.rs`, and the silent fall-through for unsubstituted
//! placeholders is the regression risk for every existing `/checkpoint` user.
//!
//! `HOME` is redirected at a temp dir so the real `~/.claude/projects` is never
//! read. `dirs::home_dir()` honours `$HOME` on both Linux and macOS.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_claude-checkpoint");

/// One JSONL line the extractor will accept, so a successful run produces a file.
const SESSION_LINE: &str = r#"{"type":"user","message":{"role":"user","content":"hello"}}"#;

struct Fixture {
    home: tempfile::TempDir,
    cwd: PathBuf,
}

impl Fixture {
    /// A fake `$HOME` with one session under the project dir for `cwd`, plus a
    /// second, newer session in an unrelated project — the file a silent
    /// fallback would wrongly pick.
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let cwd = home.path().join("work/proj");
        fs::create_dir_all(&cwd).unwrap();

        let projects = home.path().join(".claude/projects");
        let mine = projects.join(mangle(&cwd));
        fs::create_dir_all(&mine).unwrap();
        fs::write(
            mine.join("11111111-1111-1111-1111-111111111111.jsonl"),
            SESSION_LINE,
        )
        .unwrap();

        let other = projects.join("-somewhere-else-entirely");
        fs::create_dir_all(&other).unwrap();
        fs::write(
            other.join("22222222-2222-2222-2222-222222222222.jsonl"),
            SESSION_LINE,
        )
        .unwrap();

        Fixture { home, cwd }
    }

    fn extract(&self, args: &[&str], output: &Path) -> Output {
        Command::new(BIN)
            .arg("extract")
            .args(args)
            .arg("--output")
            .arg(output)
            .current_dir(&self.cwd)
            .env("HOME", self.home.path())
            .output()
            .unwrap()
    }

    fn out(&self, name: &str) -> PathBuf {
        self.home.path().join(name)
    }
}

/// Mirrors `session::mangle_cwd` — the crate's is not visible from an
/// integration test, and hardcoding it here keeps the fixture explicit.
fn mangle(p: &Path) -> String {
    p.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[test]
fn valid_session_id_resolves_to_that_session() {
    let f = Fixture::new();
    let out = f.out("ok.md");
    let r = f.extract(
        &["--session-id", "11111111-1111-1111-1111-111111111111"],
        &out,
    );

    assert!(r.status.success(), "expected exit 0, got {:?}", r.status);
    assert!(out.is_file(), "expected a checkpoint file");
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("# Session: 11111111-1111-1111-1111-111111111111"),
        "expected the named session in stderr, got:\n{stderr}"
    );
}

#[test]
fn unresolvable_session_id_is_fatal_and_writes_nothing() {
    // The behaviour change: a valid ID that cannot be resolved must not degrade
    // into CWD-scoped or global most-recent. A wrong-but-plausible checkpoint is
    // discovered only on restore, when the context it was meant to preserve is
    // already gone.
    let f = Fixture::new();
    let out = f.out("should-not-exist.md");
    let r = f.extract(
        &["--session-id", "deadbeef-0000-0000-0000-000000000000"],
        &out,
    );

    assert!(!r.status.success(), "expected a non-zero exit");
    assert_eq!(r.status.code(), Some(1));
    assert!(!out.exists(), "no file may be written on the error path");

    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("not found"),
        "expected a not-found error, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("22222222"),
        "must not have touched the unrelated session, got:\n{stderr}"
    );
}

#[test]
fn unsubstituted_placeholder_falls_back_silently() {
    // If the harness did not expand `${CLAUDE_SESSION_ID}`, the literal string
    // arrives here. That is absence of an ID, not a failed lookup, so the
    // CWD-scoped fallback must still run and exit 0.
    let f = Fixture::new();
    let out = f.out("placeholder.md");
    let r = f.extract(&["--session-id", "${CLAUDE_SESSION_ID}"], &out);

    assert!(
        r.status.success(),
        "placeholder must not be fatal, got {:?}\n{}",
        r.status,
        String::from_utf8_lossy(&r.stderr)
    );
    assert!(out.is_file(), "expected the fallback to produce a file");
}

#[test]
fn empty_session_id_falls_back_silently() {
    let f = Fixture::new();
    let out = f.out("empty.md");
    let r = f.extract(&["--session-id", ""], &out);

    assert!(r.status.success(), "empty ID must not be fatal");
    assert!(out.is_file());
}

#[test]
fn no_session_id_uses_cwd_scoped_lookup() {
    let f = Fixture::new();
    let out = f.out("plain.md");
    let r = f.extract(&[], &out);

    assert!(r.status.success());
    assert!(out.is_file());
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("# Session: 11111111-1111-1111-1111-111111111111"),
        "CWD-scoped lookup should win over the newer unrelated session, got:\n{stderr}"
    );
}

#[test]
fn cwd_scoped_lookup_works_for_paths_with_spaces_and_punctuation() {
    // The `--session-id` path has a scan to fall back on; this one does not —
    // it relies on `mangle_cwd` being right. Get the mangling wrong and the
    // project dir never matches, so resolution drops to global most-recent and
    // silently returns an unrelated conversation. Underscores were one instance
    // of that; spaces and punctuation are the same bug.
    let home = tempfile::tempdir().unwrap();
    let cwd = home.path().join("work/My Project (old)/3_Resources");
    fs::create_dir_all(&cwd).unwrap();

    let projects = home.path().join(".claude/projects");
    let mine = projects.join(mangle(&cwd));
    fs::create_dir_all(&mine).unwrap();
    let id = "44444444-4444-4444-4444-444444444444";
    fs::write(mine.join(format!("{id}.jsonl")), SESSION_LINE).unwrap();

    // Newer, unrelated session — what a mangling miss would hand back.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let other = projects.join("-somewhere-else-entirely");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("55555555-5555-5555-5555-555555555555.jsonl"),
        SESSION_LINE,
    )
    .unwrap();

    let out = home.path().join("spaces.md");
    let r = Command::new(BIN)
        .args(["extract", "--output"])
        .arg(&out)
        .current_dir(&cwd)
        .env("HOME", home.path())
        .output()
        .unwrap();

    assert!(r.status.success());
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains(&format!("# Session: {id}")),
        "expected the session for this cwd, not the unrelated newer one:\n{stderr}"
    );
}

#[test]
fn session_id_resolves_when_the_project_dir_is_named_unexpectedly() {
    // The mangling is an implementation detail of another program. Even if our
    // idea of it is wrong, an exact ID must still resolve — this is the case
    // that was silently returning someone else's conversation.
    let home = tempfile::tempdir().unwrap();
    let cwd = home.path().join("work/3_Resources/wiki");
    fs::create_dir_all(&cwd).unwrap();

    let projects = home.path().join(".claude/projects");
    let odd = projects.join("a-name-we-would-never-compute");
    fs::create_dir_all(&odd).unwrap();
    let id = "33333333-3333-3333-3333-333333333333";
    fs::write(odd.join(format!("{id}.jsonl")), SESSION_LINE).unwrap();

    // Decoy: newer, and in a directory sorted before the real one. Without the
    // scan this is what the global fallback hands back, so the test would
    // otherwise pass for the wrong reason.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let decoy = projects.join("-aaa-somewhere-else");
    fs::create_dir_all(&decoy).unwrap();
    fs::write(
        decoy.join("66666666-6666-6666-6666-666666666666.jsonl"),
        SESSION_LINE,
    )
    .unwrap();

    let out = home.path().join("odd.md");
    let r = Command::new(BIN)
        .args(["extract", "--session-id", id, "--output"])
        .arg(&out)
        .current_dir(&cwd)
        .env("HOME", home.path())
        .output()
        .unwrap();

    assert!(
        r.status.success(),
        "expected the scan to find it, got {:?}\n{}",
        r.status,
        String::from_utf8_lossy(&r.stderr)
    );
    assert!(out.is_file());
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains(&format!("# Session: {id}")),
        "expected the named session, not the decoy:\n{stderr}"
    );
}
