//! Turso DSL — SAFE Rust Trait-based DSL for Hermes Agentic Loop
//!
//! Bu crate, Gemini notebook'tan alınan "Manzume Aksaklık" tasarımını temel alır.
//! Hermes ile Turso SQLite veritabanı arasındaki ReAct (Düşün-Eylem-Gözlem) döngüsünü
//! sağlar; token-efficient compact DSL (`CALL:tool(args)`) formatı sunar.
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

// --- 3. GÜVENLİ VE PAYLAŞILABİLİR TURSO BAĞLANTISI ---
// Verileri hem birden fazla iş parçacığında paylaşmak (Arc) hem de
// güvenle değiştirebilmek (Mutex - Kilitli Emniyet Sandığı) için sarmalıyoruz.
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

    /// Veri tabanındaki kayıt sayısını döndürür
    pub fn adet_say(&self) -> usize {
        if let Ok(icerik) = self.veriler.lock() {
            icerik.len()
        } else {
            0
        }
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
        let temiz_id = girdiler.trim().replace("id=", "").replace("\"", "");
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

        let temiz_id = id_kismi.replace("id=", "").replace("\"", "");
        let temiz_veri = veri_kismi.replace("veri=", "").replace("\"", "");

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
        let temiz_id = girdiler.trim().replace("id=", "").replace("\"", "");
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
        let sayi = self.baglanti.adet_say();
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
        let temiz_sorgu = girdiler.trim().replace("sorgu=", "").replace("\"", "");

        if temiz_sorgu.is_empty() {
            return Err(ManzumeAksakligi::GirdiHatasi(
                "Lütfen aranacak yeteneğin adını belirtin!".to_string(),
            ));
        }

        let arama_url = format!(
            "https://skills-library.com/?search={}",
            temiz_sorgu.replace(' ', "+")
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

        let icerik = ham_cagri.trim_start_matches("CALL:");
        let parantez_basi = icerik
            .find('(')
            .ok_or_else(|| ManzumeAksakligi::AracCagriHatasi("Açma parantezi yok!".to_string()))?;
        let parantez_sonu = icerik
            .find(')')
            .ok_or_else(|| ManzumeAksakligi::AracCagriHatasi("Kapatma parantezi yok!".to_string()))?;

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
    fn skills_library_ara_bosluk_sonrasi_url_kur() {
        let arama = SkillsLibraryYetenegi::yeni();
        let sonuc = arama.calistir("sorgu=PDF parser");
        assert!(sonuc.is_ok());
        let veri = sonuc.unwrap();
        assert!(veri.contains("https://skills-library.com/?search=PDF+parser"));
        assert!(veri.contains("PDF parser"));
    }
}
