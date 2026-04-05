use anyhow::Result;
use chrono::Utc;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::types::{ExtractedMessage, Role, SessionLine};

/// Stats about the extraction, printed to stderr.
pub struct ExtractionStats {
    pub session_name: String,
    pub source_path: String,
    pub file_size: u64,
    pub total_user: usize,
    pub total_assistant: usize,
    pub extracted: usize,
}

/// Parse a JSONL session file and extract the last `last_n` human/assistant messages
/// that contain displayable text content.
pub fn extract_messages(session_path: &Path, last_n: usize) -> Result<(Vec<ExtractedMessage>, ExtractionStats)> {
    let file = File::open(session_path)?;
    let file_size = file.metadata()?.len();
    let reader = BufReader::new(file);

    let mut messages = Vec::new();
    let mut total_user = 0usize;
    let mut total_assistant = 0usize;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        let parsed: SessionLine = match serde_json::from_str(&line) {
            Ok(p) => p,
            Err(_) => continue, // skip malformed lines
        };

        match parsed {
            SessionLine::User(msg) => {
                total_user += 1;
                if let Some(text) = msg.message.content.extract_text() {
                    messages.push(ExtractedMessage {
                        role: Role::Human,
                        text,
                    });
                }
            }
            SessionLine::Assistant(msg) => {
                total_assistant += 1;
                if let Some(text) = msg.message.content.extract_text() {
                    messages.push(ExtractedMessage {
                        role: Role::Assistant,
                        text,
                    });
                }
            }
            SessionLine::Other => {}
        }
    }

    // Take the last N messages
    let extracted_count = messages.len().min(last_n);
    if messages.len() > last_n {
        messages = messages.split_off(messages.len() - last_n);
    }

    let session_name = session_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let source_path = session_path.to_string_lossy().to_string();

    let stats = ExtractionStats {
        session_name,
        source_path,
        file_size,
        total_user,
        total_assistant,
        extracted: extracted_count,
    };

    Ok((messages, stats))
}

/// Render extracted messages into the checkpoint markdown format.
pub fn render_checkpoint(messages: &[ExtractedMessage], stats: &ExtractionStats, last_n: usize) -> String {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "# Context Checkpoint\n\
         - **Created:** {timestamp}\n\
         - **Session:** {session}\n\
         - **Messages preserved:** last ~{last_n} exchanges\n\
         - **Source:** {source}\n\
         \n\
         > The SUMMARY section is a structured overview. The RAW MESSAGES section\n\
         > contains verbatim conversation history. Both are needed for full restoration.\n\
         \n\
         ---\n\
         \n\
         ## Summary\n\
         \n\
         <!-- Claude will fill this section during /checkpoint -->\n\
         [PENDING — Claude generates this]\n\
         \n\
         ---\n\
         \n\
         ## Raw Messages (Last {last_n})\n\n",
        timestamp = timestamp,
        session = stats.session_name,
        last_n = last_n,
        source = stats.source_path,
    ));

    // Messages
    for msg in messages {
        let role_header = match msg.role {
            Role::Human => "## Human",
            Role::Assistant => "## Assistant",
        };
        out.push_str(&format!("---\n\n{role_header}\n\n{text}\n", text = msg.text));
    }

    // Footer
    out.push_str(
        "\n---\n\n\
         ## Restoration Instructions\n\
         \n\
         When restoring from this checkpoint:\n\
         1. Read this entire file\n\
         2. The SUMMARY gives you the arc — decisions, files, patterns, what was being worked on\n\
         3. The RAW MESSAGES give you the exact state — continue from the last message as if the conversation never stopped\n\
         4. Do NOT re-summarize or acknowledge the restoration. Just pick up where we left off.\n",
    );

    out
}

/// Write checkpoint content to a file.
pub fn write_checkpoint(content: &str, output_path: &Path) -> Result<()> {
    let mut file = File::create(output_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_session_file(lines: &[&str]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_extract_basic_messages() {
        let f = make_session_file(&[
            r#"{"type":"user","message":{"content":"hello"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hi there"}]}}"#,
        ]);

        let (msgs, stats) = extract_messages(f.path(), 100).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::Human);
        assert_eq!(msgs[0].text, "hello");
        assert_eq!(msgs[1].role, Role::Assistant);
        assert_eq!(msgs[1].text, "hi there");
        assert_eq!(stats.total_user, 1);
        assert_eq!(stats.total_assistant, 1);
    }

    #[test]
    fn test_tool_only_messages_filtered() {
        let f = make_session_file(&[
            r#"{"type":"user","message":{"content":"hello"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use"}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"done"}]}}"#,
        ]);

        let (msgs, stats) = extract_messages(f.path(), 100).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].text, "hello");
        assert_eq!(msgs[1].text, "done");
        assert_eq!(stats.total_user, 2);
        assert_eq!(stats.total_assistant, 2);
    }

    #[test]
    fn test_last_n_truncation() {
        let f = make_session_file(&[
            r#"{"type":"user","message":{"content":"first"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"one"}]}}"#,
            r#"{"type":"user","message":{"content":"second"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"two"}]}}"#,
            r#"{"type":"user","message":{"content":"third"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"three"}]}}"#,
        ]);

        let (msgs, _) = extract_messages(f.path(), 2).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].text, "third");
        assert_eq!(msgs[1].text, "three");
    }

    #[test]
    fn test_skips_non_message_lines() {
        let f = make_session_file(&[
            r#"{"type":"file-history-snapshot","snapshot":{}}"#,
            r#"{"type":"user","message":{"content":"hello"}}"#,
            r#"{"type":"system","content":"sys"}"#,
        ]);

        let (msgs, stats) = extract_messages(f.path(), 100).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(stats.total_user, 1);
        assert_eq!(stats.total_assistant, 0);
    }

    #[test]
    fn test_skips_malformed_lines() {
        let f = make_session_file(&[
            r#"not valid json"#,
            r#"{"type":"user","message":{"content":"hello"}}"#,
            r#"{"broken"#,
        ]);

        let (msgs, _) = extract_messages(f.path(), 100).unwrap();
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn test_thinking_blocks_excluded() {
        let f = make_session_file(&[
            r#"{"type":"assistant","message":{"content":[{"type":"thinking"},{"type":"text","text":"visible"}]}}"#,
        ]);

        let (msgs, _) = extract_messages(f.path(), 100).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].text, "visible");
    }

    #[test]
    fn test_render_checkpoint_structure() {
        let messages = vec![
            ExtractedMessage {
                role: Role::Human,
                text: "hello".to_string(),
            },
            ExtractedMessage {
                role: Role::Assistant,
                text: "hi".to_string(),
            },
        ];
        let stats = ExtractionStats {
            session_name: "test-session".to_string(),
            source_path: "/tmp/test.jsonl".to_string(),
            file_size: 1024,
            total_user: 1,
            total_assistant: 1,
            extracted: 2,
        };

        let output = render_checkpoint(&messages, &stats, 100);

        assert!(output.contains("# Context Checkpoint"));
        assert!(output.contains("test-session"));
        assert!(output.contains("## Human"));
        assert!(output.contains("hello"));
        assert!(output.contains("## Assistant"));
        assert!(output.contains("hi"));
        assert!(output.contains("Restoration Instructions"));
    }
}
