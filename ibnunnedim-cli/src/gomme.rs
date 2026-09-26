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
//!
//! KARAR KAPISI SONUCU — hash256 vs ollama-bge-m3 (145 kayıt, 18 sorgu,
//! limit 5, aynı altın küme, gerçek kütüphanenin bir kopyası üzerinde):
//!
//!   kanal      hash256                    ollama-bge-m3
//!   gomme      11/18 · %61 · MRR 0.52     18/18 · %100 · MRR 0.89
//!   bm25       10/18 · %56 · MRR 0.46     (aynı, kanal çekirdekten bağımsız)
//!   karmasik   11/18 · %61 · MRR 0.57     17/18 · %94  · MRR 0.73
//!
//! e5s384'ün düştüğü tuzağa (izole kanal kazanıp füzyonda kaybetmesi) burada
//! DÜŞÜLMEDİ: ollama-bge-m3 hem izole gömme kanalında hem füzyonda hash256'yı
//! GERİDE BIRAKIYOR — özellikle (c) diller arası öbekte (Türkçe sorgu →
//! İngilizce belge), hash256'nın yapısı gereği çözemediği sorgularda
//! ('makale ara' → research/arxiv, 'toplantı notlarından görev çıkar' →
//! meeting-action-items, vb.) isabet sağlıyor. Bu yüzden `#[default]`
//! `BgeM3`'e taşındı — tahminle değil, ölçümle. (Ölçüm Ollama ile yapıldı;
//! 2026-09-18'den beri aynı GGUF'u llama-server servis ediyor.)
//!
//! Bedel: sorgu başına ~71 ms (ağ + model), hash256'nın ~0,05 ms'sine karşı.
//! Bir arama aracı için gözden kaybolacak kadar küçük; kabul edildi. Ollama
//! ayakta değilse/`bge-m3` çekilmemişse `gomme()` hata döner, çağıran
//! (`govde_yukle`) bunu "vektör yok" sayıp saf BM25'e düşer ve stderr'e
//! yazar — sessiz düşüş yok, yalnızca yavaşça bozulma.

use crate::arama::belirtecle;
use std::collections::HashMap;

/// Hash çekirdeğinin boyutu. pasli-beyin ile aynı — altın küme sayıları
/// karşılaştırılabilir kalsın.
pub const BOYUT_HASH: usize = 256;

/// Gömmenin hangi rolde üretildiği. `hash256` yok sayar; çok dilli E5
/// ailesi "query:" / "passage:" öneki ister ve öneksiz sessizce kalite kaybeder.
/// `ollama-bge-m3` de yok sayar — bge-m3 E5 ailesinin önek sözleşmesini
/// paylaşmaz, genel amaçlı tek-metin gömmesi olarak eğitilmiş.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rol {
    Sorgu,
    Belge,
}

/// Seçilebilir gömme çekirdeği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Cekirdek {
    /// Yerel hash n-gram: sıfır bağımlılık, ağ yok, model indirmesi yok.
    /// llama-server'a erişilemezse (`gomme()` hata verirse) düşülecek elle seçim.
    /// `hash256-k2`: imzadaki ad (bkz. `kip`); onarım ipucu onu yazdırır.
    #[value(name = "hash256", alias = "hash256-k2")]
    Hash256,
    /// `intfloat/multilingual-e5-small` (384 boyut, ONNX). Diller arası
    /// hizalama tam olarak eğitildiği şey — ama füzyonda hash256'dan GERİYE
    /// gitti (bkz. modül belgesi), o yüzden varsayılan olmadı.
    #[cfg(feature = "onnx")]
    #[value(name = "e5s384")]
    E5s384,
    /// `bge-embed-rs` (saf Rust/candle) üzerinden `bge-m3` (1024 boyut, çok
    /// dilli). Ağa çıkmaz — yalnız yerel uç (varsayılan 127.0.0.1:11434,
    /// `FIHRIST_BGE_URL` ile değişir; bkz. `bge_m3` modül belgesi). VARSAYILAN: ölçüm hem izole kanalda hem füzyonda hash256'yı
    /// geride bıraktığını gösterdi (bkz. modül belgesindeki karar kapısı).
    /// Sunucu kapalıysa `gomme()` hata döner; karma arama bunu stderr'e
    /// yazıp BM25'e düşer. `ollama-bge-m3` eski ad (2026-09-18'e dek
    /// Ollama'ydı, sonra llama-server'a geçti), takma ad kaldı.
    #[default]
    #[value(name = "bge-m3", alias = "ollama-bge-m3")]
    BgeM3,
}

impl Cekirdek {
    /// İmzaya giren kısa ad. DEĞİŞTİRME: değişirse tüm satırlar bayatlar
    /// (ki çekirdek gerçekten değiştiyse istenen de budur).
    pub fn kip(self) -> &'static str {
        match self {
            // k2 (2026-09-25): belirteçleme ortak `katla`'ya geçti (şapka,
            // aksan, ayrışık yazım artık katlanıyor) → eski hash vektörleri
            // farklı belirteçlerden üretilmişti, bayat sayılmalılar.
            Cekirdek::Hash256 => "hash256-k2",
            #[cfg(feature = "onnx")]
            Cekirdek::E5s384 => "e5s384",
            Cekirdek::BgeM3 => "bge-m3",
        }
    }

    pub fn boyut(self) -> usize {
        match self {
            Cekirdek::Hash256 => BOYUT_HASH,
            #[cfg(feature = "onnx")]
            Cekirdek::E5s384 => 384,
            Cekirdek::BgeM3 => bge_m3::BOYUT,
        }
    }

    /// Kosinüs gürültü tabanı — bu değerin altındaki benzerlik füzyona
    /// GİRMEZ. Çekirdek başına kalibrasyon düğmesi: pasli-beyin ölçümü
    /// gömme kanalının anlamadığı sorgularda 0,25-0,37 bandında güvenli
    /// görünen gürültü döndürdüğünü kaydetmiş. Normalize füzyonda bu taban
    /// onu bastırır; RRF'te bastıramazdık (RRF puan büyüklüğünü atar).
    ///
    /// E5 kosinüsleri DAR VE YÜKSEK bantta durur (ölçüldü: 0,76-0,90), yani
    /// hash'in 0,05'i orada hiçbir şey elemez. Ama E5 için ÇALIŞAN bir eşik
    /// de yok — gerekçe `E5_TABAN` sabitinde, ölçülen sayılarla.
    ///
    /// `ollama-bge-m3` için de aynı disiplin: E5'te olduğu gibi isabetli/
    /// ıskalı sorguların kosinüs bantları örtüşebilir — uydurma bir eşik
    /// yazmaktansa 0.0 (eleme yok) ile başla, `ibnunnedim olcum` gerçek
    /// dağılımı ölçünce (E5_TABAN'ın türetildiği yöntemle) kalibre et.
    pub fn taban(self) -> f32 {
        match self {
            Cekirdek::Hash256 => 0.05,
            #[cfg(feature = "onnx")]
            Cekirdek::E5s384 => E5_TABAN,
            Cekirdek::BgeM3 => 0.0,
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
    l2_normalize(&mut v);
    v
}

/// Yerinde L2 normalizasyon. Norm 0 ise dokunmaz (sıfır vektör anlamsız
/// bölme üretmesin).
fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// E5 kosinüs gürültü tabanı: **0,0 — yani eleme YOK.** Tahmin değil, ölçüm.
///
/// Plan ~0,75 öngörüyordu. 18 altın sorguda 230 belgeye karşı kosinüs
/// dağılımı ölçüldü ve tabanın burada İŞE YARAMADIĞI çıktı: isabetli ve
/// ıskalı sorguların bantları TAMAMEN örtüşüyor.
///   `github pull request` (isabet)       top1 0,8836
///   `engellenmiş sayfayı aç` (ıska)      top1 0,8497
///   `obsidian` (isabet)                  top1 0,8490
///   `kod deposu satır sayısı` (isabet)   top1 0,8420
///   `sunum için ... animasyonu` (isabet) top1 0,8296
///   `haftalık gözden geçirme` (ıska)     top1 0,8231
/// Iskanın en iyi puanı isabetinkinden YÜKSEK. Hiçbir eşik "E5 anladı" ile
/// "E5 tahmin ediyor"u ayırmıyor; eleyen her değer gerçek isabeti de eler.
/// Uydurma bir sayı yazmaktansa düğme kapalı bırakıldı ve nedeni yazıldı.
///
/// Sonucu şu: normalize füzyon, anlamayan kanalın çöpünü kendi maksimumuna
/// bölüp 1,0'a çıkarıyor ve taban bunu bastıramıyor. `karmasik` MRR'inin
/// hash256'da 0,56 iken e5s384'te 0,53'e DÜŞMESİNİN nedeni bu.
#[cfg(feature = "onnx")]
const E5_TABAN: f32 = 0.0;

/// ONNX bacağı: `multilingual-e5-small`, fastembed üzerinden.
#[cfg(feature = "onnx")]
mod e5 {
    use super::Rol;
    use std::sync::OnceLock;

    /// Model TEK SEFER yüklenir. `search` sorgu başına, `olcum` 54 kez
    /// çağırıyor; her çağrıda ONNX oturumu kurmak saniyeler sürerdi.
    ///
    /// Hata METNİ saklanır: `TextEmbedding` de fastembed hatası da `Clone`
    /// değil, ama ikinci çağrının da nedeni GÖRMESİ gerek — `OnceLock<Option>`
    /// olsaydı ilk hatadan sonra sessizce "model yok" derdik.
    static MODEL: OnceLock<Result<fastembed::TextEmbedding, String>> = OnceLock::new();

    /// Model önbelleği. fastembed'in varsayılanı ÇALIŞMA DİZİNİNDE
    /// `.fastembed_cache` — nereden koşarsan oraya ~120 MB bırakır, depoların
    /// içine bile. Kalıcı bir yere sabitlenir; `FASTEMBED_CACHE_DIR`
    /// verilmişse ona uyulur. Varsayılan yer bu makinede model zaten orada
    /// durduğu için seçildi (setup-rustified indirmiş) — yeniden inmesin.
    fn onbellek_dizini() -> std::path::PathBuf {
        if let Ok(v) = std::env::var("FASTEMBED_CACHE_DIR") {
            return v.into();
        }
        for kok in ["USERPROFILE", "HOME"] {
            if let Ok(h) = std::env::var(kok) {
                return std::path::PathBuf::from(h)
                    .join(".claude")
                    .join("fastembed_cache");
            }
        }
        ".fastembed_cache".into()
    }

    pub fn model() -> crate::Result<&'static fastembed::TextEmbedding> {
        MODEL
            .get_or_init(|| {
                fastembed::TextEmbedding::try_new(
                    fastembed::InitOptions::new(fastembed::EmbeddingModel::MultilingualE5Small)
                        .with_cache_dir(onbellek_dizini())
                        .with_show_download_progress(true),
                )
                .map_err(|h| h.to_string())
            })
            .as_ref()
            .map_err(|h| {
                crate::CliError::Girdi(format!(
                    "e5s384 yüklenemedi: {h}\n  Önbellek: {}\n  Başka yer: FASTEMBED_CACHE_DIR",
                    onbellek_dizini().display()
                ))
            })
    }

    /// E5 ÖNEK İSTER ve fastembed onu EKLEMEZ (kendi belgesindeki örnekte de
    /// çağıran ekliyor). Öneksiz hata vermez, yalnız sessizce kötü sıralar —
    /// bu yüzden `Rol` en baştan arayüzde duruyor.
    pub fn onek(rol: Rol) -> &'static str {
        match rol {
            Rol::Sorgu => "query: ",
            Rol::Belge => "passage: ",
        }
    }
}

/// bge-m3 bacağı: `bge-embed-rs`'in (saf Rust/candle, C++ ikiliye bağımsız)
/// OpenAI uyumlu `/v1/embeddings` ucu (toplu). 2026-09-19'a dek 11434'teki
/// llama-server kullanılıyordu — Open Notebook'un aynı sunucuyu 790 eserlik
/// toplu gömme backlog'u için doldurmasıyla kısa sorgular aynı kuyrukta
/// dakikalarca bekliyordu (ölçüldü: 95 sn). O bekleme llama-server'ın istekleri
/// SIRAYLA işleyen yuvasından geliyordu. 2026-09-19'da el-Fihrist ayrı bir
/// `bge-embed-rs` örneğine (11435) geçti; 2026-09-23'te `start.ps1` llama-
/// server'ın yerine `bge-embed-rs`'i 11434'te başlattı ve 11435 örneği bir
/// daha açılmadı — el-Fihrist o günden beri sessizce saf BM25'teydi (2026-09-24
/// ölçüldü: 11435'i dinleyen süreç 0). `bge-embed-rs` istekleri eşzamanlı
/// işlediğinden kuyruk sorunu yok, yalnız CPU paylaşılıyor; ikinci bir örnek
/// ~1,4 GB bellek ister. Bu yüzden varsayılan paylaşılan 11434; ayrı örnek
/// gerekirse `FIHRIST_BGE_URL`. Dizi tek istekte gider — `olcum` 145+ belgeyi
/// 145 round-trip yerine 1'de.
mod bge_m3 {
    use serde::Deserialize;

    const VARSAYILAN_UC: &str = "http://127.0.0.1:11434/v1/embeddings";
    const MODEL: &str = "bge-m3";

    fn uc_nokta() -> String {
        std::env::var("FIHRIST_BGE_URL").unwrap_or_else(|_| VARSAYILAN_UC.to_string())
    }

    /// bge-m3'ün doğal çıktı boyutu. Sabit yazılır ama KÖRÜNE güvenilmez:
    /// `gomme_toplu` her vektörün gerçekten bu uzunlukta geldiğini denetler
    /// — sunucu başka bir GGUF ile açılmışsa (llama-server `model` alanını
    /// yok sayar) sessizce yanlış boyutlu vektör kabul edilmesin.
    pub const BOYUT: usize = 1024;

    #[derive(Deserialize)]
    struct Cevap {
        data: Vec<Oge>,
    }

    #[derive(Deserialize)]
    struct Oge {
        index: usize,
        embedding: Vec<f32>,
    }

    /// Toplu gömme çağrısı. Sunucu kapalıysa hata döner; karma arama bunu
    /// stderr'e yazıp BM25'e düşer — sessiz düşüş yok.
    pub async fn gomme_toplu(
        metinler: &[String],
        sure: Option<std::time::Duration>,
    ) -> crate::Result<Vec<Vec<f32>>> {
        let mut kurucu = reqwest::Client::builder();
        if let Some(s) = sure {
            kurucu = kurucu.timeout(s);
        }
        let istemci = kurucu
            .build()
            .map_err(|e| crate::CliError::Girdi(format!("bge-m3: istemci kurulamadı: {e}")))?;
        let govde = serde_json::json!({ "model": MODEL, "input": metinler });
        let uc = uc_nokta();
        let yanit = istemci.post(&uc).json(&govde).send().await.map_err(|e| {
            if e.is_timeout() {
                return crate::CliError::Girdi(
                    "bge-m3: sunucu süresinde yanıt vermedi (kuyruk dolu olabilir)".into(),
                );
            }
            crate::CliError::Girdi(format!(
                "bge-m3: bge-embed-rs'e bağlanılamadı ({e}). {uc} çalışıyor mu?"
            ))
        })?;
        let yanit = yanit
            .error_for_status()
            .map_err(|e| crate::CliError::Girdi(format!("bge-m3: sunucu hatası: {e}")))?;
        let mut govde: Cevap = yanit
            .json()
            .await
            .map_err(|e| crate::CliError::Girdi(format!("bge-m3: yanıt ayrıştırılamadı: {e}")))?;

        if govde.data.len() != metinler.len() {
            return Err(crate::CliError::Girdi(format!(
                "bge-m3: {} metin gönderildi, {} vektör döndü",
                metinler.len(),
                govde.data.len()
            )));
        }
        // OpenAI sözleşmesi sırayı `index` ile verir, dizi sırasıyla değil.
        govde.data.sort_by_key(|o| o.index);
        for o in &govde.data {
            if o.embedding.len() != BOYUT {
                return Err(crate::CliError::Girdi(format!(
                    "bge-m3: beklenen {BOYUT} boyut, {} geldi — sunucu 'bge-m3' GGUF'uyla mı açık?",
                    o.embedding.len()
                )));
            }
        }
        Ok(govde.data.into_iter().map(|o| o.embedding).collect())
    }
}

/// Metinleri vektöre çevirir.
///
/// TOPLU imza (tekil değil): ONNX çekirdeği modeli bir kez yükleyip toplu
/// geçirmek zorunda, Ollama da 145 belgeyi tek istekte göndermek için aynı
/// şekle ihtiyaç duyar; tekil imza her ikisinde de gizli bir tutamak ya da
/// N ayrı ağ isteği dayatırdı.
pub async fn gomme(metinler: &[String], rol: Rol, k: Cekirdek) -> crate::Result<Vec<Vec<f32>>> {
    match k {
        Cekirdek::Hash256 => {
            let _ = rol; // hash çekirdeği rolü yok sayar
            Ok(metinler.iter().map(|m| hash_gomme(m)).collect())
        }
        #[cfg(feature = "onnx")]
        Cekirdek::E5s384 => {
            let model = e5::model()?;
            let onek = e5::onek(rol);
            let onekli: Vec<String> = metinler.iter().map(|m| format!("{onek}{m}")).collect();
            // fastembed çıktıyı L2 normalize ediyor (text_embedding/output.rs)
            // → `kosinus` nokta çarpımı olarak kalır, ikinci normalize yok.
            model
                .embed(onekli, None)
                .map_err(|h| crate::CliError::Girdi(format!("e5s384 gömme başarısız: {h}")))
        }
        Cekirdek::BgeM3 => {
            // bge-m3 E5 ailesinin önek sözleşmesini paylaşmaz; rol yalnız
            // bekleme süresini belirler. Sunucu Open Notebook'la paylaşılıyor
            // (bkz. `bge_m3` modül belgesi); toplu gömme CPU'yu doldurabilir.
            // Arama sorgusu (Rol::Sorgu) uzun sürmemeli: sabit 3 sn üst sınırı
            // bu durumda bile aramanın BM25'e düşmesini garanti eder.
            let sure = match rol {
                Rol::Sorgu => Some(std::time::Duration::from_secs(3)),
                Rol::Belge => None,
            };
            let mut vs = bge_m3::gomme_toplu(metinler, sure).await?;
            // Ollama'nın L2 normalize garantisi belgelenmemiş — `kosinus`
            // nokta çarpımını kosinüs SAYAR, bu yüzden burada normalize
            // ETTİĞİMİZDEN emin oluyoruz (hash256 ile aynı disiplin).
            for v in &mut vs {
                l2_normalize(v);
            }
            Ok(vs)
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

    #[tokio::test]
    async fn deterministik_ve_normalize() {
        let m = vec!["atif dogrulama motoru".to_string()];
        let a = gomme(&m, Rol::Belge, Cekirdek::Hash256).await.unwrap();
        let b = gomme(&m, Rol::Belge, Cekirdek::Hash256).await.unwrap();
        assert_eq!(a, b, "aynı girdi aynı vektörü vermeli");
        let norm: f32 = a[0].iter().map(|x| x * x).sum();
        assert!((norm - 1.0).abs() < 1e-4, "L2 normalize değil: {norm}");
        assert_eq!(a[0].len(), Cekirdek::Hash256.boyut());
    }

    /// Türkçe katlama `belirtecle`den geliyor; trigramlar biçim bilgisini
    /// taşıyor. Şapkalı/şapkasız aynı sözcük, alakasız metinden yakın olmalı.
    #[tokio::test]
    async fn turkce_katlama_benzerligi_tasir() {
        async fn v(s: &str) -> Vec<f32> {
            gomme(&[s.to_string()], Rol::Belge, Cekirdek::Hash256)
                .await
                .unwrap()[0]
                .clone()
        }
        let yakin = kosinus(&v("kaynakça belgeleri").await, &v("kaynakca").await);
        let uzak = kosinus(&v("kaynakça belgeleri").await, &v("tepsi simgesi").await);
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

    /// Modelsiz koşar: iki çekirdek KARIŞMAMALI. İmza aynı olsaydı 256'lık
    /// vektör 384'lük sanılır, boyut kontrolü olmasa saçma kosinüs üretilirdi.
    #[cfg(feature = "onnx")]
    #[test]
    fn e5_hash_ile_karismaz() {
        assert_eq!(Cekirdek::E5s384.boyut(), 384);
        assert_ne!(Cekirdek::E5s384.kip(), Cekirdek::Hash256.kip());
        assert_ne!(
            imza(Cekirdek::E5s384.kip(), "abc"),
            imza(Cekirdek::Hash256.kip(), "abc")
        );
        // Taban için "E5 daha yüksek olmalı" diye bir kural YOK: ölçüm
        // E5'te ayıran eşik bulunmadığını gösterdi (bkz. `E5_TABAN`).
        // Tutması gereken sözleşme boyut ayrımı — 256'lık vektör 384'lük
        // sanılırsa `kosinus` sessizce 0 döndürür, arama da sessizce körelir.
        assert_ne!(Cekirdek::E5s384.boyut(), Cekirdek::Hash256.boyut());
    }

    /// MODEL GEREKTİRİR (~120 MB, ilk koşumda indirir) — bu yüzden `ignore`.
    /// Koşumu: `cargo test --features onnx -- --ignored e5_`
    #[cfg(feature = "onnx")]
    #[tokio::test]
    #[ignore = "ONNX modeli gerekir"]
    async fn e5_rol_oneki_vektoru_degistirir() {
        let m = vec!["dosya arama motoru".to_string()];
        let s = gomme(&m, Rol::Sorgu, Cekirdek::E5s384).await.unwrap();
        let b = gomme(&m, Rol::Belge, Cekirdek::E5s384).await.unwrap();
        assert_eq!(s[0].len(), 384);
        let norm: f32 = s[0].iter().map(|x| x * x).sum();
        assert!((norm - 1.0).abs() < 1e-3, "L2 normalize değil: {norm}");
        // "query:" ve "passage:" AYNI vektörü verirse önek uygulanmıyordur.
        assert!(
            kosinus(&s[0], &b[0]) < 0.9999,
            "rol öneki vektöre yansımadı"
        );
    }

    /// OLLAMA GEREKTİRİR (yerel sunucu + `bge-m3` çekilmiş) — bu yüzden
    /// `ignore`. Koşumu: `cargo test -- --ignored ollama_`
    #[tokio::test]
    #[ignore = "yerel Ollama sunucusu ve bge-m3 modeli gerekir"]
    async fn ollama_boyut_ve_normalize() {
        let m = vec!["dosya arama motoru".to_string()];
        let v = gomme(&m, Rol::Belge, Cekirdek::BgeM3).await.unwrap();
        assert_eq!(v[0].len(), Cekirdek::BgeM3.boyut());
        let norm: f32 = v[0].iter().map(|x| x * x).sum();
        assert!((norm - 1.0).abs() < 1e-3, "L2 normalize değil: {norm}");
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
