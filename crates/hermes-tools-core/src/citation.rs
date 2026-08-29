use crate::error::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

pub const MAX_DOC_SIZE: usize = 5 * 1024 * 1024; // 5 MB limit
pub const MAX_PARAGRAPHS: usize = 50_000;

/// Belgedeki [[ATIF: ...]] formatındaki atıfları doğrular ve raporlar.
/// D1 (Boşluk) ve D2 (Desen/Uzunluk) kapılarını çalıştırır.
pub fn verify_citations(document_md: &str) -> ToolResult<CitationReport> {
    // D1 Kapısı: Artefakt mevcut mu?
    if document_md.trim().is_empty() {
        return Err(ToolError::InvalidInput("Belge içeriği boş olamaz".into()));
    }

    if document_md.len() > MAX_DOC_SIZE {
        return Err(ToolError::MaxLimitExceeded(format!(
            "Belge boyutu azami sınırı aştı: {} > {}",
            document_md.len(),
            MAX_DOC_SIZE
        )));
    }

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

        // D2 Kapısı: [[ATIF: ...]] deseni sınırları
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

    // D3 Kapısı: Paragraf sırası ve sayım bütünlüğü
    if total_paragraphs > MAX_PARAGRAPHS {
        return Err(ToolError::MaxLimitExceeded(format!(
            "Paragraf sayısı azami sınırı aştı: {} > {}",
            total_paragraphs, MAX_PARAGRAPHS
        )));
    }

    let coverage_percent = if total_paragraphs == 0 {
        0.0
    } else {
        (cited_paragraphs as f64 / total_paragraphs as f64) * 100.0
    };

    // R2 Kapısı: Saf hesaplama, yan etki yok
    Ok(CitationReport {
        total_paragraphs,
        cited_paragraphs,
        coverage_percent,
        markers,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn atif_dogrulama() {
        let doc = "Bu birinci cümle. [[ATIF: Kaynak A, s. 12]]\n\nBu ikinci cümle atıfsız.\n\nBu üçüncü cümle. [[ATIF: Kaynak B, s. 45]]";
        let rep = verify_citations(doc).expect("Doğrulama başarılı olmalı");
        assert_eq!(rep.total_paragraphs, 3);
        assert_eq!(rep.cited_paragraphs, 2);
        assert_eq!(rep.markers.len(), 2);
        assert_eq!(rep.markers[0].citation_content, "Kaynak A, s. 12");
    }

    #[test]
    fn bos_belge_d1_ihlali() {
        let res = verify_citations("   ");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }

    #[test]
    fn kapali_atif_d2_ihlali() {
        let doc = "Bu metinde kapalı bir [[ATIF: kaynak var ama kapanış yok";
        let res = verify_citations(doc).expect("Doğrulama successful olmalı");
        assert_eq!(res.markers.len(), 0);
        assert_eq!(res.cited_paragraphs, 0);
    }
}
