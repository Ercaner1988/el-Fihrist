use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MasaDongusuGateResult {
    pub gate_id: String,
    pub gate_name: String,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasaDongusuReport {
    pub total_gates: usize,
    pub passed_gates: usize,
    pub azimet_percentage: f64,
    pub gate_results: Vec<MasaDongusuGateResult>,
}

/// Masa Döngüsü Kapılarını temsil eden nitelik (trait).
/// Ehliyet nitelik modelini doğrudan temsil eder.
pub trait MasaKapisi {
    fn kapiyi_denetle(&self) -> crate::error::ToolResult<()>;
}

/// Masa Döngüsü 4 Kapı denetimini gerçekleştirir (D1, D2, D3, R2).
pub fn validate_masa_dongusu(
    artifact_exists: bool,
    pattern_matched: bool,
    order_preserved: bool,
    side_effects_clean: bool,
) -> MasaDongusuReport {
    let mut results = Vec::new();

    // Gate 1: D1 - Yapısal Doğruluk
    results.push(MasaDongusuGateResult {
        gate_id: "D1".into(),
        gate_name: "Yapısal Doğruluk".into(),
        passed: artifact_exists,
        details: if artifact_exists {
            "Çıktı artefaktı mevcut ve doğru".into()
        } else {
            "Artefakt bulunamadı".into()
        },
    });

    // Gate 2: D2 - Desen Sınırları
    results.push(MasaDongusuGateResult {
        gate_id: "D2".into(),
        gate_name: "Desen Sınırları".into(),
        passed: pattern_matched,
        details: if pattern_matched {
            "Desen sınırları korundu".into()
        } else {
            "Desen sınırları ihlal edildi".into()
        },
    });

    // Gate 3: D3 - Sıralama Korunumu
    results.push(MasaDongusuGateResult {
        gate_id: "D3".into(),
        gate_name: "Sıralama Korunumu".into(),
        passed: order_preserved,
        details: if order_preserved {
            "Eleman sırası korundu".into()
        } else {
            "Sıralama bozuldu".into()
        },
    });

    // Gate 4: R2 - Saydım vs Yazdım Ayrımı (Side Effects)
    results.push(MasaDongusuGateResult {
        gate_id: "R2".into(),
        gate_name: "Saydım vs Yazdım Ayrımı".into(),
        passed: side_effects_clean,
        details: if side_effects_clean {
            "Girdi ve disk temiz tutuldu (arı - pure hesaplama)".into()
        } else {
            "İzinsiz yan etki var".into()
        },
    });

    let passed_count = results.iter().filter(|r| r.passed).count();
    let total_gates = results.len();
    let azimet_percentage = (passed_count as f64 / total_gates as f64) * 100.0;

    MasaDongusuReport {
        total_gates,
        passed_gates: passed_count,
        azimet_percentage,
        gate_results: results,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn masa_dongusu_8_kapili_dogrulama() {
        let rep = validate_masa_dongusu(true, true, true, true);
        assert_eq!(rep.passed_gates, 4);
        assert_eq!(rep.azimet_percentage, 100.0);
    }
}
