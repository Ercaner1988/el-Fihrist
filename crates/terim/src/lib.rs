//! BM25 dizin terimleri. `ibnunnedim-cli` sandık sınırında (altin-kapi,
//! 3363 kod satırı) olduğu için ayrı crate.
//!
//! F5: katla belirteçlerinin ilk 5 harfi (Can vd., JASIST 2008 — Türkçede
//! biçimbilim çözümleyicisine yakın sonuç). Türkçe ekleri kaba ama tutarlı
//! keser (belgesi/belge → belge). Belge ve sorgu ikisi de buradan geçer;
//! BM25 dizini yeniden belirteçlese de terimler değişmez (katlanmış, ≤ 5 harf).
//!
//! Ölçüm (sozcuk-aile 55f083a; katla sonrası 2026-09-26 yeniden, aynı): 26 altın
//! sorgu, ilk 5 — BM25 13/26 MRR 0,365 → 14/26 0,481, 6 kazanç / 1 kayıp,
//! p 0,13. Anlamlı değil, ama denenen 19 yapılandırmadaki en büyük ve riski
//! en düşük kazanç.

/// Metni F5 terimlerine çevirip boşlukla birleştirir.
pub fn f5_metin(metin: &str) -> String {
    katla::belirtecle(metin)
        .iter()
        .map(|t| t.chars().take(5).collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ekleri_keser_ve_yeniden_belirteclemede_degismez() {
        assert_eq!(f5_metin("Belgesi belge"), "belge belge");
        let bir = f5_metin("Toplantı notlarından görev çıkar");
        assert_eq!(
            f5_metin(&bir),
            bir,
            "dizin yeniden belirteçleyince değişmemeli"
        );
        assert_eq!(katla::belirtecle(&bir).join(" "), bir);
    }
}
