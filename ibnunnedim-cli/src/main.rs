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
//! ```

mod arama;
mod gomme;
mod mcp;
mod tara;

use arama::{Belge, Indeks};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use thiserror::Error;
use turso::{params, Builder, Connection};

/// Kütüphane dosyasının adı. Aranacak yerler için `kutuphane_yolu`.
const DB_ADI: &str = "kutup_kutuphane.db";

/// Depo kataloğu — ana kütüphanenin yanında ayrı dosya. Gerekçesi `depo_baglan`.
const DEPO_DB_ADI: &str = "kutup_depolar.db";

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
    /// Stdio MCP sunucusu olarak koşar (JSON-RPC 2.0). stdout protokole aittir.
    Mcp,
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
}

#[derive(Error, Debug)]
enum CliError {
    #[error("Veritabanı hatası: {0}")]
    Database(#[from] turso::Error),
    #[error("IO hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("Girdi hatası: {0}")]
    Girdi(String),
}

type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Deserialize, Serialize)]
struct Skill {
    id: String,
    ad: String,
    aciklama: String,
    basari_puani_ort: Option<f64>,
    kategori: Option<String>,
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

/// Saf Rust BM25 ile bellek içi arama yapar.
///
/// Veritabanındaki tüm yetenek kayıtlarını tek sorgu ile okur, `Indeks::kur` ile
/// bellekte BM25 indeksini oluşturur ve `ara` ile en yüksek puanlı `limit` kaydı döndürür.
/// BM25 + gömme kosinüsü, normalize füzyonla birleştirilir.
///
/// Gömme sütunu yoksa ya da imzası tutmuyorsa o satır KOSİNÜSE GİRMEZ ama
/// BM25'te kalır — hiçbir kayıt vektörü eksik diye kaybolmaz. Kullanılabilir
/// vektör hiç yoksa saf BM25'e düşer ve bunu stderr'e YAZAR: sessiz düşüş,
/// "arama bozuldu" diye görünen ama sebebi görünmeyen hatanın ta kendisidir.
async fn search_skills(
    conn: &Connection,
    query: &str,
    limit: usize,
    kip: gomme::Cekirdek,
) -> Result<(Vec<Skill>, usize, std::time::Duration)> {
    let baslangic = Instant::now();

    // gomme/gomme_imza göç edilmemiş DB'de yoktur; o yüzden şema önce yoklanır.
    let gomme_var = gomme_sutunu_var(conn).await?;
    let vektor_sutunlari = if gomme_var {
        ", gomme, gomme_imza, icerik_hash"
    } else {
        ""
    };
    let (mut belgeler, mut skill_map, mut vektorler) = satirlari_oku(
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
        skill_map.extend(s);
        vektorler.extend(v);
    }

    let toplam_kayit = belgeler.len();
    let indeks = Indeks::kur(belgeler);
    // Füzyon iki kanalın BİRLEŞİMİNİ alır; iç tarama limitin 4 katı yapılır ki
    // BM25 kesiminden düşen ama kosinüsün öne çıkardığı belge kaybolmasın.
    let genis = limit.saturating_mul(4).max(8);
    let bm = indeks.ara(query, genis);

    let kullanilabilir = vektorler.iter().filter(|v| v.is_some()).count();
    let kos: Vec<(usize, f64)> = if kullanilabilir == 0 {
        if gomme_var {
            eprintln!("! gömme bayat (0/{toplam_kayit} satır kullanılabilir) — yalnız BM25.");
            eprintln!("  Onarım: ibnunnedim gomme --kip {}", kip.kip());
        }
        Vec::new()
    } else {
        let q = gomme::gomme(&[query.to_string()], gomme::Rol::Sorgu, kip)?;
        let taban = kip.taban();
        let mut v: Vec<(usize, f64)> = vektorler
            .iter()
            .enumerate()
            .filter_map(|(i, ov)| ov.as_ref().map(|x| (i, gomme::kosinus(&q[0], x))))
            .filter(|(_, p)| *p > taban)
            .map(|(i, p)| (i, p as f64))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(genis);
        v
    };

    // 0,5/0,5: pasli-beyin'de ölçülmüş ağırlık (kopru.rs Kanal::Karmasik).
    let arama_sonuclari = arama::harmanla(&bm, &kos, 0.5, limit);

    let mut sonuc = Vec::with_capacity(arama_sonuclari.len());
    for (idx, _puan) in arama_sonuclari {
        sonuc.push(Skill {
            id: skill_map[idx].id.clone(),
            ad: skill_map[idx].ad.clone(),
            aciklama: skill_map[idx].aciklama.clone(),
            basari_puani_ort: skill_map[idx].basari_puani_ort,
            kategori: skill_map[idx].kategori.clone(),
        });
    }

    let sure = baslangic.elapsed();
    Ok((sonuc, toplam_kayit, sure))
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
    // Toplu gömme: ONNX çekirdeği modeli bir kez yükleyebilsin.
    let metinler: Vec<String> = isler.iter().map(|(_, m, _)| m.clone()).collect();
    let vektorler = gomme::gomme(&metinler, gomme::Rol::Belge, k)?;

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
    let yol = kutuphane_yolu()
        .map_err(CliError::Girdi)?
        .with_file_name(DEPO_DB_ADI);
    if !yol.is_file() && !yarat {
        return Ok(None);
    }
    let db = Builder::new_local(yol.to_string_lossy().as_ref())
        .build()
        .await?;
    Ok(Some(db.connect()?))
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
        Command::Mcp => mcp::calistir(&conn).await?,
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
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

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
