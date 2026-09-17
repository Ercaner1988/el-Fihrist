//! `turso-dsl` iskeletinin CodSpeed benchmark'ları.
//!
//! NE ÖLÇÜLÜYOR: `CALL:arac(args)` biçimli kompakt DSL'in TAM yolu — önek
//! denetimi, parantez sınırları, yetenek tablosundan arama ve seçilen
//! yeteneğin koşması. ReAct döngüsü her adımda tam olarak bunu yapar, yani
//! ajanın tur başına ödediği bedel bu.
//!
//! DEPO: crate'in içindeki `TursoBaglantisi` bellekte bir `HashMap`'tir
//! (modül belgesindeki uyarıya bakın) — burada ağ ya da disk yok, ölçüm
//! tamamen ayrıştırma + kilit + dizge kurma maliyetidir.

use turso_dsl::{
    alan_soy, url_kodla, SkillsLibraryYetenegi, TursoBaglantisi, TursoOkuYetenegi,
    TursoSayYetenegi, TursoSilYetenegi, TursoYazYetenegi, YetenekYoneticisi,
};

fn main() {
    divan::main();
}

/// Gerçek bir ajan oturumundaki yönetici: dört Turso yeteneği + kütüphane
/// arama. Çağrı çözümü tablo araması içerdiği için yetenek sayısı önemlidir.
fn yonetici(baglanti: TursoBaglantisi) -> YetenekYoneticisi {
    let mut y = YetenekYoneticisi::yeni();
    y.yetenek_ekle(Box::new(TursoOkuYetenegi::yeni(baglanti.clone())));
    y.yetenek_ekle(Box::new(TursoYazYetenegi::yeni(baglanti.clone())));
    y.yetenek_ekle(Box::new(TursoSilYetenegi::yeni(baglanti.clone())));
    y.yetenek_ekle(Box::new(TursoSayYetenegi::yeni(baglanti)));
    y.yetenek_ekle(Box::new(SkillsLibraryYetenegi::yeni()));
    y
}

/// `adet` kayıtlı depo: sayma ve okuma yollarının ölçeği buradan gelir.
fn dolu_depo(adet: usize) -> TursoBaglantisi {
    let db = TursoBaglantisi::yeni();
    for i in 0..adet {
        db.yaz(
            format!("kullanici:{i}"),
            format!("Ad: Kullanıcı {i}, Rol: Mühendis, Bölge: İstanbul"),
        )
        .expect("bellek içi yazma başarısız olmaz");
    }
    db
}

// --- sınır ayrıştırma ---

/// `alan_soy` her yetenek çağrısında en az bir kez koşar; tırnaklı ve
/// öneksiz iki biçim de gerçek girdilerde görülüyor.
#[divan::bench(args = ["id=\"42\"", "  sorgu=\"hukukun rasyonelleşmesi, Weber\"  "])]
fn alan_soyma(girdi: &str) -> String {
    alan_soy(divan::black_box(girdi), divan::black_box("sorgu"))
}

/// Yüzde kodlama çok baytlı her karakter için `format!` çağırır — Türkçe
/// sorgularda bu yol baskın. 12 ve 120 karakterlik iki sorgu.
#[divan::bench(args = [12, 120])]
fn url_kodlama(bencher: divan::Bencher, uzunluk: usize) {
    let sorgu: String = "hukukun rasyonelleşmesi & İstanbul şüphesi #ölçüm "
        .chars()
        .cycle()
        .take(uzunluk)
        .collect();
    bencher.bench(|| url_kodla(divan::black_box(&sorgu)));
}

// --- DSL çağrı çözümü ---

#[divan::bench]
fn cagri_oku(bencher: divan::Bencher) {
    let y = yonetici(dolu_depo(500));
    bencher.bench(|| {
        y.cagiriyi_coz_ve_calistir(divan::black_box("CALL:turso_oku(id=\"321\")"))
            .expect("kayıt var")
    });
}

#[divan::bench]
fn cagri_yaz(bencher: divan::Bencher) {
    let y = yonetici(dolu_depo(500));
    bencher.bench(|| {
        y.cagiriyi_coz_ve_calistir(divan::black_box(
            "CALL:turso_yaz(id=\"321\", veri=\"Ad: Ayşe, Rol: Mühendis\")",
        ))
        .expect("yazma başarısız olmaz")
    });
}

/// Sayma: kilidi alıp `len()` döner. Kayıt sayısından bağımsız olması
/// gerekir — regresyon buradan görünür.
#[divan::bench(args = [10, 1_000])]
fn cagri_say(bencher: divan::Bencher, adet: usize) {
    let y = yonetici(dolu_depo(adet));
    bencher.bench(|| {
        y.cagiriyi_coz_ve_calistir(divan::black_box("CALL:turso_say()"))
            .expect("sayım başarısız olmaz")
    });
}

/// Kütüphane arama: `alan_soy` + `url_kodla` + iki `format!`. Ağa çıkmaz,
/// yalnız bağlantıyı kurar.
#[divan::bench]
fn cagri_kutuphane_arama(bencher: divan::Bencher) {
    let y = yonetici(TursoBaglantisi::yeni());
    bencher.bench(|| {
        y.cagiriyi_coz_ve_calistir(divan::black_box(
            "CALL:skills_library_ara(sorgu=\"türkçe belirteçleme & BM25\")",
        ))
        .expect("sorgu boş değil")
    });
}

/// Bilinmeyen araç: hata yolu da ölçülür, çünkü modeller sık sık uydurma
/// araç adı üretir ve bu dal her turda koşabilir.
#[divan::bench]
fn cagri_bilinmeyen_arac(bencher: divan::Bencher) {
    let y = yonetici(TursoBaglantisi::yeni());
    bencher.bench(|| {
        y.cagiriyi_coz_ve_calistir(divan::black_box("CALL:olmayan_arac(id=\"1\")"))
            .is_err()
    });
}

/// Sistem komutu menüsü: her istemde modele verilen araç listesi.
#[divan::bench]
fn sistem_menusu(bencher: divan::Bencher) {
    let y = yonetici(TursoBaglantisi::yeni());
    bencher.bench(|| y.sistem_komutu_menusu());
}
