//! ibnunnedim arama çekirdeğinin CodSpeed benchmark'ları.
//!
//! NEDEN `ibnunnedim_cli::arama`: BM25'in tek kopyası burada. Workspace'te
//! bir zamanlar ikinci bir kopya (`crates/fihrist-search`) vardı; hiçbir crate
//! ona bağlı değildi ve 2026-09-17'de kaldırıldı. Aynı hataya düşülmesin diye
//! not: ölçülecek ve iyileştirilecek kod `ibnunnedim` ikilisinin kullandığı
//! `src/arama.rs`'tir.
//!
//! VERİ: gerçek kütüphanenin şeklinde sentetik gövde (2026-09-17 ölçümü,
//! `kutup_kutuphane.db` + `kutup_depolar.db`):
//!   yetenekler 145 satır — ad ~15, aciklama ~79, tam_metin_md ort. 8.289,
//!              en uzun 71.328 karakter
//!   depolar     85 satır — tam_metin_md ort. 189 karakter
//! Toplam ~1,2 milyon karakter. CLI bu gövdenin tamamını HER aramada `Indeks::kur` ile
//! yeniden belirteçliyor; `indeks_kur` o maliyeti ölçüyor.

use ibnunnedim_cli::arama::{belirtecle, Belge, Indeks};

fn main() {
    divan::main();
}

/// Türkçe katlama yolunu (İ/I, ç ş ğ ü ö ı) ve markdown noktalamasını
/// çalıştıran bir sözlük. Rastgele ASCII, belirteçleyicinin asıl işini atlardı.
const KELIMELER: &[&str] = &[
    "yetenek",
    "arama",
    "İstanbul",
    "kaynakça",
    "ağaç",
    "şüphe",
    "örgü",
    "çözümleme",
    "IŞIK",
    "belge",
    "rasyonelleşme",
    "hukuk",
    "Weber",
    "düzenleme",
    "sıralama",
    "gömme",
    "vektör",
    "dizin",
    "ölçüm",
    "kapı",
    "Rust",
    "async",
    "tokio",
    "ayrıştırıcı",
    "önbellek",
    "eşzamanlılık",
    "güvenlik",
    "yapılandırma",
    "doğrulama",
    "çıktı",
];
const AYIRICILAR: &[&str] = &[
    " ", " ", " ", ", ", ". ", "\n", "\n## ", "\n- ", " `", "` ", " (",
];

/// Tohumlu LCG: bağımlılıksız ve her koşuda aynı gövde. Değişken veri
/// benchmark'lar arası karşılaştırmayı anlamsızlaştırır.
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

/// `hedef` KARAKTER, bayt değil: gerçek ölçüm SQLite `LENGTH()` ile yapıldı ve
/// o karakter sayar. Türkçe harfler UTF-8'de 2 bayt; bayt saysaydık gövde
/// gerçekteki boyutun altında kalırdı.
fn metin(rng: &mut Lcg, hedef: usize) -> String {
    let mut s = String::with_capacity(hedef * 11 / 10 + 32);
    let mut n = 0;
    while n < hedef {
        let k = KELIMELER[rng.sonraki() % KELIMELER.len()];
        let a = AYIRICILAR[rng.sonraki() % AYIRICILAR.len()];
        n += k.chars().count() + a.chars().count();
        s.push_str(k);
        s.push_str(a);
    }
    s
}

fn belge(rng: &mut Lcg, md: usize) -> Belge {
    Belge {
        id: String::new(),
        ad: metin(rng, 15),
        aciklama: metin(rng, 79),
        tam_metin_md: metin(rng, md),
    }
}

/// Gerçek gövdenin `kat` katı. Ölçülen bölgenin dışında kurulur.
fn govde(kat: usize) -> Vec<Belge> {
    let mut rng = Lcg(0x5EED_F1A5);
    let mut v = Vec::with_capacity(230 * kat);
    for _ in 0..kat {
        // Uzun kuyruklu dağılım: bir 71k'lık dev, dört ~30k, gerisi ~7,2k ±%50.
        // Böylece 145 yeteneğin ortalaması gerçekteki ~8,3k'ya oturuyor.
        v.push(belge(&mut rng, 71_328));
        for _ in 0..4 {
            v.push(belge(&mut rng, 30_000));
        }
        for _ in 0..140 {
            let md = 3_609 + rng.sonraki() % 7_218;
            v.push(belge(&mut rng, md));
        }
        for _ in 0..85 {
            v.push(belge(&mut rng, 189));
        }
    }
    // Gövde gerçeklikten koparsa sayılar anlamını yitirir: burada patlasın.
    let karakter: usize = v
        .iter()
        .map(|b| b.tam_metin_md.chars().count())
        .sum::<usize>()
        / kat;
    assert!(
        (1_100_000..1_350_000).contains(&karakter),
        "sentetik gövde gerçek kütüphaneden saptı: kat başına {karakter} karakter (beklenen ~1,2 milyon)"
    );
    v
}

/// Ortalama boyutta tek bir yetenek metni: belirteçleyicinin çıplak maliyeti.
#[divan::bench]
fn belirtecle_belge(bencher: divan::Bencher) {
    let s = metin(&mut Lcg(7), 8_289);
    bencher.bench(|| belirtecle(divan::black_box(&s)));
}

/// Her aramada bir kez belirteçlenen tipik sorgu.
#[divan::bench]
fn belirtecle_sorgu() -> Vec<String> {
    belirtecle(divan::black_box("Weber hukukun rasyonelleşmesi İstanbul"))
}

/// Tam gövdeden indeks kurmak — CLI bunu her aramada yapıyor.
#[divan::bench]
fn indeks_kur(bencher: divan::Bencher) {
    bencher.with_inputs(|| govde(1)).bench_values(Indeks::kur);
}

/// Hazır indekste BM25 sorgusu. `kat`: 1 = bugünkü kütüphane, 4 = büyüme.
/// `ara` her sorguda belge sayısı kadar puan dizisi ayırıp taradığı için
/// ölçeklenme burada görünür.
#[divan::bench(args = [1, 4])]
fn ara(bencher: divan::Bencher, kat: usize) {
    let indeks = Indeks::kur(govde(kat));
    bencher.bench(|| indeks.ara(divan::black_box("Weber hukukun rasyonelleşmesi"), 20));
}
