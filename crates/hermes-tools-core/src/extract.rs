use crate::error::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedLink {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub title: String,
    pub content_markdown: String,
    pub links: Vec<ExtractedLink>,
    pub word_count: usize,
}

pub const MAX_HTML_SIZE: usize = 10 * 1024 * 1024; // 10 MB limit
pub const MAX_TAG_DEPTH: usize = 100;

/// HTML metninden etiketleri ayıklayarak temiz Markdown ve bağlantıları çıkarır.
/// Güvenli, DoS korumalı ve `ToolResult` dönen versiyondur.
pub fn html_ayikla(html: &str) -> ToolResult<ExtractResult> {
    // D1 Kapısı: Girdi boşluk kontrolü
    if html.trim().is_empty() {
        return Err(ToolError::InvalidInput("HTML içeriği boş olamaz".into()));
    }

    // Girdi boyutu sınırı (OOM / DoS engelleme)
    if html.len() > MAX_HTML_SIZE {
        return Err(ToolError::MaxLimitExceeded(format!(
            "HTML boyutu azami sınırı aştı: {} > {}",
            html.len(),
            MAX_HTML_SIZE
        )));
    }

    // Karakter bütünlüğü kontrolü (Null byte koruması)
    if html.contains('\0') {
        return Err(ToolError::InvalidInput("HTML içinde geçersiz null karakter saptandı".into()));
    }

    let mut title = String::new();
    let mut links = Vec::new();
    let mut markdown = String::new();
    let mut in_title = false;
    let mut in_a = false;
    let mut current_href = String::new();
    let mut current_a_text = String::new();

    let mut tag_depth = 0;
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            tag_depth += 1;
            if tag_depth > MAX_TAG_DEPTH {
                return Err(ToolError::ParseError("Etiket yuvalama derinliği sınırı aşıldı (Olası DoS)".into()));
            }

            let mut current_tag = String::new();
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
                    tag_depth = tag_depth.saturating_sub(1);
                    break;
                }
                current_tag.push(nc);
                chars.next();
            }

            let lower_tag = current_tag.trim().to_lowercase();
            if lower_tag == "title" {
                in_title = true;
            } else if lower_tag == "/title" {
                in_title = false;
            } else if lower_tag.starts_with("a ") || lower_tag == "a" {
                in_a = true;
                current_href = ayikla_href(&current_tag);
                current_a_text.clear();
            } else if lower_tag == "/a" {
                if in_a {
                    if !current_href.is_empty() {
                        links.push(ExtractedLink {
                            text: current_a_text.trim().to_string(),
                            url: current_href.clone(),
                        });
                        markdown.push_str(&format!(" [{}]({}) ", current_a_text.trim(), current_href));
                    }
                    in_a = false;
                }
            } else if lower_tag == "p" || lower_tag == "/p" || lower_tag == "br" || lower_tag == "br/" {
                markdown.push('\n');
            } else if lower_tag.starts_with("h1") {
                markdown.push_str("\n# ");
            } else if lower_tag.starts_with("h2") {
                markdown.push_str("\n## ");
            } else if lower_tag.starts_with("h3") {
                markdown.push_str("\n### ");
            }
            continue;
        }

        if in_title {
            title.push(c);
        } else if in_a {
            current_a_text.push(c);
        } else {
            markdown.push(c);
        }
    }

    let clean_md = temizle_bosluklar(&markdown);
    let word_count = clean_md.split_whitespace().count();

    Ok(ExtractResult {
        title: title.trim().to_string(),
        content_markdown: clean_md,
        links,
        word_count,
    })
}

fn ayikla_href(tag: &str) -> String {
    if let Some(pos) = tag.to_lowercase().find("href=") {
        let rest = tag[pos + 5..].trim();
        let quote = rest.chars().next().unwrap_or('"');
        if quote == '"' || quote == '\'' {
            if let Some(end) = rest[1..].find(quote) {
                return rest[1..end + 1].to_string();
            }
        } else if let Some(end) = rest.find(' ') {
            return rest[..end].to_string();
        } else {
            return rest.to_string();
        }
    }
    String::new()
}

fn temizle_bosluklar(s: &str) -> String {
    let mut result = String::new();
    let mut prev_newline = false;
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_newline {
                result.push('\n');
                prev_newline = true;
            }
        } else {
            result.push_str(trimmed);
            result.push('\n');
            prev_newline = false;
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn html_ayiklama_temel() {
        let html = "<html><head><title>Test Sayfasi</title></head><body><h1>Baslik</h1><p>Merhaba <a href=\"https://example.com\">Link</a> dunya.</p></body></html>";
        let res = html_ayikla(html).expect("Ayrıştırma başarılı olmalı");
        assert_eq!(res.title, "Test Sayfasi");
        assert!(res.content_markdown.contains("# Baslik"));
        assert_eq!(res.links.len(), 1);
        assert_eq!(res.links[0].url, "https://example.com");
    }

    #[test]
    fn bos_html_hatasi() {
        let res = html_ayikla("   ");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }

    #[test]
    fn null_byte_hatasi() {
        let res = html_ayikla("<html><title>Null\0Test</title></html>");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }

    #[test]
    fn tirnak_icinde_buyuktur_isareti() {
        let html = r#"<a href="https://example.com?a=1&b=>2">Link</a>"#;
        let res = html_ayikla(html).expect("Tırnak içindeki > ayrıştırmayı bozmamalı");
        assert_eq!(res.links.len(), 1);
        assert_eq!(res.links[0].url, "https://example.com?a=1&b=>2");
    }

    #[test]
    fn ic_ice_etiketler_ve_kendiliginden_kapananlar() {
        let html = "<div><p>Satır 1<br/>Satır 2</p><a href='test'><b>kalın link</b></a></div>";
        let res = html_ayikla(html).expect("Başarılı ayrıştırmalı");
        assert!(res.content_markdown.contains("Satır 1"));
        assert!(res.content_markdown.contains("Satır 2"));
        assert_eq!(res.links.len(), 1);
        assert_eq!(res.links[0].text, "kalın link");
    }
}
