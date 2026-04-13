use serde::Deserialize;

/// A single line from the Claude Code session JSONL file.
/// We only care about user and assistant messages.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum SessionLine {
    User(MessageLine),
    Assistant(MessageLine),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct MessageLine {
    pub message: MessagePayload,
}

#[derive(Debug, Deserialize)]
pub struct MessagePayload {
    pub content: Content,
}

/// Content can be a plain string (common for user messages) or an array of blocks.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {},
    ToolUse {},
    ToolResult {},
    Thinking {},
    #[serde(other)]
    Unknown,
}

/// A filtered, extracted message ready for rendering.
#[derive(Debug)]
pub struct ExtractedMessage {
    pub role: Role,
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub enum Role {
    Human,
    Assistant,
}

impl Content {
    /// Extract displayable text from content, filtering out tool-only and thinking blocks.
    pub fn extract_text(&self) -> Option<String> {
        match self {
            Content::Text(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
            Content::Blocks(blocks) => {
                let mut parts = Vec::new();
                for block in blocks {
                    match block {
                        ContentBlock::Text { text } => parts.push(text.clone()),
                        ContentBlock::Image {} => parts.push("[Image attached]".to_string()),
                        // Skip tool_use, tool_result, thinking, and unknown blocks
                        _ => {}
                    }
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("\n"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_string_content() {
        let json = r#"{"type":"user","message":{"content":"hello world"}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::User(msg) => {
                assert_eq!(msg.message.content.extract_text().unwrap(), "hello world");
            }
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn test_parse_user_array_content() {
        let json = r#"{"type":"user","message":{"content":[{"type":"text","text":"hello"},{"type":"text","text":"world"}]}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::User(msg) => {
                assert_eq!(msg.message.content.extract_text().unwrap(), "hello\nworld");
            }
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn test_parse_assistant_text_blocks() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"response here"}]}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::Assistant(msg) => {
                assert_eq!(msg.message.content.extract_text().unwrap(), "response here");
            }
            _ => panic!("expected Assistant"),
        }
    }

    #[test]
    fn test_tool_only_assistant_returns_none() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use"}]}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::Assistant(msg) => {
                assert!(msg.message.content.extract_text().is_none());
            }
            _ => panic!("expected Assistant"),
        }
    }

    #[test]
    fn test_thinking_block_filtered() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"thinking"},{"type":"text","text":"visible"}]}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::Assistant(msg) => {
                assert_eq!(msg.message.content.extract_text().unwrap(), "visible");
            }
            _ => panic!("expected Assistant"),
        }
    }

    #[test]
    fn test_image_block_placeholder() {
        let json = r#"{"type":"user","message":{"content":[{"type":"image"},{"type":"text","text":"check this"}]}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::User(msg) => {
                let text = msg.message.content.extract_text().unwrap();
                assert!(text.contains("[Image attached]"));
                assert!(text.contains("check this"));
            }
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn test_other_line_types_skipped() {
        let json = r#"{"type":"file-history-snapshot","snapshot":{}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        assert!(matches!(line, SessionLine::Other));
    }

    #[test]
    fn test_tool_result_user_returns_none() {
        let json = r#"{"type":"user","message":{"content":[{"type":"tool_result"}]}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::User(msg) => {
                assert!(msg.message.content.extract_text().is_none());
            }
            _ => panic!("expected User"),
        }
    }

    #[test]
    fn test_empty_string_content_returns_none() {
        let json = r#"{"type":"user","message":{"content":""}}"#;
        let line: SessionLine = serde_json::from_str(json).unwrap();
        match line {
            SessionLine::User(msg) => {
                assert!(msg.message.content.extract_text().is_none());
            }
            _ => panic!("expected User"),
        }
    }
}
