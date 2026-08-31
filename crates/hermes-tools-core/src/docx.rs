use crate::error::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocxParagraph {
    pub index: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxExtractResult {
    pub paragraph_count: usize,
    pub word_count: usize,
    pub paragraphs: Vec<DocxParagraph>,
}

pub const MAX_XML_SIZE: usize = 10 * 1024 * 1024; // 10 MB limit
pub const MAX_XML_DEPTH: usize = 200;

/// OpenXML document.xml dizesinden paragraf metinlerini çıkarır.
/// Güvenli, DoS korumalı ve `ToolResult` dönen versiyondur.
pub fn extract_paragraphs_from_xml(xml_content: &str) -> ToolResult<DocxExtractResult> {
    // D1 Kapısı: Girdi boşluk kontrolü
    if xml_content.trim().is_empty() {
        return Err(ToolError::InvalidInput("XML içeriği boş olamaz".into()));
    }

    if xml_content.len() > MAX_XML_SIZE {
        return Err(ToolError::MaxLimitExceeded(format!(
            "XML boyutu azami sınırı aştı: {} > {}",
            xml_content.len(),
            MAX_XML_SIZE
        )));
    }

    if xml_content.contains('\0') {
        return Err(ToolError::InvalidInput(
            "XML içinde geçersiz null karakter var".into(),
        ));
    }

    let mut paragraphs = Vec::new();
    let mut current_paragraph = String::new();
    let mut in_wt = false;
    let mut in_wp = false;
    let mut depth = 0;

    let mut idx = 0;
    let mut chars = xml_content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            depth += 1;
            if depth > MAX_XML_DEPTH {
                return Err(ToolError::ParseError(
                    "XML etiket yuvalama sınırı aşıldı".into(),
                ));
            }

            let mut tag = String::new();
            let mut in_quote = false;
            let mut quote_char = '"';

            while let Some(&nc) = chars.peek() {
                if (nc == '"' || nc == '\'') && !in_quote {
                    in_quote = true;
                    quote_char = nc;
                } else if in_quote && nc == quote_char {
                    in_quote = false;
                } else if !in_quote && nc == '>' {
                    chars.next();
                    depth = depth.saturating_sub(1);
                    break;
                }
                tag.push(nc);
                chars.next();
            }

            let tag_name = tag.trim();
            if tag_name == "w:p" || tag_name.starts_with("w:p ") {
                in_wp = true;
                current_paragraph.clear();
            } else if tag_name == "/w:p" {
                if in_wp {
                    let trimmed = current_paragraph.trim().to_string();
                    if !trimmed.is_empty() {
                        idx += 1;
                        paragraphs.push(DocxParagraph {
                            index: idx,
                            text: trimmed,
                        });
                    }
                    in_wp = false;
                }
            } else if tag_name == "w:t" || tag_name.starts_with("w:t ") {
                in_wt = true;
            } else if tag_name == "/w:t" {
                in_wt = false;
            }
            continue;
        }

        if in_wt && in_wp {
            current_paragraph.push(c);
        }
    }

    let word_count = paragraphs
        .iter()
        .map(|p| p.text.split_whitespace().count())
        .sum();

    Ok(DocxExtractResult {
        paragraph_count: paragraphs.len(),
        word_count,
        paragraphs,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn docx_xml_paragraf_ayiklama() {
        let xml = "<w:document><w:body><w:p><w:r><w:t>Birinci paragraf</w:t></w:r></w:p><w:p><w:r><w:t>Ikinci paragraf</w:t></w:r></w:p></w:body></w:document>";
        let res = extract_paragraphs_from_xml(xml).expect("Başarılı olmalı");
        assert_eq!(res.paragraph_count, 2);
        assert_eq!(res.paragraphs[0].text, "Birinci paragraf");
        assert_eq!(res.paragraphs[1].text, "Ikinci paragraf");
    }

    #[test]
    fn bos_xml_hatasi() {
        let res = extract_paragraphs_from_xml("  ");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }

    #[test]
    fn derin_yuvalanmis_xml() {
        let mut xml = String::new();
        for _ in 0..50 {
            xml.push_str("<w:p>");
        }
        xml.push_str("<w:t>Derin metin</w:t>");
        for _ in 0..50 {
            xml.push_str("</w:p>");
        }
        let res = extract_paragraphs_from_xml(&xml).expect("Başarılı ayrıştırmalı");
        assert_eq!(res.paragraph_count, 1);
        assert_eq!(res.paragraphs[0].text, "Derin metin");
    }
}
