use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub role: String,
    pub content: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSearchResult {
    pub total_turns: usize,
    pub matches: Vec<SessionTurn>,
}

pub fn parse_session_transcript(transcript: &str) -> Vec<SessionTurn> {
    let mut turns = Vec::new();
    let mut current_role = String::new();
    let mut current_content = String::new();
    let mut turn_line = 0;

    for (line_idx, line) in transcript.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("[user]") || trimmed.starts_with("[USER]") {
            if !current_role.is_empty() {
                turns.push(SessionTurn {
                    role: current_role.clone(),
                    content: current_content.trim().to_string(),
                    line_number: turn_line,
                });
            }
            current_role = "user".to_string();
            current_content = trimmed[6..].trim().to_string();
            turn_line = line_idx + 1;
        } else if trimmed.starts_with("[assistant]") || trimmed.starts_with("[ASSISTANT]") {
            if !current_role.is_empty() {
                turns.push(SessionTurn {
                    role: current_role.clone(),
                    content: current_content.trim().to_string(),
                    line_number: turn_line,
                });
            }
            current_role = "assistant".to_string();
            current_content = trimmed[11..].trim().to_string();
            turn_line = line_idx + 1;
        } else {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    if !current_role.is_empty() {
        turns.push(SessionTurn {
            role: current_role,
            content: current_content.trim().to_string(),
            line_number: turn_line,
        });
    }

    turns
}

pub fn search_session(transcript: &str, query: &str, role_filter: Option<&str>) -> SessionSearchResult {
    let turns = parse_session_transcript(transcript);
    let q_lower = query.to_lowercase();

    let matches = turns
        .into_iter()
        .filter(|t| {
            if let Some(rf) = role_filter {
                if t.role.to_lowercase() != rf.to_lowercase() {
                    return false;
                }
            }
            t.content.to_lowercase().contains(&q_lower)
        })
        .collect::<Vec<_>>();

    SessionSearchResult {
        total_turns: matches.len(),
        matches,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn oturum_gecmisi_arama() {
        let transcript = "[user] Selam, Rust kodunu incele\n[assistant] Tabii ki inceleyelim.\n[user] FTS5 hatası neydi?";
        let res = search_session(transcript, "FTS5", Some("user"));
        assert_eq!(res.total_turns, 1);
        assert_eq!(res.matches[0].role, "user");
        assert!(res.matches[0].content.contains("FTS5"));
    }
}
