//! el-Fihrist ↔ Tikel Gözlemci: ek arama kanalı olarak yerel dosya İÇERİĞİ.
//!
//! el-Fihrist'in kendi kayıt/yetenek/kural araması (BM25 + gömme) değişmez.
//! Tikel sonuçları AYRI BÖLÜM olarak gelir, `harmanla` ile karıştırılmaz: iki
//! kanalın derlemi başka (yetenek kaydı ↔ kullanıcı dosyalarının paragrafları),
//! skorlar aynı ölçekte değil; "normalize ağırlıklı karışım" bu yüzden anlamsız.
//!
//! `tikeld` kapalıysa SESSİZ DÜŞÜLMEZ: bölüm "KAPALI" nedenini söyler.

use std::time::Duration;
use tikel_istemci::tikel_proto::{AramaYaniti, Duzey, Sorgu};
use tikel_istemci::Istemci;

/// Daemon kabul edip yanıtlamazsa arama sonsuza dek asılmasın.
const ZAMAN_ASIMI: Duration = Duration::from_secs(8);
/// Paragraf özeti üst sınırı (karakter).
const OZET: usize = 200;

/// Karakter sınırında kırpar ve **kırptığını söyler**.
///
/// Bayt dilimi (`&s[..n]`) çok baytlı UTF-8'in ortasına düşerse panikler; bu
/// kütüphanenin metinleri Türkçe. Sessiz kırpma da sayıyı gizler — ne kadarının
/// gizlendiği çıktıya yazılır. (Tek tanım: ibnunnedim-cli de bunu kullanır.)
pub fn kisalt(s: &str, azami: usize) -> String {
    let toplam = s.chars().count();
    if toplam <= azami {
        return s.to_string();
    }
    let kesik: String = s.chars().take(azami).collect();
    format!("{kesik}… (+{} karakter)", toplam - azami)
}

/// Tikel'e anlam destekli arama sorar. Hata metni "tikel kapalı" nedenidir.
pub async fn ara(sorgu: &str, limit: usize) -> Result<AramaYaniti, String> {
    let mut s = Sorgu::yeni(sorgu, Duzey::Anlam);
    s.limit = limit;
    let is = async {
        let mut istemci = Istemci::baglan_varsayilan().await?;
        istemci.ara_ayrintili(&s).await
    };
    match tokio::time::timeout(ZAMAN_ASIMI, is).await {
        Ok(r) => r.map_err(|h| h.to_string()),
        Err(_) => Err(format!("{} sn içinde yanıt yok", ZAMAN_ASIMI.as_secs())),
    }
}

/// Arama çıktısına eklenecek bölüm; `acik` değilse boş (tikel'e hiç sorulmaz).
pub async fn ek_bolum(acik: bool, sorgu: &str, limit: usize) -> String {
    if !acik {
        return String::new();
    }
    bolum_metni(ara(sorgu, limit).await)
}

/// Saf: tikel yanıtını (ya da hatasını) insan-okunur bölüme çevirir.
pub fn bolum_metni(yanit: Result<AramaYaniti, String>) -> String {
    const BASLIK: &str = "\nYEREL DOSYA İÇERİĞİ (tikel)";
    let y = match yanit {
        Ok(y) => y,
        Err(neden) => {
            return format!(
                "{BASLIK} — KAPALI: {neden}. Dosya içi sonuç YOK; yalnız kayıt/yetenek arandı.\n"
            );
        }
    };
    let mut s = format!(
        "{BASLIK} — {} sonuç · {} ms · ayrı kanal: skorlar yukarıdaki kayıt skorlarıyla karşılaştırılamaz\n",
        y.sonuclar.len(),
        y.sure_ms
    );
    for u in &y.uyarilar {
        s.push_str(&format!("  ! tikel uyarı: {u}\n"));
    }
    for r in &y.sonuclar {
        s.push_str(&format!(
            "  [{} {:.3}] {}",
            r.kanal,
            r.skor,
            r.yol.display()
        ));
        if let Some(yaprak) = r.yaprak {
            s.push_str(&format!(" (yaprak {yaprak})"));
        }
        s.push('\n');
        if let Some(p) = &r.eslesen_paragraf {
            let tek: Vec<&str> = p.split_whitespace().collect();
            s.push_str(&format!("     {}\n", kisalt(&tek.join(" "), OZET)));
        }
    }
    s
}

#[cfg(test)]
mod testler {
    use super::*;
    use tikel_istemci::tikel_proto::AramaSonucu;

    #[test]
    fn kapali_daemon_nedenini_soyler() {
        let m = bolum_metni(Err("boru yok".into()));
        assert!(m.contains("KAPALI: boru yok"), "{m}");
        assert!(m.contains("YOK"), "{m}");
    }

    #[test]
    fn sonuclar_ayri_bolum_olarak_yazilir() {
        let y = AramaYaniti {
            sonuclar: vec![AramaSonucu {
                yol: r"C:\tez\kuhn.pdf".into(),
                eslesen_paragraf: Some("paradigma   kayması\nbilimde".into()),
                yaprak: Some(12),
                skor: 0.8,
                kanal: "harman".into(),
                ..AramaSonucu::default()
            }],
            uyarilar: vec!["anlam kanalı kapalı".into()],
            sure_ms: 4,
            ..AramaYaniti::default()
        };
        let m = bolum_metni(Ok(y));
        assert!(m.contains("1 sonuç · 4 ms"), "{m}");
        assert!(m.contains("! tikel uyarı: anlam kanalı kapalı"), "{m}");
        assert!(
            m.contains(r"[harman 0.800] C:\tez\kuhn.pdf (yaprak 12)"),
            "{m}"
        );
        assert!(m.contains("paradigma kayması bilimde"), "{m}");
    }

    #[test]
    fn kapaliyken_hic_sorulmaz() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        assert_eq!(rt.block_on(ek_bolum(false, "x", 3)), "");
    }

    #[test]
    fn kisalt_kirptigini_soyler() {
        assert_eq!(kisalt("abc", 5), "abc");
        assert_eq!(kisalt("çğıöşü", 2), "çğ… (+4 karakter)");
    }
}
