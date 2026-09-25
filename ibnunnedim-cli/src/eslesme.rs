//! Olay günlüğü ↔ Claude Code dökümü eşleştirici (F2c).
//!
//! Olay günlüğü (olay.rs) "ne soruldu, ne döndü"yü bilir ama gerçek döküm
//! oturumunu bilmez: Claude Code MCP sunucusuna oturum kimliği geçirmiyor.
//! Döküm (kayit.rs → `arac_cagrilari`) oturumu ve SONRASINI bilir: YZ dönen
//! sonuçlardan hangisini gerçekten çağırdı. İkisi birleşince her arama için
//! "bu aradığım araç mıydı" etiketi çıkar — Laya ince ayarının (F4) verisi.
//!
//! Bağlama: aynı araç + aynı parametre özeti (iki tarafta da `kayit::ozetle`,
//! kırpma farkı olmasın) + zamanca en yakın, en çok `TOLERANS_MS`.
//! Claude Desktop ve Antigravity dökümü burada yok: onların olayları
//! "dökümsüz" kalır, sonuçları yine günlükte.

use crate::kayit;
use serde_json::{json, Value};

/// Dökümdeki `tool_use` anı ile hizmetin isteği aldığı an arası: aktarıcı +
/// izin sorusu. Ölçülen ilk gerçek çağrıda ~4 sn.
pub const TOLERANS_MS: i64 = 60_000;
/// Aramadan sonra "kullanıldı" sayılacak pencere; sonraki aramada da kapanır.
pub const PENCERE_MS: i64 = 30 * 60_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Olay {
    pub oturum: String,
    pub arayuz: String,
    pub zaman_ms: i64,
    pub arac: String,
    pub ozet: String,
    pub sonuclar: Vec<String>,
}

impl Olay {
    /// Olay günlüğü satırından. Bozuk satır `None` — atlanır.
    pub fn satirdan(s: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(s).ok()?;
        Some(Self {
            oturum: v["oturum"].as_str()?.to_string(),
            arayuz: v["arayuz"].as_str().unwrap_or("bilinmiyor").to_string(),
            zaman_ms: v["zaman_ms"].as_i64()?,
            arac: v["arac"].as_str()?.to_string(),
            ozet: kayit::ozetle(&v["arguman"]),
            sonuclar: v["sonuclar"]
                .as_array()
                .map(|d| {
                    d.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

/// `arac_cagrilari` satırı, zaman milisaniyeye çevrilmiş.
#[derive(Debug, Clone, PartialEq)]
pub struct DokumCagri {
    pub oturum: String,
    pub arac: String,
    pub ozet: String,
    pub zaman_ms: i64,
}

fn dokum_adi(arac: &str) -> String {
    format!("mcp__el-fihrist__{arac}")
}

/// Olayın dökümdeki karşılığı: zamanca en yakın aday.
// ponytail: bir döküm çağrısı iki olaya da verilebilir; aynı sorgu aynı dakikada iki kez gelirse ayrıştırılmaz.
pub fn eslestir<'a>(o: &Olay, cagrilar: &'a [DokumCagri]) -> Option<&'a DokumCagri> {
    let ad = dokum_adi(&o.arac);
    cagrilar
        .iter()
        .filter(|c| c.arac == ad && c.ozet == o.ozet)
        .filter(|c| (c.zaman_ms - o.zaman_ms).abs() <= TOLERANS_MS)
        .min_by_key(|c| (c.zaman_ms - o.zaman_ms).abs())
}

/// Sonuç kimliği bu döküm çağrısıyla kullanılmış mı?
/// `mcp/<sunucu>/<araç>` → `mcp__…<sunucu>…__<araç>` (büyük/küçük harf
/// duyarsız: kimlik `codspeed`, araç adı `mcp__CodSpeed__…`; ikisi de ASCII).
/// Yetenek (`claude-code/…/<ad>`) → `Skill` çağrısı ya da SKILL.md okuması.
pub fn kullanir_mi(kimlik: &str, c: &DokumCagri) -> bool {
    let parca: Vec<&str> = kimlik.split('/').collect();
    let arac = c.arac.to_ascii_lowercase();
    if let ["mcp", sunucu, ad] = parca.as_slice() {
        return arac.starts_with("mcp__")
            && arac.ends_with(&format!("__{}", ad.to_ascii_lowercase()))
            && arac.contains(&sunucu.to_ascii_lowercase());
    }
    let Some(ad) = parca.last().filter(|_| parca.len() >= 2) else {
        return false;
    };
    match c.arac.as_str() {
        "Skill" => c.ozet == *ad || c.ozet.ends_with(&format!(":{ad}")),
        "Read" => c
            .ozet
            .replace('\\', "/")
            .ends_with(&format!("/{ad}/SKILL.md")),
        _ => false,
    }
}

/// Eşleşen aramadan sonra aynı oturumda kullanılan sonuçlar, sonuç sırasıyla.
/// `oturum_cagrilari` zamana göre sıralı olmalı.
pub fn kullanilanlar(
    sonuclar: &[String],
    eslesen: &DokumCagri,
    oturum_cagrilari: &[DokumCagri],
) -> Vec<String> {
    let arama = dokum_adi("search_skills");
    let sonra: Vec<&DokumCagri> = oturum_cagrilari
        .iter()
        .filter(|c| c.zaman_ms > eslesen.zaman_ms && c.zaman_ms - eslesen.zaman_ms <= PENCERE_MS)
        .take_while(|c| c.arac != arama)
        .collect();
    sonuclar
        .iter()
        .filter(|k| sonra.iter().any(|c| kullanir_mi(k, c)))
        .cloned()
        .collect()
}

/// Tek çıktı satırı: olay, dökümdeki karşılığı (yoksa null) ve kullanılanlar.
pub fn satir(o: &Olay, eslesen: Option<&DokumCagri>, kullanilan: &[String]) -> Value {
    json!({
        "olay_oturum": o.oturum,
        "arayuz": o.arayuz,
        "zaman_ms": o.zaman_ms,
        "sorgu": o.ozet,
        "sonuclar": o.sonuclar,
        "dokum_oturum": eslesen.map(|c| &c.oturum),
        "fark_ms": eslesen.map(|c| c.zaman_ms - o.zaman_ms),
        "kullanilan": kullanilan,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    fn c(oturum: &str, arac: &str, ozet: &str, zaman_ms: i64) -> DokumCagri {
        DokumCagri {
            oturum: oturum.into(),
            arac: arac.into(),
            ozet: ozet.into(),
            zaman_ms,
        }
    }

    fn olay(zaman_ms: i64) -> Olay {
        Olay::satirdan(&format!(
            r#"{{"oturum":"42-1","arayuz":"claude-code","zaman_ms":{zaman_ms},"arac":"search_skills","arguman":{{"query":"tez başlığı","limit":2}},"sonuclar":["claude-code/tez-bolum","mcp/codspeed/compare_runs","claude-code/yok"]}}"#
        ))
        .unwrap()
    }

    #[test]
    fn zamanca_en_yakin_ayni_sorgu_eslesir() {
        let d = [
            c(
                "uzak",
                "mcp__el-fihrist__search_skills",
                "tez başlığı",
                100_000 - 50_000,
            ),
            c(
                "yakin",
                "mcp__el-fihrist__search_skills",
                "tez başlığı",
                100_000 - 4_000,
            ),
            c(
                "baska",
                "mcp__el-fihrist__search_skills",
                "başka sorgu",
                100_000,
            ),
            c(
                "gec",
                "mcp__el-fihrist__search_skills",
                "tez başlığı",
                100_000 + 70_000,
            ),
        ];
        assert_eq!(eslestir(&olay(100_000), &d).unwrap().oturum, "yakin");
        assert!(
            eslestir(&olay(1_000_000), &d).is_none(),
            "tolerans dışı eşleşmez"
        );
    }

    #[test]
    fn kullanilan_sonuclar_pencerede_ve_sonraki_aramaya_kadar() {
        let e = c("o", "mcp__el-fihrist__search_skills", "tez başlığı", 0);
        let oturum = [
            e.clone(),
            c("o", "Bash", "ls", 1),
            c("o", "mcp__CodSpeed__compare_runs", "base=1", 2),
            c("o", "mcp__el-fihrist__search_skills", "başka", 3),
            c("o", "Skill", "tez-bolum", 4), // sonraki aramadan sonra: sayılmaz
        ];
        let s = olay(0).sonuclar;
        assert_eq!(
            kullanilanlar(&s, &e, &oturum),
            ["mcp/codspeed/compare_runs"]
        );
        let oturum2 = [
            e.clone(),
            c("o", "Skill", "plugin:tez-bolum", PENCERE_MS + 1),
        ];
        assert!(kullanilanlar(&s, &e, &oturum2).is_empty(), "pencere dışı");
    }

    #[test]
    fn yetenek_skill_ya_da_skill_md_okumasiyla_kullanilir() {
        assert!(kullanir_mi(
            "claude-code/tez-bolum",
            &c("o", "Skill", "plugin:tez-bolum", 0)
        ));
        assert!(kullanir_mi(
            "claude-code/synced/docx",
            &c("o", "Read", r"C:\Users\x\.claude\skills\docx\SKILL.md", 0)
        ));
        assert!(!kullanir_mi(
            "claude-code/docx",
            &c("o", "Skill", "docx-eski", 0)
        ));
        assert!(!kullanir_mi(
            "mcp/codspeed/get_run",
            &c("o", "mcp__CodSpeed__compare_runs", "", 0)
        ));
    }
}
