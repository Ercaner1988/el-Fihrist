//! İbnünnedîm CLI — Saf Rust BM25 Arama & Turso SQLite Yetenek Kütüphanesi
//!
//! `kutup_kutuphane.db` üzerinde saf Rust BM25 ile arama, yetenek listeleme ve tekmil puanlama
//! yapar. Arama bellekte kurulan BM25 indeksi üzerinden Türkçe katlamalı olarak gerçekleştirilir.
//!
//! ```bash
//! ibnunnedim search "rust"
//! ibnunnedim list --kategori "software-development"
//! ibnunnedim info
//! ibnunnedim graph --kur
//! ibnunnedim graph "atif dogrulama nerede gecer"
//! ibnunnedim mcp   # stdio MCP sunucusu (ajan/istemci bağlanır)
//! ibnunnedim tara ~/Desktop/Github   # depoları kataloğa yaz
//! ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Dış koşu geçti"
//! ibnunnedim kural-ekle "Bun kullan, Node'a kaçma" --kaynak "CLAUDE.md" --etiketler "araç,kural"
//! ibnunnedim kural-ara "bun"
//! ```

mod arama;
mod error;
mod gomme;
mod kayit;
mod mcp;
mod olcum;
mod tara;

use arama::{Belge, Indeks};
use clap::Parser;
use error::{CliError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use turso::{params, Builder, Connection};

/// Kütüphane dosyasının adı. Aranacak yerler için `kutuphane_yolu`.
const DB_ADI: &str = "kutup_kutuphane.db";

/// Depo kataloğu — ana kütüphanenin yanında ayrı dosya. Gerekçesi `depo_baglan`.
const DEPO_DB_ADI: &str = "kutup_depolar.db";

/// Araç çağrısı günlüğü — ayrı dosya: büyür, budanabilir, katalogla ömrü
/// ortak değil. Ana kütüphaneye yazılmaz (fts5 tuzağı, bkz. `depo_baglan`).
const KAYIT_DB_ADI: &str = "kutup_kayitlar.db";

/// Paylaşılan kural kataloğu (AGENTS.md/CLAUDE.md tarzı davranış kuralları,
/// araçlar-arası ortak) — ayrı dosya, aynı fts5 gerekçesiyle (bkz. `depo_baglan`).
const KURAL_DB_ADI: &str = "kutup_kurallar.db";

#[derive(Parser, Debug)]
#[command(name = "ibnunnedim")]
#[command(about = "İbnünnedîm Kütüphaneci CLI — Saf Rust BM25 Arama & Turso SQLite")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Saf Rust BM25 algoritması ile yetenek araması yapar
    Search {
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Gömme çekirdeği. DB'deki imzayla tutmayan satırlar kosinüse girmez.
        #[arg(long, value_enum, default_value_t = gomme::Cekirdek::default())]
        kip: gomme::Cekirdek,
    },
    /// Yetenekleri listeler
    List {
        #[arg(short, long)]
        kategori: Option<String>,
    },
    /// Veritabanı istatistiklerini gösterir
    Info,
    /// Bilgi grafını kurar (--kur) ya da doğal dille sorgular
    Graph {
        /// Sorulacak soru; boş bırakılırsa --kur gerekir
        soru: Option<String>,
        /// Grafı baştan kur (graphify-rs build)
        #[arg(long)]
        kur: bool,
    },
    /// Gömme sütunlarını kurar ve vektörleri (yeniden) üretir.
    Gomme {
        /// Gömme çekirdeği. Derlenmiş çekirdekler cargo feature'ına bağlıdır.
        #[arg(long, value_enum, default_value_t = gomme::Cekirdek::default())]
        kip: gomme::Cekirdek,
        /// İmzası tutan satırları da yeniden üret.
        #[arg(long)]
        zorla: bool,
    },
    /// Altın sorgu setini üç kanala koşar: geri çağrım@k, MRR, ms/sorgu.
    /// ÖLÇER, karar vermez.
    Olcum {
        /// Altın sorgu seti (TOML).
        #[arg(long, default_value = "olcum/altin-sorgular.toml")]
        set: PathBuf,
        #[arg(long, default_value = "5")]
        limit: usize,
        /// Ölçülecek gömme çekirdeği — iki çekirdek aynı sette karşılaştırılır.
        #[arg(long, value_enum, default_value_t = gomme::Cekirdek::default())]
        kip: gomme::Cekirdek,
    },
    /// Stdio MCP sunucusu olarak koşar (JSON-RPC 2.0). stdout protokole aittir.
    Mcp,
    /// Claude Code dökümlerindeki araç çağrılarını günlüğe aktarır.
    /// Kanca gerekmez: dökümler zaten append-only günlük (bkz. kayit.rs).
    KayitAl {
        /// Döküm kökü. Varsayılan: `CLAUDE_CONFIG_DIR` ya da `~/.claude`, altında `projects`.
        #[arg(long)]
        kok: Option<PathBuf>,
    },
    /// Verilen kökler altındaki git depolarını kataloğa yazar (depo başına TEK satır).
    Tara {
        /// Taranacak kökler. Kişisel yol depoya gömülmesin diye varsayılan YOK.
        #[arg(required = true)]
        koklar: Vec<PathBuf>,
        /// Kökten itibaren kaç kat inilir.
        #[arg(long, default_value = "3")]
        derinlik: usize,
        /// Yazma; yalnız ne olacağını göster.
        #[arg(long)]
        kuru: bool,
    },
    /// Tekmil puanı ekler
    Tekmil {
        #[arg(long)]
        ajan: String,
        /// Yetenek ID (metin slug, örn. "zopay-rust-porting")
        #[arg(long)]
        yetenek: String,
        #[arg(long)]
        puan: f64,
        #[arg(long)]
        gerekce: String,
        /// Hafta numarası (YYYYWW)
        #[arg(long, default_value = "202634")]
        hafta: i64,
    },
    /// Paylaşılan kurallar kataloğuna bir kural ekler (ayrı dosya, ana kütüphaneye dokunmaz).
    KuralEkle {
        /// Kural metni (ör. "Bun kullan, Node'a kaçma")
        metin: String,
        /// Nereden geldiği (ör. dosya yolu, proje adı)
        #[arg(long, default_value = "elle")]
        kaynak: String,
        /// Virgülle ayrılmış etiketler
        #[arg(long, default_value = "")]
        etiketler: String,
    },
    /// Kurallarda alt-dize arar (aktif olanlar).
    KuralAra {
        sorgu: String,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Kuralları listeler; istenirse tek etiketle sınırlar.
    KuralListele {
        #[arg(long)]
        etiket: Option<String>,
    },
    /// Bir kuralı pasifleştirir (silmez — geri alınabilir).
    KuralKaldir { id: String },
}

#[derive(Debug, Deserialize, Serialize)]
struct Skill {
    id: String,
    ad: String,
    aciklama: String,
    basari_puani_ort: Option<f64>,
    kategori: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Kural {
    id: String,
    metin: String,
    kaynak: String,
    etiketler: Vec<String>,
}

/// Karakter sınırında kırpar ve **kırptığını söyler**.
///
/// Bayt dilimi (`&s[..n]`) çok baytlı UTF-8'in ortasına düşerse panikler; bu
/// kütüphanenin metinleri Türkçe. Sessiz kırpma da sayıyı gizler — ne kadarının
/// gizlendiği çıktıya yazılır.
fn kisalt(s: &str, azami: usize) -> String {
    let toplam = s.chars().count();
    if toplam <= azami {
        return s.to_string();
    }
    let kesik: String = s.chars().take(azami).collect();
    format!("{kesik}… (+{} karakter)", toplam - azami)
}

/// Kütüphaneyi bul: önce `TURSO_DB_PATH`, sonra çalışma dizini, sonra bilinen yeri.
///
/// Eski hâli çıplak bir göreli addı ve `Builder::new_local` olmayan dosyayı
/// **yaratır**. Kütüphane dizini dışından çalıştırınca sessizce boş bir DB
/// açılıyor, komut "no such table: yetenekler" diye düşüyor ve geride bir
/// çöp dosya kalıyordu — okuyan kişiye kütüphane bozukmuş gibi görünüyor.
/// Bulunamadıysa yaratmak değil, nereye baktığını söyleyip durmak doğrusu.
fn kutuphane_yolu() -> std::result::Result<PathBuf, String> {
    let mut denenen = Vec::new();
    let mut aday = |p: PathBuf| -> Option<PathBuf> {
        if p.is_file() {
            return Some(p);
        }
        denenen.push(p.display().to_string());
        None
    };

    if let Ok(v) = std::env::var("TURSO_DB_PATH") {
        // Açıkça verilmişse tahmin yürütme: ya odur ya hata.
        let p = PathBuf::from(&v);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("TURSO_DB_PATH bir dosyayı göstermiyor: {v}"))
        };
    }
    if let Some(p) = aday(PathBuf::from(DB_ADI)) {
        return Ok(p);
    }
    for kok in ["USERPROFILE", "HOME"] {
        if let Ok(h) = std::env::var(kok) {
            let p = PathBuf::from(h)
                .join("Desktop")
                .join("hermes yazılım")
                .join("kutuphane")
                .join(DB_ADI);
            if let Some(p) = aday(p) {
                return Ok(p);
            }
        }
    }
    Err(format!(
        "kütüphane bulunamadı. Bakılan yerler:\n  {}\n\
         TURSO_DB_PATH ile açıkça gösterebilirsin.",
        denenen.join("\n  ")
    ))
}

async fn baglan() -> Result<Connection> {
    let yol = kutuphane_yolu().map_err(CliError::Girdi)?;
    let db = Builder::new_local(yol.to_string_lossy().as_ref())
        .build()
        .await?;
    Ok(db.connect()?)
}

/// Bir sorgunun satırlarını üç PARALEL diziye okur.
///
/// Ayrı fonksiyon çünkü gövde iki DOSYADAN geliyor (`yetenekler` + depo
/// kataloğu) ve iki kopya okuma döngüsü tutmak, ikisinden birinin sessizce
/// kaymasını beklemek demek. Sütun sırası sabit: id, ad, açıklama, tam metin,
/// puan, kategori, [gomme, gomme_imza, icerik_hash].
async fn satirlari_oku(
    conn: &Connection,
    sql: &str,
    gomme_var: bool,
    kip: gomme::Cekirdek,
) -> Result<(Vec<Belge>, Vec<Skill>, Vec<Option<Vec<f32>>>)> {
    let mut satirlar = conn.query(sql, ()).await?;
    let mut belgeler = Vec::new();
    let mut skill_map = Vec::new();
    // Vektörler PARALEL taşınır. `arama::Belge`'ye alan EKLENMEZ: pasli-beyin
    // (`cekirdek/src/kopru.rs`) onu struct literal ile kuruyor, yeni alan
    // o deponun derlemesini kırar.
    let mut vektorler: Vec<Option<Vec<f32>>> = Vec::new();

    while let Some(r) = satirlar.next().await? {
        let id = r.get::<String>(0)?;
        let ad = r.get::<String>(1)?;
        let aciklama = r.get::<String>(2).unwrap_or_default();
        let tam_metin_md = r.get::<String>(3).unwrap_or_default();
        let basari_puani_ort = r.get::<f64>(4).ok();
        let kategori = r.get::<String>(5).ok();

        belgeler.push(Belge {
            id: id.clone(),
            ad: ad.clone(),
            aciklama: aciklama.clone(),
            tam_metin_md,
        });

        // Vektör yalnız imzası TUTUYORSA kullanılır: `<kip>:<icerik_hash>`
        // hem içerik bayatlığını hem çekirdek değişimini tek testte yakalar.
        vektorler.push(if gomme_var {
            let beklenen = gomme::imza(kip.kip(), &r.get::<String>(8).unwrap_or_default());
            match (r.get::<Vec<u8>>(6).ok(), r.get::<String>(7).ok()) {
                // İmza tutsa bile boyut YANLIŞSA kabul etme: ikinci, gereksiz
                // görünen ama ucuz olan kontrol. Bozuk blob sessizce saçma
                // kosinüs üretmektense hiç girmesin.
                (Some(b), Some(im)) if im == beklenen => {
                    gomme::blob_oku(&b).filter(|v| v.len() == kip.boyut())
                }
                _ => None,
            }
        } else {
            None
        });

        skill_map.push(Skill {
            id,
            ad,
            aciklama,
            basari_puani_ort,
            kategori,
        });
    }
    Ok((belgeler, skill_map, vektorler))
}

/// Aranacak gövde: BM25 indeksi + kayıtlar + PARALEL vektörler.
struct Govde {
    indeks: Indeks,
    kayitlar: Vec<Skill>,
    vektorler: Vec<Option<Vec<f32>>>,
}

/// Gövdeyi BİR KEZ yükler (iki dosya, tek indeks).
///
/// `search`ten ayrı durması `olcum` için şart: 18 sorgu koşarken gövde her
/// sorguda yeniden yüklenirse ölçülen şey arama değil disk olur.
///
/// Gömme sütunu yoksa ya da imzası tutmuyorsa o satır KOSİNÜSE GİRMEZ ama
/// BM25'te kalır — hiçbir kayıt vektörü eksik diye kaybolmaz. Kullanılabilir
/// vektör hiç yoksa saf BM25'e düşer ve bunu stderr'e YAZAR: sessiz düşüş,
/// "arama bozuldu" diye görünen ama sebebi görünmeyen hatanın ta kendisidir.
async fn govde_yukle(conn: &Connection, kip: gomme::Cekirdek) -> Result<Govde> {
    // gomme/gomme_imza göç edilmemiş DB'de yoktur; o yüzden şema önce yoklanır.
    let gomme_var = gomme_sutunu_var(conn).await?;
    let vektor_sutunlari = if gomme_var {
        ", gomme, gomme_imza, icerik_hash"
    } else {
        ""
    };
    let (mut belgeler, mut kayitlar, mut vektorler) = satirlari_oku(
        conn,
        &format!(
            "SELECT id, ad, aciklama, tam_metin_md, basari_puani_ort, kategori{vektor_sutunlari} \
             FROM yetenekler"
        ),
        gomme_var,
        kip,
    )
    .await?;

    // Depo satırları AYRI DOSYADA (nedeni: `depo_baglan` belgesi). İki gövde
    // aynı BM25 indeksine ve aynı füzyona girer; ayrım yalnız nereden okundukları.
    if let Some(depo) = depo_baglan(false).await? {
        let (b, s, v) = satirlari_oku(
            &depo,
            &format!(
                "SELECT id, ad, aciklama, tam_metin_md, NULL, 'depo'{vektor_sutunlari} \
                 FROM depolar"
            ),
            gomme_var,
            kip,
        )
        .await?;
        belgeler.extend(b);
        kayitlar.extend(s);
        vektorler.extend(v);
    }

    if gomme_var && !vektorler.iter().any(|v| v.is_some()) {
        eprintln!(
            "! gömme bayat (0/{} satır kullanılabilir) — yalnız BM25.",
            belgeler.len()
        );
        eprintln!("  Onarım: ibnunnedim gomme --kip {}", kip.kip());
    }
    Ok(Govde {
        indeks: Indeks::kur(belgeler),
        kayitlar,
        vektorler,
    })
}

/// Hangi kanalın koşacağı. `olcum` üçünü de AYNI sıralama kodundan geçirir —
/// kopya bir sıralayıcıyı ölçmek, aramanın kendisini ölçmemek olurdu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kanal {
    Bm25,
    Gomme,
    Karmasik,
}

/// Gömme kanalı: sorgu vektörü ile gövdenin kosinüsü, taban altı elenmiş.
async fn kosinusla(
    g: &Govde,
    sorgu: &str,
    kip: gomme::Cekirdek,
    genis: usize,
) -> Result<Vec<(usize, f64)>> {
    // Kullanılabilir vektör yoksa sorguyu gömmek boşa ağ çağrısıdır — ve
    // sunucu kapalıysa bütün aramayı düşürürdü.
    if g.vektorler.iter().all(|v| v.is_none()) {
        return Ok(Vec::new());
    }
    let q = gomme::gomme(&[sorgu.to_string()], gomme::Rol::Sorgu, kip).await?;
    let taban = kip.taban();
    let mut v: Vec<(usize, f64)> = g
        .vektorler
        .iter()
        .enumerate()
        .filter_map(|(i, ov)| ov.as_ref().map(|x| (i, gomme::kosinus(&q[0], x))))
        .filter(|(_, p)| *p > taban)
        .map(|(i, p)| (i, p as f64))
        .collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(genis);
    Ok(v)
}

/// Tek sorguyu sıralar. `search` de `olcum` da BURADAN geçer.
async fn sirala(
    g: &Govde,
    sorgu: &str,
    limit: usize,
    kip: gomme::Cekirdek,
    kanal: Kanal,
) -> Result<Vec<(usize, f64)>> {
    // Füzyon iki kanalın BİRLEŞİMİNİ alır; iç tarama limitin 4 katı yapılır ki
    // BM25 kesiminden düşen ama kosinüsün öne çıkardığı belge kaybolmasın.
    let genis = limit.saturating_mul(4).max(8);
    // Saf kanalda öbür dizi BOŞ verilir — ağırlığı 0'a çekmek YETMEZ: `harmanla`
    // birleşim döndürür, 0 puanlı yabancı belgeler kuyruğa girer ve limit
    // dolmadığında sahte isabet üretir. Ölçüm tam da orada yalan söylerdi.
    let bm = match kanal {
        Kanal::Gomme => Vec::new(),
        _ => g.indeks.ara(sorgu, genis),
    };
    // Karma aramada gömme sunucusu düşerse BM25 yine sonuç verir; saf gömme
    // kanalında (ölçüm) hata yutulmaz, yoksa sıfır isabet diye ölçülürdü.
    let kos = match kanal {
        Kanal::Bm25 => Vec::new(),
        Kanal::Gomme => kosinusla(g, sorgu, kip, genis).await?,
        Kanal::Karmasik => kosinusla(g, sorgu, kip, genis).await.unwrap_or_else(|h| {
            eprintln!("! gömme kanalı düştü, yalnız BM25: {h}");
            Vec::new()
        }),
    };
    // 0,5/0,5: pasli-beyin'de ölçülmüş ağırlık (kopru.rs Kanal::Karmasik).
    let agirlik = match kanal {
        Kanal::Bm25 => 1.0,
        Kanal::Gomme => 0.0,
        Kanal::Karmasik => 0.5,
    };
    Ok(arama::harmanla(&bm, &kos, agirlik, limit))
}

/// Saf Rust BM25 + gömme kosinüsü, normalize füzyonla birleştirilir.
async fn search_skills(
    conn: &Connection,
    query: &str,
    limit: usize,
    kip: gomme::Cekirdek,
) -> Result<(Vec<Skill>, usize, std::time::Duration)> {
    let baslangic = Instant::now();
    let g = govde_yukle(conn, kip).await?;
    let sonuc = sirala(&g, query, limit, kip, Kanal::Karmasik)
        .await?
        .into_iter()
        .map(|(idx, _puan)| Skill {
            id: g.kayitlar[idx].id.clone(),
            ad: g.kayitlar[idx].ad.clone(),
            aciklama: g.kayitlar[idx].aciklama.clone(),
            basari_puani_ort: g.kayitlar[idx].basari_puani_ort,
            kategori: g.kayitlar[idx].kategori.clone(),
        })
        .collect();
    Ok((sonuc, g.kayitlar.len(), baslangic.elapsed()))
}

async fn list_all_skills(conn: &Connection, kategori: Option<&str>) -> Result<Vec<Skill>> {
    let mut satirlar = match kategori {
        Some(kat) => {
            conn.query(
                "SELECT id, ad, kategori, basari_puani_ort FROM yetenekler \
                 WHERE kategori = ? ORDER BY ad ASC",
                params![kat],
            )
            .await?
        }
        None => {
            conn.query(
                "SELECT id, ad, kategori, basari_puani_ort FROM yetenekler \
                 ORDER BY kategori, ad ASC",
                (),
            )
            .await?
        }
    };
    let mut sonuc = Vec::new();
    while let Some(r) = satirlar.next().await? {
        sonuc.push(Skill {
            id: r.get::<String>(0)?,
            ad: r.get::<String>(1)?,
            kategori: r.get::<String>(2).ok(),
            basari_puani_ort: r.get::<f64>(3).ok(),
            aciklama: String::new(),
        });
    }
    Ok(sonuc)
}

async fn say(conn: &Connection, tablo: &str) -> Result<i64> {
    let mut satirlar = conn
        .query(&format!("SELECT COUNT(*) FROM {tablo}"), ())
        .await?;
    match satirlar.next().await? {
        Some(r) => Ok(r.get::<i64>(0)?),
        None => Err(CliError::Girdi(format!("{tablo}: COUNT satır döndürmedi"))),
    }
}

async fn fts_indeksleri(conn: &Connection) -> Result<Vec<String>> {
    let mut satirlar = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%_fts' ORDER BY name",
            (),
        )
        .await?;
    let mut adlar = Vec::new();
    while let Some(r) = satirlar.next().await? {
        adlar.push(r.get::<String>(0)?);
    }
    Ok(adlar)
}

async fn tekmil_ver(
    conn: &Connection,
    hafta: i64,
    ajan: &str,
    yetenek_id: &str,
    puan: f64,
    gerekce: &str,
) -> Result<()> {
    if !(0.0..=100.0).contains(&puan) {
        return Err(CliError::Girdi(format!(
            "puan 0-100 aralığında olmalı: {puan}"
        )));
    }
    conn.execute(
        "INSERT INTO ajan_tekmilleri (hafta_no, ajan_adi, yetenek_id, verilen_puan, degerlendirme_gerekcesi) \
         VALUES (?, ?, ?, ?, ?)",
        params![hafta, ajan, yetenek_id, puan, gerekce],
    )
    .await?;

    let mut satirlar = conn
        .query(
            "SELECT AVG(verilen_puan), COUNT(*) FROM ajan_tekmilleri WHERE yetenek_id = ?",
            params![yetenek_id],
        )
        .await?;
    let (ort, adet) = match satirlar.next().await? {
        Some(r) => (r.get::<f64>(0)?, r.get::<i64>(1)?),
        None => return Err(CliError::Girdi("ortalama hesaplanamadı".into())),
    };

    conn.execute(
        "UPDATE yetenekler SET basari_puani_ort = ?, puanlayan_ajan_sayisi = ? WHERE id = ?",
        params![ort, adet, yetenek_id],
    )
    .await?;
    Ok(())
}

/// `graphify-rs` alt süreç argümanlarını kurar.
///
/// NEDEN ayrı fonksiyon: dış süreç çağrısının kendisi sınanamaz, argüman
/// kurulumu sınanabilir. Yanlış bayrak sessizce YANLIŞ grafı sorgular —
/// kırılması gereken yer burası.
fn graf_argumanlari(soru: Option<&str>, kur: bool, cikti: &str) -> Result<Vec<String>> {
    if kur {
        return Ok(vec!["build".into(), "--path".into(), ".".into()]);
    }
    match soru {
        Some(s) if !s.trim().is_empty() => Ok(vec![
            "query".into(),
            s.to_string(),
            "--graph".into(),
            format!("{cikti}/graph.json"),
        ]),
        _ => Err(CliError::Girdi(
            "graph: ya bir soru ver ya da --kur kullan".into(),
        )),
    }
}

/// Grafı kurar ya da sorgular. Binary PATH'tedir (`cargo install`).
fn graf_calistir(soru: Option<&str>, kur: bool) -> Result<()> {
    let cikti = std::env::var("GRAPHIFY_OUT").unwrap_or_else(|_| "graphify-rs-out".to_string());
    let argumanlar = graf_argumanlari(soru, kur, &cikti)?;
    let durum = std::process::Command::new("graphify-rs")
        .args(&argumanlar)
        .status()
        .map_err(|e| {
            CliError::Girdi(format!(
                "graphify-rs çalıştırılamadı ({e}). Kurulum:                  cargo install --path <graphify-rs klasörü>"
            ))
        })?;
    if !durum.success() {
        return Err(CliError::Girdi(format!("graphify-rs başarısız: {durum}")));
    }
    Ok(())
}

/// `gomme` sütunu var mı? Göç edilmemiş DB'de SELECT'i patlatmamak için.
async fn gomme_sutunu_var(conn: &Connection) -> Result<bool> {
    let mut satirlar = conn
        .query(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='yetenekler'",
            (),
        )
        .await?;
    Ok(match satirlar.next().await? {
        Some(r) => r.get::<String>(0).unwrap_or_default().contains("gomme"),
        None => false,
    })
}

/// Gömme sütunlarını ekler. Şemayı okuyup karar verir — iki kez koşmak zararsız.
///
/// DB'de `yetenekler_fts` / `kod_hazinesi_fts` fts5 SANAL tabloları var ve Turso
/// fts5 uygulamıyor (kendi Tantivy FTS'i ayrı bir şey). Bu yüzden `ALTER TABLE`ın
/// bu dosyada çalışması ÖNCEDEN SINANMALI; çalışmazsa B planı sütunları DB'nin
/// sahibi olan Python tarafının eklemesi, Rust'ın yalnız değer yazmasıdır.
///
/// `search` bunu ASLA çağırmaz: yazma işleri açık komutlarda kalır.
async fn gomme_gocu(conn: &Connection) -> Result<bool> {
    let mut satirlar = conn
        .query(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='yetenekler'",
            (),
        )
        .await?;
    let sema = match satirlar.next().await? {
        Some(r) => r.get::<String>(0).unwrap_or_default(),
        None => return Err(CliError::Girdi("yetenekler tablosu yok".into())),
    };
    if sema.contains("gomme") {
        return Ok(false);
    }
    conn.execute("ALTER TABLE yetenekler ADD COLUMN gomme BLOB", ())
        .await?;
    conn.execute("ALTER TABLE yetenekler ADD COLUMN gomme_imza TEXT", ())
        .await?;
    Ok(true)
}

/// Vektörleri üretir. İmzası tutan satırlar atlanır (`--zorla` hepsini yeniler).
///
/// `tablo` ile çağrılır çünkü gövde iki tabloya yayılmış durumda
/// (`yetenekler` + `depolar`); ikisinin de bu altı sütunu var.
async fn gomme_uret(
    conn: &Connection,
    tablo: &str,
    k: gomme::Cekirdek,
    zorla: bool,
) -> Result<(usize, usize, std::time::Duration)> {
    let baslangic = Instant::now();
    let mut satirlar = conn
        .query(
            &format!("SELECT id, ad, aciklama, tam_metin_md, icerik_hash, gomme_imza FROM {tablo}"),
            (),
        )
        .await?;

    let mut isler: Vec<(String, String, String)> = Vec::new(); // (id, metin, imza)
    let mut toplam = 0usize;
    while let Some(r) = satirlar.next().await? {
        toplam += 1;
        let id = r.get::<String>(0)?;
        let ad = r.get::<String>(1).unwrap_or_default();
        let aciklama = r.get::<String>(2).unwrap_or_default();
        let tam = r.get::<String>(3).unwrap_or_default();
        let icerik_hash = r.get::<String>(4).unwrap_or_default();
        let mevcut = r.get::<String>(5).ok();

        let yeni_imza = gomme::imza(k.kip(), &icerik_hash);
        if !zorla && mevcut.as_deref() == Some(yeni_imza.as_str()) {
            continue;
        }
        isler.push((id, gomme::belge_metni(&ad, &aciklama, &tam), yeni_imza));
    }

    if isler.is_empty() {
        return Ok((0, toplam, baslangic.elapsed()));
    }
    // Toplu gömme: ONNX çekirdeği modeli bir kez yükleyebilsin, Ollama tek istekte gönderebilsin.
    let metinler: Vec<String> = isler.iter().map(|(_, m, _)| m.clone()).collect();
    let vektorler = gomme::gomme(&metinler, gomme::Rol::Belge, k).await?;

    for ((id, _, imza), v) in isler.iter().zip(vektorler.iter()) {
        conn.execute(
            &format!("UPDATE {tablo} SET gomme = ?, gomme_imza = ? WHERE id = ?"),
            params![gomme::blob_yaz(v), imza.as_str(), id.as_str()],
        )
        .await?;
    }
    Ok((isler.len(), toplam, baslangic.elapsed()))
}

/// Bir tablo var mı? Göç edilmemiş DB'de SELECT'i patlatmamak için.
///
/// Ad SQL'e gömülü, bağlı DEĞİL: Turso 0.7.2'de `sqlite_master` üzerinde
/// parametre bağlaması eşleşme döndürmüyor (var olan tabloya "yok" dedi).
/// Çağrı yerlerinin hepsi sabit — dışarıdan ad gelmiyor.
async fn tablo_var(conn: &Connection, ad: &str) -> Result<bool> {
    let mut satirlar = conn
        .query(
            &format!("SELECT 1 FROM sqlite_master WHERE type='table' AND name='{ad}'"),
            (),
        )
        .await?;
    Ok(satirlar.next().await?.is_some())
}

/// Depo kataloğu tablosu. `IF NOT EXISTS` YOK: Turso 0.7.2 onu yok sayıp yine
/// de "table already exists" ile düşüyor; varlık `tablo_var` ile önce sorulur.
const DEPO_TABLOSU: &str = "CREATE TABLE depolar (\
     id TEXT PRIMARY KEY, ad TEXT NOT NULL, aciklama TEXT NOT NULL, \
     tam_metin_md TEXT NOT NULL, icerik_hash TEXT NOT NULL, \
     gomme BLOB, gomme_imza TEXT, \
     guncelleme_tarihi DATETIME DEFAULT CURRENT_TIMESTAMP)";

/// Depo kataloğunu açar — ana kütüphanenin YANINDA, AYRI DOSYA.
///
/// NEDEN AYRI DOSYA (2026-09-10, gerçek DB'nin kopyaları üzerinde ölçüldü):
/// ana kütüphanede fts5 SANAL TABLOLARI var (`yetenekler_fts` 145 satır,
/// `kod_hazinesi_fts` 11) ve Turso 0.7.2 fts5'i uygulamıyor. İki sonucu var,
/// ikisi de sessiz:
///
/// 1. **İndeksler sürdürülmüyor.** Aynı tablo iki DB'de kuruldu, tek fark
///    fts5'in varlığıydı: fts5 yok → `integrity_check` = ok, `COUNT(*)` = 21;
///    fts5 var → `wrong # of entries in index sqlite_autoindex_hedef_1`,
///    `COUNT(*)` = 1 ama tam tarama 21 — indeks taraması yeni satırları
///    GÖRMÜYOR. `REINDEX` de onarmıyor.
/// 2. **Şema fts5'te kesiliyor.** Aynı dosyada kurulan `depolar` tablosu
///    `sqlite_master`ın SONUNA (fts5 girdilerinden sonra) düştü; Turso onu
///    bir sonraki bağlantıda "no such table" diye reddetti, oysa satır
///    oradaydı ve SQLite 85 satırı okuyordu.
///
/// Bu fts5 tabloları ölü değil — Python tarafı (`ibnunnedim_cli.py`,
/// `ilkleme.py`) onları kullanıyor, silinemezler. O yüzden depo satırları
/// hiç oraya girmiyor: kendi dosyasında Turso indeksi de şemayı da düzgün
/// yönetiyor ve `yetenekler` ile fts5 hiç dokunulmamış kalıyor.
///
/// `yarat` false ise dosya yoksa `None` döner — arama yanında çöp dosya
/// bırakmasın (`kutuphane_yolu`nun aynı gerekçesi).
async fn depo_baglan(yarat: bool) -> Result<Option<Connection>> {
    yan_baglan(DEPO_DB_ADI, yarat).await
}

/// Ana kütüphanenin YANINDAKİ bir el-Fihrist dosyasını açar (depo kataloğu,
/// araç çağrısı günlüğü). Hepsi aynı gerekçeyle ayrı: bkz. `depo_baglan`.
async fn yan_baglan(ad: &str, yarat: bool) -> Result<Option<Connection>> {
    let yol = kutuphane_yolu()
        .map_err(CliError::Girdi)?
        .with_file_name(ad);
    if !yol.is_file() && !yarat {
        return Ok(None);
    }
    let db = Builder::new_local(yol.to_string_lossy().as_ref())
        .build()
        .await?;
    Ok(Some(db.connect()?))
}

/// Paylaşılan kurallar dosyasını açar. Aynı gerekçeyle ayrı: bkz. `depo_baglan`.
async fn kural_baglan(yarat: bool) -> Result<Option<Connection>> {
    yan_baglan(KURAL_DB_ADI, yarat).await
}

/// `IF NOT EXISTS` yok (Turso 0.7.2 güvenilmez, bkz. `tablo_var`) — varlık önce sorulur.
const KURAL_TABLOSU: &str = "CREATE TABLE kurallar (\
     id TEXT PRIMARY KEY, metin TEXT NOT NULL, kaynak TEXT NOT NULL, \
     etiketler TEXT NOT NULL, aktif INTEGER NOT NULL DEFAULT 1, \
     olusturma_tarihi DATETIME DEFAULT CURRENT_TIMESTAMP)";

async fn kural_tablosunu_garantile(conn: &Connection) -> Result<()> {
    if !tablo_var(conn, "kurallar").await? {
        conn.execute(KURAL_TABLOSU, ()).await?;
    }
    Ok(())
}

/// Aynı metin iki kez eklenirse bile ayrı id üretir (zaman damgası karışıma girer);
/// çakışma riski bu ölçekte (onlarca-yüzlerce kural) göz ardı edilebilir.
fn kural_id_uret(metin: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    metin.hash(&mut h);
    std::time::SystemTime::now().hash(&mut h);
    format!("{:016x}", h.finish())
}

fn kural_satirdan(row: &turso::Row) -> Result<Kural> {
    let etiketler_raw: String = row.get(3)?;
    Ok(Kural {
        id: row.get(0)?,
        metin: row.get(1)?,
        kaynak: row.get(2)?,
        etiketler: etiketler_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
    })
}

async fn kural_ekle(conn: &Connection, metin: &str, kaynak: &str, etiketler: &str) -> Result<String> {
    if metin.trim().is_empty() {
        return Err(CliError::Girdi("kural metni boş olamaz".into()));
    }
    kural_tablosunu_garantile(conn).await?;
    let id = kural_id_uret(metin);
    conn.execute(
        "INSERT INTO kurallar (id, metin, kaynak, etiketler) VALUES (?, ?, ?, ?)",
        params![id.as_str(), metin, kaynak, etiketler],
    )
    .await?;
    Ok(id)
}

async fn kural_listele(conn: &Connection, etiket: Option<&str>) -> Result<Vec<Kural>> {
    kural_tablosunu_garantile(conn).await?;
    let mut satirlar = match etiket {
        Some(e) => {
            conn.query(
                "SELECT id, metin, kaynak, etiketler FROM kurallar \
                 WHERE aktif = 1 AND etiketler LIKE ? ORDER BY olusturma_tarihi",
                params![format!("%{e}%")],
            )
            .await?
        }
        None => {
            conn.query(
                "SELECT id, metin, kaynak, etiketler FROM kurallar \
                 WHERE aktif = 1 ORDER BY olusturma_tarihi",
                (),
            )
            .await?
        }
    };
    let mut kurallar = Vec::new();
    while let Some(row) = satirlar.next().await? {
        kurallar.push(kural_satirdan(&row)?);
    }
    Ok(kurallar)
}

async fn kural_ara(conn: &Connection, sorgu: &str, limit: usize) -> Result<Vec<Kural>> {
    if sorgu.trim().is_empty() {
        return Err(CliError::Girdi("arama sorgusu boş olamaz".into()));
    }
    kural_tablosunu_garantile(conn).await?;
    let desen = format!("%{sorgu}%");
    let mut satirlar = conn
        .query(
            "SELECT id, metin, kaynak, etiketler FROM kurallar \
             WHERE aktif = 1 AND (metin LIKE ? OR kaynak LIKE ? OR etiketler LIKE ?) \
             ORDER BY olusturma_tarihi LIMIT ?",
            params![desen.clone(), desen.clone(), desen, limit as i64],
        )
        .await?;
    let mut kurallar = Vec::new();
    while let Some(row) = satirlar.next().await? {
        kurallar.push(kural_satirdan(&row)?);
    }
    Ok(kurallar)
}

/// Silmez, pasifleştirir — geri alınabilir (bkz. AÇIK EYLEM ilkesi).
async fn kural_kaldir(conn: &Connection, id: &str) -> Result<()> {
    kural_tablosunu_garantile(conn).await?;
    conn.execute("UPDATE kurallar SET aktif = 0 WHERE id = ?", params![id])
        .await?;
    Ok(())
}

/// Dökümlerdeki araç çağrılarını günlüğe aktarır; zaten alınmışı atlar.
///
/// Tekillik `tool_use.id` ile — çağrı kimliği oturumlar arası tekil. Var olan
/// kimlikler bir kez okunup kümeye alınır: Turso'nun çakışma çözümüne
/// (`INSERT OR IGNORE`) yaslanmak yerine sayım da buradan çıkıyor.
/// ponytail: her koşumda bütün dökümler baştan okunur (bugün 612 MB, 163
/// dosya). Yavaşlarsa yükseltme yolu, dosya başına alınan bayt ofsetini
/// saklayıp yalnız sonrasını okumak — dökümler append-only.
async fn kayitlari_al(kok: &std::path::Path) -> Result<(usize, usize, usize)> {
    let conn = yan_baglan(KAYIT_DB_ADI, true)
        .await?
        .expect("yarat=true iken bağlantı hep döner");
    if !tablo_var(&conn, "arac_cagrilari").await? {
        conn.execute(
            "CREATE TABLE arac_cagrilari (kimlik TEXT PRIMARY KEY, oturum TEXT NOT NULL, \
             proje TEXT NOT NULL, arac TEXT NOT NULL, ozet TEXT NOT NULL, \
             hata INTEGER NOT NULL, baslangic TEXT NOT NULL, sure_ms INTEGER)",
            (),
        )
        .await?;
    }
    let mut alinmis = std::collections::HashSet::new();
    let mut oku = conn.query("SELECT kimlik FROM arac_cagrilari", ()).await?;
    while let Some(r) = oku.next().await? {
        alinmis.insert(r.get::<String>(0)?);
    }

    let dosyalar = kayit::dokumleri_bul(kok);
    let mut yeni = 0usize;
    // Tek işlem: binlerce INSERT'i tek tek işlemek her birine ayrı fsync demek.
    conn.execute("BEGIN", ()).await?;
    for dosya in &dosyalar {
        let Ok(icerik) = std::fs::read_to_string(dosya) else {
            continue; // o an yazılan ya da kilitli döküm: bir sonraki koşumda
        };
        let oturum = dosya
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        // `projects/<proje>/...` — kökten sonraki ilk bileşen.
        let proje = dosya
            .strip_prefix(kok)
            .ok()
            .and_then(|g| g.components().next())
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default();
        for c in kayit::dokumden_cagrilar(&icerik, &oturum, &proje) {
            if !alinmis.insert(c.kimlik.clone()) {
                continue;
            }
            conn.execute(
                "INSERT INTO arac_cagrilari \
                 (kimlik, oturum, proje, arac, ozet, hata, baslangic, sure_ms) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    c.kimlik.as_str(),
                    c.oturum.as_str(),
                    c.proje.as_str(),
                    c.arac.as_str(),
                    c.ozet.as_str(),
                    c.hata as i64,
                    c.baslangic.as_str(),
                    c.sure_ms
                ],
            )
            .await?;
            yeni += 1;
        }
    }
    conn.execute("COMMIT", ()).await?;
    Ok((dosyalar.len(), yeni, alinmis.len()))
}

/// Depo satırlarını yazar: değişmeyeni atlar, değişeni günceller, yenisini ekler.
///
/// Kimlik `id` üzerinden ve tablo TEK SEFERDE okunup eşlenerek kurulur —
/// indeks olmadığı için satır başına `WHERE id = ?` tam tarama demek olurdu.
///
/// `icerik_hash` değişince `gomme_imza` kendiliğinden bayatlar (o sütuna
/// dokunulmaz) — bir sonraki `ibnunnedim gomme` vektörü yeniler.
async fn depolari_yaz(
    conn: &Connection,
    satirlar: &[tara::DepoSatiri],
    kuru: bool,
) -> Result<(usize, usize, usize)> {
    let mut mevcut: std::collections::HashMap<String, String> = Default::default();
    if tablo_var(conn, "depolar").await? {
        let mut oku = conn
            .query("SELECT id, icerik_hash FROM depolar", ())
            .await?;
        while let Some(r) = oku.next().await? {
            mevcut.insert(r.get::<String>(0)?, r.get::<String>(1).unwrap_or_default());
        }
    } else if !kuru {
        conn.execute(DEPO_TABLOSU, ()).await?;
    }

    let (mut eklenen, mut guncellenen, mut ayni) = (0, 0, 0);
    for s in satirlar {
        match mevcut.get(&s.id) {
            Some(h) if *h == s.icerik_hash => {
                ayni += 1;
                continue;
            }
            Some(_) => {
                guncellenen += 1;
                if !kuru {
                    conn.execute(
                        "UPDATE depolar SET ad = ?, aciklama = ?, tam_metin_md = ?, \
                         icerik_hash = ?, guncelleme_tarihi = CURRENT_TIMESTAMP WHERE id = ?",
                        params![
                            s.ad.as_str(),
                            s.aciklama.as_str(),
                            s.tam_metin_md.as_str(),
                            s.icerik_hash.as_str(),
                            s.id.as_str()
                        ],
                    )
                    .await?;
                }
            }
            None => {
                eklenen += 1;
                if !kuru {
                    conn.execute(
                        "INSERT INTO depolar (id, ad, aciklama, tam_metin_md, icerik_hash) \
                         VALUES (?, ?, ?, ?, ?)",
                        params![
                            s.id.as_str(),
                            s.ad.as_str(),
                            s.aciklama.as_str(),
                            s.tam_metin_md.as_str(),
                            s.icerik_hash.as_str()
                        ],
                    )
                    .await?;
                }
            }
        }
    }
    Ok((eklenen, guncellenen, ayni))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // ponytail: graf komutu Turso'ya dokunmaz — DB bağlantısından ÖNCE ele
    // alınır ki kütüphane dosyası olmasa da graf kurulup sorgulanabilsin.
    if let Command::Graph { soru, kur } = &args.command {
        return graf_calistir(soru.as_deref(), *kur);
    }

    // kayit-al ana kütüphaneyi AÇMAZ, yalnız yanındaki dizini bulur. Açmaya
    // kalksaydı düşerdi: Turso dosyayı süreç ömrünce tekel kilitliyor ve açık
    // bir Claude oturumunun `ibnunnedim mcp` sunucusu kilidi hep tutuyor
    // (ölçüldü 2026-09-18: "os error 33", kilit sahibi `ibnunnedim.exe mcp`).
    if let Command::KayitAl { kok } = &args.command {
        let kok = kok
            .clone()
            .or_else(kayit::varsayilan_kok)
            .ok_or_else(|| CliError::Girdi("döküm kökü bulunamadı; --kok ile ver".into()))?;
        let basla = Instant::now();
        let (dosya, yeni, toplam) = kayitlari_al(&kok).await?;
        println!(
            "KAYIT {dosya} döküm tarandı · {yeni} yeni çağrı · günlükte {toplam} · {:.1} sn\n  kök: {}",
            basla.elapsed().as_secs_f64(),
            kok.display()
        );
        return Ok(());
    }

    // MCP sunucusu kütüphaneyi süreç ömrünce tutamaz; her çağrıda açıp bırakır.
    if let Command::Mcp = args.command {
        return mcp::calistir().await;
    }

    // Kural komutları ayrı dosyada çalışır, ana kütüphaneyi hiç açmaz — aynı
    // kilit gerekçesiyle (bkz. KayitAl yukarısı, `depo_baglan` belgesi).
    match &args.command {
        Command::KuralEkle {
            metin,
            kaynak,
            etiketler,
        } => {
            let kconn = kural_baglan(true)
                .await?
                .expect("yarat=true iken bağlantı hep döner");
            let id = kural_ekle(&kconn, metin, kaynak, etiketler).await?;
            println!("KURAL EKLENDİ: [{id}] {metin}");
            return Ok(());
        }
        Command::KuralAra { sorgu, limit } => {
            let kconn = match kural_baglan(false).await? {
                Some(c) => c,
                None => {
                    println!("Kural kataloğu boş (henüz hiç kural eklenmemiş).");
                    return Ok(());
                }
            };
            let bulunan = kural_ara(&kconn, sorgu, *limit).await?;
            println!("KURAL ARAMASI ('{sorgu}') — {} sonuç", bulunan.len());
            for k in bulunan {
                println!(
                    "  [{}] {} (kaynak: {}, etiketler: {})",
                    k.id,
                    k.metin,
                    k.kaynak,
                    k.etiketler.join(", ")
                );
            }
            return Ok(());
        }
        Command::KuralListele { etiket } => {
            let kconn = match kural_baglan(false).await? {
                Some(c) => c,
                None => {
                    println!("Kural kataloğu boş (henüz hiç kural eklenmemiş).");
                    return Ok(());
                }
            };
            let liste = kural_listele(&kconn, etiket.as_deref()).await?;
            println!("KURAL DÖKÜMÜ — {} kayıt", liste.len());
            for k in liste {
                println!(
                    "  [{}] {} (kaynak: {}, etiketler: {})",
                    k.id,
                    k.metin,
                    k.kaynak,
                    k.etiketler.join(", ")
                );
            }
            return Ok(());
        }
        Command::KuralKaldir { id } => {
            let kconn = kural_baglan(true)
                .await?
                .expect("yarat=true iken bağlantı hep döner");
            kural_kaldir(&kconn, id).await?;
            println!("KURAL PASİFLEŞTİRİLDİ: [{id}]");
            return Ok(());
        }
        _ => {}
    }

    let conn = baglan().await?;

    match args.command {
        Command::Search { query, limit, kip } => {
            let (skills, toplam_kayit, sure) = search_skills(&conn, &query, limit, kip).await?;
            println!("\nARAMA SONUÇLARI ('{query}') — {} kayıt", skills.len());
            #[cfg(debug_assertions)]
            let dbg = " · DEBUG derlemesi, release ~12 kat hızlı";
            #[cfg(not(debug_assertions))]
            let dbg = "";
            println!(
                "(BM25 + gömme[{}] · {toplam_kayit} kayıt tarandı · {:.1} ms{dbg})",
                kip.kip(),
                sure.as_secs_f64() * 1000.0
            );
            println!("{}", "-".repeat(70));
            for s in skills {
                println!(
                    "[{}] {} | Kategori: {} | Başarı: {}",
                    s.id,
                    s.ad,
                    s.kategori.as_deref().unwrap_or("-"),
                    s.basari_puani_ort
                        .map(|p| format!("{p:.1}"))
                        .unwrap_or_else(|| "-".into())
                );
                println!("   {}", kisalt(&s.aciklama, 140));
            }
        }
        Command::List { kategori } => {
            let skills = list_all_skills(&conn, kategori.as_deref()).await?;
            println!("\nYETENEK DÖKÜMÜ — {} kayıt", skills.len());
            println!("{}", "=".repeat(70));
            let mut simdiki: Option<String> = None;
            for s in skills {
                let kat = s.kategori.unwrap_or_else(|| "GENEL".to_string());
                if simdiki.as_ref() != Some(&kat) {
                    println!("\nKATEGORİ: {kat}\n{}", "-".repeat(50));
                    simdiki = Some(kat);
                }
                println!(
                    "  [{}] {} (Başarı: {})",
                    s.id,
                    s.ad,
                    s.basari_puani_ort
                        .map(|p| format!("{p:.1}/100"))
                        .unwrap_or_else(|| "puanlanmamış".into())
                );
            }
        }
        Command::Info => {
            let yetenek = say(&conn, "yetenekler").await?;
            let tekmil = say(&conn, "ajan_tekmilleri").await?;
            let kod = say(&conn, "kod_hazinesi").await?;
            let fts = fts_indeksleri(&conn).await?;
            println!("\nKÜTÜPHANE İSTATİSTİĞİ\n{}", "=".repeat(65));
            println!(
                "  Veritabanı  : {}",
                // Cozulen yol, ham env degeri degil: "hangi dosyaya baktin"
                // sorusunun cevabi bu satirsa, tahmini degil gercegi yazmali.
                kutuphane_yolu()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|e| e)
            );
            println!("  Yetenek     : {yetenek}");
            let depo = match depo_baglan(false).await? {
                Some(d) => say(&d, "depolar").await?.to_string(),
                None => "YOK (ibnunnedim tara <kök>)".to_string(),
            };
            println!("  Depo        : {depo}");
            // Günlük tablosu yoksa (hiç `kayit-al` koşmadıysa) sayılamaz.
            let cagri = match yan_baglan(KAYIT_DB_ADI, false).await? {
                Some(k) if tablo_var(&k, "arac_cagrilari").await? => {
                    let mut r = k
                        .query(
                            "SELECT COUNT(*), SUM(hata), MAX(baslangic) FROM arac_cagrilari",
                            (),
                        )
                        .await?;
                    match r.next().await? {
                        Some(s) => format!(
                            "{} ({} hata · son {})",
                            s.get::<i64>(0).unwrap_or(0),
                            s.get::<i64>(1).unwrap_or(0),
                            s.get::<String>(2).unwrap_or_else(|_| "—".into())
                        ),
                        None => "0".into(),
                    }
                }
                _ => "YOK (ibnunnedim kayit-al)".to_string(),
            };
            println!("  Araç çağrısı: {cagri}");
            println!("  Tekmil      : {tekmil}");
            println!("  Kod hazinesi: {kod}");
            println!(
                "  FTS5 indeks : {} (tanımlı; arama saf Rust BM25 ile yapılıyor)",
                if fts.is_empty() {
                    "YOK".to_string()
                } else {
                    fts.join(", ")
                }
            );
            println!("{}", "=".repeat(65));
        }
        // Yukarıda ele alındı; buraya düşmez.
        Command::Graph { .. } => unreachable!("graf komutu DB bağlantısından önce ele alınır"),
        Command::Gomme { kip, zorla } => {
            if gomme_gocu(&conn).await? {
                println!("göç: gomme + gomme_imza sütunları eklendi.");
            }
            // Gövde iki dosyaya yayılı; ikisi de vektörlenmeli, yoksa depo
            // satırları BM25'te kalıp kosinüse hiç girmez.
            let depo = depo_baglan(false).await?;
            let hedefler: Vec<(&Connection, &str)> = std::iter::once((&conn, "yetenekler"))
                .chain(depo.as_ref().map(|d| (d, "depolar")))
                .collect();
            for (c, t) in hedefler {
                let (yenilenen, toplam, sure) = gomme_uret(c, t, kip, zorla).await?;
                println!(
                    "GÖMME [{}] {t}: {yenilenen}/{toplam} satır yenilendi · {:.1} ms",
                    kip.kip(),
                    sure.as_secs_f64() * 1000.0
                );
            }
        }
        Command::Olcum { set, limit, kip } => {
            let metin = std::fs::read_to_string(&set).map_err(|h| {
                CliError::Girdi(format!("altın set okunamadı ({}): {h}", set.display()))
            })?;
            let sorgular = olcum::ayristir(&metin);
            if sorgular.is_empty() {
                return Err(CliError::Girdi(format!(
                    "sette sorgu yok: {}",
                    set.display()
                )));
            }
            // Gövde BİR KEZ yüklenir: ölçülen sorgu süresi olsun, disk değil.
            let mut g = govde_yukle(&conn, kip).await?;

            // Belge vektörleri BELLEKTE üretilir, DB'dekiler kullanılmaz.
            // Neden: tek bir `gomme` sütunu var, onu son koşan çekirdek tutar.
            // DB'dekine güvenseydik `--kip e5s384`/`--kip ollama-bge-m3` ölçmek
            // için hash256'nın vektörlerini EZMEK gerekirdi — iki çekirdeği
            // karşılaştırmanın yolu, karşılaştırılanlardan birini yok etmek
            // olamaz. Yan fayda: bayat satır ölçümü kirletemez.
            let uretim = Instant::now();
            let metinler: Vec<String> = g
                .indeks
                .belgeler
                .iter()
                .map(|b| gomme::belge_metni(&b.ad, &b.aciklama, &b.tam_metin_md))
                .collect();
            g.vektorler = gomme::gomme(&metinler, gomme::Rol::Belge, kip)
                .await?
                .into_iter()
                .map(Some)
                .collect();
            let uretim_ms = uretim.elapsed().as_secs_f64() * 1000.0;
            // Derleme kipi yazılmazsa ms sayıları yanıltır: debug ~12 kat yavaş.
            #[cfg(debug_assertions)]
            let dbg = " · DEBUG derlemesi, süreler release'te ~12 kat düşer";
            #[cfg(not(debug_assertions))]
            let dbg = " · release";
            println!(
                "\naltın set: {} sorgu · limit {limit} · {} kayıt · çekirdek {} \
                 (belge vektörleri bellekte, {uretim_ms:.0} ms){dbg}\n",
                sorgular.len(),
                g.kayitlar.len(),
                kip.kip()
            );
            // Rapor sırası pasli-beyin ile aynı: gömme → bm25 → karmasik.
            for (ad, kanal) in [
                ("gomme", Kanal::Gomme),
                ("bm25", Kanal::Bm25),
                ("karmasik", Kanal::Karmasik),
            ] {
                let (mut isabet_toplam, mut ks_toplam, mut sure_toplam) = (0usize, 0f64, 0f64);
                println!("== kanal: {ad}");
                for s in &sorgular {
                    let basla = Instant::now();
                    let sirali = sirala(&g, &s.metin, limit, kip, kanal).await?;
                    sure_toplam += basla.elapsed().as_secs_f64() * 1000.0;
                    let kimlikler: Vec<String> = sirali
                        .iter()
                        .map(|(i, _)| g.kayitlar[*i].id.clone())
                        .collect();
                    let (isabet, ks) = olcum::puanla(&kimlikler, &s.beklenen);
                    isabet_toplam += isabet;
                    ks_toplam += ks;
                    println!(
                        "  {} '{}' → {} · {} sonuç",
                        if isabet == 1 { "✓" } else { "✗" },
                        s.metin,
                        if isabet == 1 {
                            format!("{}. sırada", (1.0 / ks).round() as usize)
                        } else {
                            format!("ilk {limit} içinde YOK ({})", s.beklenen.join(" | "))
                        },
                        kimlikler.len()
                    );
                }
                let n = sorgular.len() as f64;
                println!(
                    // ms iki hane: BM25 tek başına 0,05 ms'nin altında kalıyor,
                    // tek hanede "0.0" görünüp ölçülmemiş sanılıyordu.
                    "  → {ad}: geri çağrım@{limit} {isabet_toplam}/{} = {:.1}% · MRR {:.2} · ort. {:.2} ms/sorgu\n",
                    sorgular.len(),
                    isabet_toplam as f64 / n * 100.0,
                    ks_toplam / n,
                    sure_toplam / n
                );
            }
            println!(
                "(Kanallar AYNI sette karşılaştırılır ve aynı `sirala`dan geçer. `beklenen`\n\
                 ALTERNATİFTİR — pasli-beyin her varyantı ayrı sayar, paydalar birebir\n\
                 karşılaştırılamaz. Gömme karmasik'i geçmiyorsa varsayılan OLMAZ.)"
            );
        }
        Command::Mcp => unreachable!("mcp DB bağlantısından önce ele alınır"),
        Command::KayitAl { .. } => {
            unreachable!("kayit-al DB bağlantısından önce ele alınır")
        }
        Command::Tara {
            koklar,
            derinlik,
            kuru,
        } => {
            let baslangic = Instant::now();
            let mut satirlar: Vec<tara::DepoSatiri> = Vec::new();
            let mut gorulen: std::collections::HashMap<String, PathBuf> = Default::default();
            for kok in &koklar {
                if !kok.is_dir() {
                    return Err(CliError::Girdi(format!(
                        "kök dizin değil: {}",
                        kok.display()
                    )));
                }
                for yol in tara::depolari_bul(kok, derinlik) {
                    let Some(s) = tara::dizinden_satir(&yol) else {
                        continue;
                    };
                    // Aynı ada sahip iki depo (örn. iki "turso" klonu) tek id'ye
                    // düşer; ikincisi birincisini SESSİZCE ezerdi. Söyle, geç.
                    if let Some(onceki) = gorulen.insert(s.id.clone(), yol.clone()) {
                        eprintln!(
                            "! ad çakışması '{}': {} yerine {} kullanılıyor",
                            s.ad,
                            onceki.display(),
                            yol.display()
                        );
                        satirlar.retain(|v| v.id != s.id);
                    }
                    satirlar.push(s);
                }
            }
            // `kuru` iken dosya YOKSA yaratma: kuru koşum diske dokunmamalı.
            let depo = depo_baglan(!kuru).await?;
            let (eklenen, guncellenen, ayni) = match &depo {
                Some(d) => depolari_yaz(d, &satirlar, kuru).await?,
                None => (satirlar.len(), 0, 0),
            };
            if kuru {
                for s in &satirlar {
                    println!("{}  —  {}", s.id, kisalt(&s.aciklama, 90));
                }
            }
            println!(
                "TARAMA{} {} depo · {eklenen} eklendi · {guncellenen} güncellendi · {ayni} değişmedi · {:.0} ms",
                if kuru { " [KURU]" } else { "" },
                satirlar.len(),
                baslangic.elapsed().as_secs_f64() * 1000.0
            );
            if !kuru && eklenen + guncellenen > 0 {
                println!("Vektörler bayat — çalıştır: ibnunnedim gomme");
            }
        }
        Command::Tekmil {
            ajan,
            yetenek,
            puan,
            gerekce,
            hafta,
        } => {
            tekmil_ver(&conn, hafta, &ajan, &yetenek, puan, &gerekce).await?;
            println!("TEKMİL KAYDEDİLDİ: {ajan} → yetenek {yetenek} (puan {puan}, hafta {hafta})");
        }
        // Kural komutları ana kütüphaneyi hiç açmadan yukarıda ele alınıp döndü.
        Command::KuralEkle { .. }
        | Command::KuralAra { .. }
        | Command::KuralListele { .. }
        | Command::KuralKaldir { .. } => {
            unreachable!("kural komutları `let conn = baglan()` öncesinde döner")
        }
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Şema kurulumundan arama/pasifleştirmeye tam döngü — hedef DB dosyası
    /// olmadan (kural_baglan'ın kutuphane_yolu bağımlılığını atlar).
    #[tokio::test]
    async fn kural_ekle_ara_kaldir_dongusu() {
        let yol = std::env::temp_dir().join(format!(
            "ibnunnedim-kural-test-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&yol);
        let db = Builder::new_local(yol.to_string_lossy().as_ref())
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();

        let id = kural_ekle(&conn, "Bun kullan, Node'a kaçma", "CLAUDE.md", "araç,kural")
            .await
            .unwrap();

        let bulunan = kural_ara(&conn, "bun", 10).await.unwrap();
        assert_eq!(bulunan.len(), 1, "LIKE araması büyük/küçük harf duyarsız olmalı");
        assert_eq!(bulunan[0].id, id);
        assert_eq!(bulunan[0].etiketler, vec!["araç", "kural"]);

        let liste = kural_listele(&conn, Some("araç")).await.unwrap();
        assert_eq!(liste.len(), 1);

        kural_kaldir(&conn, &id).await.unwrap();
        assert!(
            kural_ara(&conn, "bun", 10).await.unwrap().is_empty(),
            "pasifleştirilen kural aramada görünmemeli"
        );

        let _ = std::fs::remove_file(&yol);
    }

    // Asil gerileme: eskiden olmayan yol sessizce YARATILIYORDU. Artik hata
    // vermeli ve geride dosya birakmamali.
    #[test]
    fn olmayan_kutuphane_yaratilmaz_hata_verir() {
        let yok = std::env::temp_dir().join("ibnunnedim-olmayan-kutuphane.db");
        let _ = std::fs::remove_file(&yok);
        // SAFETY: cargo testleri paralel kosar, yani "tek is parcacigi" degil.
        // Gecerli kilan sey su: TURSO_DB_PATH'i okuyan baska bir test yok.
        // Boyle bir test eklenirse ikisi de bu degiskeni ceker; o zaman ya
        // serial_test ya da yol cozumunu parametre alan bir ic fonksiyon gerekir.
        unsafe { std::env::set_var("TURSO_DB_PATH", &yok) };
        let sonuc = kutuphane_yolu();
        unsafe { std::env::remove_var("TURSO_DB_PATH") };

        assert!(sonuc.is_err(), "olmayan yol icin Ok dondu: {sonuc:?}");
        assert!(
            !yok.exists(),
            "yol cozumu dosyayi yaratti — Builder::new_local'a dusmus olmali"
        );
    }

    #[test]
    fn graf_argumanlari_kur_build_der() {
        let a = graf_argumanlari(None, true, "cikti").unwrap();
        assert_eq!(a, vec!["build", "--path", "."]);
    }

    #[test]
    fn graf_argumanlari_sorgu_graf_yolunu_verir() {
        let a = graf_argumanlari(Some("nedir"), false, "cikti").unwrap();
        assert_eq!(a, vec!["query", "nedir", "--graph", "cikti/graph.json"]);
    }

    /// Sessizce yanlış graf sorgulamaktansa açıkça reddetsin.
    #[test]
    fn graf_argumanlari_bos_soru_reddeder() {
        assert!(graf_argumanlari(None, false, "c").is_err());
        assert!(graf_argumanlari(Some("   "), false, "c").is_err());
    }

    /// Kapı düşebilmeli: eski `&s[..140]` bayt dilimi çok baytlı sınırda paniklerdi.
    #[test]
    fn kisalt_utf8_sinirinda_paniklemez() {
        let s = "şçğüöışçğüöı";
        for n in 0..s.chars().count() + 3 {
            let c = kisalt(s, n);
            assert!(
                std::str::from_utf8(c.as_bytes()).is_ok(),
                "n={n} geçersiz UTF-8"
            );
        }
    }

    #[test]
    fn kisalt_kirptigini_soyler() {
        let s = "abcdefghij";
        let c = kisalt(s, 4);
        assert!(c.starts_with("abcd"), "önek korunmadı: {c}");
        assert!(c.contains("+6 karakter"), "kırpma gizlendi: {c}");
    }

    #[test]
    fn kisalt_sinir_altinda_dokunmaz() {
        let s = "kısa metin";
        assert_eq!(kisalt(s, 100), s);
        assert_eq!(kisalt(s, s.chars().count()), s, "tam sınırda kırpmamalı");
    }

    /// Dedektörün kendisi: bayt dilimi gerçekten tehlikeli mi?
    #[test]
    fn bayt_dilimi_gercekten_tehlikeli() {
        let s = "şçğ"; // 6 bayt, 3 karakter
        assert_eq!(s.len(), 6, "fikstür varsayımı bozuldu");
        assert!(
            s.get(..3).is_none(),
            "3. bayt karakter sınırıymış — fikstür kötü seçilmiş"
        );
        assert!(s.get(..2).is_some(), "2. bayt sınır olmalıydı");
    }
}
