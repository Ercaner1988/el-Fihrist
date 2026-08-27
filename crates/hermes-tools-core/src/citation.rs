use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationMarker {
    pub raw_tag: String,
    pub citation_content: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationReport {
    pub total_paragraphs: usize,
    pub cited_paragraphs: usize,
    pub coverage_percent: f64,
    pub markers: Vec<CitationMarker>,
}

pub fn verify_citations(document_md: &str) -> CitationReport {
    let mut markers = Vec::new();
    let mut total_paragraphs = 0;
    let mut cited_paragraphs = 0;

    for (line_idx, line) in document_md.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        total_paragraphs += 1;
        let mut has_citation = false;

        // Check for [[ATIF: ...]] format
        let mut start_idx = 0;
        while let Some(pos) = trimmed[start_idx..].find("[[ATIF:") {
            let abs_pos = start_idx + pos;
            if let Some(end) = trimmed[abs_pos..].find("]]") {
                let raw_tag = trimmed[abs_pos..=abs_pos + end + 1].to_string();
                let citation_content = trimmed[abs_pos + 7..abs_pos + end].trim().to_string();
                markers.push(CitationMarker {
                    raw_tag,
                    citation_content,
                    line_number: line_idx + 1,
                });
                has_citation = true;
                start_idx = abs_pos + end + 2;
            } else {
                break;
            }
        }

        if has_citation {
            cited_paragraphs += 1;
        }
    }

    let coverage_percent = if total_paragraphs == 0 {
        0.0
    } else {
        (cited_paragraphs as f64 / total_paragraphs as f64) * 100.0
    };

    CitationReport {
        total_paragraphs,
        cited_paragraphs,
        coverage_percent,
        markers,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn atif_dogrulama() {
        let doc = "Bu birinci cümledir. [[ATIF: Kaynak A, s. 12]]\n\nBu ikinci cümledir atıfsız.\n\nBu üçüncü cümledir. [[ATIF: Kaynak B, s. 45]]";
        let rep = verify_citations(doc);
        assert_eq!(rep.total_paragraphs, 3);
        assert_eq!(rep.cited_paragraphs, 2);
        assert_eq!(rep.markers.len(), 2);
        assert_eq!(rep.markers[0].citation_content, "Kaynak A, s. 12");
    }
}
