//! Hermes ReAct Orkestrator — Gemini Tasarımından Alınan Temsilci Döngüsü
//!
//! Bu crate, Gemini notebook'tan alınan "Temsilci Döngüsü" tasarımını temel alır.
//! Hermes'in ReAct (Düşün-Eylem-Gözlem) döngüsünü simüle eder ve
//! Rust kütüphanemizle nasıl etkileşime girdiğini gösterir.

// turso-dsl crate'inden gelen yapılar
use turso_dsl::{ManzumeAksakligi, TursoBaglantisi, YetenekYoneticisi};

// --- 1. HERMES BENZETİM MOTORU (SIMULATOR) ---
// Bu yapı, Hermes'in ReAct döngüsündeki davranışlarını taklit eder.
pub struct HermesBenzetimi;

impl HermesBenzetimi {
    /// Kullanıcının sorusuna ve önceki gözlem sonuçlarına göre Hermes'in vereceği tepkileri simüle eder.
    pub fn yanit_uret(&self, adim: usize, soru: &str, gozlem: Option<&str>) -> String {
        match adim {
            1 => {
                // Hermes ilk adımda düşünür ve aracı çağırmaya karar verir
                format!(
                    "DÜŞÜNCE: Kullanıcı bana '{}' diye sordu. Bu bilgiye erişmek için veri tabanını okumam gerek.\n\
                     EYLEM: CALL:turso_oku(1)",
                    soru
                )
            }
            2 => {
                // Hermes ikinci adımda veri tabanından gelen gözlem sonucunu alır ve nihai cevabı üretir
                let veri = gozlem.unwrap_or("Veri yok");
                format!(
                    "DÜŞÜNCE: Veri tabanından gelen bilgi şu şekilde: {}. Artık kullanıcının sorusunu net bir şekilde cevaplayabilirim.\n\
                     CEVAP: Sorduğunuz kullanıcının adı Ahmet'tir ve kendisi sistemimizde 'Yönetici' olarak görev yapmaktadır.",
                    veri
                )
            }
            _ => "DÜŞÜNCE: İşlem tamamlandı.".to_string(),
        }
    }
}

// --- 2. REACT ORKESTRATÖR ---
// Hermes benzetimi ve Rust kütüphanesi arasındaki tam döngüyü yönetir.
pub struct ReActOrkestrator {
    hermes: HermesBenzetimi,
    yetenek_yonetici: YetenekYoneticisi,
}

impl ReActOrkestrator {
    pub fn yeni() -> Self {
        let db = TursoBaglantisi::yeni();
        let mut yonetici = YetenekYoneticisi::yeni();
        yonetici.yetenek_ekle(Box::new(turso_dsl::TursoOkuYetenegi::yeni(db.clone())));
        yonetici.yetenek_ekle(Box::new(turso_dsl::TursoYazYetenegi::yeni(db.clone())));
        yonetici.yetenek_ekle(Box::new(turso_dsl::TursoSilYetenegi::yeni(db)));

        Self {
            hermes: HermesBenzetimi,
            yetenek_yonetici: yonetici,
        }
    }

    pub fn sistem_komutu_menusu(&self) -> String {
        self.yetenek_yonetici.sistem_komutu_menusu()
    }

    /// Hermes'ten gelen çağrıları alıp Rust kütüphanesini çağıran ana döngü
    pub fn calistir(&self, kullanici_sorusu: &str) -> Result<String, ManzumeAksakligi> {
        // Adım 1: Hermes'ten ilk düşünce ve araç çağrı talebini alıyoruz
        let hermes_yaniti_1 = self.hermes.yanit_uret(1, kullanici_sorusu, None);
        println!("\n[Hermes - 1. Adım]:\n{}", hermes_yaniti_1);

        // Satırlar arasından "CALL:" komutunu cımbızla çekiyoruz (Gözetleme Aşaması)
        let mut arac_cagrisi = String::new();
        for satir in hermes_yaniti_1.lines() {
            if satir.starts_with("EYLEM:") {
                arac_cagrisi = satir.replace("EYLEM:", "").trim().to_string();
            }
        }

        if !arac_cagrisi.is_empty() {
            println!("\n--- [Rust Düzeni Araya Giriyor] ---");
            println!("Yakalanan Sıkıştırılmış Komut: {}", arac_cagrisi);

            // Kütüphanemiz komutu yorumlayıp çalıştırıyor
            match self
                .yetenek_yonetici
                .cagiriyi_coz_ve_calistir(&arac_cagrisi)
            {
                Ok(gozlem_sonucu) => {
                    println!("Veri Tabanından Alınan Sonuç (Gözlem): {}", gozlem_sonucu);

                    // Adım 2: Çıkan sonucu telsizden Hermes'e geri fısıldıyoruz
                    let hermes_yaniti_2 =
                        self.hermes
                            .yanit_uret(2, kullanici_sorusu, Some(&gozlem_sonucu));
                    println!("\n[Hermes - 2. Adım]:\n{}", hermes_yaniti_2);
                    Ok(hermes_yaniti_2)
                }
                Err(e) => {
                    println!("Yetenek çalıştırılırken aksaklık çıktı: {:?}", e);
                    Err(e)
                }
            }
        } else {
            println!("Yapay zeka herhangi bir araç çağırma eyleminde bulunmadı.");
            Err(ManzumeAksakligi::AracCagriHatasi(
                "Hermes araç çağrısı üretmedi!".to_string(),
            ))
        }
    }
}

// --- 3. ANA FONKSİYON (TEST DÜZENİ) ---
#[tokio::main]
async fn main() {
    println!("=== Hermes ReAct Test ve Benzetim Düzeni Başlatıldı ===");

    let orkestrator = ReActOrkestrator::yeni();

    println!("\n{}", orkestrator.sistem_komutu_menusu());

    let kullanici_sorusu = "1 numaralı kullanıcının kim olduğunu ve yetkisini bulur musun?";

    println!("\n[Kullanıcı Sorusu]: {}", kullanici_sorusu);

    match orkestrator.calistir(kullanici_sorusu) {
        Ok(_) => {}
        Err(e) => {
            println!("\nDöngü tamamlanamadı: {:?}", e);
        }
    }
}
