//! Ayna güdümlü ayıklama: silmeden önce yedeği say, sonra sil.
//!
//! Bir çalışma dizinini (`hedef`) bir ayna dizinle (`ayna`, örn. OneDrive kopyası)
//! karşılaştırır. Üç aşama vardır ve **sıra bağlayıcıdır**:
//!
//! 1. [`ayikla_plani_kur`] — her adayı ayna sayısıyla karşılaştırıp
//!    [`AyiklamaKarari`] üretir; **hiçbir şey silmez**.
//! 2. [`senkron_plani_kur`] — aynadaki hangi dosyaların hedefe kopyalanacağını
//!    belirler (yeni ya da daha güncel olanlar); **hiçbir şey kopyalamaz**.
//! 3. [`eksik_dosyalari_bul`] — senkron sonrası doğrulama; aynada olup hedefte
//!    olmayan dosyaları listeler.
//!
//! Bu modül **arı (pure)**: girdiyi bir [`DizinSayimi`] anlık görüntüsünden alır,
//! diske dokunmaz. Sayımı toplayan ve kararı uygulayan taraf çağırandır. Amaç,
//! "sildim" ile "sayacağımı saydım" ayrımını korumaktır (Masa Döngüsü R2 kapısı).
//!
//! # Neden ayna sayımı
//!
//! Silme kararı tek bir soruya dayanır: *bu ağacın kaybı geri alınabilir mi?*
//! Cevap yalnızca aynada en az o kadar dosya varsa evettir. Boş bir ağaç
//! (0 dosya) kaybedilecek veri taşımadığı için aynasız da silinebilir.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::{ToolError, ToolResult};

/// Bir dosyanın sayım anındaki durumu.
///
/// `deg_zaman` bir Unix damgası ya da başka monotonik sayaç olabilir; modül
/// yalnızca **karşılaştırır**, birimini yorumlamaz.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DosyaDurumu {
    /// Bayt cinsinden boyut.
    pub boyut: u64,
    /// Değişiklik zamanı (çağıranın seçtiği birimde, monotonik).
    pub deg_zaman: i64,
}

impl DosyaDurumu {
    /// Yeni durum kaydı kurar.
    pub fn yeni(boyut: u64, deg_zaman: i64) -> Self {
        Self { boyut, deg_zaman }
    }
}

/// Bir dizin ağacının anlık görüntüsü: göreli yol → dosya durumu.
///
/// Yollar dizin kökünden **göreli** ve ileri eğik çizgi (`/`) ile normalize
/// edilmiş olmalıdır; [`DizinSayimi::ekle`] normalizasyonu kendisi yapar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DizinSayimi {
    dosyalar: BTreeMap<String, DosyaDurumu>,
}

impl DizinSayimi {
    /// Boş sayım kurar.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Sayıma bir dosya ekler. Ters eğik çizgileri normalize eder, baştaki
    /// ayırıcıları kırpar.
    ///
    /// Aynı yol yeniden eklenirse son değer geçerlidir.
    pub fn ekle(&mut self, gor_yol: &str, durum: DosyaDurumu) {
        let anahtar = yolu_normalize_et(gor_yol);
        self.dosyalar.insert(anahtar, durum);
    }

    /// Yinelenebilir bir kaynaktan sayım kurar.
    pub fn kaynaktan<I, S>(girdiler: I) -> Self
    where
        I: IntoIterator<Item = (S, DosyaDurumu)>,
        S: AsRef<str>,
    {
        let mut sayim = Self::yeni();
        for (yol, durum) in girdiler {
            sayim.ekle(yol.as_ref(), durum);
        }
        sayim
    }

    /// Toplam dosya sayısı.
    pub fn dosya_sayisi(&self) -> usize {
        self.dosyalar.len()
    }

    /// Toplam bayt.
    pub fn toplam_boyut(&self) -> u64 {
        self.dosyalar.values().map(|d| d.boyut).sum()
    }

    /// Verilen dosyanın durumunu döndürür.
    pub fn durum(&self, gor_yol: &str) -> Option<&DosyaDurumu> {
        self.dosyalar.get(&yolu_normalize_et(gor_yol))
    }

    /// Sıralı (yol, durum) çiftleri.
    pub fn girdiler(&self) -> impl Iterator<Item = (&String, &DosyaDurumu)> {
        self.dosyalar.iter()
    }

    /// Belirli bir alt ağaçtaki dosya sayısı.
    ///
    /// `alt_yol` boşsa tüm ağacı sayar. Ad benzerliğinden doğan yanlış eşleşmeyi
    /// önlemek için sınır denetimi yapar: `ham-veri` sorgusu `ham-veriler/x`
    /// dosyasını **saymaz**.
    pub fn alt_agac_sayisi(&self, alt_yol: &str) -> usize {
        self.alt_agac_girdileri(alt_yol).count()
    }

    /// Belirli bir alt ağaçtaki toplam bayt.
    pub fn alt_agac_boyutu(&self, alt_yol: &str) -> u64 {
        self.alt_agac_girdileri(alt_yol).map(|(_, d)| d.boyut).sum()
    }

    /// Alt ağaçtaki girdiler; sınır denetimli önek eşleşmesi kullanır.
    fn alt_agac_girdileri(&self, alt_yol: &str) -> impl Iterator<Item = (&String, &DosyaDurumu)> {
        let onek = yolu_normalize_et(alt_yol);
        self.dosyalar
            .iter()
            .filter(move |(yol, _)| yol_kapsaminda_mi(yol, &onek))
    }
}

/// Bir ayıklama adayı için verilen karar.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Karar {
    /// Hedefte hiç yok; yapılacak iş kalmadı.
    ZatenYok,
    /// Ağaç boş (0 dosya); kaybedilecek veri taşımıyor.
    BosAgac,
    /// Aynada en az o kadar dosya var; silme geri alınabilir.
    AynaYeterli,
    /// Aynada karşılığı eksik; **silinmemeli**.
    AynaEksik,
    /// Koruma listesinde; hiçbir koşulda silinmez.
    Korumali,
}

impl Karar {
    /// Bu karar silmeye izin veriyor mu?
    pub fn silinebilir(&self) -> bool {
        matches!(self, Karar::BosAgac | Karar::AynaYeterli)
    }

    /// Kararın Türkçe kısa etiketi (rapor çıktısı için).
    pub fn etiket(&self) -> &'static str {
        match self {
            Karar::ZatenYok => "ZATEN YOK",
            Karar::BosAgac => "BOŞ AĞAÇ",
            Karar::AynaYeterli => "AYNA YETERLİ",
            Karar::AynaEksik => "AYNA EKSİK",
            Karar::Korumali => "KORUMALI",
        }
    }
}

/// Tek bir ayıklama adayının değerlendirmesi.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AyiklamaKarari {
    /// Adayın hedef köke göreli yolu.
    pub yol: String,
    /// Hedefteki dosya sayısı.
    pub hedef_sayisi: usize,
    /// Aynadaki dosya sayısı.
    pub ayna_sayisi: usize,
    /// Hedefte bu ağacın kapladığı bayt.
    pub hedef_boyutu: u64,
    /// Verilen karar.
    pub karar: Karar,
    /// Kararın Türkçe gerekçesi.
    pub gerekce: String,
}

impl AyiklamaKarari {
    /// Silme uygulanabilir mi?
    pub fn silinebilir(&self) -> bool {
        self.karar.silinebilir()
    }
}

/// Ayıklama planının tümü.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AyiklamaPlani {
    /// Değerlendirilen aday sayısı.
    pub aday_sayisi: usize,
    /// Silinebilir bulunan aday sayısı.
    pub silinebilir_sayisi: usize,
    /// Silinecek adaylardaki toplam dosya sayısı.
    pub kazanilacak_dosya: usize,
    /// Silinecek adaylardaki toplam bayt.
    pub kazanilacak_boyut: u64,
    /// Her adayın kararı, girdi sırasında.
    pub kararlar: Vec<AyiklamaKarari>,
}

impl AyiklamaPlani {
    /// Yalnızca silinebilir kararlar.
    pub fn silinecekler(&self) -> impl Iterator<Item = &AyiklamaKarari> {
        self.kararlar.iter().filter(|k| k.silinebilir())
    }

    /// Ayna eksikliği yüzünden reddedilen adaylar — raporda görünmesi gereken
    /// asıl bilgi budur.
    pub fn reddedilenler(&self) -> impl Iterator<Item = &AyiklamaKarari> {
        self.kararlar
            .iter()
            .filter(|k| matches!(k.karar, Karar::AynaEksik))
    }
}

/// Aynayla karşılaştırarak ayıklama planı kurar. **Hiçbir şeyi silmez.**
///
/// Karar sırası bağlayıcıdır: koruma listesi her şeyi ezer, sonra varlık,
/// sonra boşluk, en son ayna yeterliliği bakılır.
///
/// # Hatalar
///
/// - Aday listesi boşsa [`ToolError::InvalidInput`].
/// - Bir aday yolu normalize edildiğinde boşsa (`"/"`, `"."` gibi) kök dizinin
///   tamamını silme riski doğduğu için [`ToolError::InvalidInput`].
pub fn ayikla_plani_kur(
    hedef: &DizinSayimi,
    ayna: &DizinSayimi,
    adaylar: &[&str],
    korumalilar: &[&str],
) -> ToolResult<AyiklamaPlani> {
    if adaylar.is_empty() {
        return Err(ToolError::InvalidInput(
            "Ayıklama adayı listesi boş; değerlendirilecek bir şey yok".into(),
        ));
    }

    let korumali_normal: Vec<String> = korumalilar.iter().map(|k| yolu_normalize_et(k)).collect();

    let mut kararlar = Vec::with_capacity(adaylar.len());

    for aday in adaylar {
        let yol = yolu_normalize_et(aday);
        if yol.is_empty() {
            return Err(ToolError::InvalidInput(format!(
                "Aday yolu kök dizine çözümlendi ({aday:?}); tüm ağacı silme riski var"
            )));
        }

        let hedef_sayisi = hedef.alt_agac_sayisi(&yol);
        let ayna_sayisi = ayna.alt_agac_sayisi(&yol);
        let hedef_boyutu = hedef.alt_agac_boyutu(&yol);

        let korumali = korumali_normal
            .iter()
            .any(|k| yol_kapsaminda_mi(&yol, k) || yol_kapsaminda_mi(k, &yol));

        let (karar, gerekce) = if korumali {
            (
                Karar::Korumali,
                "Koruma listesinde; hiçbir koşulda silinmez".to_string(),
            )
        } else if hedef_sayisi == 0 && hedef_boyutu == 0 {
            // Sayımda hiç izi yok: ya dizin mevcut değil ya da tamamen boş.
            // İkisi de silme açısından aynı sonucu verir, ancak rapor ayırır.
            if ayna_sayisi == 0 {
                (
                    Karar::BosAgac,
                    "Hedefte 0 dosya; kaybedilecek veri yok".to_string(),
                )
            } else {
                (
                    Karar::BosAgac,
                    format!("Hedefte 0 dosya (aynada {ayna_sayisi} dosya duruyor)"),
                )
            }
        } else if ayna_sayisi >= hedef_sayisi {
            (
                Karar::AynaYeterli,
                format!(
                    "Aynada {ayna_sayisi} dosya var, hedefte {hedef_sayisi}; silme geri alınabilir"
                ),
            )
        } else {
            (
                Karar::AynaEksik,
                format!(
                    "Aynada yalnızca {ayna_sayisi} dosya var, hedefte {hedef_sayisi}; \
                     {} dosyanın yedeği yok",
                    hedef_sayisi - ayna_sayisi
                ),
            )
        };

        kararlar.push(AyiklamaKarari {
            yol,
            hedef_sayisi,
            ayna_sayisi,
            hedef_boyutu,
            karar,
            gerekce,
        });
    }

    let silinebilir_sayisi = kararlar.iter().filter(|k| k.silinebilir()).count();
    let kazanilacak_dosya = kararlar
        .iter()
        .filter(|k| k.silinebilir())
        .map(|k| k.hedef_sayisi)
        .sum();
    let kazanilacak_boyut = kararlar
        .iter()
        .filter(|k| k.silinebilir())
        .map(|k| k.hedef_boyutu)
        .sum();

    Ok(AyiklamaPlani {
        aday_sayisi: kararlar.len(),
        silinebilir_sayisi,
        kazanilacak_dosya,
        kazanilacak_boyut,
        kararlar,
    })
}

/// Bir dosyanın aynadan hedefe neden kopyalanacağı.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KopyaNedeni {
    /// Hedefte hiç yok.
    Yeni,
    /// Aynadaki kopya daha güncel.
    DahaGuncel,
    /// Zaman damgaları yakın ama boyut farklı.
    BoyutFarki,
}

impl KopyaNedeni {
    /// Nedenin Türkçe kısa etiketi.
    pub fn etiket(&self) -> &'static str {
        match self {
            KopyaNedeni::Yeni => "YENİ",
            KopyaNedeni::DahaGuncel => "DAHA GÜNCEL",
            KopyaNedeni::BoyutFarki => "BOYUT FARKI",
        }
    }
}

/// Kopyalanacak tek dosya.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KopyaIsi {
    /// Dosyanın göreli yolu.
    pub yol: String,
    /// Kopyalama nedeni.
    pub neden: KopyaNedeni,
    /// Aynadaki boyut (aktarılacak bayt).
    pub boyut: u64,
}

/// Senkron planının tümü.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenkronPlani {
    /// Aynada taranan dosya sayısı.
    pub taranan: usize,
    /// Hedefte zaten güncel olduğu için atlanan dosya sayısı.
    pub atlanan: usize,
    /// Aktarılacak toplam bayt.
    pub aktarilacak_boyut: u64,
    /// Kopyalanacak işler, yola göre sıralı.
    pub isler: Vec<KopyaIsi>,
}

impl SenkronPlani {
    /// Belirli bir nedene sahip iş sayısı.
    pub fn neden_sayisi(&self, neden: KopyaNedeni) -> usize {
        self.isler.iter().filter(|i| i.neden == neden).count()
    }
}

/// Aynadan hedefe senkron planı kurar. **Hiçbir şey kopyalamaz.**
///
/// Yalnızca aynada olup hedefte olmayan ya da aynada daha güncel olan dosyaları
/// seçer; hedefteki fazlalıklara dokunmaz (robocopy `/XO` davranışı).
///
/// `tolerans` saniye cinsinden zaman damgası toleransıdır: dosya sistemleri
/// arası damga yuvarlaması yüzünden birebir eşit kopyalar "daha güncel"
/// görünebilir. Boyut farkı toleransa **tabi değildir** — damgalar yakın olsa
/// bile boyut ayrıysa dosya kopyalanır.
///
/// # Hatalar
///
/// `tolerans` negatifse [`ToolError::InvalidInput`]; negatif tolerans, gerçekten
/// güncel dosyaları sessizce atlar.
pub fn senkron_plani_kur(
    ayna: &DizinSayimi,
    hedef: &DizinSayimi,
    tolerans: i64,
) -> ToolResult<SenkronPlani> {
    if tolerans < 0 {
        return Err(ToolError::InvalidInput(format!(
            "Zaman toleransı negatif olamaz (verilen: {tolerans})"
        )));
    }

    let mut isler = Vec::new();
    let mut atlanan = 0usize;

    for (yol, ayna_durumu) in ayna.girdiler() {
        match hedef.durum(yol) {
            None => isler.push(KopyaIsi {
                yol: yol.clone(),
                neden: KopyaNedeni::Yeni,
                boyut: ayna_durumu.boyut,
            }),
            Some(hedef_durumu) => {
                if ayna_durumu.deg_zaman > hedef_durumu.deg_zaman.saturating_add(tolerans) {
                    isler.push(KopyaIsi {
                        yol: yol.clone(),
                        neden: KopyaNedeni::DahaGuncel,
                        boyut: ayna_durumu.boyut,
                    });
                } else if ayna_durumu.boyut != hedef_durumu.boyut {
                    isler.push(KopyaIsi {
                        yol: yol.clone(),
                        neden: KopyaNedeni::BoyutFarki,
                        boyut: ayna_durumu.boyut,
                    });
                } else {
                    atlanan += 1;
                }
            }
        }
    }

    let aktarilacak_boyut = isler.iter().map(|i| i.boyut).sum();

    Ok(SenkronPlani {
        taranan: ayna.dosya_sayisi(),
        atlanan,
        aktarilacak_boyut,
        isler,
    })
}

/// Senkron sonrası doğrulama: aynada olup hedefte olmayan dosyalar.
///
/// Dönen liste boş değilse senkron tamamlanmamıştır. Bu, "kopyaladım" iddiasını
/// bağımsız yolla ölçen kapıdır; senkronu yapan kodun kendi raporuna
/// güvenilmez.
pub fn eksik_dosyalari_bul(ayna: &DizinSayimi, hedef: &DizinSayimi) -> Vec<String> {
    ayna.girdiler()
        .filter(|(yol, _)| hedef.durum(yol).is_none())
        .map(|(yol, _)| yol.clone())
        .collect()
}

/// Yolu karşılaştırmaya uygun biçime getirir: ters eğik çizgileri düzeltir,
/// yinelenen ayırıcıları teker indirir, baş/son ayırıcıları ve `.` parçalarını atar.
fn yolu_normalize_et(yol: &str) -> String {
    let duz = yol.replace('\\', "/");
    duz.split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// `yol`, `onek` ağacının içinde mi? Sınır denetimlidir.
///
/// Boş önek her yolu kapsar (kök). `ham-veri` öneki `ham-veriler/x` yolunu
/// **kapsamaz**; ayırıcı sınırı zorunludur.
fn yol_kapsaminda_mi(yol: &str, onek: &str) -> bool {
    if onek.is_empty() {
        return true;
    }
    if yol == onek {
        return true;
    }
    yol.len() > onek.len() && yol.starts_with(onek) && yol.as_bytes()[onek.len()] == b'/'
}

#[cfg(test)]
mod testler {
    use super::*;

    fn sayim(girdiler: &[(&str, u64, i64)]) -> DizinSayimi {
        DizinSayimi::kaynaktan(
            girdiler
                .iter()
                .map(|(y, b, z)| (*y, DosyaDurumu::yeni(*b, *z))),
        )
    }

    #[test]
    fn ayna_yeterliyse_silinebilir() {
        let hedef = sayim(&[("cop/a.log", 10, 1), ("cop/b.log", 20, 1)]);
        let ayna = sayim(&[("cop/a.log", 10, 1), ("cop/b.log", 20, 1)]);

        let plan = ayikla_plani_kur(&hedef, &ayna, &["cop"], &[]).unwrap();

        assert_eq!(plan.kararlar[0].karar, Karar::AynaYeterli);
        assert!(plan.kararlar[0].silinebilir());
        assert_eq!(plan.kazanilacak_dosya, 2);
        assert_eq!(plan.kazanilacak_boyut, 30);
    }

    #[test]
    fn ayna_eksikse_silinmez() {
        let hedef = sayim(&[("is/a", 1, 1), ("is/b", 1, 1), ("is/c", 1, 1)]);
        let ayna = sayim(&[("is/a", 1, 1)]);

        let plan = ayikla_plani_kur(&hedef, &ayna, &["is"], &[]).unwrap();

        assert_eq!(plan.kararlar[0].karar, Karar::AynaEksik);
        assert!(!plan.kararlar[0].silinebilir());
        assert_eq!(plan.kazanilacak_dosya, 0);
        assert_eq!(plan.reddedilenler().count(), 1);
        assert!(plan.kararlar[0].gerekce.contains("2 dosyanın yedeği yok"));
    }

    #[test]
    fn bos_agac_aynasiz_da_silinir() {
        let hedef = sayim(&[("baska/x", 5, 1)]);
        let ayna = DizinSayimi::yeni();

        let plan = ayikla_plani_kur(&hedef, &ayna, &["output"], &[]).unwrap();

        assert_eq!(plan.kararlar[0].karar, Karar::BosAgac);
        assert!(plan.kararlar[0].silinebilir());
        assert_eq!(plan.kazanilacak_dosya, 0);
    }

    #[test]
    fn koruma_listesi_ayna_yeterli_olsa_bile_ezer() {
        let hedef = sayim(&[("projeler/arabuluculuk/dosya.udf", 100, 1)]);
        let ayna = sayim(&[
            ("projeler/arabuluculuk/dosya.udf", 100, 1),
            ("projeler/arabuluculuk/ek.udf", 100, 1),
        ]);

        let plan = ayikla_plani_kur(
            &hedef,
            &ayna,
            &["projeler/arabuluculuk"],
            &["projeler/arabuluculuk"],
        )
        .unwrap();

        assert_eq!(plan.kararlar[0].karar, Karar::Korumali);
        assert!(!plan.kararlar[0].silinebilir());
    }

    #[test]
    fn koruma_alt_dizini_de_kapsar() {
        let hedef = sayim(&[("projeler/arabuluculuk/uyap_mcp/a.py", 10, 1)]);
        let ayna = sayim(&[("projeler/arabuluculuk/uyap_mcp/a.py", 10, 1)]);

        // Aday alt dizin, koruma üst dizin: yine korunmalı.
        let plan = ayikla_plani_kur(
            &hedef,
            &ayna,
            &["projeler/arabuluculuk/uyap_mcp"],
            &["projeler/arabuluculuk"],
        )
        .unwrap();
        assert_eq!(plan.kararlar[0].karar, Karar::Korumali);

        // Aday üst dizin, koruma alt dizin: korunan şeyi içerdiği için yine korunmalı.
        let plan = ayikla_plani_kur(
            &hedef,
            &ayna,
            &["projeler"],
            &["projeler/arabuluculuk/uyap_mcp"],
        )
        .unwrap();
        assert_eq!(plan.kararlar[0].karar, Karar::Korumali);
    }

    #[test]
    fn ad_benzerligi_yanlis_eslesmez() {
        // "ham-veri" adayı "ham-veriler" ağacını saymamalı.
        let hedef = sayim(&[("ham-veriler/onemli.csv", 999, 1)]);
        let ayna = DizinSayimi::yeni();

        let plan = ayikla_plani_kur(&hedef, &ayna, &["ham-veri"], &[]).unwrap();

        assert_eq!(plan.kararlar[0].hedef_sayisi, 0);
        assert_eq!(plan.kararlar[0].karar, Karar::BosAgac);
        // Asıl kanıt: benzer adlı ağaç plana hiç girmedi.
        assert_eq!(plan.kazanilacak_boyut, 0);
    }

    #[test]
    fn ters_egik_cizgi_ve_yinelenen_ayirici_normalize_edilir() {
        let hedef = sayim(&[("projeler\\\\Ar-Ge/./derme/a.md", 7, 1)]);
        assert_eq!(hedef.alt_agac_sayisi("projeler/Ar-Ge"), 1);
        assert_eq!(hedef.alt_agac_sayisi("projeler\\Ar-Ge\\derme"), 1);
        assert!(hedef.durum("projeler/Ar-Ge/derme/a.md").is_some());
    }

    #[test]
    fn bos_aday_listesi_hata_verir() {
        let bos = DizinSayimi::yeni();
        let hata = ayikla_plani_kur(&bos, &bos, &[], &[]).unwrap_err();
        assert!(matches!(hata, ToolError::InvalidInput(_)));
    }

    #[test]
    fn kok_dizine_cozumlenen_aday_reddedilir() {
        let bos = DizinSayimi::yeni();
        for tehlikeli in ["/", ".", "", "//", "./"] {
            let hata = ayikla_plani_kur(&bos, &bos, &[tehlikeli], &[]).unwrap_err();
            assert!(
                matches!(hata, ToolError::InvalidInput(_)),
                "{tehlikeli:?} reddedilmeliydi"
            );
        }
    }

    #[test]
    fn senkron_yeni_ve_guncel_dosyalari_secer() {
        let ayna = sayim(&[
            ("yeni.txt", 10, 100),
            ("guncel.txt", 20, 200),
            ("ayni.txt", 30, 100),
        ]);
        let hedef = sayim(&[("guncel.txt", 20, 100), ("ayni.txt", 30, 100)]);

        let plan = senkron_plani_kur(&ayna, &hedef, 2).unwrap();

        assert_eq!(plan.taranan, 3);
        assert_eq!(plan.atlanan, 1);
        assert_eq!(plan.isler.len(), 2);
        assert_eq!(plan.neden_sayisi(KopyaNedeni::Yeni), 1);
        assert_eq!(plan.neden_sayisi(KopyaNedeni::DahaGuncel), 1);
        assert_eq!(plan.aktarilacak_boyut, 30);
    }

    #[test]
    fn tolerans_icindeki_damga_farki_atlanir() {
        let ayna = sayim(&[("a.txt", 10, 101)]);
        let hedef = sayim(&[("a.txt", 10, 100)]);

        // 2 saniye tolerans: 1 saniyelik fark kopyalamayı tetiklememeli.
        let plan = senkron_plani_kur(&ayna, &hedef, 2).unwrap();
        assert_eq!(plan.isler.len(), 0);
        assert_eq!(plan.atlanan, 1);

        // Toleranssız: aynı fark kopyalanır.
        let plan = senkron_plani_kur(&ayna, &hedef, 0).unwrap();
        assert_eq!(plan.neden_sayisi(KopyaNedeni::DahaGuncel), 1);
    }

    #[test]
    fn boyut_farki_toleransa_takilmaz() {
        // Damgalar birebir aynı ama boyut ayrı: yarım kalmış kopya belirtisi.
        let ayna = sayim(&[("a.bin", 4096, 100)]);
        let hedef = sayim(&[("a.bin", 1024, 100)]);

        let plan = senkron_plani_kur(&ayna, &hedef, 60).unwrap();

        assert_eq!(plan.neden_sayisi(KopyaNedeni::BoyutFarki), 1);
        assert_eq!(plan.aktarilacak_boyut, 4096);
    }

    #[test]
    fn hedefteki_fazlalik_senkrona_girmez() {
        let ayna = sayim(&[("a.txt", 10, 100)]);
        let hedef = sayim(&[("a.txt", 10, 100), ("yerel-sadece.txt", 50, 100)]);

        let plan = senkron_plani_kur(&ayna, &hedef, 2).unwrap();

        assert_eq!(plan.isler.len(), 0);
        assert_eq!(plan.taranan, 1);
    }

    #[test]
    fn negatif_tolerans_hata_verir() {
        let bos = DizinSayimi::yeni();
        let hata = senkron_plani_kur(&bos, &bos, -1).unwrap_err();
        assert!(matches!(hata, ToolError::InvalidInput(_)));
    }

    #[test]
    fn eksik_dosya_dogrulamasi() {
        let ayna = sayim(&[("a", 1, 1), ("b/c", 1, 1), ("d", 1, 1)]);
        let hedef = sayim(&[("a", 1, 1)]);

        let eksikler = eksik_dosyalari_bul(&ayna, &hedef);
        assert_eq!(eksikler, vec!["b/c".to_string(), "d".to_string()]);

        // Senkron tamamlandıktan sonra eksik kalmamalı.
        let tam = sayim(&[("a", 1, 1), ("b/c", 1, 1), ("d", 1, 1)]);
        assert!(eksik_dosyalari_bul(&ayna, &tam).is_empty());
    }

    #[test]
    fn sayim_toplamlari() {
        let s = sayim(&[("a", 100, 1), ("alt/b", 200, 1), ("alt/c", 300, 1)]);
        assert_eq!(s.dosya_sayisi(), 3);
        assert_eq!(s.toplam_boyut(), 600);
        assert_eq!(s.alt_agac_sayisi("alt"), 2);
        assert_eq!(s.alt_agac_boyutu("alt"), 500);
        assert_eq!(s.alt_agac_sayisi(""), 3);
    }

    #[test]
    fn karar_etiketleri_turkce() {
        assert_eq!(Karar::AynaYeterli.etiket(), "AYNA YETERLİ");
        assert_eq!(Karar::AynaEksik.etiket(), "AYNA EKSİK");
        assert_eq!(Karar::Korumali.etiket(), "KORUMALI");
        assert_eq!(KopyaNedeni::DahaGuncel.etiket(), "DAHA GÜNCEL");
    }
}
