use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// OpenXML document.xml dizesinden paragraf metinlerini çıkarır.
pub fn extract_paragraphs_from_xml(xml_content: &str) -> DocxExtractResult {
    let mut paragraphs = Vec::new();
    let mut current_paragraph = String::new();
    let mut in_wt = false;
    let mut in_wp = false;

    let mut idx = 0;

    let mut chars = xml_content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '>' {
                    chars.next();
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

    DocxExtractResult {
        paragraph_count: paragraphs.len(),
        word_count,
        paragraphs,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn docx_xml_paragraf_ayiklama() {
        let xml = "<w:document><w:body><w:p><w:r><w:t>Birinci paragraf</w:t></w:r></w:p><w:p><w:r><w:t>Ikinci paragraf</w:t></w:r></w:p></w:body></w:document>";
        let res = extract_paragraphs_from_xml(xml);
        assert_eq!(res.paragraph_count, 2);
        assert_eq!(res.paragraphs[0].text, "Birinci paragraf");
        assert_eq!(res.paragraphs[1].text, "Ikinci paragraf");
    }
}
