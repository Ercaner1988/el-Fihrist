use crate::error::{ToolError, ToolResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleViolation {
    pub line_number: usize,
    pub rule_name: String,
    pub snippet: String,
    pub severity: String, // "High", "Medium", "Low"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub total_lines: usize,
    pub safety_score: f64,
    pub violations: Vec<RuleViolation>,
}

pub const MAX_CODE_SIZE: usize = 5 * 1024 * 1024; // 5 MB limit

/// Rust kaynak kod kurallarını ve güvenlik ihlallerini denetler.
pub fn check_rust_code_rules(code: &str) -> ToolResult<QualityReport> {
    if code.trim().is_empty() {
        return Err(ToolError::InvalidInput(
            "Denetlenecek kod içeriği boş olamaz".into(),
        ));
    }

    if code.len() > MAX_CODE_SIZE {
        return Err(ToolError::MaxLimitExceeded(format!(
            "Kod boyutu azami sınırı aştı: {} > {}",
            code.len(),
            MAX_CODE_SIZE
        )));
    }

    let mut violations = Vec::new();
    let mut penalty = 0.0;
    let mut total_lines = 0;

    for (idx, line) in code.lines().enumerate() {
        total_lines += 1;
        let line_num = idx + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        if trimmed.contains(".unwrap()") {
            violations.push(RuleViolation {
                line_number: line_num,
                rule_name: "UnwrapUsage".into(),
                snippet: trimmed.to_string(),
                severity: "Medium".into(),
            });
            penalty += 10.0;
        }

        if trimmed.contains("panic!(") {
            violations.push(RuleViolation {
                line_number: line_num,
                rule_name: "PanicCall".into(),
                snippet: trimmed.to_string(),
                severity: "High".into(),
            });
            penalty += 20.0;
        }

        if trimmed.contains("unsafe ") || trimmed.starts_with("unsafe{") {
            violations.push(RuleViolation {
                line_number: line_num,
                rule_name: "UnsafeBlock".into(),
                snippet: trimmed.to_string(),
                severity: "High".into(),
            });
            penalty += 25.0;
        }

        if trimmed.contains("todo!(") || trimmed.contains("unimplemented!(") {
            violations.push(RuleViolation {
                line_number: line_num,
                rule_name: "IncompleteCode".into(),
                snippet: trimmed.to_string(),
                severity: "High".into(),
            });
            penalty += 15.0;
        }
    }

    let safety_score = f64::max(100.0 - penalty, 0.0);

    Ok(QualityReport {
        total_lines,
        safety_score,
        violations,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kural_denetimi_unwrap_ve_panic() {
        let code = "fn run() {\n    let val = res.unwrap();\n    panic!(\"Hata\");\n}\n";
        let report = check_rust_code_rules(code).expect("Denetim başarılı olmalı");
        assert_eq!(report.violations.len(), 2);
        assert_eq!(report.safety_score, 70.0);
    }

    #[test]
    fn bos_kod_hatasi() {
        let res = check_rust_code_rules("   ");
        assert!(matches!(res, Err(ToolError::InvalidInput(_))));
    }
}
