use crate::error::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageStat {
    pub language: String,
    pub file_count: usize,
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseSummary {
    pub total_files: usize,
    pub total_lines: usize,
    pub languages: Vec<LanguageStat>,
}

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "json", "toml", "md", "html", "htm", "css", "sql", "c",
    "h", "cpp", "hpp", "cc", "xml", "txt", "yaml", "yml",
];

/// Dosya içeriğini analiz eder ve dil istatistiklerini döndürür.
/// Masa Döngüsü D1 (Varlık) ve D2 (Desen/Uzantı) kapılarını çalıştırır.
pub fn analyze_file_content(filename: &str, content: &str) -> ToolResult<LanguageStat> {
    // D1 Kapısı: Artefakt boş mu?
    if content.trim().is_empty() {
        return Err(ToolError::InvalidInput(format!(
            "Dosya içeriği boş olduğundan analiz edilemez: {}",
            filename
        )));
    }

    // D2 Kapısı: Dosya adı desen/uzantı denetimi
    if !filename.contains('.') {
        return Err(ToolError::InvalidInput(format!(
            "Dosya adında geçerli bir uzantı bulunamadı: {}",
            filename
        )));
    }

    let ext = filename.split('.').last().unwrap_or("").to_lowercase();
    let lang = match ext.as_str() {
        "rs" => "Rust",
        "py" => "Python",
        "js" | "jsx" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "json" => "JSON",
        "toml" => "TOML",
        "md" => "Markdown",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "sql" => "SQL",
        "c" | "h" => "C",
        "cpp" | "hpp" | "cc" => "C++",
        "xml" => "XML",
        "txt" => "Metin",
        "yaml" | "yml" => "YAML",
        _ => "Diğer",
    };

    let mut total = 0;
    let mut code = 0;
    let mut comment = 0;
    let mut blank = 0;

    // D3 Kapısı: Satır sırası ve bütünlüğü korunarak okunur
    for line in content.lines() {
        total += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank += 1;
        } else if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            comment += 1;
        } else {
            code += 1;
        }
    }

    // R2 Kapısı: Bellekte saf hesaplama yapıldı, yan etki yok
    Ok(LanguageStat {
        language: lang.to_string(),
        file_count: 1,
        total_lines: total,
        code_lines: code,
        comment_lines: comment,
        blank_lines: blank,
    })
}

/// Tüm dosya istatistiklerini birleştirerek kod tabanı özetini oluşturur.
pub fn summarize_codebase(stats: Vec<LanguageStat>) -> ToolResult<CodebaseSummary> {
    if stats.is_empty() {
        return Ok(CodebaseSummary {
            total_files: 0,
            total_lines: 0,
            languages: Vec::new(),
        });
    }

    let mut map: HashMap<String, LanguageStat> = HashMap::new();
    let mut total_files = 0;
    let mut total_lines = 0;

    for s in stats {
        total_files += s.file_count;
        total_lines += s.total_lines;

        let entry = map.entry(s.language.clone()).or_insert_with(|| LanguageStat {
            language: s.language.clone(),
            file_count: 0,
            total_lines: 0,
            code_lines: 0,
            comment_lines: 0,
            blank_lines: 0,
        });

        entry.file_count += s.file_count;
        entry.total_lines += s.total_lines;
        entry.code_lines += s.code_lines;
        entry.comment_lines += s.comment_lines;
        entry.blank_lines += s.blank_lines;
    }

    let mut languages: Vec<LanguageStat> = map.into_values().collect();
    languages.sort_by(|a, b| b.total_lines.cmp(&a.total_lines));

    Ok(CodebaseSummary {
        total_files,
        total_lines,
        languages,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dil_analizi_rust() {
        let code = "// Bu bir testtir\nfn main() {\n    println!(\"Selam\");\n}\n";
        let stat = analyze_file_content("main.rs", code).expect("Analiz başarılı olmalı");
        assert_eq!(stat.language, "Rust");
        assert_eq!(stat.total_lines, 4);
        assert_eq!(stat.comment_lines, 1);
        assert_eq!(stat.code_lines, 3);
    }

    #[test]
    fn bos_icerik_d1_ihlali() {
        let res = analyze_file_content("test.rs", "   ");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }

    #[test]
    fn uzantisiz_dosya_d2_ihlali() {
        let res = analyze_file_content("Makefile", "all: build");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }

    #[test]
    fn kod_tabani_ozetleme() {
        let s1 = analyze_file_content("a.rs", "fn a() {}").unwrap();
        let s2 = analyze_file_content("b.py", "def b(): pass").unwrap();
        let summary = summarize_codebase(vec![s1, s2]).expect("Özetleme başarılı olmalı");
        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.languages.len(), 2);
    }
}
