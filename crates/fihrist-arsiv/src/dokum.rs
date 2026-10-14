//! Claude Code dökümü (`~/.claude/projects/**/*.jsonl`): satırdan arşiv kaydı çıkarma. SAF.
//!
//! Arşive girer: Ercan'ın mesajları, modelin cevapları, araç çağrısının adı ve girdisi, sınırdaki
//! özet. **Girmez:** araç çıktısının gövdesi (yalnız izi: kimlik, hata, uzunluk; asıl yeri
//! `dokum:satir` gösterir), düşünme blokları, ekler, alt ajanın düz metni (yalnız araç çağrıları).

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Araç girdisinin toplam üst sınırı (karakter).
pub const GIRDI_AZAMI: usize = 16 * 1024;
/// Girdideki tek bir metin alanının üst sınırı. serde_json alanları harf sırasına dizer (`content`
/// `file_path`ten önce gelir): uzun bir Write gövdesi yolu kesip atmasın diye kesme alan alan yapılır;
/// kesilen miktar metne yazılır.
const ALAN_AZAMI: usize = 2000;

/// Arşivdeki bir kayıt: dökümün bir satırındaki bir blok. `(uuid, blok)` tekildir.
#[derive(Debug, Clone, PartialEq)]
pub struct Kayit {
    pub uuid: String,
    /// `message.content` içindeki sıra (içerik düz metinse 0).
    pub blok: u32,
    pub oturum: String,
    /// `ercan` | `model` | `arac` | `sonuc` (yalnız iz) | `ozet` (sıkıştırma özeti) | `sinir`.
    pub tur: &'static str,
    /// Dökümde yazıldığı gibi (ISO-8601).
    pub zaman: String,
    pub zaman_ms: Option<i64>,
    /// `arac` türünde araç adı (`mcp__nazar__nazar_ara`, `Read`…).
    pub arac: Option<String>,
    /// Alt ajan (sidechain) satırı.
    pub yan: bool,
    pub metin: String,
}

/// Girdideki uzun metin alanlarını tek tek keser (iç içe nesne ve dizilerde de).
fn alanlari_sinirla(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(kes(s, ALAN_AZAMI)),
        Value::Array(l) => Value::Array(l.iter().map(alanlari_sinirla).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, d)| (k.clone(), alanlari_sinirla(d)))
                .collect(),
        ),
        d => d.clone(),
    }
}

/// Karakter sınırında keser ve kesileni söyler.
fn kes(s: &str, azami: usize) -> String {
    let n = s.chars().count();
    if n <= azami {
        return s.to_string();
    }
    let bas: String = s.chars().take(azami).collect();
    format!("{bas}… (+{} karakter)", n - azami)
}

fn sonuc_uzunlugu(b: &Value) -> usize {
    match b.get("content") {
        Some(Value::String(s)) => s.chars().count(),
        Some(Value::Array(l)) => l
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .map(|t| t.chars().count())
            .sum(),
        _ => 0,
    }
}

/// Tek döküm satırından kayıtlar. JSON değilse `None` (çağıran sayar: bozuk satır); ilgisiz satır `Some(boş)`.
pub fn ayristir(satir: &str) -> Option<Vec<Kayit>> {
    // Ucuz ön süzgeç: dökümün yarısı meta satırı; yalnız uuid taşıyanlar kayıt üretebilir.
    if !satir.contains("\"uuid\"") {
        return if satir.trim().starts_with('{') {
            Some(Vec::new())
        } else {
            None
        };
    }
    let d: Value = serde_json::from_str(satir).ok()?;
    let metin_alani = |a: &str| d.get(a).and_then(Value::as_str).map(str::to_string);
    let (Some(uuid), Some(zaman)) = (metin_alani("uuid"), metin_alani("timestamp")) else {
        return Some(Vec::new());
    };
    let oturum = metin_alani("sessionId").unwrap_or_default();
    let yan = d.get("isSidechain").and_then(Value::as_bool) == Some(true);
    let mut cikti = Vec::new();
    let mut ekle = |blok: u32, tur: &'static str, arac: Option<String>, metin: String| {
        cikti.push(Kayit {
            uuid: uuid.clone(),
            blok,
            oturum: oturum.clone(),
            tur,
            zaman_ms: crate::zaman::iso_ms(&zaman),
            zaman: zaman.clone(),
            arac,
            yan,
            metin,
        });
    };
    let tip = d.get("type").and_then(Value::as_str);
    if tip == Some("system") && d.get("subtype").and_then(Value::as_str) == Some("compact_boundary")
    {
        let ayrinti = d
            .get("compactMetadata")
            .map(Value::to_string)
            .unwrap_or_default();
        let icerik = metin_alani("content").unwrap_or_default();
        ekle(
            0,
            "sinir",
            None,
            format!("{icerik} {ayrinti}").trim().to_string(),
        );
        return Some(cikti);
    }
    if tip != Some("user") && tip != Some("assistant") {
        return Some(Vec::new());
    }
    let ozet = d.get("isCompactSummary").and_then(Value::as_bool) == Some(true);
    if !ozet && d.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return Some(Vec::new());
    }
    let bloklar: Vec<Value> = match d.pointer("/message/content") {
        Some(Value::String(s)) => vec![serde_json::json!({"type": "text", "text": s})],
        Some(Value::Array(l)) => l.clone(),
        _ => return Some(Vec::new()),
    };
    for (i, b) in bloklar.iter().enumerate() {
        let i = i as u32;
        match b.get("type").and_then(Value::as_str) {
            Some("text") => {
                let t = b.get("text").and_then(Value::as_str).unwrap_or_default();
                if t.trim().is_empty() {
                    continue;
                }
                if ozet {
                    ekle(i, "ozet", None, t.to_string());
                } else if !yan {
                    ekle(
                        i,
                        if tip == Some("user") {
                            "ercan"
                        } else {
                            "model"
                        },
                        None,
                        t.to_string(),
                    );
                }
            }
            Some("tool_use") if !ozet => {
                let ad = b
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_string();
                let girdi = b
                    .get("input")
                    .map(|g| alanlari_sinirla(g).to_string())
                    .unwrap_or_default();
                ekle(i, "arac", Some(ad), kes(&girdi, GIRDI_AZAMI));
            }
            Some("tool_result") if !ozet && !yan => {
                let id = b.get("tool_use_id").and_then(Value::as_str).unwrap_or("?");
                let hata = b.get("is_error").and_then(Value::as_bool) == Some(true);
                ekle(
                    i,
                    "sonuc",
                    None,
                    format!(
                        "tool_use_id={id} hata={} uzunluk={}",
                        hata as u8,
                        sonuc_uzunlugu(b)
                    ),
                );
            }
            _ => {} // thinking, image, bilinmeyen blok
        }
    }
    Some(cikti)
}

/// Kök altındaki bütün `.jsonl` dökümleri (alt ajan dökümleri dahil: onların araç çağrıları da gerçek çağrılar).
pub fn dokumleri_bul(kok: &Path) -> Vec<PathBuf> {
    let mut bulunan = Vec::new();
    let mut yigin = vec![kok.to_path_buf()];
    while let Some(dizin) = yigin.pop() {
        let Ok(girdiler) = std::fs::read_dir(&dizin) else {
            continue;
        };
        for g in girdiler.flatten() {
            let y = g.path();
            if y.is_dir() {
                yigin.push(y);
            } else if y.extension().is_some_and(|e| e == "jsonl") {
                bulunan.push(y);
            }
        }
    }
    bulunan.sort();
    bulunan
}

/// Varsayılan döküm kökü: `CLAUDE_CONFIG_DIR` yoksa `<home>/.claude`, altında `projects`.
pub fn varsayilan_kok() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(v).join("projects"));
    }
    ["USERPROFILE", "HOME"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .map(|h| PathBuf::from(h).join(".claude").join("projects"))
}

#[cfg(test)]
mod testler {
    use super::*;
    use serde_json::json;

    fn satir(tip: &str, ek: Value, icerik: Value) -> String {
        let mut d = json!({
            "type": tip, "uuid": "u1", "timestamp": "2026-10-14T10:00:00.000Z",
            "sessionId": "o1", "message": {"content": icerik}
        });
        for (k, v) in ek.as_object().unwrap() {
            d[k] = v.clone();
        }
        d.to_string()
    }

    fn turler(k: &[Kayit]) -> Vec<(&str, u32)> {
        k.iter().map(|k| (k.tur, k.blok)).collect()
    }

    #[test]
    fn ercan_model_arac_ve_sonuc_izi() {
        let u = ayristir(&satir("user", json!({}), json!("merhaba"))).unwrap();
        assert_eq!(turler(&u), [("ercan", 0)]);
        assert_eq!(
            (u[0].oturum.as_str(), u[0].zaman_ms),
            ("o1", Some(1_791_972_000_000))
        );

        let a = ayristir(&satir(
            "assistant",
            json!({}),
            json!([
                {"type": "thinking", "thinking": "gizli düşünce"},
                {"type": "text", "text": "cevap"},
                {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "C:\\a\\b.txt"}}
            ]),
        ))
        .unwrap();
        assert_eq!(
            turler(&a),
            [("model", 1), ("arac", 2)],
            "düşünme blokları girmez"
        );
        assert_eq!(a[1].arac.as_deref(), Some("Read"));
        assert!(a[1].metin.contains("b.txt"));

        let s = ayristir(&satir(
            "user",
            json!({}),
            json!([{"type": "tool_result", "tool_use_id": "t1", "content": "ÇOK UZUN GÖVDE", "is_error": true}]),
        ))
        .unwrap();
        assert_eq!(turler(&s), [("sonuc", 0)]);
        assert_eq!(s[0].metin, "tool_use_id=t1 hata=1 uzunluk=14");
        assert!(
            !s[0].metin.contains("GÖVDE"),
            "araç çıktısının gövdesi arşive girmez"
        );
    }

    #[test]
    fn sinir_ozet_meta_ve_alt_ajan() {
        let sinir = json!({"type": "system", "subtype": "compact_boundary", "uuid": "b1",
            "timestamp": "2026-10-14T10:00:00.000Z", "sessionId": "o1", "content": "Conversation compacted",
            "compactMetadata": {"trigger": "auto", "preTokens": 190000}})
        .to_string();
        let s = ayristir(&sinir).unwrap();
        assert_eq!(turler(&s), [("sinir", 0)]);
        assert!(s[0].metin.contains("preTokens"));

        let o = ayristir(&satir(
            "user",
            json!({"isCompactSummary": true}),
            json!([{"type": "text", "text": "özet metni"}]),
        ))
        .unwrap();
        assert_eq!(turler(&o), [("ozet", 0)], "özet Ercan'ın mesajı sayılmaz");

        let meta = ayristir(&satir(
            "user",
            json!({"isMeta": true}),
            json!("hook çıktısı"),
        ))
        .unwrap();
        assert!(meta.is_empty(), "meta satır girmez");

        // Alt ajan: düz metin ve sonuç izi girmez, araç çağrısı girer (Seçim eşlemesi için).
        let yan = ayristir(&satir(
            "assistant",
            json!({"isSidechain": true}),
            json!([{"type": "text", "text": "alt ajan konuşması"}, {"type": "tool_use", "id": "t9", "name": "Grep", "input": {"pattern": "x"}}]),
        ))
        .unwrap();
        assert_eq!(turler(&yan), [("arac", 1)]);
        assert!(yan[0].yan);
    }

    #[test]
    fn bozuk_ve_ilgisiz_satirlar() {
        assert_eq!(
            ayristir("yarım {satır"),
            None,
            "JSON olmayan satır bozuk sayılır"
        );
        assert_eq!(
            ayristir(r#"{"type":"last-prompt","sessionId":"o1"}"#),
            Some(vec![])
        );
        assert_eq!(
            ayristir(r#"{"type":"attachment","uuid":"x","timestamp":"2026-10-14T10:00:00.000Z"}"#),
            Some(vec![])
        );
        assert_eq!(ayristir(r#"{"uuid":"x" bozuk"#), None);
    }

    #[test]
    fn uzun_arac_girdisi_kesilir_ve_soylenir() {
        let uzun = "ş".repeat(ALAN_AZAMI + 50); // çok baytlı: bayt dilimi panik verirdi
        let a = ayristir(&satir(
            "assistant",
            json!({}),
            json!([{"type": "tool_use", "id": "t", "name": "Write", "input": {"file_path": "a.txt", "content": uzun}}]),
        ))
        .unwrap();
        // `content` harf sırasıyla `file_path`ten ÖNCE gelir; yine de yol kalmalı (Seçim eşlemesi ona bakar).
        assert!(a[0].metin.contains("a.txt"), "yol kesilmemeli");
        assert!(a[0].metin.contains("(+50 karakter)"), "kesilen söylenmeli");
    }
}
