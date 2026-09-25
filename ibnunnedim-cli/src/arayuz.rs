//! Ortak katalog beslemesi (F1) — YZ arayüzlerinin yetenek ve araçları.
//!
//! Claude Code, Claude Desktop ve Antigravity'nin kurulu yeteneklerini
//! (SKILL.md) ve Antigravity'nin MCP araç önbelleğini TEK biçimde okur.
//! Aynı içerik birden çok arayüzde kuruluysa TEK satır olur, hangi
//! arayüzlerde bulunduğu `arayuzler`de tutulur (CodSpeed, penpot, agent-reach
//! üç arayüzde birden kurulu).
//!
//! Hermes'in `yetenekler` tablosuna YAZILMAZ: ana kütüphanede fts5 var ve
//! Turso onu uygulamadığı için yeni satırı indeks görmüyor (bkz. main.rs
//! `depo_baglan`). Satırlar kendi yan dosyasına gider (`kutup_ortak.db`).
//!
//! Gizli anahtar tutan ayar dosyalarından (`~/.claude.json`,
//! `claude_desktop_config.json`, `mcp_config.json`) YALNIZ `mcpServers`
//! anahtar adları okunur; `env`/`headers` değerlerine dokunulmaz.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::tara::icerik_anahtari;

/// Ortak kataloğun tek satırı.
#[derive(Debug, Clone, PartialEq)]
pub struct OrtakSatir {
    pub id: String,
    /// "yetenek" ya da "arac".
    pub tur: &'static str,
    pub ad: String,
    pub aciklama: String,
    pub tam_metin_md: String,
    pub icerik_hash: String,
    /// Bu içeriğin kurulu olduğu arayüzler (sıralı, virgülle yazılır).
    pub arayuzler: BTreeSet<String>,
    /// İlk görüldüğü dosya — getirmek ve hata ayıklamak için.
    pub yol: String,
}

// ------------------------------------------------------------ ön bilgi --

/// SKILL.md'nin YAML ön bilgisinden `name` ve `description`'ı çıkarır; gövdeyi
/// de döndürür. Tam bir YAML ayrıştırıcısı DEĞİL — yalnız üst düzey iki anahtar:
/// düz değer, tırnaklı değer, girintili devam satırları ve blok skalerler
/// (`>`, `|`, `>-`, `|-`). Bugünkü katalogda bir kaydın açıklaması yalnız `>`
/// görünüyordu: blok skaler okunmamıştı.
pub fn on_bilgi(md: &str) -> Option<(String, String, String)> {
    let md = md.strip_prefix('\u{feff}').unwrap_or(md);
    let mut satirlar = md.lines();
    if satirlar.next()?.trim_end() != "---" {
        return None;
    }
    let mut ust: Vec<&str> = Vec::new();
    let mut kapandi = false;
    for s in satirlar.by_ref() {
        if s.trim_end() == "---" {
            kapandi = true;
            break;
        }
        ust.push(s);
    }
    if !kapandi {
        return None;
    }
    let govde: String = satirlar.collect::<Vec<_>>().join("\n");
    let ad = anahtar(&ust, "name")?;
    let aciklama = anahtar(&ust, "description").unwrap_or_default();
    Some((ad, aciklama, govde.trim().to_string()))
}

fn anahtar(ust: &[&str], ad: &str) -> Option<String> {
    let bas = ust
        .iter()
        .position(|s| s.strip_prefix(ad).is_some_and(|k| k.starts_with(':')))?;
    let deger = ust[bas][ad.len() + 1..].trim();
    // Devam satırları: bir sonraki üst düzey anahtara (girintisiz satır) dek.
    let devam: Vec<&str> = ust[bas + 1..]
        .iter()
        .take_while(|s| s.trim().is_empty() || s.starts_with([' ', '\t']))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let metin = if matches!(deger, ">" | "|" | ">-" | "|-" | ">+" | "|+") || deger.is_empty() {
        devam.join(" ")
    } else if let Some(t) = tirnaksiz(deger) {
        t
    } else {
        std::iter::once(deger)
            .chain(devam)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let metin = metin.trim().to_string();
    (!metin.is_empty()).then_some(metin)
}

fn tirnaksiz(d: &str) -> Option<String> {
    for q in ['"', '\''] {
        if d.len() >= 2 && d.starts_with(q) && d.ends_with(q) {
            return Some(d[1..d.len() - 1].replace("\\\"", "\"").replace("''", "'"));
        }
    }
    None
}

// -------------------------------------------------------------- birleştirme --

/// Aynı içeriği (icerik_hash) tek satıra indirir, arayüzlerini birleştirir.
/// İlk görülenin kimliği ve yolu kalır; girdi sırası kaynak önceliğidir.
pub fn birlestir(satirlar: Vec<OrtakSatir>) -> Vec<OrtakSatir> {
    let mut sira: Vec<String> = Vec::new();
    let mut tablo: BTreeMap<String, OrtakSatir> = BTreeMap::new();
    let mut kimlikler: BTreeSet<String> = BTreeSet::new();
    for mut s in satirlar {
        if let Some(var) = tablo.get_mut(&s.icerik_hash) {
            var.arayuzler.append(&mut s.arayuzler);
            continue;
        }
        // Aynı kimlik, FARKLI içerik (iki eklentide aynı adlı yetenek): ikincisi
        // ezmesin, numaralansın.
        let mut id = s.id.clone();
        let mut n = 2;
        while !kimlikler.insert(id.clone()) {
            id = format!("{}#{n}", s.id);
            n += 1;
        }
        s.id = id;
        sira.push(s.icerik_hash.clone());
        tablo.insert(s.icerik_hash.clone(), s);
    }
    sira.into_iter().filter_map(|h| tablo.remove(&h)).collect()
}

// ------------------------------------------------------------- disk kabuğu --

/// Bir yetenek kaynağı: arayüz adı, kök, kimliğin öneki.
struct Kaynak {
    arayuz: &'static str,
    kok: PathBuf,
}

fn kaynaklar(ev: &Path, appdata: Option<&Path>) -> Vec<Kaynak> {
    let mut k = vec![
        Kaynak {
            arayuz: "claude-code",
            kok: ev.join(".claude/skills"),
        },
        Kaynak {
            arayuz: "claude-code",
            kok: ev.join(".claude/plugins/cache"),
        },
        Kaynak {
            arayuz: "antigravity",
            kok: ev.join(".gemini/antigravity/builtin/skills"),
        },
        Kaynak {
            arayuz: "antigravity",
            kok: ev.join(".gemini/config/plugins"),
        },
    ];
    // Yalnız kurulu eklenti yetenekleri; oturumların outputs/ klasörlerindeki
    // SKILL.md'ler taslaktır (2026-09-25 sayıldı: 42 taslak, 28 kurulu).
    if let Some(a) = appdata {
        k.push(Kaynak {
            arayuz: "claude-desktop",
            kok: a.join("Claude/local-agent-mode-sessions/skills-plugin"),
        });
    }
    k
}

/// Kök altındaki SKILL.md dosyaları (sıralı). Noktalı ve ağır dizinlere girmez.
fn skill_dosyalari(kok: &Path, derinlik: usize) -> Vec<PathBuf> {
    let mut bulunan = Vec::new();
    in_(kok, derinlik, &mut bulunan);
    bulunan.sort();
    return bulunan;

    fn in_(dizin: &Path, kalan: usize, cikti: &mut Vec<PathBuf>) {
        let aday = dizin.join("SKILL.md");
        if aday.is_file() {
            cikti.push(aday);
        }
        if kalan == 0 {
            return;
        }
        let Ok(girdiler) = std::fs::read_dir(dizin) else {
            return;
        };
        for g in girdiler.flatten() {
            let ad = g.file_name();
            let ad = ad.to_string_lossy();
            if ad.starts_with('.') || matches!(&*ad, "target" | "node_modules") {
                continue;
            }
            let y = g.path();
            if y.is_dir() {
                in_(&y, kalan - 1, cikti);
            }
        }
    }
}

/// Kimlik: `<arayüz>/<kökten göreli yol>` — "skills" parçaları ve sürüm
/// klasörleri atılır ki kimlik okunur kalsın (`claude-code/codspeed/codspeed-optimize`).
fn kimlik(arayuz: &str, kok: &Path, dosya: &Path) -> String {
    let goreli = dosya
        .parent()
        .and_then(|d| d.strip_prefix(kok).ok())
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .filter(|c| c != "skills" && !c.chars().next().is_some_and(|h| h.is_ascii_digit()))
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default();
    if goreli.is_empty() {
        arayuz.to_string()
    } else {
        format!("{arayuz}/{goreli}")
    }
}

pub fn yetenekleri_oku(ev: &Path, appdata: Option<&Path>) -> Vec<OrtakSatir> {
    let mut cikti = Vec::new();
    for k in kaynaklar(ev, appdata) {
        for dosya in skill_dosyalari(&k.kok, 8) {
            let Ok(md) = std::fs::read_to_string(&dosya) else {
                continue;
            };
            let Some((ad, aciklama, govde)) = on_bilgi(&md) else {
                eprintln!("! ön bilgisi okunamadı, atlandı: {}", dosya.display());
                continue;
            };
            let tam_metin_md = format!("{ad}\n{aciklama}\n\n{govde}");
            cikti.push(OrtakSatir {
                id: kimlik(k.arayuz, &k.kok, &dosya),
                tur: "yetenek",
                icerik_hash: icerik_anahtari(&tam_metin_md),
                ad,
                aciklama,
                tam_metin_md,
                arayuzler: BTreeSet::from([k.arayuz.to_string()]),
                yol: dosya.to_string_lossy().to_string(),
            });
        }
    }
    cikti
}

/// Bir ayar dosyasının `mcpServers` anahtar adları (küçük harf). Değerler okunmaz.
fn sunucu_adlari(yol: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(yol)
        .ok()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok())
        .and_then(|v| {
            v.get("mcpServers")
                .and_then(|s| s.as_object())
                .map(|o| o.keys().map(|k| k.to_lowercase()).collect())
        })
        .unwrap_or_default()
}

/// Antigravity'nin MCP araç önbelleği: `mcp/<sunucu>/<araç>.json`
/// ({name, description, parameters}). Sunucuyu çalıştırmadan araç kataloğu.
pub fn araclari_oku(ev: &Path, appdata: Option<&Path>) -> Vec<OrtakSatir> {
    let diger: Vec<(&str, BTreeSet<String>)> = vec![
        ("claude-code", sunucu_adlari(&ev.join(".claude.json"))),
        (
            "claude-desktop",
            appdata
                .map(|a| sunucu_adlari(&a.join("Claude/claude_desktop_config.json")))
                .unwrap_or_default(),
        ),
    ];
    let kok = ev.join(".gemini/antigravity/mcp");
    let Ok(sunucular) = std::fs::read_dir(&kok) else {
        return Vec::new();
    };
    let mut dizinler: Vec<PathBuf> = sunucular
        .flatten()
        .map(|g| g.path())
        .filter(|p| p.is_dir())
        .collect();
    dizinler.sort();
    let mut cikti = Vec::new();
    for dizin in dizinler {
        let sunucu = dizin
            .file_name()
            .map(|a| a.to_string_lossy().to_string())
            .unwrap_or_default();
        // Eklenti sunucuları `<eklenti>_<sunucu>` diye önbelleğe alınıyor.
        let kanonik = sunucu.rsplit('_').next().unwrap_or(&sunucu).to_lowercase();
        let mut arayuzler = BTreeSet::from(["antigravity".to_string()]);
        for (ad, adlar) in &diger {
            if adlar.contains(&kanonik) {
                arayuzler.insert((*ad).to_string());
            }
        }
        let Ok(dosyalar) = std::fs::read_dir(&dizin) else {
            continue;
        };
        let mut dosyalar: Vec<PathBuf> = dosyalar
            .flatten()
            .map(|g| g.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        dosyalar.sort();
        for dosya in dosyalar {
            let Some(v) = std::fs::read_to_string(&dosya)
                .ok()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok())
            else {
                continue;
            };
            let ad = v["name"].as_str().unwrap_or_default().to_string();
            if ad.is_empty() {
                continue;
            }
            let aciklama = v["description"].as_str().unwrap_or_default().to_string();
            let parametreler: Vec<String> = v["parameters"]["properties"]
                .as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            // Hash sunucu kimliğini içermez: aynı araç iki önbellek dizininde (CodSpeed,
            // codspeed_CodSpeed) tek satıra insin.
            let tam_metin_md = format!(
                "{kanonik} {ad}\n{aciklama}\nparametreler: {}",
                parametreler.join(", ")
            );
            cikti.push(OrtakSatir {
                id: format!("mcp/{kanonik}/{ad}"),
                tur: "arac",
                icerik_hash: icerik_anahtari(&tam_metin_md),
                ad,
                aciklama,
                tam_metin_md,
                arayuzler: arayuzler.clone(),
                yol: dosya.to_string_lossy().to_string(),
            });
        }
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn blok_skaler_aciklama_okunur() {
        let md = "---\nname: turkish-academic-pdf\ndescription: >\n  Türkçe akademik PDF'ten\n  metin çıkarır.\nlicense: MIT\n---\n# Gövde\n";
        let (ad, aciklama, govde) = on_bilgi(md).unwrap();
        assert_eq!(ad, "turkish-academic-pdf");
        assert_eq!(
            aciklama, "Türkçe akademik PDF'ten metin çıkarır.",
            "'>' tek başına açıklama olmamalı"
        );
        assert_eq!(govde, "# Gövde");
    }

    #[test]
    fn tirnakli_ve_devam_satirli_deger() {
        let md = "---\nname: \"docx\"\ndescription: Word belgesi\n  okur ve yazar\nmetadata:\n  name: ic-ice\n---\n";
        let (ad, aciklama, _) = on_bilgi(md).unwrap();
        assert_eq!(ad, "docx", "iç içe `name` üst düzeyi ezmemeli");
        assert_eq!(aciklama, "Word belgesi okur ve yazar");
    }

    #[test]
    fn on_bilgisiz_ya_da_kapanmamis_dosya_none() {
        assert!(on_bilgi("# başlık yok\n").is_none());
        assert!(on_bilgi("---\nname: x\n").is_none());
        assert!(on_bilgi("---\ndescription: adsız\n---\n").is_none());
    }

    fn satir(id: &str, hash: &str, arayuz: &str) -> OrtakSatir {
        OrtakSatir {
            id: id.into(),
            tur: "yetenek",
            ad: id.into(),
            aciklama: String::new(),
            tam_metin_md: String::new(),
            icerik_hash: hash.into(),
            arayuzler: BTreeSet::from([arayuz.to_string()]),
            yol: String::new(),
        }
    }

    #[test]
    fn ayni_icerik_tek_satir_arayuzler_birlesir() {
        let s = birlestir(vec![
            satir("claude-code/codspeed", "h1", "claude-code"),
            satir("antigravity/codspeed", "h1", "antigravity"),
            satir("claude-code/docx", "h2", "claude-code"),
        ]);
        assert_eq!(s.len(), 2);
        assert_eq!(
            s[0].id, "claude-code/codspeed",
            "ilk görülenin kimliği kalır"
        );
        assert_eq!(s[0].arayuzler.len(), 2);
    }

    #[test]
    fn ayni_kimlik_farkli_icerik_numaralanir() {
        let s = birlestir(vec![
            satir("claude-code/docx", "h1", "claude-code"),
            satir("claude-code/docx", "h2", "claude-code"),
        ]);
        assert_eq!(
            s.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
            ["claude-code/docx", "claude-code/docx#2"]
        );
    }

    #[test]
    fn kimlik_skills_ve_surum_parcalarini_atar() {
        let kok = Path::new("/k/cache");
        let d = Path::new(
            "/k/cache/claude-plugins-official/codspeed/1.0.0/skills/codspeed-optimize/SKILL.md",
        );
        assert_eq!(
            kimlik("claude-code", kok, d),
            "claude-code/claude-plugins-official/codspeed/codspeed-optimize"
        );
    }
}
