//! Altın küme koşumu — ÖLÇER, karar vermez.
//!
//! Kural (pasli-beyin PB-06 ile aynı): yeni bir arama kanalı ya da gömme
//! çekirdeği AYNI sette ölçülmeden varsayılan seçilmez. Üç kanal da aramanın
//! kendi `sirala`sından geçer; ölçüm kopya bir sıralayıcı tutmaz.

/// Altın kümedeki tek sorgu.
#[derive(Debug, PartialEq)]
pub struct Sorgu {
    pub metin: String,
    /// Beklenen kimlik PARÇALARI (alt dize). ALTERNATİF — biri yeterli.
    pub beklenen: Vec<String>,
}

/// `[[sorgu]]` bloklarını okur.
///
/// Elle ayrıştırma: `toml` bağımlılığı yalnız bu dosya için gelirdi ve biçim
/// bizim — üç anahtar, kaçış yok, tek satırlık diziler. `tara::adlar_cargo`
/// ile aynı gerekçe.
/// ponytail: `beklenen` çok satıra yayılırsa bu ayrıştırıcı onu görmez;
/// yükseltme yolu `toml` crate'i. Bugün sette öyle bir satır yok.
pub fn ayristir(metin: &str) -> Vec<Sorgu> {
    let mut sorgular = Vec::new();
    let mut son_metin: Option<String> = None;
    for ham in metin.lines() {
        let s = ham.trim();
        if s.starts_with('#') {
            continue;
        }
        if let Some(d) = s.strip_prefix("metin") {
            if let Some(v) = tirnak_ici(d) {
                son_metin = v.into_iter().next();
            }
        } else if let Some(d) = s.strip_prefix("beklenen") {
            // `metin` görülmeden `beklenen` gelirse blok bozuktur; sessizce
            // bir öncekine iliştirmek yanlış sorguyu ölçmek olurdu.
            if let (Some(m), Some(b)) = (son_metin.take(), tirnak_ici(d)) {
                if !b.is_empty() {
                    sorgular.push(Sorgu {
                        metin: m,
                        beklenen: b,
                    });
                }
            }
        }
    }
    sorgular
}

/// `= ...` sağındaki çift tırnaklı parçalar. `=` yoksa None (başka anahtar).
fn tirnak_ici(deger: &str) -> Option<Vec<String>> {
    let d = deger.trim_start().strip_prefix('=')?;
    Some(
        d.split('"')
            .skip(1)
            .step_by(2)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

/// Bir sorgunun puanı: `(isabet 0/1, karşılıklı sıra 1/r)`.
///
/// `beklenen` ALTERNATİFTİR, hepsi şart değil: set varyantları eşanlamlı
/// yazıyor (`codebase-inspection` zaten `rust-codebase-inspection`i de
/// kapsıyor), hepsini istemek aynı belgeyi iki kez istemek olurdu.
/// **pasli-beyin'den ayrılan yer burası** — orada her varyant ayrı sayılır,
/// bu yüzden iki deponun payda sayıları doğrudan karşılaştırılamaz.
///
/// MRR şart: füzyonun tüm etkisi bulunmuş ama 5. sıradaki sonucu 1.'ye
/// taşımak olabilir; geri çağrım@5 bunu göremez.
pub fn puanla(sirali: &[String], beklenen: &[String]) -> (usize, f64) {
    for (i, kimlik) in sirali.iter().enumerate() {
        if beklenen.iter().any(|b| kimlik.contains(b.as_str())) {
            return (1, 1.0 / (i + 1) as f64);
        }
    }
    (0, 0.0)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn puan_sirayi_yansitir() {
        let sirali: Vec<String> = ["a/bir", "b/iki", "c/uc"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(puanla(&sirali, &["c/uc".into()]), (1, 1.0 / 3.0));
        assert_eq!(puanla(&sirali, &["a/bir".into()]), (1, 1.0));
        assert_eq!(puanla(&sirali, &["yok".into()]), (0, 0.0));
        assert_eq!(puanla(&[], &["a/bir".into()]), (0, 0.0));
    }

    /// Alternatif anlamı: biri bulunduysa isabet, ve İLK bulunanın sırası sayar.
    #[test]
    fn beklenen_alternatiftir() {
        let sirali: Vec<String> = ["x/bir", "y/iki"].iter().map(|s| s.to_string()).collect();
        assert_eq!(puanla(&sirali, &["yok".into(), "y/iki".into()]), (1, 0.5));
        // Alt dize eşleşmesi: kısa parça uzun kimliği de yakalar.
        assert_eq!(puanla(&sirali, &["bir".into()]), (1, 1.0));
    }

    #[test]
    fn set_ayristirma() {
        let s = r#"
# yorum satırı: metin = "tuzak"
[[sorgu]]
metin = "docx"
beklenen = ["productivity/docx", "rust-docx-openxml"]

[[sorgu]]
metin = "obsidian"
beklenen = ["note-taking/obsidian"]
"#;
        let g = ayristir(s);
        assert_eq!(g.len(), 2, "yorum satırı sorgu sayılmamalı");
        assert_eq!(g[0].metin, "docx");
        assert_eq!(g[0].beklenen.len(), 2);
        assert_eq!(g[1].beklenen, vec!["note-taking/obsidian"]);
    }

    /// SESSİZCE BOZULAN SÖZLEŞME: `beklenen`i olmayan blok sorguya
    /// dönüşmemeli, yoksa ölçüm hiç kimsenin yazmadığı bir beklentiyi ölçer.
    #[test]
    fn eksik_alan_sorgu_uretmez() {
        assert!(ayristir("[[sorgu]]\nmetin = \"yalnız metin\"\n").is_empty());
        assert!(ayristir("[[sorgu]]\nbeklenen = [\"yalnız beklenen\"]\n").is_empty());
        assert!(ayristir("").is_empty());
    }
}
