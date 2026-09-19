//! ibnunnedim gömme (vektör) kanalının CodSpeed benchmark'ları.
//!
//! YALNIZ `hash256`: diğer iki çekirdek (`e5s384`, `ollama-bge-m3`) ya ONNX
//! ikilisi indirir ya 127.0.0.1:11434'teki bir sunucuya konuşur — ikisi de
//! CI'da yok ve ikisi de ölçülen şeyi (kendi kodumuz) değil dış bir servisi
//! ölçerdi. `hash256` sıfır bağımlılıklı ve tamamen bizim kodumuz:
//! belirteçleme + FNV-1a + n-gram serpiştirme + L2 normalizasyon.
//!
//! VERİ: `arama.rs` benchmark'ıyla aynı gerçek ölçüme dayanır (2026-09-17,
//! `kutup_kutuphane.db`): 145 yetenek, `tam_metin_md` ortalama 8.289 karakter.
//! `belge_metni` bunu 2.000 karaktere kırptığı için gömülen metin ~2.100
//! karakterdir — benchmark tam bu boru hattını koşar.
//!
//! `gomme()` async imzalıdır (diğer çekirdekler ağ/model bekler); `hash256`
//! dalında hiç `await` noktası yoktur, o yüzden çalışma zamanı ölçülen
//! bölgenin dışında bir kez kurulur.

use ibnunnedim_cli::gomme::{belge_metni, blob_oku, blob_yaz, gomme, imza, kosinus, Cekirdek, Rol};
use tokio::runtime::Runtime;

fn main() {
    divan::main();
}

/// Türkçe katlama yolunu (İ/I, ç ş ğ ü ö ı) çalıştıran sözlük; rastgele
/// ASCII belirteçleyicinin asıl işini atlardı.
const KELIMELER: &[&str] = &[
    "yetenek",
    "İstanbul",
    "kaynakça",
    "ağaç",
    "şüphe",
    "çözümleme",
    "IŞIK",
    "rasyonelleşme",
    "hukuk",
    "gömme",
    "vektör",
    "ölçüm",
    "Rust",
    "ayrıştırıcı",
    "önbellek",
    "eşzamanlılık",
    "doğrulama",
    "belirteçleme",
];

/// Tohumlu LCG: bağımlılıksız, her koşuda aynı gövde.
struct Lcg(u64);

impl Lcg {
    fn sonraki(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }
}

/// `hedef` KARAKTER (bayt değil): gerçek ölçüm SQLite `LENGTH()` ile yapıldı
/// ve o karakter sayar. Türkçe harfler UTF-8'de 2 bayt.
fn metin(rng: &mut Lcg, hedef: usize) -> String {
    let mut s = String::with_capacity(hedef * 11 / 10 + 32);
    let mut n = 0;
    while n < hedef {
        let k = KELIMELER[rng.sonraki() % KELIMELER.len()];
        n += k.chars().count() + 1;
        s.push_str(k);
        s.push(' ');
    }
    s
}

/// Gerçek kütüphanenin şeklinde `adet` belge metni (ad ~15, açıklama ~79,
/// gövde ~8.289 karakter → `belge_metni` ile 2.000'e kırpılır).
fn govde(adet: usize) -> Vec<String> {
    let mut rng = Lcg(0x603E_0000);
    (0..adet)
        .map(|_| {
            belge_metni(
                &metin(&mut rng, 15),
                &metin(&mut rng, 79),
                &metin(&mut rng, 8_289),
            )
        })
        .collect()
}

/// Tek belgenin gömülmesi: boru hattının çıplak birim maliyeti.
#[divan::bench]
fn belge_gom(bencher: divan::Bencher) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let metinler = govde(1);
    bencher.bench(|| {
        rt.block_on(gomme(
            divan::black_box(&metinler),
            Rol::Belge,
            Cekirdek::Hash256,
        ))
        .expect("hash çekirdeği hata vermez")
    });
}

/// Tüm kütüphanenin yeniden gömülmesi: `ibnunnedim gom` imzası bayatlayan
/// her satır için bunu koşar (2026-09-17'de 145 yetenek + 85 depo).
#[divan::bench]
fn govde_gom(bencher: divan::Bencher) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let metinler = govde(230);
    bencher.bench(|| {
        rt.block_on(gomme(
            divan::black_box(&metinler),
            Rol::Belge,
            Cekirdek::Hash256,
        ))
        .expect("hash çekirdeği hata vermez")
    });
}

/// Sorgu gömmesi: arama başına tam bir kez koşan yol.
#[divan::bench]
fn sorgu_gom(bencher: divan::Bencher) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let sorgular = vec!["Weber hukukun rasyonelleşmesi İstanbul".to_string()];
    bencher.bench(|| {
        rt.block_on(gomme(
            divan::black_box(&sorgular),
            Rol::Sorgu,
            Cekirdek::Hash256,
        ))
        .expect("hash çekirdeği hata vermez")
    });
}

/// Gömülecek metnin kurulması (iki kez `ad` + kırpma). Satır başına bir kez.
#[divan::bench]
fn belge_metni_kur(bencher: divan::Bencher) {
    let mut rng = Lcg(0x11);
    let ad = metin(&mut rng, 15);
    let aciklama = metin(&mut rng, 79);
    let md = metin(&mut rng, 8_289);
    bencher.bench(|| {
        belge_metni(
            divan::black_box(&ad),
            divan::black_box(&aciklama),
            divan::black_box(&md),
        )
    });
}

/// Kosinüs taraması: sorgu vektörü × tüm kütüphane. Vektör kanalının sıralama
/// maliyeti bu; `ara` her sorguda belge sayısı kadar nokta çarpımı yapar.
#[divan::bench(args = [230, 2_300])]
fn kosinus_tarama(bencher: divan::Bencher, adet: usize) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let metinler = govde(8);
    let vektorler = rt
        .block_on(gomme(&metinler, Rol::Belge, Cekirdek::Hash256))
        .expect("hash çekirdeği hata vermez");
    // Gövde 8 belgeden çoğaltılır: ölçülen şey nokta çarpımı, veri çeşitliliği
    // değil — 2.300 ayrı belge üretmek kurulum süresini boşa şişirirdi.
    let govde_vek: Vec<Vec<f32>> = (0..adet).map(|i| vektorler[i % 8].clone()).collect();
    let sorgu = rt
        .block_on(gomme(
            &["hukukun rasyonelleşmesi".to_string()],
            Rol::Sorgu,
            Cekirdek::Hash256,
        ))
        .expect("hash çekirdeği hata vermez")
        .remove(0);

    bencher.bench(|| {
        let mut en_iyi = f32::MIN;
        for v in &govde_vek {
            let s = kosinus(divan::black_box(&sorgu), divan::black_box(v));
            if s > en_iyi {
                en_iyi = s;
            }
        }
        en_iyi
    });
}

/// BLOB gidiş-dönüş: her satır okunurken `blob_oku`, her yazılırken
/// `blob_yaz` koşar (256 boyut × 4 bayt).
#[divan::bench]
fn blob_gidis_donus(bencher: divan::Bencher) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let vektor = rt
        .block_on(gomme(&govde(1), Rol::Belge, Cekirdek::Hash256))
        .expect("hash çekirdeği hata vermez")
        .remove(0);
    let blob = blob_yaz(&vektor);

    bencher.bench(|| {
        let b = blob_yaz(divan::black_box(&vektor));
        let v = blob_oku(divan::black_box(&blob)).expect("blob 4'ün katı");
        (b.len(), v.len())
    });
}

/// İmza kurma: bayatlık denetiminin tamamı tek bir dizge karşılaştırmasına
/// indiği için satır başına bir kez koşar.
#[divan::bench]
fn imza_kur() -> String {
    imza(
        divan::black_box(Cekirdek::Hash256.kip()),
        divan::black_box("9f2c1b7d5e4a3f8c"),
    )
}
