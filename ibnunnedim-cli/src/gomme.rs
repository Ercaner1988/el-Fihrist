//! Gömme katmanı — değişken çekirdek.
//!
//! Çekirdek ÇALIŞMA ZAMANINDA `--kip` ile seçilir; hangi çekirdeklerin
//! derlendiğini cargo feature'ı belirler. Böylece iki çekirdek TEK KOŞUDA
//! aynı altın küme üzerinde karşılaştırılabilir — "hangisi daha iyi" sorusu
//! yeniden derleme gerektirmeden ölçülür.
//!
//! DÜRÜSTLÜK KURALI (pasli-beyin `gomme.rs`'ten devralındı): `hash256`
//! vektörleri SEMANTİK iddia taşımaz — morfolojik benzerliktir. Raporda
//! "yerel gömme (hash n-gram)" diye anılır. Ağa çıkmaz, model indirmez.
//!
//! ÖLÇÜLEN GERÇEK (2026-09-09): katalogdaki 145 açıklamanın 127'si İNGİLİZCE,
//! sorgular ise Türkçe. Yani asıl problem diller arası erişim. Karakter
//! üçlüsü çeviri bilmez — `hash256`ın altın kümenin (c) öbeğini çözmesi
//! BEKLENMİYOR. Beklentiyi ölçüm doğrulasın ya da çürütsün: `ibnunnedim olcum`.

use crate::arama::belirtecle;
use std::collections::HashMap;

/// Hash çekirdeğinin boyutu. pasli-beyin ile aynı — altın küme sayıları
/// karşılaştırılabilir kalsın.
pub const BOYUT_HASH: usize = 256;

/// Gömmenin hangi rolde üretildiği. `hash256` yok sayar; çok dilli E5
/// ailesi "query:" / "passage:" öneki ister ve öneksiz sessizce kalite kaybeder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rol {
    Sorgu,
    Belge,
}

/// Seçilebilir gömme çekirdeği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Cekirdek {
    /// Yerel hash n-gram: sıfır bağımlılık, ağ yok, model indirmesi yok.
    #[default]
    #[value(name = "hash256")]
    Hash256,
}

impl Cekirdek {
    /// İmzaya giren kısa ad. DEĞİŞTİRME: değişirse tüm satırlar bayatlar
    /// (ki çekirdek gerçekten değiştiyse istenen de budur).
    pub fn kip(self) -> &'static str {
        match self {
            Cekirdek::Hash256 => "hash256",
        }
    }

    pub fn boyut(self) -> usize {
        match self {
            Cekirdek::Hash256 => BOYUT_HASH,
        }
    }

    /// Kosinüs gürültü tabanı — bu değerin altındaki benzerlik füzyona
    /// GİRMEZ. Çekirdek başına kalibrasyon düğmesi: pasli-beyin ölçümü
    /// gömme kanalının anlamadığı sorgularda 0,25-0,37 bandında güvenli
    /// görünen gürültü döndürdüğünü kaydetmiş. Normalize füzyonda bu taban
    /// onu bastırır; RRF'te bastıramazdık (RRF puan büyüklüğünü atar).
    pub fn taban(self) -> f32 {
        match self {
            Cekirdek::Hash256 => 0.05,
        }
    }
}

/// `<kip>:<icerik_hash>` — tek eşitlik testi hem içerik bayatlığını hem
/// çekirdek değişimini yakalar. İki ayrı sütun tutmanın anlamı yok.
pub fn imza(kip: &str, icerik_hash: &str) -> String {
    format!("{kip}:{icerik_hash}")
}

/// Bir yeteneğin gömülecek metni. TEK tanım: iki çekirdek de bunu kullanır,
/// yoksa altın küme karşılaştırması elmayla armut olur.
///
/// `ad` iki kez geçer — başlık BM25'te de ×3 ağırlıklı (`arama.rs`), iki
/// kanalın aynı sinyali önemsemesi füzyonu tutarlı kılar.
pub fn belge_metni(ad: &str, aciklama: &str, tam_metin_md: &str) -> String {
    // ponytail: ilk 2000 karakter. Ölçüldü: tam_metin_md ortalama 8.289
    // karakter, E5-small ise 512 belirteçte kesiyor — kırpmazsak model zaten
    // görmediği metne göre vektör üretmiş gibi davranırdık. Parça-parça
    // gömme (ayrı tablo, chunk başına satır) gerekirse yükseltme yolu budur.
    let kirpik: String = tam_metin_md.chars().take(2000).collect();
    format!("{ad} {ad} {aciklama} {kirpik}")
}

fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Hash n-gram gömmesi: katlanmış belirteçler + karakter üçlüleri FNV ile
/// hücrelere dağıtılır, `ln(sayım)+1` ağırlıklanır, L2 normalize edilir.
///
/// Belirteçleme `arama::belirtecle`den gelir — BM25 kanalıyla AYNI kural.
/// Bu şart: iki kanal farklı belirteçlerse füzyonun anlamı kalmaz. (Aynı
/// katlama pasli-beyin ile 2026-09-04'te bilerek eşitlenmişti.)
fn hash_gomme(metin: &str) -> Vec<f32> {
    let mut v = vec![0f32; BOYUT_HASH];
    let mut say: HashMap<usize, f32> = HashMap::new();
    let mut koy = |anahtar: String, agirlik: f32| {
        let hucre = (fnv1a64(&anahtar) % BOYUT_HASH as u64) as usize;
        *say.entry(hucre).or_default() += agirlik;
    };
    for t in belirtecle(metin) {
        koy(format!("t:{t}"), 1.0);
        let harfler: Vec<char> = t.chars().collect();
        if harfler.len() >= 3 {
            for w in harfler.windows(3) {
                koy(format!("g:{}", w.iter().collect::<String>()), 0.5);
            }
        } else if !t.is_empty() {
            koy(format!("g:{t}"), 0.5);
        }
    }
    for (h, a) in say {
        v[h] = (a.ln() + 1.0).max(0.0);
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

/// Metinleri vektöre çevirir.
///
/// TOPLU imza (tekil değil): ONNX çekirdeği modeli bir kez yükleyip toplu
/// geçirmek zorunda; tekil imza ya her çağrıda yeniden yükleme ya da
/// çağıranda gizli bir tutamak dayatırdı.
pub fn gomme(metinler: &[String], rol: Rol, k: Cekirdek) -> crate::Result<Vec<Vec<f32>>> {
    match k {
        Cekirdek::Hash256 => {
            let _ = rol; // hash çekirdeği rolü yok sayar
            Ok(metinler.iter().map(|m| hash_gomme(m)).collect())
        }
    }
}

/// Kosinüs benzerliği. Normalize vektörlerde nokta çarpımıdır.
///
/// Uzunluk uyuşmazsa 0.0 — farklı çekirdeklerin vektörleri karşılaştırılamaz
/// ve karşılaştırılırmış gibi davranmak sessiz saçmalık üretir.
pub fn kosinus(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// f32 dizisini little-endian BLOB'a yazar.
pub fn blob_yaz(v: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(v.len() * 4);
    for x in v {
        b.extend_from_slice(&x.to_le_bytes());
    }
    b
}

/// BLOB'u f32 dizisine okur. 4'e bölünmeyen uzunlukta `None` — bozuk blob
/// `chunks_exact` ile sessizce kırpılmaktansa açıkça reddedilir.
pub fn blob_oku(b: &[u8]) -> Option<Vec<f32>> {
    if b.is_empty() || !b.len().is_multiple_of(4) {
        return None;
    }
    let (dortlukler, _artik) = b.as_chunks::<4>(); // uzunluk yukarıda 4'ün katı
    Some(dortlukler.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn deterministik_ve_normalize() {
        let m = vec!["atif dogrulama motoru".to_string()];
        let a = gomme(&m, Rol::Belge, Cekirdek::Hash256).unwrap();
        let b = gomme(&m, Rol::Belge, Cekirdek::Hash256).unwrap();
        assert_eq!(a, b, "aynı girdi aynı vektörü vermeli");
        let norm: f32 = a[0].iter().map(|x| x * x).sum();
        assert!((norm - 1.0).abs() < 1e-4, "L2 normalize değil: {norm}");
        assert_eq!(a[0].len(), Cekirdek::Hash256.boyut());
    }

    /// Türkçe katlama `belirtecle`den geliyor; trigramlar biçim bilgisini
    /// taşıyor. Şapkalı/şapkasız aynı sözcük, alakasız metinden yakın olmalı.
    #[test]
    fn turkce_katlama_benzerligi_tasir() {
        let v =
            |s: &str| gomme(&[s.to_string()], Rol::Belge, Cekirdek::Hash256).unwrap()[0].clone();
        let yakin = kosinus(&v("kaynakça belgeleri"), &v("kaynakca"));
        let uzak = kosinus(&v("kaynakça belgeleri"), &v("tepsi simgesi"));
        assert!(yakin > uzak, "katlama taşımadı: yakın={yakin} uzak={uzak}");
    }

    /// SESSİZCE BOZULAN SÖZLEŞME: saklama biçimi. Bu kırılırsa DB'deki her
    /// vektör çöp olur ve arama hata vermeden yanlış sıralar.
    #[test]
    fn blob_gidis_donus() {
        let v = vec![0.0f32, 1.0, -0.5, 3.25e-8, f32::MIN_POSITIVE];
        assert_eq!(blob_oku(&blob_yaz(&v)), Some(v.clone()));
        // 4'e bölünmeyen uzunluk reddedilmeli — sessizce kırpılmamalı.
        assert_eq!(blob_oku(&[1, 2, 3]), None);
        assert_eq!(blob_oku(&[]), None);
        assert_eq!(blob_yaz(&v).len(), v.len() * 4);
    }

    #[test]
    fn imza_kip_degisince_bayatlar() {
        let h = "abc123";
        assert_eq!(imza("hash256", h), "hash256:abc123");
        assert_ne!(imza("hash256", h), imza("e5s384", h));
        assert_ne!(imza("hash256", h), imza("hash256", "baska"));
    }

    /// Farklı boyutlu vektörler karşılaştırılamaz; 0 dönmeli, panik değil.
    #[test]
    fn kosinus_uzunluk_uyusmazsa_sifir() {
        assert_eq!(kosinus(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
        assert_eq!(kosinus(&[1.0, 0.0], &[1.0, 0.0]), 1.0);
    }

    /// Uzun tam metin kırpılmalı — yoksa E5'in 512 belirteç sınırını aşan
    /// metne göre vektör üretmiş gibi davranırız.
    #[test]
    fn belge_metni_uzun_metni_kirpar() {
        let uzun = "x".repeat(9000);
        let m = belge_metni("ad", "aciklama", &uzun);
        assert!(
            m.chars().count() < 2100,
            "kırpılmadı: {}",
            m.chars().count()
        );
        assert!(
            m.starts_with("ad ad aciklama "),
            "ad iki kez geçmeli: {}",
            &m[..30]
        );
    }
}
