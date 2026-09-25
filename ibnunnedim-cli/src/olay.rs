//! Oturum olay günlüğü (F2a) — el-Fihrist'e hangi arayüzden, hangi oturumdan,
//! hangi sorguyla gelindi ve ne döndü.
//!
//! **Veritabanı DEĞİL, süreç başına JSONL.** Her YZ oturumu kendi stdio MCP
//! sürecini açıyor (2026-09-24: 10 süreç); turso dosyayı süreç başına kilitliyor
//! ve yazma o an diğerleriyle çakışıyor (os error 33). Claude Code'un kendi
//! dökümü gibi: yalnız eklenen, süreç başına bir dosya → kilit hiç devreye
//! girmez. İçe aktarım (ve dökümlerle birleştirme) ayrı adımdır (F2c).
//!
//! **Oturum kimliği.** Claude Code stdio MCP sunucusuna oturum kimliği
//! geçirmiyor (belgelenen tek değişken `CLAUDE_PROJECT_DIR`, code.claude.com/
//! docs/en/mcp.md). Kimlik: `AGIT_SESSION` varsa o, yoksa `<pid>-<başlangıç ms>`.
//! Gerçek döküm oturumuna bağlama, sorgu metni + zaman üzerinden dökümdeki
//! aynı `tool_use` ile yapılır (F2c).
//!
//! `arayuz`, MCP `initialize`'daki `clientInfo.name`'dir — ne gönderildiği
//! belgelenmemiş; ilk gerçek bağlantılar bu günlükte ölçer.

use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Bir MCP bağlantısının (= bir YZ oturumunun) bağlamı. stdio'da süreç
/// başına bir tane; hizmette (F2b) TCP bağlantısı başına bir tane.
#[derive(Debug, Clone, PartialEq)]
pub struct Baglam {
    /// `initialize`'da öğrenilen `clientInfo` (ad, sürüm).
    pub arayuz: Option<(String, String)>,
    pub oturum: String,
    pub proje: Option<String>,
}

impl Baglam {
    /// Bu sürecin kendi bağlamı: `AGIT_SESSION` ya da `<pid>-<şimdi ms>`.
    pub fn yerel() -> Self {
        Self {
            arayuz: None,
            oturum: std::env::var("AGIT_SESSION")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("{}-{}", std::process::id(), simdi_ms())),
            proje: std::env::var("CLAUDE_PROJECT_DIR").ok(),
        }
    }

    /// Aktarıcının gönderdiği `el-fihrist/baglam` parametresinden. Oturum
    /// kimliği aktarıcının sürecine aittir (onun pid'i), hizmetinkine değil.
    pub fn uzaktan(p: &Value) -> Self {
        Self {
            arayuz: None,
            oturum: p["oturum"]
                .as_str()
                .filter(|s| !s.is_empty() && !s.contains(['/', '\\', '.']))
                .map(str::to_string)
                .unwrap_or_else(|| format!("uzak-{}", simdi_ms())),
            proje: p["proje"].as_str().map(str::to_string),
        }
    }

    /// `initialize` parametresinden `clientInfo`'yu saklar.
    pub fn arayuzu_kaydet(&mut self, parametre: &Value) {
        let ad = parametre["clientInfo"]["name"]
            .as_str()
            .unwrap_or("bilinmiyor");
        let surum = parametre["clientInfo"]["version"].as_str().unwrap_or("");
        self.arayuz = Some((ad.to_string(), surum.to_string()));
    }
}

pub fn simdi_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Arama sonucundan (`"N / M kayıt · ms\n[...]"`) dönen kimlikler, sırasıyla.
/// Biçim tanınmazsa boş — günlük eksik kalır, çağrı bozulmaz.
pub fn sonuc_kimlikleri(metin: &str) -> Vec<String> {
    let Some((_, govde)) = metin.split_once('\n') else {
        return Vec::new();
    };
    serde_json::from_str::<Value>(govde)
        .ok()
        .and_then(|v| {
            v.as_array().map(|d| {
                d.iter()
                    .filter_map(|o| o["id"].as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Uzun argümanları (kural metni gibi) kırpar; günlük küçük kalsın.
fn kirp(arg: &Value) -> Value {
    match arg {
        Value::String(s) if s.chars().count() > 500 => {
            Value::String(s.chars().take(500).collect::<String>() + "…")
        }
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), kirp(v))).collect()),
        diger => diger.clone(),
    }
}

/// Tek olay satırı. SAF: sınanabilir.
#[allow(clippy::too_many_arguments)]
pub fn olay_satiri(
    zaman_ms: i64,
    arayuz: Option<&(String, String)>,
    oturum: &str,
    proje: Option<&str>,
    arac: &str,
    arg: &Value,
    sonuc: &Result<String, String>,
    sure_ms: i64,
) -> Value {
    let (ad, surum) = arayuz
        .map(|(a, s)| (a.as_str(), s.as_str()))
        .unwrap_or(("bilinmiyor", ""));
    json!({
        "zaman_ms": zaman_ms,
        "arayuz": ad,
        "arayuz_surum": surum,
        "oturum": oturum,
        "proje": proje,
        "arac": arac,
        "arguman": kirp(arg),
        "sonuclar": sonuc.as_ref().map(|t| sonuc_kimlikleri(t)).unwrap_or_default(),
        "hata": sonuc.as_ref().err(),
        "sure_ms": sure_ms,
    })
}

/// Olay günlüklerinin dizini: kütüphanenin yanında `kutup_olaylar/`.
pub fn dizin() -> Option<PathBuf> {
    Some(crate::kutuphane_yolu().ok()?.with_file_name("kutup_olaylar"))
}

fn gunluk_yolu(oturum: &str) -> Option<PathBuf> {
    let kok = dizin()?;
    std::fs::create_dir_all(&kok).ok()?;
    Some(kok.join(format!("{oturum}.jsonl")))
}

/// Olayı oturumun günlüğüne ekler (oturum başına bir dosya, tek yazar). Hata
/// YUTULUR (stderr'e yazılır): günlük yazılamadı diye araç çağrısı düşmemeli.
pub fn yaz(b: &Baglam, arac: &str, arg: &Value, sonuc: &Result<String, String>, baslangic_ms: i64) {
    let satir = olay_satiri(
        baslangic_ms,
        b.arayuz.as_ref(),
        &b.oturum,
        b.proje.as_deref(),
        arac,
        arg,
        sonuc,
        simdi_ms() - baslangic_ms,
    );
    let sonuc = gunluk_yolu(&b.oturum)
        .ok_or_else(|| "günlük dizini yok".to_string())
        .and_then(|y| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(y)
                .and_then(|mut f| writeln!(f, "{satir}"))
                .map_err(|e| e.to_string())
        });
    if let Err(e) = sonuc {
        eprintln!("! olay günlüğü yazılamadı: {e}");
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn arama_sonucundan_kimlikler_sirasiyla() {
        let t = "2 / 675 kayıt · 390.2 ms\n[{\"id\":\"claude-code/mentese\"},{\"id\":\"claude-code/tez-bolum\"}]";
        assert_eq!(
            sonuc_kimlikleri(t),
            ["claude-code/mentese", "claude-code/tez-bolum"]
        );
        assert!(sonuc_kimlikleri("biçimsiz metin").is_empty());
    }

    #[test]
    fn olay_satiri_alanlari_ve_kirpma() {
        let arayuz = ("claude-code".to_string(), "2.1.281".to_string());
        let uzun = "ş".repeat(600);
        let s = olay_satiri(
            1,
            Some(&arayuz),
            "42-1",
            Some("C:/proje"),
            "kural_ekle",
            &json!({"metin": uzun, "limit": 5}),
            &Err("hata".into()),
            7,
        );
        assert_eq!(s["arayuz"], "claude-code");
        assert_eq!(s["oturum"], "42-1");
        assert_eq!(s["hata"], "hata");
        assert_eq!(s["sonuclar"], json!([]));
        assert_eq!(s["arguman"]["limit"], 5);
        assert_eq!(
            s["arguman"]["metin"].as_str().unwrap().chars().count(),
            501,
            "500 + …"
        );
    }

    #[test]
    fn uzak_oturum_kimligi_yol_karakteri_tasiyamaz() {
        // Kimlik dosya adına dönüşüyor: aktarıcıdan gelen "../x" günlük dizininden kaçmasın.
        let b = Baglam::uzaktan(&json!({"oturum": "../../x", "proje": "C:/p"}));
        assert!(b.oturum.starts_with("uzak-"), "{}", b.oturum);
        assert_eq!(b.proje.as_deref(), Some("C:/p"));
        assert_eq!(
            Baglam::uzaktan(&json!({"oturum": "123-456"})).oturum,
            "123-456"
        );
    }

    #[test]
    fn arayuz_bilinmiyorsa_acikca_yazilir() {
        let s = olay_satiri(1, None, "o", None, "info", &json!({}), &Ok("x".into()), 0);
        assert_eq!(
            s["arayuz"], "bilinmiyor",
            "boşluk mutabakat değildir: bilinmeyen açık yazılır"
        );
    }
}
