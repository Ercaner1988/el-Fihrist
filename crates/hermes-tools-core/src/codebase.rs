use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub fn analyze_file_content(filename: &str, content: &str) -> LanguageStat {
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
        _ => "Diğer",
    };

    let mut total = 0;
    let mut code = 0;
    let mut comment = 0;
    let mut blank = 0;

    for line in content.lines() {
        total += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank += 1;
        } else if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            comment += 1;
        } else {
            code += 1;
        }
    }

    LanguageStat {
        language: lang.to_string(),
        file_count: 1,
        total_lines: total,
        code_lines: code,
        comment_lines: comment,
        blank_lines: blank,
    }
}

pub fn summarize_codebase(stats: Vec<LanguageStat>) -> CodebaseSummary {
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

    CodebaseSummary {
        total_files,
        total_lines,
        languages,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dil_analizi_rust() {
        let code = "// Bu bir testtir\nfn main() {\n    println!(\"Selam\");\n}\n";
        let stat = analyze_file_content("main.rs", code);
        assert_eq!(stat.language, "Rust");
        assert_eq!(stat.total_lines, 4);
        assert_eq!(stat.comment_lines, 1);
        assert_eq!(stat.code_lines, 3);
    }
}
