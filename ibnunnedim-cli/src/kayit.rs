//! Araç çağrısı denetim günlüğü — agentfs'in `tool_calls` semantiği,
//! agentfs'siz ve Windows'ta.
//!
//! **KANCA YOK, bilerek.** Plan `PostToolUse` kancası → JSONL → içe aktarım
//! öngörüyordu. Ölçüm (2026-09-18) bunu gereksiz çıkardı: Claude Code her
//! oturumu ZATEN append-only JSONL olarak yazıyor
//! (`~/.claude/projects/<proje>/<oturum>.jsonl`) ve o döküm, kancanın
//! alacağından FAZLASINI taşıyor. Tek bir 6,5 MB'lık oturumda:
//!   397 `tool_use` · 396'sı `tool_result` ile eşleşti (açıkta kalan: o an
//!   süren çağrı) · 14 açık `is_error: true` · iki zaman damgası arası süre
//!   (medyan 3,67 sn, p90 34 sn).
//! Kancanın yükünde SÜRE YOKTU (Cursor'da `duration_ms` var, Claude Code'da
//! yok). Yani kanca, diskte duran günlüğün eksik bir kopyasını, üstelik her
//! araç çağrısında bir süreç başlatma bedeliyle üretecekti. `settings.json`a
//! dokunmak da gerekmiyor. Paslı beyin aynı dökümleri zaten okuyor
//! (`cekirdek/src/uret/izler.rs`) — okuma yolu kanıtlı.
//!
//! Süre DUVAR SAATİDİR: izin sorusunda beklenen süre de içindedir. Aracın
//! kendi çalışma süresi değil, "istekten sonuca" geçen süre.

use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Dökümden çıkarılan tek araç çağrısı.
#[derive(Debug, Clone, PartialEq)]
pub struct Cagri {
    /// `tool_use.id` — oturumlar arası tekil; içe aktarımın tekillik anahtarı.
    pub kimlik: String,
    pub oturum: String,
    /// `~/.claude/projects` altındaki kodlanmış proje dizini adı.
    pub proje: String,
    pub arac: String,
    /// Parametre ÖZETİ, tam girdi değil — gövde küçük kalsın.
    pub ozet: String,
    pub hata: bool,
    /// ISO-8601, dökümde yazıldığı gibi.
    pub baslangic: String,
    /// Sonuç gelmediyse (kesilmiş ya da o an süren çağrı) `None`.
    pub sure_ms: Option<i64>,
}

/// "Bu aracın hangi parametresi anlamlı" sorusunun cevabı. Sıra önemli:
/// Edit'te hem `file_path` hem `old_string` var — dosya yolu alınır.
const ANAHTARLAR: &[&str] = &[
    "command",
    "file_path",
    "pattern",
    "query",
    "description",
    "skill",
    "url",
    "prompt",
    "path",
];

/// Parametre özeti çıkarır, kırpar.
pub fn ozetle(girdi: &Value) -> String {
    let Some(nesne) = girdi.as_object() else {
        return crate::kisalt(&girdi.to_string(), 200);
    };
    for a in ANAHTARLAR {
        if let Some(d) = nesne.get(*a).and_then(Value::as_str) {
            return crate::kisalt(d.trim(), 200);
        }
    }
    // Tanınmayan araç (yeni bir MCP aracı): ilk metin alanı, adıyla.
    if let Some((ad, d)) = nesne.iter().find_map(|(k, v)| v.as_str().map(|s| (k, s))) {
        return crate::kisalt(&format!("{ad}={}", d.trim()), 200);
    }
    // Hiç metin yok: hangi alanlar geldi, onu söyle — boş satırdan iyidir.
    let adlar: Vec<&str> = nesne.keys().map(String::as_str).collect();
    crate::kisalt(&format!("({})", adlar.join(", ")), 200)
}

/// `1970-01-01`den bu yana gün (Hinnant'ın `days_from_civil`i).
fn gun(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-09-09T16:28:32.381Z` → Unix milisaniyesi.
///
/// Tarih kütüphanesi YOK: Claude Code tek bir biçim yazıyor (UTC, `Z`,
/// milisaniye). `Z` ile bitmeyen damga `None` — saat dilimi kaymasını
/// sessizce yanlış süreye çevirmektense reddeder.
pub fn iso_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 || !s.ends_with('Z') {
        return None;
    }
    if [4, 7].iter().any(|&i| b[i] != b'-') || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let n = |a: usize, z: usize| s.get(a..z).and_then(|p| p.parse::<i64>().ok());
    let (y, mo, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (h, mi, se) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    let ms = if b[19] == b'.' {
        let kesir = &s[20..s.len() - 1];
        let uc: String = kesir.chars().chain("000".chars()).take(3).collect();
        uc.parse::<i64>().ok()?
    } else {
        0
    };
    Some(((gun(y, mo, d) * 24 + h) * 60 + mi) * 60_000 + se * 1000 + ms)
}

/// Tek bir döküm dosyasının içeriğinden araç çağrılarını çıkarır. SAF.
///
/// Çağrı ile sonucu AYRI satırlarda (asistan mesajı / kullanıcı mesajı)
/// durur; `tool_use_id` ile eşlenir.
pub fn dokumden_cagrilar(icerik: &str, oturum: &str, proje: &str) -> Vec<Cagri> {
    // (kimlik, araç, özet, başlangıç) — dökümdeki sırayla.
    let mut kullanim: Vec<(String, String, String, String)> = Vec::new();
    let mut sonuc: HashMap<String, (String, bool)> = HashMap::new();
    for satir in icerik.lines() {
        // Ucuz ön süzgeç: dökümün çoğu düz metin; JSON yalnız araç satırı
        // için çözülür. Asıl ayrım aşağıdaki `type` alanında — metnin içinde
        // "tool_use" geçmesi yalnız fazladan bir çözümleme maliyeti.
        if !satir.contains("\"tool_use\"") && !satir.contains("\"tool_result\"") {
            continue;
        }
        let Ok(d) = serde_json::from_str::<Value>(satir) else {
            continue; // yarım yazılmış son satır: günlük sürerken olağan
        };
        let ts = d
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(bloklar) = d.pointer("/message/content").and_then(Value::as_array) else {
            continue;
        };
        for b in bloklar {
            let metin = |a: &str| b.get(a).and_then(Value::as_str).map(str::to_string);
            match b.get("type").and_then(Value::as_str) {
                Some("tool_use") => {
                    if let (Some(id), Some(ad)) = (metin("id"), metin("name")) {
                        let ozet = b.get("input").map(ozetle).unwrap_or_default();
                        kullanim.push((id, ad, ozet, ts.to_string()));
                    }
                }
                Some("tool_result") => {
                    if let Some(id) = metin("tool_use_id") {
                        let hata = b.get("is_error").and_then(Value::as_bool) == Some(true);
                        sonuc.insert(id, (ts.to_string(), hata));
                    }
                }
                _ => {}
            }
        }
    }
    kullanim
        .into_iter()
        .map(|(kimlik, arac, ozet, baslangic)| {
            let s = sonuc.get(&kimlik);
            let sure_ms = s.and_then(|(bitis, _)| Some(iso_ms(bitis)? - iso_ms(&baslangic)?));
            Cagri {
                hata: s.is_some_and(|(_, h)| *h),
                oturum: oturum.to_string(),
                proje: proje.to_string(),
                kimlik,
                arac,
                ozet,
                baslangic,
                sure_ms,
            }
        })
        .collect()
}

/// Kök altındaki bütün `.jsonl` dökümleri (alt ajan dökümleri de dahil —
/// onların araç çağrıları da gerçek çağrılar).
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

/// Varsayılan döküm kökü: `CLAUDE_CONFIG_DIR` (Claude Code'un kendi
/// yer değiştirme değişkeni) yoksa `<home>/.claude`, altında `projects`.
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

    #[test]
    fn ozet_araca_gore_anlamli_alani_secer() {
        assert_eq!(
            ozetle(&json!({"command": "cargo test", "timeout": 5})),
            "cargo test"
        );
        assert_eq!(
            ozetle(&json!({"file_path": "src/main.rs", "old_string": "a"})),
            "src/main.rs",
            "Edit'te dosya yolu alınmalı, metin gövdesi değil"
        );
        assert_eq!(ozetle(&json!({"foo": "bar"})), "foo=bar");
        assert_eq!(ozetle(&json!({"sayi": 3})), "(sayi)");
        let uzun = ozetle(&json!({ "command": "x".repeat(500) }));
        assert!(
            uzun.contains("+300 karakter"),
            "kırptığını söylemedi: {uzun}"
        );
    }

    /// Süre bu damgalardan hesaplanıyor; biri kayarsa bütün süreler yalan olur.
    #[test]
    fn iso_ms_bilinen_anlar() {
        assert_eq!(iso_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(iso_ms("2000-01-01T00:00:00.000Z"), Some(946_684_800_000));
        // Artık gün + yarım saniye.
        assert_eq!(iso_ms("2024-02-29T12:00:00.500Z"), Some(1_709_208_000_500));
        // Kesirsiz ve kısa kesirli biçim.
        assert_eq!(iso_ms("2000-01-01T00:00:01Z"), Some(946_684_801_000));
        assert_eq!(iso_ms("2000-01-01T00:00:00.5Z"), Some(946_684_800_500));
    }

    /// Saat dilimli ya da bozuk damga sessizce yanlış süre üretmemeli.
    #[test]
    fn iso_ms_bicim_disini_reddeder() {
        assert_eq!(iso_ms("2024-02-29T12:00:00.500+03:00"), None);
        assert_eq!(iso_ms("2024-02-29 12:00:00Z"), None);
        assert_eq!(iso_ms(""), None);
        assert_eq!(iso_ms("çöp"), None);
    }

    fn satir(ts: &str, bloklar: Value) -> String {
        json!({"timestamp": ts, "message": {"content": bloklar}}).to_string()
    }

    #[test]
    fn cagri_sonucla_eslesir_sure_ve_hata_tasir() {
        let dokum = [
            satir(
                "2026-09-09T10:00:00.000Z",
                json!([{"type": "tool_use", "id": "t1", "name": "Bash", "input": {"command": "ls"}}]),
            ),
            satir(
                "2026-09-09T10:00:02.500Z",
                json!([{"type": "tool_result", "tool_use_id": "t1", "is_error": true}]),
            ),
            // Sonucu hiç gelmeyen çağrı (kesilmiş ya da o an süren).
            satir(
                "2026-09-09T10:00:03.000Z",
                json!([{"type": "tool_use", "id": "t2", "name": "Read", "input": {"file_path": "a.rs"}}]),
            ),
            "yarım yazılmış {satır \"tool_use\"".to_string(),
        ]
        .join("\n");
        let c = dokumden_cagrilar(&dokum, "oturum-1", "proje-x");
        assert_eq!(c.len(), 2, "bozuk satır çağrı sayılmamalı");
        assert_eq!(
            c[0],
            Cagri {
                kimlik: "t1".into(),
                oturum: "oturum-1".into(),
                proje: "proje-x".into(),
                arac: "Bash".into(),
                ozet: "ls".into(),
                hata: true,
                baslangic: "2026-09-09T10:00:00.000Z".into(),
                sure_ms: Some(2500),
            }
        );
        assert_eq!(c[1].sure_ms, None, "sonuçsuz çağrıya süre uydurulmamalı");
        assert!(!c[1].hata);
    }

    /// SESSİZCE YALAN SÖYLEYEN SAYIM: metinde "error" geçmesi hata DEĞİLDİR;
    /// yalnız açık `is_error: true` sayılır.
    #[test]
    fn hata_yalniz_acik_isaretle() {
        let dokum = [
            satir("2026-09-09T10:00:00.000Z", json!([{"type": "tool_use", "id": "t1", "name": "Grep", "input": {"pattern": "error"}}])),
            satir("2026-09-09T10:00:01.000Z", json!([{"type": "tool_result", "tool_use_id": "t1", "content": "src/a.rs: Error handling"}])),
        ]
        .join("\n");
        assert!(!dokumden_cagrilar(&dokum, "o", "p")[0].hata);
    }

    /// Araç çağrısı olmayan satırlar (düz sohbet) hiçbir şey üretmemeli.
    #[test]
    fn duz_metin_cagri_uretmez() {
        let dokum = satir(
            "2026-09-09T10:00:00.000Z",
            json!([{"type": "text", "text": "merhaba"}]),
        );
        assert!(dokumden_cagrilar(&dokum, "o", "p").is_empty());
        assert!(dokumden_cagrilar("", "o", "p").is_empty());
    }
}
