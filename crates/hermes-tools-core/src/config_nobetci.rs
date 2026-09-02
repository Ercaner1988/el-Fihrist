//! Yapılandırma nöbeti: `config.yaml` gibi dosyaların sürüm sürüm karşılaştırması.
//!
//! Amaç, bir güncellemenin yapılandırmayı sessizce bozup bozmadığını yakalamak.
//! Akış üç parçadır:
//!
//! 1. Kapanışta dosyanın metni saklanır (çağıranın işi).
//! 2. Açılışta saklanan metinle güncel metin [`nobet_tut`] ile karşılaştırılır.
//! 3. Dönen [`NobetRaporu`] neyin değiştiğini ve **ne kadar şüpheli** olduğunu söyler.
//!
//! Bu modül **arı (pure)**: dosya okumaz, yazmaz. Metin alır, rapor verir.
//!
//! # Kapsam sınırı — dürüstçe
//!
//! Bu bir YAML **ayrıştırıcısı değildir**. Girinti, blok sözdizimi, çapa (anchor)
//! ya da çok satırlı dizgi (`|`, `>`) semantiğini anlamaz. Yaptığı iş, üst düzey
//! anahtarları yüzeysel tarayıp satır bazlı fark çıkarmaktır. Gerçek bir söz
//! dizimi denetimi istiyorsan dosyayı ayrıca bir YAML ayrıştırıcısına ver;
//! [`SuphePuani::SozDizimiSuphesi`] yalnızca *kaba* belirtileri yakalar.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ToolError, ToolResult};

/// Bir satırın iki sürüm arasındaki durumu.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SatirDurumu {
    /// Yeni sürümde eklenmiş.
    Eklendi,
    /// Yeni sürümde kaybolmuş.
    Silindi,
}

impl SatirDurumu {
    /// Türkçe kısa etiket.
    pub fn etiket(&self) -> &'static str {
        match self {
            SatirDurumu::Eklendi => "EKLENDİ",
            SatirDurumu::Silindi => "SİLİNDİ",
        }
    }
}

/// Değişen tek bir satır.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SatirFarki {
    /// 1'den başlayan satır numarası (kendi sürümünde).
    pub satir_no: usize,
    /// Eklendi mi silindi mi.
    pub durum: SatirDurumu,
    /// Satırın kırpılmamış içeriği.
    pub icerik: String,
}

/// Nöbetin genel sonucu.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuphePuani {
    /// İki sürüm birebir aynı.
    Degismemis,
    /// Değişim var ama beklenen türden (anahtar eklenmiş, değer güncellenmiş).
    Olagan,
    /// Üst düzey anahtar kaybolmuş ya da dosya ciddi biçimde küçülmüş.
    IcerikKaybi,
    /// Kaba söz dizimi belirtisi: sekme ile girinti, boş dosya, NUL baytı.
    SozDizimiSuphesi,
}

impl SuphePuani {
    /// Kullanıcıya sorulması gereken bir durum mu?
    pub fn dikkat_ister(&self) -> bool {
        matches!(self, SuphePuani::IcerikKaybi | SuphePuani::SozDizimiSuphesi)
    }

    /// Türkçe kısa etiket.
    pub fn etiket(&self) -> &'static str {
        match self {
            SuphePuani::Degismemis => "DEĞİŞMEMİŞ",
            SuphePuani::Olagan => "OLAĞAN DEĞİŞİM",
            SuphePuani::IcerikKaybi => "İÇERİK KAYBI",
            SuphePuani::SozDizimiSuphesi => "SÖZ DİZİMİ ŞÜPHESİ",
        }
    }
}

/// Nöbet sonucunun tamamı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NobetRaporu {
    /// Genel değerlendirme.
    pub suphe: SuphePuani,
    /// Önceki sürümün bayt uzunluğu.
    pub onceki_boyut: usize,
    /// Güncel sürümün bayt uzunluğu.
    pub guncel_boyut: usize,
    /// Önceki sürümün satır sayısı.
    pub onceki_satir: usize,
    /// Güncel sürümün satır sayısı.
    pub guncel_satir: usize,
    /// Kaybolan üst düzey anahtarlar (alfabetik).
    pub kaybolan_anahtarlar: Vec<String>,
    /// Eklenen üst düzey anahtarlar (alfabetik).
    pub eklenen_anahtarlar: Vec<String>,
    /// Satır bazlı farklar; `azami_fark` ile sınırlanır.
    pub farklar: Vec<SatirFarki>,
    /// Fark listesi sınıra takılıp kırpıldı mı.
    pub farklar_kirpildi: bool,
    /// İnsan okunur gerekçeler.
    pub gerekceler: Vec<String>,
}

impl NobetRaporu {
    /// Kullanıcıya bildirim gerekiyor mu?
    pub fn bildirilmeli(&self) -> bool {
        self.suphe != SuphePuani::Degismemis
    }

    /// Tek satırlık özet.
    pub fn ozet(&self) -> String {
        match self.suphe {
            SuphePuani::Degismemis => "Yapılandırma değişmemiş.".to_string(),
            _ => format!(
                "{}: {} satır → {} satır, {} anahtar kayıp, {} anahtar yeni",
                self.suphe.etiket(),
                self.onceki_satir,
                self.guncel_satir,
                self.kaybolan_anahtarlar.len(),
                self.eklenen_anahtarlar.len()
            ),
        }
    }
}

/// Küçülme bu oranı aşarsa içerik kaybı sayılır (yüzde 25).
const KUCULME_ESIGI: f64 = 0.75;

/// İki sürümü karşılaştırır.
///
/// `azami_fark`, dönen satır farkı sayısını sınırlar; 0 verilirse fark listesi
/// boş kalır ama özet ve anahtar karşılaştırması yine çalışır. Bu, koca
/// dosyalarda raporun şişmesini engeller.
///
/// # Hatalar
///
/// Güncel metin boş ama önceki doluysa [`ToolError::ValidationFailed`]; bu,
/// karşılaştırılacak bir şey olmadığı anlamına gelir ve sessizce "değişti"
/// demek yanıltıcı olur.
pub fn nobet_tut(onceki: &str, guncel: &str, azami_fark: usize) -> ToolResult<NobetRaporu> {
    if guncel.trim().is_empty() && !onceki.trim().is_empty() {
        return Err(ToolError::ValidationFailed(
            "Güncel yapılandırma boş; önceki sürüm doluydu. Dosya silinmiş ya da sıfırlanmış olabilir".into(),
        ));
    }

    let onceki_satirlar: Vec<&str> = onceki.lines().collect();
    let guncel_satirlar: Vec<&str> = guncel.lines().collect();

    let onceki_anahtarlar = ust_duzey_anahtarlar(onceki);
    let guncel_anahtarlar = ust_duzey_anahtarlar(guncel);

    let kaybolan: Vec<String> = onceki_anahtarlar
        .difference(&guncel_anahtarlar)
        .cloned()
        .collect();
    let eklenen: Vec<String> = guncel_anahtarlar
        .difference(&onceki_anahtarlar)
        .cloned()
        .collect();

    let (farklar, kirpildi) = satir_farklari(&onceki_satirlar, &guncel_satirlar, azami_fark);

    let mut gerekceler = Vec::new();
    let mut suphe = if onceki == guncel {
        SuphePuani::Degismemis
    } else {
        SuphePuani::Olagan
    };

    // Kaba söz dizimi belirtileri — en ağır bulgu, diğerlerini ezer.
    if guncel.contains('\0') {
        suphe = SuphePuani::SozDizimiSuphesi;
        gerekceler.push("Dosyada NUL baytı var; ikili bozulma belirtisi".into());
    }
    if let Some(no) = sekmeyle_girintili_satir(&guncel_satirlar) {
        suphe = SuphePuani::SozDizimiSuphesi;
        gerekceler.push(format!(
            "Satır {no}: girintide sekme karakteri var; YAML sekme kabul etmez"
        ));
    }

    // İçerik kaybı — söz dizimi şüphesi yoksa geçerli.
    if suphe != SuphePuani::SozDizimiSuphesi {
        if !kaybolan.is_empty() {
            suphe = SuphePuani::IcerikKaybi;
            gerekceler.push(format!(
                "Üst düzey anahtar kayboldu: {}",
                kaybolan.join(", ")
            ));
        }
        if !onceki.is_empty() && (guncel.len() as f64) < (onceki.len() as f64) * KUCULME_ESIGI {
            suphe = SuphePuani::IcerikKaybi;
            let yuzde = 100.0 - (guncel.len() as f64 / onceki.len() as f64 * 100.0);
            gerekceler.push(format!(
                "Dosya %{yuzde:.0} küçüldü ({} → {} bayt)",
                onceki.len(),
                guncel.len()
            ));
        }
    }

    if suphe == SuphePuani::Olagan {
        if !eklenen.is_empty() {
            gerekceler.push(format!("Yeni anahtar: {}", eklenen.join(", ")));
        }
        if gerekceler.is_empty() {
            gerekceler.push(format!(
                "{} satır değişti, üst düzey anahtarlar yerinde",
                farklar.len()
            ));
        }
    }

    Ok(NobetRaporu {
        suphe,
        onceki_boyut: onceki.len(),
        guncel_boyut: guncel.len(),
        onceki_satir: onceki_satirlar.len(),
        guncel_satir: guncel_satirlar.len(),
        kaybolan_anahtarlar: kaybolan,
        eklenen_anahtarlar: eklenen,
        farklar,
        farklar_kirpildi: kirpildi,
        gerekceler,
    })
}

/// Üst düzey anahtarları yüzeysel tarar: girintisiz, `ad:` biçimindeki satırlar.
///
/// Yorum satırlarını, liste öğelerini (`- x`) ve belge ayırıcılarını (`---`)
/// atlar. Çok satırlı dizgi bloklarının içindeki metni ayırt **edemez**; bu,
/// bilinçli bir sadelik ödünüdür (bkz. modül başlığındaki kapsam sınırı).
fn ust_duzey_anahtarlar(metin: &str) -> BTreeSet<String> {
    let mut anahtarlar = BTreeSet::new();
    for satir in metin.lines() {
        // Girintili satır üst düzey değildir.
        if satir.starts_with(' ') || satir.starts_with('\t') {
            continue;
        }
        let kirpik = satir.trim_end();
        if kirpik.is_empty() || kirpik.starts_with('#') || kirpik.starts_with("---") {
            continue;
        }
        if kirpik.starts_with('-') {
            continue;
        }
        if let Some(ikinokta) = kirpik.find(':') {
            let ad = &kirpik[..ikinokta];
            if !ad.is_empty()
                && ad
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                anahtarlar.insert(ad.to_string());
            }
        }
    }
    anahtarlar
}

/// Girintisinde sekme bulunan ilk satırın numarasını döndürür.
fn sekmeyle_girintili_satir(satirlar: &[&str]) -> Option<usize> {
    satirlar.iter().enumerate().find_map(|(i, s)| {
        let girinti: String = s.chars().take_while(|c| c.is_whitespace()).collect();
        if girinti.contains('\t') {
            Some(i + 1)
        } else {
            None
        }
    })
}

/// Satır bazlı fark: çokluk sayımıyla eklenen/silinen satırları bulur.
///
/// Sıralamayı değil **içeriği** karşılaştırır; satır taşınması fark üretmez.
/// Bu, yapılandırma dosyaları için istenen davranıştır (anahtar sırası anlamsız).
fn satir_farklari(onceki: &[&str], guncel: &[&str], azami: usize) -> (Vec<SatirFarki>, bool) {
    let mut onceki_sayim: BTreeMap<&str, isize> = BTreeMap::new();
    for s in onceki {
        *onceki_sayim.entry(s).or_insert(0) += 1;
    }
    for s in guncel {
        *onceki_sayim.entry(s).or_insert(0) -= 1;
    }

    let mut farklar = Vec::new();

    // Silinenler: önceki sürümde fazla kalanlar.
    let mut kalan_silinen: BTreeMap<&str, isize> = onceki_sayim
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(s, n)| (*s, *n))
        .collect();
    for (i, s) in onceki.iter().enumerate() {
        if let Some(n) = kalan_silinen.get_mut(s) {
            if *n > 0 {
                *n -= 1;
                farklar.push(SatirFarki {
                    satir_no: i + 1,
                    durum: SatirDurumu::Silindi,
                    icerik: (*s).to_string(),
                });
            }
        }
    }

    // Eklenenler: güncel sürümde fazla olanlar.
    let mut kalan_eklenen: BTreeMap<&str, isize> = onceki_sayim
        .iter()
        .filter(|(_, n)| **n < 0)
        .map(|(s, n)| (*s, -*n))
        .collect();
    for (i, s) in guncel.iter().enumerate() {
        if let Some(n) = kalan_eklenen.get_mut(s) {
            if *n > 0 {
                *n -= 1;
                farklar.push(SatirFarki {
                    satir_no: i + 1,
                    durum: SatirDurumu::Eklendi,
                    icerik: (*s).to_string(),
                });
            }
        }
    }

    farklar.sort_by_key(|f| (f.satir_no, f.durum == SatirDurumu::Eklendi));

    let kirpildi = farklar.len() > azami;
    if kirpildi {
        farklar.truncate(azami);
    }
    (farklar, kirpildi)
}

#[cfg(test)]
mod testler {
    use super::*;

    const ORNEK: &str = "model:\n  default: hermes-beyin\nproviders:\n  omniroute:\n    key: x\ntools:\n  enabled: true\n";

    #[test]
    fn ayni_metin_degismemis_doner() {
        let r = nobet_tut(ORNEK, ORNEK, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::Degismemis);
        assert!(!r.bildirilmeli());
        assert!(r.farklar.is_empty());
        assert_eq!(r.ozet(), "Yapılandırma değişmemiş.");
    }

    #[test]
    fn deger_degisimi_olagan() {
        let yeni = ORNEK.replace("hermes-beyin", "gpt-5");
        let r = nobet_tut(ORNEK, &yeni, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::Olagan);
        assert!(r.bildirilmeli());
        assert!(!r.suphe.dikkat_ister());
        assert!(r.kaybolan_anahtarlar.is_empty());
        assert_eq!(r.farklar.len(), 2); // bir silinen, bir eklenen
    }

    #[test]
    fn ust_duzey_anahtar_kaybi_yakalanir() {
        let yeni = "model:\n  default: hermes-beyin\ntools:\n  enabled: true\n";
        let r = nobet_tut(ORNEK, yeni, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::IcerikKaybi);
        assert!(r.suphe.dikkat_ister());
        assert_eq!(r.kaybolan_anahtarlar, vec!["providers".to_string()]);
        assert!(r.gerekceler.iter().any(|g| g.contains("providers")));
    }

    #[test]
    fn yeni_anahtar_olagan_sayilir() {
        let yeni = format!("{ORNEK}yeni_bolum:\n  a: 1\n");
        let r = nobet_tut(ORNEK, &yeni, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::Olagan);
        assert_eq!(r.eklenen_anahtarlar, vec!["yeni_bolum".to_string()]);
        assert!(r.kaybolan_anahtarlar.is_empty());
    }

    #[test]
    fn ciddi_kuculme_icerik_kaybi() {
        let uzun = "kok:\n".to_string() + &"  a: 1\n".repeat(200);
        let kisa = "kok:\n  a: 1\n";
        let r = nobet_tut(&uzun, kisa, 10).unwrap();
        assert_eq!(r.suphe, SuphePuani::IcerikKaybi);
        assert!(r.gerekceler.iter().any(|g| g.contains("küçüldü")));
    }

    #[test]
    fn sekme_girintisi_soz_dizimi_suphesi() {
        let yeni = "model:\n\tdefault: hermes-beyin\n";
        let r = nobet_tut("model:\n  default: x\n", yeni, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::SozDizimiSuphesi);
        assert!(r.gerekceler.iter().any(|g| g.contains("sekme")));
    }

    #[test]
    fn nul_bayti_soz_dizimi_suphesi() {
        let r = nobet_tut("a: 1\n", "a: 1\n\0", 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::SozDizimiSuphesi);
        assert!(r.gerekceler.iter().any(|g| g.contains("NUL")));
    }

    #[test]
    fn soz_dizimi_suphesi_icerik_kaybini_ezer() {
        // Hem anahtar kaybı hem sekme var; en ağır bulgu kazanmalı.
        let yeni = "model:\n\tdefault: x\n";
        let r = nobet_tut(ORNEK, yeni, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::SozDizimiSuphesi);
    }

    #[test]
    fn bos_guncel_metin_hata_verir() {
        let hata = nobet_tut(ORNEK, "   \n", 10).unwrap_err();
        assert!(matches!(hata, ToolError::ValidationFailed(_)));
    }

    #[test]
    fn ilk_kurulumda_bos_onceki_sorun_degil() {
        // Önceki yoksa (ilk çalıştırma) hata verilmemeli.
        let r = nobet_tut("", ORNEK, 100).unwrap();
        assert_eq!(r.suphe, SuphePuani::Olagan);
        assert_eq!(r.onceki_satir, 0);
    }

    #[test]
    fn yorum_ve_liste_satirlari_anahtar_sayilmaz() {
        let metin = "# yorum: bu anahtar degil\n- liste: ogesi\n---\ngercek: 1\n";
        let anahtarlar = ust_duzey_anahtarlar(metin);
        assert_eq!(anahtarlar.len(), 1);
        assert!(anahtarlar.contains("gercek"));
    }

    #[test]
    fn girintili_anahtar_ust_duzey_sayilmaz() {
        let metin = "kok:\n  ic_anahtar: 1\nikinci:\n";
        let anahtarlar = ust_duzey_anahtarlar(metin);
        assert_eq!(
            anahtarlar,
            ["ikinci".to_string(), "kok".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn fark_listesi_kirpilir() {
        let onceki = (0..50).map(|i| format!("a{i}: 1\n")).collect::<String>();
        let guncel = (0..50).map(|i| format!("b{i}: 2\n")).collect::<String>();
        let r = nobet_tut(&onceki, &guncel, 5).unwrap();
        assert_eq!(r.farklar.len(), 5);
        assert!(r.farklar_kirpildi);
    }

    #[test]
    fn azami_sifir_farksiz_rapor_verir() {
        let yeni = ORNEK.replace("true", "false");
        let r = nobet_tut(ORNEK, &yeni, 0).unwrap();
        assert!(r.farklar.is_empty());
        assert!(r.farklar_kirpildi);
        // Fark listesi boş olsa da karşılaştırma çalışmalı.
        assert_eq!(r.suphe, SuphePuani::Olagan);
    }

    #[test]
    fn satir_tasinmasi_fark_uretmez() {
        let onceki = "a: 1\nb: 2\nc: 3\n";
        let guncel = "c: 3\na: 1\nb: 2\n";
        let r = nobet_tut(onceki, guncel, 100).unwrap();
        assert!(r.farklar.is_empty(), "sıra değişimi fark sayılmamalı");
        assert_eq!(r.suphe, SuphePuani::Olagan);
    }

    #[test]
    fn etiketler_turkce() {
        assert_eq!(SuphePuani::IcerikKaybi.etiket(), "İÇERİK KAYBI");
        assert_eq!(SuphePuani::SozDizimiSuphesi.etiket(), "SÖZ DİZİMİ ŞÜPHESİ");
        assert_eq!(SatirDurumu::Silindi.etiket(), "SİLİNDİ");
    }
}
