//! Oturum başına sıcak küme (F3a).
//!
//! `search_skills`in ilk `ARAMA_BASI` yetenek sonucu o oturumun araç listesine
//! girer (`y_<kimlik>`, açıklaması yeteneğinki; çağrılınca tam metni döner).
//! `FIHRIST_SICAK_DK` (varsayılan 30) dakika ne aranıp ne çağrılan düşer. Her
//! değişiklik istemciye `notifications/tools/list_changed` ile bildirilir
//! (Claude Code destekliyor: code.claude.com/docs/en/mcp.md). Böylece Desktop'un
//! yeteneği Claude Code'da, Claude Code'unki Antigravity'de araç olur.
//!
//! MCP araçları (`mcp/…`) GİRMEZ: arayüzde zaten doğrudan bağlılar; bağlı
//! olmadıkları yerde çağırmak vekillik ister (F3b).

use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const ARAMA_BASI: usize = 3;
/// ponytail: sabit tavan; en eski düşer. Bağlam taşarsa ölçüp küçült.
pub const TAVAN: usize = 12;

#[derive(Debug, Clone, PartialEq)]
struct Kalem {
    kimlik: String,
    aciklama: String,
    son_ms: i64,
}

#[derive(Debug, Default)]
pub struct Sicak {
    /// araç adı → kalem
    kalemler: BTreeMap<String, Kalem>,
    omur_ms: i64,
    /// Her değişiklikte artar; `konus` bakıp bildirim iter.
    pub surum: u64,
}

pub fn omur_ms() -> i64 {
    std::env::var("FIHRIST_SICAK_DK")
        .ok()
        .and_then(|d| d.parse::<f64>().ok())
        .filter(|d| *d > 0.0)
        .map_or(30 * 60_000, |d| (d * 60_000.0) as i64)
}

/// MCP araç adı: `[A-Za-z0-9_-]`, en çok 64 karakter.
pub fn arac_adi(kimlik: &str) -> String {
    let govde: String = kimlik
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("y_{govde}").chars().take(64).collect()
}

/// Arama çıktısından (`"N / M kayıt · ms\n[...]"`) sıcak kümeye aday
/// yetenekler: (kimlik, açıklama), sırasıyla, `mcp/` hariç.
pub fn adaylar(metin: &str) -> Vec<(String, String)> {
    let Some((_, govde)) = metin.split_once('\n') else {
        return Vec::new();
    };
    let Ok(Value::Array(d)) = serde_json::from_str::<Value>(govde) else {
        return Vec::new();
    };
    d.iter()
        .filter_map(|o| {
            let id = o["id"].as_str()?;
            (!id.starts_with("mcp/")).then(|| {
                (
                    id.to_string(),
                    o["aciklama"].as_str().unwrap_or("").to_string(),
                )
            })
        })
        .take(ARAMA_BASI)
        .collect()
}

impl Sicak {
    pub fn yeni(omur_ms: i64) -> Self {
        Self {
            omur_ms,
            ..Default::default()
        }
    }

    /// Adayları ekler ya da tazeler. Liste değiştiyse sürüm artar.
    pub fn ekle(&mut self, adaylar: Vec<(String, String)>, simdi: i64) {
        let mut degisti = false;
        for (kimlik, aciklama) in adaylar {
            let ad = arac_adi(&kimlik);
            degisti |= !self.kalemler.contains_key(&ad);
            self.kalemler.insert(
                ad,
                Kalem {
                    kimlik,
                    aciklama,
                    son_ms: simdi,
                },
            );
        }
        while self.kalemler.len() > TAVAN {
            let eski = self
                .kalemler
                .iter()
                .min_by_key(|(_, k)| k.son_ms)
                .map(|(a, _)| a.clone())
                .expect("tavan aşıldıysa boş değil");
            self.kalemler.remove(&eski);
        }
        self.surum += degisti as u64;
    }

    /// Sıcak araç adıysa yeteneğin kimliği; çağrı onu tazeler.
    pub fn cagir(&mut self, ad: &str, simdi: i64) -> Option<String> {
        let k = self.kalemler.get_mut(ad)?;
        k.son_ms = simdi;
        Some(k.kimlik.clone())
    }

    /// Ömrü dolanları düşürür.
    pub fn dusur(&mut self, simdi: i64) {
        let once = self.kalemler.len();
        let omur = self.omur_ms;
        self.kalemler.retain(|_, k| simdi - k.son_ms < omur);
        self.surum += (self.kalemler.len() != once) as u64;
    }

    /// En yakın düşüş anı; küme boşsa `None`.
    pub fn siradaki_dusus(&self) -> Option<i64> {
        self.kalemler
            .values()
            .map(|k| k.son_ms + self.omur_ms)
            .min()
    }

    pub fn araclar(&self) -> Vec<Value> {
        self.kalemler
            .iter()
            .map(|(ad, k)| {
                json!({
                    "name": ad,
                    "description": format!(
                        "[el-Fihrist yeteneği {}] {} — Çağırınca yeteneğin tam metnini (talimatlarını) döndürür.",
                        k.kimlik, k.aciklama
                    ),
                    "inputSchema": {"type": "object", "properties": {}}
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const ARAMA: &str = "3 / 9 kayıt · 1 ms\n[{\"id\":\"claude-desktop/docx\",\"aciklama\":\"Word\"},{\"id\":\"mcp/codspeed/get_run\",\"aciklama\":\"x\"},{\"id\":\"claude-code/tez-bolum\",\"aciklama\":\"tez\"}]";

    #[test]
    fn adaylar_mcp_disarida_sirasiyla() {
        assert_eq!(
            adaylar(ARAMA),
            [
                ("claude-desktop/docx".to_string(), "Word".to_string()),
                ("claude-code/tez-bolum".to_string(), "tez".to_string())
            ]
        );
        assert!(adaylar("biçimsiz").is_empty());
    }

    #[test]
    fn arac_adi_mcp_kuralina_uyar() {
        assert_eq!(
            arac_adi("claude-code/synced/docx"),
            "y_claude-code_synced_docx"
        );
        assert_eq!(arac_adi("ş/ğ").len(), 5, "ASCII dışı _ olur");
        assert_eq!(arac_adi(&"a".repeat(100)).len(), 64);
    }

    #[test]
    fn eklenir_cagri_tazeler_omru_dolan_duser() {
        let mut s = Sicak::yeni(1000);
        s.ekle(adaylar(ARAMA), 0);
        assert_eq!((s.araclar().len(), s.surum), (2, 1));
        s.ekle(adaylar(ARAMA), 10);
        assert_eq!(s.surum, 1, "aynı küme yeniden bildirilmez");
        assert_eq!(
            s.cagir("y_claude-desktop_docx", 900).as_deref(),
            Some("claude-desktop/docx")
        );
        assert_eq!(s.siradaki_dusus(), Some(1010));
        s.dusur(1500);
        let adlar: Vec<_> = s.araclar().iter().map(|a| a["name"].clone()).collect();
        assert_eq!(adlar, [json!("y_claude-desktop_docx")], "çağrılan kaldı");
        assert_eq!(s.surum, 2);
        assert!(s.cagir("search_skills", 0).is_none());
    }

    #[test]
    fn tavan_asilinca_en_eski_duser() {
        let mut s = Sicak::yeni(1_000_000);
        for i in 0..TAVAN + 2 {
            s.ekle(vec![(format!("k/{i}"), String::new())], i as i64);
        }
        assert_eq!(s.araclar().len(), TAVAN);
        assert!(s.cagir("y_k_0", 0).is_none());
        assert!(s.cagir(&format!("y_k_{}", TAVAN + 1), 0).is_some());
    }
}
