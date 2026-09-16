//! Katalog beslemesi — ince depo katmanı.
//!
//! el-Fihrist'te bugüne dek besleme komutu YOKTU: katalog, DB'nin yanındaki
//! Python betikleriyle elle dolduruluyordu. Bu modül depo tarafını kapatır.
//!
//! İNCE olması bilinçli: depo BAŞINA tek satır (ne yapar, hangi ikili, nereden
//! gelir). Dosya başına indeks DEĞİL — gövde yüzlerce kat büyürse brute-force
//! kosinüs çöker ve Turso'da yoğun ANN indeksi yok. Ajanın sorduğu soru zaten
//! "hangi aracım bu işi yapar", "şu satır hangi dosyada" değil; ikincisi
//! graft'ın işi.
//!
//! ponytail: kökler gezilir, `goz` KULLANILMAZ. goz'un değeri 3,97 milyon
//! dosyada disk geneli arama; burada iki kök de belli (Desktop\Github,
//! ~/.claude/skills) ve dizin gezmek kesin, dış ikilisiz ve sınanabilir.
//! "C:'deki HER SKILL.md'yi bul" istendiğinde yükseltme yolu goz'dur.

use std::path::Path;

/// Kataloğa yazılacak tek satır.
#[derive(Debug, PartialEq)]
pub struct DepoSatiri {
    pub id: String,
    pub ad: String,
    pub aciklama: String,
    pub tam_metin_md: String,
    pub icerik_hash: String,
}

/// Değişiklik anahtarı — kriptografik DEĞİL, yalnız "içerik değişti mi?".
///
/// `fnv` öneki bilerek: Python tarafının ürettiği 32 haneli anahtarlardan
/// GÖRÜNÜR biçimde ayrılsın, aynı şema sanılmasın. Sütun opak ve karşılaştırma
/// hep satırın kendi içinde (`gomme_imza == kip:icerik_hash`), satırlar arası
/// karşılaştırma hiç olmuyor — iki şemanın yan yana durması sorun değil.
pub fn icerik_anahtari(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv{h:016x}")
}

/// HTML etiketlerini söker.
fn etiketsiz(satir: &str) -> String {
    let mut cikti = String::new();
    let mut icerde = false;
    for c in satir.chars() {
        match c {
            '<' => icerde = true,
            '>' if icerde => icerde = false,
            _ if icerde => {}
            _ => cikti.push(c),
        }
    }
    cikti
}

/// Gösterilecek metin: bağlantı HEDEFLERİ (`](url)`) atılır, etiket metni kalır.
fn metin_ozu(satir: &str) -> String {
    let duz = etiketsiz(satir.trim());
    let mut baglantisiz = String::new();
    let mut kalan = duz.as_str();
    while let Some(i) = kalan.find("](") {
        baglantisiz.push_str(&kalan[..i]);
        match kalan[i + 2..].find(')') {
            Some(j) => kalan = &kalan[i + 2 + j + 1..],
            None => {
                kalan = "";
                break;
            }
        }
    }
    baglantisiz.push_str(kalan);
    baglantisiz
        .chars()
        .map(|c| match c {
            '*' | '_' | '`' | '[' | ']' | '!' | '>' | '#' | '|' => ' ',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parantez içi HER ŞEYİ atar — bağlantı ve rozet ETİKETLERİ de düşer.
///
/// Yalnız ölçmek için; gösterilmez. Rozet satırını düzyazıdan ayıran tek
/// şey bu: `[![GHA Status]][gha] [![Latest Version]][crates.io]` etiket
/// metinleriyle dört sözcüğü aşar, ama bağlantı dışında TEK sözcüğü yoktur.
fn baglanti_disi(s: &str) -> String {
    let mut cikti = String::new();
    let mut derinlik = 0u32;
    for c in s.chars() {
        match c {
            '[' | '(' => derinlik += 1,
            ']' | ')' if derinlik > 0 => derinlik -= 1,
            _ if derinlik > 0 => {}
            _ => cikti.push(c),
        }
    }
    cikti
}

/// Gerçek sözcük sayısı: tek harflik ve harfsiz parçalar sayılmaz.
fn sozcuk_sayisi(s: &str) -> usize {
    s.split_whitespace()
        .filter(|w| w.chars().count() >= 2 && w.chars().any(char::is_alphabetic))
        .count()
}

/// README'den özet çıkarır: ilk düzyazı öbeği, en çok 300 karakter.
///
/// Yasak liste TUTULMUYOR — 85 depoluk kuru koşumda rozet, `<p align=…>`,
/// `![banner](…)`, `[!WARNING]`, yalnız `\` ve dil menüsü hep AYNI nedenle
/// eleniyordu: biçim sökülünce geriye dört sözcük bile kalmıyor. Tek ölçü o:
/// **bu satırda insan cümlesi var mı?** Yeni bir rozet biçimi çıkarsa liste
/// güncellenmez, kural zaten kapsar.
pub fn ozet_cikar(readme: &str) -> Option<String> {
    let mut parcalar: Vec<String> = Vec::new();
    let mut uzunluk = 0usize;
    for ham in readme.lines() {
        let s = ham.trim();
        // Başlıklar ve dil menüsü depo adını tekrarlar, bilgi katmaz.
        if s.starts_with('#') || s.starts_with("```") || s.contains("](README") {
            continue;
        }
        // Ölçü BAĞLANTI DIŞINDAKİ metne bakar, gösterilen ise etiketleri de
        // içerir: rozet satırı elenir, içinde bağlantı geçen cümle korunur.
        let oz = metin_ozu(s);
        if sozcuk_sayisi(&baglanti_disi(&etiketsiz(s))) < 4 {
            // Toplamaya başladıysak öbek bitti; başlamadıysak henüz gürültüdeyiz.
            if parcalar.is_empty() {
                continue;
            }
            break;
        }
        uzunluk += oz.chars().count();
        parcalar.push(oz);
        if uzunluk >= 120 || parcalar.len() >= 3 {
            break;
        }
    }
    if parcalar.is_empty() {
        return None;
    }
    Some(parcalar.join(" ").chars().take(300).collect())
}

/// `Cargo.toml`daki `name = "..."` satırları: paket ve ikili adları.
///
/// Bilerek düz metin taraması — `toml` bağımlılığı eklemeye değmez.
/// `[dependencies]` girdileri `serde = "1.0"` biçiminde olduğu için buraya
/// düşmez; yalnız `[package]` ve `[[bin]]`/`[[example]]` `name = ` kullanır.
pub fn adlar_cargo(cargo_toml: &str) -> Vec<String> {
    cargo_toml
        .lines()
        .filter_map(|l| {
            let s = l.trim();
            let deger = s
                .strip_prefix("name")?
                .trim_start()
                .strip_prefix('=')?
                .trim();
            let ad = deger.trim_matches('"');
            (!ad.is_empty() && !ad.contains('"')).then(|| ad.to_string())
        })
        .collect()
}

/// Toplanan parçalardan satırı kurar. SAF: disk gerekmez, sınanabilir.
pub fn depo_satiri(
    ad: &str,
    uzak: Option<&str>,
    ozet: Option<&str>,
    ikililer: &[String],
) -> DepoSatiri {
    let aciklama = ozet.unwrap_or("(README özeti yok)").to_string();
    // Aranan metin: ad + ad'ın sözcüklere ayrılmış hâli (tire/alt çizgi ile
    // yazılan depo adı böyle de bulunur) + özet + ikili adları + uzak adres.
    let tam_metin_md = format!(
        "{ad} {} {aciklama} {} {}",
        ad.replace(['-', '_', '/', '.'], " "),
        ikililer.join(" "),
        uzak.unwrap_or_default()
    );
    DepoSatiri {
        id: format!("depo/{ad}"),
        ad: ad.to_string(),
        icerik_hash: icerik_anahtari(&tam_metin_md),
        aciklama,
        tam_metin_md,
    }
}

/// Kök altındaki git depolarını bulur (`.git` taşıyan dizinler).
///
/// `derinlik` kökten itibaren kaç kat inileceği. Bir depo bulunca İÇİNE
/// İNMEZ: submodule/vendor kopyaları ayrı depo olarak sayılmasın.
pub fn depolari_bul(kok: &Path, derinlik: usize) -> Vec<std::path::PathBuf> {
    let mut bulunan = Vec::new();
    in_(kok, derinlik, &mut bulunan);
    bulunan.sort();
    return bulunan;

    fn in_(dizin: &Path, kalan: usize, cikti: &mut Vec<std::path::PathBuf>) {
        if dizin.join(".git").exists() {
            cikti.push(dizin.to_path_buf());
            return; // iç içe depoya inme
        }
        if kalan == 0 {
            return;
        }
        let Ok(girdiler) = std::fs::read_dir(dizin) else {
            return;
        };
        for g in girdiler.flatten() {
            let y = g.path();
            let ad = g.file_name();
            let ad = ad.to_string_lossy();
            // Ağır ve anlamsız dizinlere hiç girme.
            if ad.starts_with('.') || matches!(&*ad, "target" | "node_modules" | "vendor") {
                continue;
            }
            if y.is_dir() {
                in_(&y, kalan - 1, cikti);
            }
        }
    }
}

/// Bir depo dizininden satır üretir (diske dokunan ince kabuk).
pub fn dizinden_satir(yol: &Path) -> Option<DepoSatiri> {
    let ad = yol.file_name()?.to_string_lossy().to_string();
    let oku = |d: &str| std::fs::read_to_string(yol.join(d)).ok();
    let ozet = ["README.md", "README.en.md", "readme.md"]
        .iter()
        .find_map(|d| oku(d))
        .and_then(|m| ozet_cikar(&m));
    let ikililer = oku("Cargo.toml")
        .map(|c| adlar_cargo(&c))
        .unwrap_or_default();
    // `.git/config`ten uzak adres — `git` çağırmadan, süreç başlatmadan.
    let uzak = oku(".git/config").and_then(|c| {
        c.lines()
            .find_map(|l| l.trim().strip_prefix("url = ").map(str::to_string))
    });
    Some(depo_satiri(
        &ad,
        uzak.as_deref(),
        ozet.as_deref(),
        &ikililer,
    ))
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Üç gerçek README deseni — hepsinde doğru satır seçilmeli.
    #[test]
    fn ozet_uc_desende_de_dogru_satiri_bulur() {
        // 1) Rozetli (goz)
        let goz =
            "# goz\n\n[![CI](https://x/y.svg)](https://x)\n[![License: MIT](https://a)](L)\n\n\
                   **goz** is an instant file-search engine for Windows NTFS.\n";
        assert!(ozet_cikar(goz).unwrap().contains("file-search engine"));

        // 2) Dil menülü (kervan)
        let kervan = "**🌍 [Türkçe](README.md) | [English](README.en.md)**\n\n# Kervan\n\n\
                      Web üzerinden yapay zekâ kullanımını yönlendiren geçit altyapısı.\n";
        assert!(ozet_cikar(kervan).unwrap().starts_with("Web üzerinden"));

        // 3) Alıntı başlıklı (agent-reach-rs)
        let arr = "**🌍 [Türkçe](README.md)**\n\n# Agent Reach RS\n\n\
                   > Ajanlara interneti görecek gözler veren saf Rust okuma motoru\n";
        assert_eq!(
            ozet_cikar(arr).unwrap(),
            "Ajanlara interneti görecek gözler veren saf Rust okuma motoru",
            "'>' işareti soyulmalı"
        );
    }

    /// 85 depoluk kuru koşumda ÖZET YERİNE GELEN gerçek çöp satırlar.
    /// Hepsi düzyazı ölçüsüne takılıp elenmeli, aşağıdaki cümle seçilmeli.
    #[test]
    fn bicim_gurultusu_ozet_sayilmaz() {
        for gurultu in [
            "<p align=\"center\">",
            "<h1 align=\"center\" style=\"margin:0;\">",
            "<div align=\"center\">",
            "<details>",
            "![llama](https://raw.githubusercontent.com/ggml-org/cover/llama.png)",
            "[!WARNING]",
            "\\",
            "regex",
            "|--------|--------|",
            // Rozet dizisi: etiket metinleri dört sözcüğü aşar ama bağlantı
            // dışında tek sözcük yoktur — libc, rust-clippy, ratatui, tesseract.
            "[![GHA Status]][gha] [![Latest Version]][crates.io] [![License]][lic]",
            "[Website][web] | [Getting started][gs] | [Learn][l] | [Contributing][c]",
        ] {
            let m = format!("# Başlık\n\n{gurultu}\n\nBu depo şunu yapar: dosya arar.\n");
            assert_eq!(
                ozet_cikar(&m).as_deref(),
                Some("Bu depo şunu yapar: dosya arar."),
                "elenemedi: {gurultu}"
            );
        }
    }

    #[test]
    fn ozet_bulunamazsa_none() {
        assert_eq!(ozet_cikar("# Yalnız başlık\n\n---\n"), None);
        assert_eq!(
            ozet_cikar("<p align=\"center\">\n<img src=\"a.png\">\n"),
            None
        );
        assert_eq!(ozet_cikar(""), None);
    }

    #[test]
    fn cargo_adlari_paket_ve_ikilileri_alir() {
        let c = "[package]\nname = \"el-fihrist\"\nversion = \"0.1.0\"\n\n\
                 [[bin]]\nname = \"ibnunnedim\"\npath = \"src/main.rs\"\n\n\
                 [dependencies]\nserde = \"1.0\"\nturso = { version = \"0.7\" }\n";
        assert_eq!(adlar_cargo(c), vec!["el-fihrist", "ibnunnedim"]);
    }

    /// SESSİZCE BOZULAN SÖZLEŞME: içerik değişince anahtar DEĞİŞMELİ, yoksa
    /// `ibnunnedim gomme` satırı bayat saymaz ve vektör asla yenilenmez.
    #[test]
    fn icerik_degisince_anahtar_degisir() {
        let a = depo_satiri("goz", None, Some("dosya arama"), &[]);
        let b = depo_satiri("goz", None, Some("dosya arama motoru"), &[]);
        let ayni = depo_satiri("goz", None, Some("dosya arama"), &[]);
        assert_ne!(
            a.icerik_hash, b.icerik_hash,
            "özet değişti, anahtar değişmedi"
        );
        assert_eq!(
            a.icerik_hash, ayni.icerik_hash,
            "aynı girdi aynı anahtar vermeli"
        );
        assert!(a.icerik_hash.starts_with("fnv"), "şema görünür olmalı");
    }

    #[test]
    fn satir_kimligi_ve_aranan_metin() {
        let s = depo_satiri(
            "agent-reach-rs",
            Some("https://github.com/x/agent-reach-rs.git"),
            Some("web okuma motoru"),
            &["agent-reach-mcp".into()],
        );
        assert_eq!(s.id, "depo/agent-reach-rs");
        // Tireli ad sözcüklere de ayrılmalı ki "agent reach" sorgusu bulsun.
        assert!(s.tam_metin_md.contains("agent reach rs"));
        assert!(s.tam_metin_md.contains("agent-reach-mcp"));
        assert!(s.tam_metin_md.contains("web okuma motoru"));
    }

    #[test]
    fn ozet_yoksa_aciklama_durustce_soyler() {
        let s = depo_satiri("bos-depo", None, None, &[]);
        assert_eq!(s.aciklama, "(README özeti yok)");
    }
}
