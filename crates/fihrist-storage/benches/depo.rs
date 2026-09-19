//! `fihrist-storage` sorgu yollarının CodSpeed benchmark'ları.
//!
//! VERİTABANI: `:memory:` Turso — disk yok, ölçüm SQL çözümleme + B-ağacı
//! taraması + satırdan `Skill` kurma maliyetidir. Gerçek `kutup_kutuphane.db`
//! depoya girmez ve CI'da bulunmaz; şekli (145 yetenek, ~80 karakter açıklama,
//! virgüllü etiket dizgisi) burada sentetik olarak yeniden kurulur.
//!
//! ÖLÇEK: 145 satır bugünkü kütüphane, 1.450 satır büyüme senaryosu.
//! `search_skills` üç sütunda `LIKE '%…%'` koştuğu için tam tarama yapar —
//! ölçeklenme tam olarak orada görünür.
//!
//! Tokio çalışma zamanı ölçülen bölgenin DIŞINDA kurulur; her yinelemede
//! `block_on` ile yalnız sorgu koşar.

use fihrist_core::QueryFilter;
use fihrist_storage::{SkillStore, TursoSkillStore};
use tokio::runtime::Runtime;
use turso::Builder;

fn main() {
    divan::main();
}

const KATEGORILER: &[&str] = &["arama", "kod", "belge", "ölçüm", "ajan"];
const ETIKET_HAVUZU: &[&str] = &[
    "türkçe", "bm25", "gömme", "vektör", "docx", "atıf", "graf", "şüphe",
];

/// Tohumlu LCG: her koşuda aynı gövde. Değişken veri karşılaştırmayı bozar.
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

/// `adet` yetenekli bellek içi depo. Şema `kutup_kutuphane.db`'nin
/// `yetenekler` tablosuyla aynı sütunlara sahiptir.
fn depo(rt: &Runtime, adet: usize) -> TursoSkillStore {
    rt.block_on(async move {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("bellek içi Turso kurulabilir");
        let conn = db.connect().expect("bağlantı açılabilir");

        conn.execute(
            "CREATE TABLE yetenekler (
                 id TEXT PRIMARY KEY,
                 ad TEXT NOT NULL,
                 aciklama TEXT NOT NULL,
                 kategori TEXT NOT NULL,
                 etiketler TEXT NOT NULL
             )",
            (),
        )
        .await
        .expect("şema kurulabilir");

        let mut rng = Lcg(0x5EED_D0B0);
        for i in 0..adet {
            let etiketler: Vec<&str> = (0..3)
                .map(|_| ETIKET_HAVUZU[rng.sonraki() % ETIKET_HAVUZU.len()])
                .collect();
            // Aranan terim ("gömme") satırların yaklaşık beşte birinde
            // geçsin: LIKE taraması hem eşleşen hem eşleşmeyen dalı görsün.
            let aciklama = if i % 5 == 0 {
                format!("Yetenek {i}: çok dilli gömme ve vektör füzyonu üzerine ölçülmüş bir akış")
            } else {
                format!("Yetenek {i}: Türkçe katlamalı BM25 araması ve rapor üretimi üzerine akış")
            };
            conn.execute(
                "INSERT INTO yetenekler (id, ad, aciklama, kategori, etiketler) VALUES (?, ?, ?, ?, ?)",
                turso::params![
                    format!("yetenek-{i:05}"),
                    format!("Yetenek {i} — kaynakça çözümleyici"),
                    aciklama,
                    KATEGORILER[i % KATEGORILER.len()].to_string(),
                    etiketler.join(", "),
                ],
            )
            .await
            .expect("satır eklenebilir");
        }

        TursoSkillStore::new(conn)
    })
}

/// `count(*)` — CLI her açılışta kütüphane boyutunu bununla yazar.
#[divan::bench(args = [145, 1_450])]
fn yetenek_say(bencher: divan::Bencher, adet: usize) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let store = depo(&rt, adet);
    bencher.bench(|| rt.block_on(store.count_skills()).expect("sayım başarılı"));
}

/// Sayfalı listeleme: `ibnunnedim listele` bir sayfa (20 satır) çeker.
#[divan::bench(args = [20, 100])]
fn yetenek_listele(bencher: divan::Bencher, limit: usize) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let store = depo(&rt, 1_450);
    bencher.bench(|| {
        rt.block_on(store.list_skills(divan::black_box(limit), divan::black_box(0)))
            .expect("listeleme başarılı")
    });
}

/// Birincil anahtarla tek satır: gömme boru hattı bunu satır başına çağırır.
#[divan::bench]
fn yetenek_getir(bencher: divan::Bencher) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let store = depo(&rt, 1_450);
    bencher.bench(|| {
        rt.block_on(store.get_skill(divan::black_box("yetenek-00720")))
            .expect("sorgu başarılı")
            .expect("kayıt var")
    });
}

/// Üç sütunda `LIKE '%terim%'`. Vektör/BM25 kanalına düşmeden önceki en
/// kaba arama yolu ve iki ayrı maliyet profili:
///   `gömme`          → eşleşme yoğun, `LIMIT 20` taramayı erken kesiyor
///   `bulunmayanterim` → hiç eşleşme yok, tablo SONUNA kadar taranıyor
/// İkisini birlikte ölçmek şart: yalnız ilki ölçülse tam tarama regresyonu
/// görünmez (1.450 satırda ilk ölçüm 145 satırdaki ile aynı çıkıyor).
#[divan::bench(args = ["gömme", "bulunmayanterim"])]
fn yetenek_ara(bencher: divan::Bencher, terim: &str) {
    let rt = Runtime::new().expect("çalışma zamanı kurulabilir");
    let store = depo(&rt, 1_450);
    let filtre = QueryFilter {
        terim: terim.to_string(),
        kategori: None,
        limit: 20,
        offset: 0,
    };
    bencher.bench(|| {
        rt.block_on(store.search_skills(divan::black_box(&filtre)))
            .expect("arama başarılı")
    });
}
