//! Turso DSL — Hermes ajan döngüsü için trait tabanlı güvenli DSL
//!
//! ⚠️ **Bu crate Turso'ya bağlanmaz.** `Cargo.toml`'unda `turso` bağımlılığı
//! yoktur; `TursoBaglantisi` bellekte bir `HashMap<String, String>`'tir. Adı,
//! gerçek Turso bağlantısının **takılacağı yeri** işaretler — bugün orada
//! duran şey bir taklittir (stub). Gerçek Turso kullanan tek yer
//! `ibnunnedim-cli` crate'idir.
//!
//! Bu crate'in verdiği şey depo değil **iskelet**: `Yetenek` ehliyeti,
//! `ManzumeAksakligi` hata tipi ve `CALL:tool(args)` biçimli kompakt DSL.
//! ReAct (Düşün-Eylem-Gözlem) döngüsü bu iskelet üzerine kurulur.
//!
//! Gemineden alınan kavramlar:
//! - Yetenek trait (trait-based abstraction, ehliyet sistemi)
//! - Arc<Mutex<T>> (çoklu iş parçacığı güvenliği, kilitli emniyet sandığı)
//! - ManzumeAksakligi enum (unwrap-free error handling)
//! - Compact DSL (CALL:tool(id) formatı, yüzlerce token tasarrufu)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// --- 1. GÜVENLİ AKSAKLIK TANIMLARI ---

#[derive(Debug)]
pub enum ManzumeAksakligi {
    VeriTabaniHatasi(String),
    AracCagriHatasi(String),
    GirdiHatasi(String),
}

// Kolay okunabilir bir takı (Type Alias) tanımlıyoruz
pub type Sonuc<T> = Result<T, ManzumeAksakligi>;

// --- 1b. SINIR GÜVENLİ AYRIŞTIRMA ---
// `replace` deseni **her yerde** siler; veri içinde geçen `id=` de gider,
// tırnak içeren veri tırnağını kaybeder. Ayrıştırma sınırda yapılır.

/// `ad=` önekini **yalnız baştan** soyar, sonra **yalnız çevreleyen** tırnak
/// çiftini kaldırır. Metnin ortasındaki hiçbir şeye dokunmaz.
pub fn alan_soy(girdi: &str, ad: &str) -> String {
    let s = girdi.trim();
    let s = s.strip_prefix(&format!("{ad}=")).unwrap_or(s).trim();
    match (s.strip_prefix('"'), s.strip_suffix('"')) {
        (Some(_), Some(_)) if s.len() >= 2 => s[1..s.len() - 1].to_string(),
        _ => s.to_string(),
    }
}

/// Sorgu dizesi için yüzde kodlama. Yalnız ayrılmamış (unreserved) küme olduğu
/// gibi kalır; `&`, `=`, `#` ve çok baytlı karakterler kodlanır.
///
/// Boşluğu `+` yapmak tek başına yetmez: `a&b=c` girdisi `&` yüzünden ikinci
/// bir parametreye dönüşür, `#` ise kalanı parçaya çevirir.
pub fn url_kodla(s: &str) -> String {
    let mut cikti = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                cikti.push(*b as char)
            }
            _ => cikti.push_str(&format!("%{b:02X}")),
        }
    }
    cikti
}

// --- 2. YETENEK (TRAIT) EHLİYETİ ---
// Bu trait, bir sürücü ehliyeti gibidir. Düzenimiz, aracın arkasında
// kimin olduğuyla ilgilenmez; yalnızca bu ehliyet niteliğine sahip olup olmadığına bakar.
pub trait Yetenek: Send + Sync {
    /// Aracın yapay zekaya sunulacak adı
    fn ad(&self) -> &'static str;

    /// Aracın ne işe yaradığını bildiren çok kısa açıklama
    fn tanim(&self) -> &'static str;

    /// Kompakt (az token tüketen) yazılım imzası
    fn imza(&self) -> &'static str;

    /// Gelen sıkıştırılmış metin girdisini ayrıştırıp işi yürüten ana işlev
    fn calistir(&self, girdiler: &str) -> Sonuc<String>;
}

// --- 3. BELLEK İÇİ DEPO (Turso'nun takılacağı yer) ---
// Verileri hem birden fazla iş parçacığında paylaşmak (Arc) hem de
// güvenle değiştirebilmek (Mutex - Kilitli Emniyet Sandığı) için sarmalıyoruz.

/// **Taklit depo.** Adı gerçek Turso bağlantısının geleceği yeri işaretler;
/// bugün içi `HashMap`'tir ve süreç bitince veri kaybolur. Kalıcılık isteyen
/// hiçbir şey buna dayanmamalıdır.
#[derive(Clone)]
pub struct TursoBaglantisi {
    veriler: Arc<Mutex<HashMap<String, String>>>,
}

impl TursoBaglantisi {
    pub fn yeni() -> Self {
        let mut mock_db = HashMap::new();
        mock_db.insert(
            "kullanici:1".to_string(),
            "Ad: Ahmet, Rol: Yönetici".to_string(),
        );
        mock_db.insert(
            "kullanici:2".to_string(),
            "Ad: Ayşe, Rol: Mühendis".to_string(),
        );

        Self {
            veriler: Arc::new(Mutex::new(mock_db)),
        }
    }

    /// Veri tabanından güvenli okuma yapan işlevimiz
    pub fn oku(&self, anahtar: &str) -> Sonuc<String> {
        // Sandığın kilidini geçici olarak açıp içeriği okuyoruz
        let icerik = self
            .veriler
            .lock()
            .map_err(|_| ManzumeAksakligi::VeriTabaniHatasi("Kilidi açarken aksaklık oluştu!".to_string()))?;

        icerik
            .get(anahtar)
            .cloned()
            .ok_or_else(|| ManzumeAksakligi::VeriTabaniHatasi("Kayıt bulunamadı!".to_string()))
    }

    /// Veri tabanına yeni veri yazan güvenli işlevimiz
    pub fn yaz(&self, anahtar: String, deger: String) -> Sonuc<()> {
        // Sandığın kilidini açıp yazma işlemi için tek kişilik geçici yetki (&mut) alıyoruz
        let mut icerik = self
            .veriler
            .lock()
            .map_err(|_| ManzumeAksakligi::VeriTabaniHatasi("Yazma kilidi açılırken aksaklık oluştu!".to_string()))?;

        icerik.insert(anahtar, deger);
        Ok(())
    }

    /// Veri tabanından güvenli silme yapan işlevimiz
    pub fn sil(&self, anahtar: &str) -> Sonuc<()> {
        let mut icerik = self
            .veriler
            .lock()
            .map_err(|_| ManzumeAksakligi::VeriTabaniHatasi("Silme kilidi açılırken aksaklık oluştu!".to_string()))?;

        icerik.remove(anahtar);
        Ok(())
    }

    /// Veri tabanındaki kayıt sayısını döndürür.
    ///
    /// Kilit açılamazsa **hata döner, 0 dönmez**: "okuyamadım" ile "boş" aynı
    /// sayıya inerse sayı yalan söyler.
    pub fn adet_say(&self) -> Sonuc<usize> {
        let icerik = self.veriler.lock().map_err(|_| {
            ManzumeAksakligi::VeriTabaniHatasi("Sayım kilidi açılırken aksaklık oluştu!".to_string())
        })?;
        Ok(icerik.len())
    }
}

// --- 4. OKUMA YETENEĞİ ARACI ---
pub struct TursoOkuYetenegi {
    baglanti: TursoBaglantisi,
}

impl TursoOkuYetenegi {
    pub fn yeni(baglanti: TursoBaglantisi) -> Self {
        Self { baglanti }
    }
}

impl Yetenek for TursoOkuYetenegi {
    fn ad(&self) -> &'static str {
        "turso_oku"
    }

    fn tanim(&self) -> &'static str {
        "Turso SQLite tablosundan kayıt getirir."
    }

    fn imza(&self) -> &'static str {
        "turso_oku(id: sayi)"
    }

    fn calistir(&self, girdiler: &str) -> Sonuc<String> {
        let temiz_id = alan_soy(girdiler, "id");
        let anahtar = format!("kullanici:{}", temiz_id);
        self.baglanti.oku(&anahtar)
    }
}

// --- 5. YAZMA YETENEĞİ ARACI ---
pub struct TursoYazYetenegi {
    baglanti: TursoBaglantisi,
}

impl TursoYazYetenegi {
    pub fn yeni(baglanti: TursoBaglantisi) -> Self {
        Self { baglanti }
    }
}

impl Yetenek for TursoYazYetenegi {
    fn ad(&self) -> &'static str {
        "turso_yaz"
    }

    fn tanim(&self) -> &'static str {
        "Turso SQLite tablosuna yeni bir kayıt ekler veya mevcut olanı günceller."
    }

    fn imza(&self) -> &'static str {
        "turso_yaz(id: sayi, veri: metin)"
    }

    fn calistir(&self, girdiler: &str) -> Sonuc<String> {
        // Girdiler örn: "3, Ad: Veli, Rol: Tasarımcı"
        let virgul_konumu = girdiler
            .find(',')
            .ok_or_else(|| ManzumeAksakligi::GirdiHatasi("Yazma girdisi eksik! Örnek: 3, Ad: Veli".to_string()))?;

        let id_kismi = girdiler[..virgul_konumu].trim();
        let veri_kismi = girdiler[virgul_konumu + 1..].trim();

        let temiz_id = alan_soy(id_kismi, "id");
        let temiz_veri = alan_soy(veri_kismi, "veri");

        let anahtar = format!("kullanici:{}", temiz_id);

        self.baglanti.yaz(anahtar, temiz_veri.clone())?;

        Ok(format!(
            "Başarıyla Yazıldı -> Anahtar: {}, Değer: {}",
            temiz_id, temiz_veri
        ))
    }
}

// --- 6. SİLME YETENEĞİ ARACI ---
pub struct TursoSilYetenegi {
    baglanti: TursoBaglantisi,
}

impl TursoSilYetenegi {
    pub fn yeni(baglanti: TursoBaglantisi) -> Self {
        Self { baglanti }
    }
}

impl Yetenek for TursoSilYetenegi {
    fn ad(&self) -> &'static str {
        "turso_sil"
    }

    fn tanim(&self) -> &'static str {
        "Turso SQLite tablosundan bir kaydı siler."
    }

    fn imza(&self) -> &'static str {
        "turso_sil(id: sayi)"
    }

    fn calistir(&self, girdiler: &str) -> Sonuc<String> {
        let temiz_id = alan_soy(girdiler, "id");
        let anahtar = format!("kullanici:{}", temiz_id);

        self.baglanti.sil(&anahtar)?;

        Ok(format!("Başarıyla Silindi -> Anahtar: {}", temiz_id))
    }
}

// --- 7. SAYMA YETENEĞİ ARACI ---
pub struct TursoSayYetenegi {
    baglanti: TursoBaglantisi,
}

impl TursoSayYetenegi {
    pub fn yeni(baglanti: TursoBaglantisi) -> Self {
        Self { baglanti }
    }
}

impl Yetenek for TursoSayYetenegi {
    fn ad(&self) -> &'static str {
        "turso_say"
    }

    fn tanim(&self) -> &'static str {
        "Turso SQLite tablosundaki kayıt sayısını döndürür."
    }

    fn imza(&self) -> &'static str {
        "turso_say()"
    }

    fn calistir(&self, _girdiler: &str) -> Sonuc<String> {
        let sayi = self.baglanti.adet_say()?;
        Ok(format!("Toplam kayıt sayısı: {}", sayi))
    }
}

// --- 8. SKILLS LIBRARY ARAMA YETENEĞİ ---
pub struct SkillsLibraryYetenegi;

impl SkillsLibraryYetenegi {
    pub fn yeni() -> Self {
        Self
    }
}

impl Yetenek for SkillsLibraryYetenegi {
    fn ad(&self) -> &'static str {
        "skills_library_ara"
    }

    fn tanim(&self) -> &'static str {
        "Yapay zeka için yeni bir yeteneğe ihtiyaç duyulduğunda, skills-library.com üzerindeki açık kütüphanede arama yapar ve kurulum bağlantılarını döner."
    }

    fn imza(&self) -> &'static str {
        "skills_library_ara(sorgu: metin)"
    }

    fn calistir(&self, girdiler: &str) -> Sonuc<String> {
        let temiz_sorgu = alan_soy(girdiler, "sorgu");

        if temiz_sorgu.is_empty() {
            return Err(ManzumeAksakligi::GirdiHatasi(
                "Lütfen aranacak yeteneğin adını belirtin!".to_string(),
            ));
        }

        let arama_url = format!(
            "https://skills-library.com/?search={}",
            url_kodla(&temiz_sorgu)
        );

        Ok(format!(
            "Aradığınız '{}' yeteneği için skills-library.com bağlantısı:\n{}",
            temiz_sorgu, arama_url
        ))
    }
}

// --- 9. ARAÇ ÇAĞRI MERKEZİ (DISPATCHER) ---
pub struct YetenekYoneticisi {
    yetenekler: HashMap<&'static str, Box<dyn Yetenek>>,
}

impl YetenekYoneticisi {
    pub fn yeni() -> Self {
        Self {
            yetenekler: HashMap::new(),
        }
    }

    pub fn yetenek_ekle(&mut self, yetenek: Box<dyn Yetenek>) {
        self.yetenekler.insert(yetenek.ad(), yetenek);
    }

    pub fn sistem_komutu_menusu(&self) -> String {
        let mut menu = String::new();
        menu.push_str("Kullanabileceğin yetenekler:\n");
        for yetenek in self.yetenekler.values() {
            menu.push_str(&format!(
                "- {} -> Açıklama: {}, İmza: {}\n",
                yetenek.ad(),
                yetenek.tanim(),
                yetenek.imza()
            ));
        }
        menu
    }

    pub fn cagiriyi_coz_ve_calistir(&self, ham_cagri: &str) -> Sonuc<String> {
        if !ham_cagri.starts_with("CALL:") {
            return Err(ManzumeAksakligi::AracCagriHatasi(
                "Geçersiz çağrı! FORMAT: CALL:tool_name(args)".to_string(),
            ));
        }

        let icerik = ham_cagri.strip_prefix("CALL:").expect("starts_with yukarida denetlendi");
        let parantez_basi = icerik
            .find('(')
            .ok_or_else(|| ManzumeAksakligi::AracCagriHatasi("Açma parantezi yok!".to_string()))?;
        let parantez_sonu = icerik
            .rfind(')')
            .ok_or_else(|| ManzumeAksakligi::AracCagriHatasi("Kapatma parantezi yok!".to_string()))?;

        // Kapanış açılıştan önce gelirse dilim ters döner ve **panikler**.
        // Sınır denetimi olmadan `CALL:arac)1(` süreci düşürüyordu.
        if parantez_sonu < parantez_basi {
            return Err(ManzumeAksakligi::AracCagriHatasi(
                "Parantezler ters sırada! FORMAT: CALL:tool_name(args)".to_string(),
            ));
        }

        let arac_adi = &icerik[..parantez_basi];
        let girdi = &icerik[parantez_basi + 1..parantez_sonu];

        if let Some(yetenek) = self.yetenekler.get(arac_adi) {
            yetenek.calistir(girdi)
        } else {
            Err(ManzumeAksakligi::AracCagriHatasi(format!(
                "{} adında bir yetenek düzenimizde kayıtlı değil!",
                arac_adi
            )))
        }
    }
}

// --- 10. TESTLER ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turso_oku_temel_calisir() {
        let db = TursoBaglantisi::yeni();
        let oku_araci = TursoOkuYetenegi::yeni(db);

        let sonuc = oku_araci.calistir("1");
        assert!(sonuc.is_ok());
        let veri = sonuc.unwrap();
        assert!(veri.contains("Ahmet"));
        assert!(veri.contains("Yönetici"));
    }

    #[test]
    fn turso_yaz_ve_oku_entegrasyonu() {
        let db = TursoBaglantisi::yeni();
        let yaz_araci = TursoYazYetenegi::yeni(db.clone());
        let oku_araci = TursoOkuYetenegi::yeni(db);

        // Yaz
        let yaz_sonuc = yaz_araci.calistir("3, Ad: Veli, Rol: Tasarımcı");
        assert!(yaz_sonuc.is_ok());

        // Oku
        let oku_sonuc = oku_araci.calistir("3");
        assert!(oku_sonuc.is_ok());
        let veri = oku_sonuc.unwrap();
        assert!(veri.contains("Veli"));
        assert!(veri.contains("Tasarımcı"));
    }

    #[test]
    fn turso_sil_temel_calisir() {
        let db = TursoBaglantisi::yeni();
        let sil_araci = TursoSilYetenegi::yeni(db.clone());
        let oku_araci = TursoOkuYetenegi::yeni(db);

        // Sil
        let sil_sonuc = sil_araci.calistir("1");
        assert!(sil_sonuc.is_ok());

        // Tekrar oku (artık yok olmalı)
        let oku_sonuc = oku_araci.calistir("1");
        assert!(oku_sonuc.is_err());
    }

    #[test]
    fn turso_say_kayit_sayisini_doner() {
        let db = TursoBaglantisi::yeni();
        let say_araci = TursoSayYetenegi::yeni(db.clone());
        let sonuc = say_araci.calistir("");
        assert!(sonuc.is_ok());
        let veri = sonuc.unwrap();
        assert!(veri.contains("Toplam kayıt sayısı: 2"));
    }

    #[test]
    fn skills_library_ara_url_kurar() {
        let arama = SkillsLibraryYetenegi::yeni();
        let veri = arama.calistir("sorgu=PDF parser").expect("arama düştü");
        assert!(
            veri.contains("https://skills-library.com/?search=PDF%20parser"),
            "boşluk kodlanmadı: {veri}"
        );
        assert!(veri.contains("PDF parser"), "sorgu metni kayboldu");
    }

    // ---------------------------------------------------------------------
    // Düşebilen kapılar — yeşil yanan ama hiçbir şey söylemeyen kapı, kapı
    // değildir. Her biri bozuk girdide kırmızı, sağlamda yeşil olmalı.
    // ---------------------------------------------------------------------

    /// D2 · Desen sınırı geçmez. Verinin ortasındaki `id=` silinmemeli.
    #[test]
    fn kapi_d2_gomulu_desen_silinmez() {
        let girdi = "kayit-id=3";
        let eski_davranis = girdi.trim().replace("id=", "").replace('"', "");
        assert_eq!(eski_davranis, "kayit-3", "eski davranış varsayımı bozuldu");
        assert_eq!(alan_soy(girdi, "id"), "kayit-id=3", "gömülü desen silindi");
        assert_ne!(
            eski_davranis,
            alan_soy(girdi, "id"),
            "düzeltme davranışı değiştirmiyorsa süstür"
        );
    }

    /// D2 · Yalnız **çevreleyen** tırnak çifti kalkar; içerdeki tırnak kalır.
    #[test]
    fn kapi_d2_ic_tirnak_korunur() {
        assert_eq!(alan_soy(r#"id="7""#, "id"), "7", "çevreleyen çift kalkmadı");
        assert_eq!(
            alan_soy(r#"veri=Alinti: "onemli" nokta"#, "veri"),
            r#"Alinti: "onemli" nokta"#,
            "içerdeki tırnaklar silindi"
        );
        assert_eq!(alan_soy("id=5", "veri"), "id=5", "yanlış alan adı soyuldu");
    }

    /// D1 · URL ayırıcıları kodlanır; yoksa `&` ikinci bir parametre açar.
    #[test]
    fn kapi_d1_url_ayiricilari_kodlanir() {
        let arama = SkillsLibraryYetenegi::yeni();
        for (girdi, beklenen) in [("a&b=c", "%26"), ("x#y", "%23"), ("türkçe", "%C3%BC")] {
            let veri = arama
                .calistir(&format!("sorgu={girdi}"))
                .expect("arama düştü");
            let url = veri.lines().last().expect("URL satırı yok");
            let sorgu = url
                .split_once("?search=")
                .expect("URL biçimi değişmiş")
                .1;
            assert!(sorgu.contains(beklenen), "{girdi}: {beklenen} yok → {url}");
            for ham in ['&', '#', '?'] {
                assert!(!sorgu.contains(ham), "{girdi}: ham '{ham}' kaldı → {url}");
            }
        }
    }

    /// R2 · "saydım" ≠ "yazdım". Okuyamayan sayım 0 dönmez, hata döner.
    #[test]
    fn kapi_r2_sayim_kilit_bozulunca_yalan_soylemez() {
        let db = TursoBaglantisi::yeni();
        let klon = db.clone();
        // Kilidi tutarken panikleyen iş parçacığı mutex'i zehirler.
        let _ = std::thread::spawn(move || {
            let _tut = klon.veriler.lock().expect("ilk kilit");
            panic!("kasıtlı panik — kilit zehirleniyor");
        })
        .join();
        match db.adet_say() {
            Err(ManzumeAksakligi::VeriTabaniHatasi(_)) => {}
            Ok(n) => panic!("zehirli kilitte {n} döndü — 'okuyamadım' 'boş' sanıldı"),
            Err(e) => panic!("beklenmeyen hata türü: {e:?}"),
        }
        // Sağlam bağlantıda susmalı (yanlış alarm vermemeli).
        assert_eq!(TursoBaglantisi::yeni().adet_say().expect("sağlam sayım"), 2);
    }

    /// Çağrı çözümü: iç parantezli girdi son kapanışa kadar alınmalı ve
    /// ters dilim (panik) oluşmamalı.
    #[test]
    fn kapi_cagri_ic_parantez_ve_bozuk_girdi() {
        let db = TursoBaglantisi::yeni();
        let mut y = YetenekYoneticisi::yeni();
        y.yetenek_ekle(Box::new(TursoYazYetenegi::yeni(db.clone())));
        y.yetenek_ekle(Box::new(TursoOkuYetenegi::yeni(db)));

        y.cagiriyi_coz_ve_calistir("CALL:turso_yaz(9, Not: (ek bilgi) burada)")
            .expect("iç parantezli girdi düştü");
        let okunan = y
            .cagiriyi_coz_ve_calistir("CALL:turso_oku(9)")
            .expect("okuma düştü");
        assert!(okunan.contains("(ek bilgi)"), "iç parantez kırpıldı: {okunan}");

        // Bozuk girdiler: hata dönmeli, panik değil.
        for bozuk in ["turso_oku(1)", "CALL:turso_oku", "CALL:turso_oku)1("] {
            assert!(
                y.cagiriyi_coz_ve_calistir(bozuk).is_err(),
                "bozuk girdi kabul edildi: {bozuk}"
            );
        }
    }
}
