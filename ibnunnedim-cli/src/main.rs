//! İbnünnedîm CLI — Turso SQLite yetenek kütüphanesi arayüzü
//!
//! `kutup_kutuphane.db` üzerinde arama, yetenek listeleme ve tekmil puanlama
//! yapar. Arama **alt dize** eşleşmesidir: veritabanında FTS5 indeksi tanımlı
//! olsa da turso 0.7 FTS5 uygulamıyor.
//!
//! ```bash
//! ibnunnedim search "rust"
//! ibnunnedim list --kategori "software-development"
//! ibnunnedim info
//! ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Dış koşu geçti"
//! ```

use clap::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use turso::{params, Builder, Connection};

/// Varsayılan veritabanı yolu. `TURSO_DB_PATH` ile ezilebilir.
const DB_PATH: &str = r"C:\Users\buzbe\OneDrive\Masaüstü\hermes yazılım\kutuphane\kutup_kutuphane.db";

#[derive(Parser, Debug)]
#[command(name = "ibnunnedim")]
#[command(about = "İbnünnedîm Kütüphaneci CLI — Turso SQLite arayüzü")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Alt dize araması yapar (FTS5 değil — bkz. search_skills)
    Search {
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Yetenekleri listeler
    List {
        #[arg(short, long)]
        kategori: Option<String>,
    },
    /// Veritabanı istatistiklerini gösterir
    Info,
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

async fn baglan() -> Result<Connection> {
    let yol = std::env::var("TURSO_DB_PATH").unwrap_or_else(|_| DB_PATH.to_string());
    let db = Builder::new_local(&yol).build().await?;
    Ok(db.connect()?)
}

/// Alt dize araması — **FTS5 değil.**
///
/// `yetenekler_fts` gerçek bir FTS5 sanal tablosu (129 satır, `sqlite_master`'da
/// tanımlı) ama **turso 0.7.2 FTS5 uygulamıyor**: kaynağında `fts5` hiç geçmiyor
/// ve `MATCH` sorgusu `no such table: yetenekler_fts` ile düşüyor. Bu yüzden
/// arama `LIKE` ile yapılır; sıralama (relevance) yoktur, çıktı bunu söyler.
async fn search_skills(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Skill>> {
    let sql = r#"
        SELECT id, ad, aciklama, basari_puani_ort, kategori
        FROM yetenekler
        WHERE ad LIKE ? OR aciklama LIKE ? OR tam_metin_md LIKE ?
        ORDER BY ad ASC
        LIMIT ?
    "#;
    let kalip = format!("%{query}%");
    let mut satirlar = conn
        .query(sql, params![kalip.clone(), kalip.clone(), kalip, limit as i64])
        .await?;
    let mut sonuc = Vec::new();
    while let Some(r) = satirlar.next().await? {
        sonuc.push(Skill {
            id: r.get::<String>(0)?,
            ad: r.get::<String>(1)?,
            aciklama: r.get::<String>(2).unwrap_or_default(),
            basari_puani_ort: r.get::<f64>(3).ok(),
            kategori: r.get::<String>(4).ok(),
        });
    }
    Ok(sonuc)
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
    let mut satirlar = conn.query(&format!("SELECT COUNT(*) FROM {tablo}"), ()).await?;
    match satirlar.next().await? {
        Some(r) => Ok(r.get::<i64>(0)?),
        None => Err(CliError::Girdi(format!("{tablo}: COUNT satır döndürmedi"))),
    }
}

/// FTS5 indeksinin **gerçekten** var olup olmadığını sorar.
///
/// Önceki sürüm bunu sabit metin olarak "ETKİN" yazıyordu; iddia kaynağa
/// bağlanmadan basılan bir cümleydi.
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
        return Err(CliError::Girdi(format!("puan 0-100 aralığında olmalı: {puan}")));
    }
    conn.execute(
        "INSERT INTO ajan_tekmilleri \
         (hafta_no, ajan_adi, yetenek_id, verilen_puan, degerlendirme_gerekcesi) \
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let conn = baglan().await?;

    match args.command {
        Command::Search { query, limit } => {
            let skills = search_skills(&conn, &query, limit).await?;
            println!("\nARAMA SONUÇLARI ('{query}') — {} kayıt", skills.len());
            println!("(alt dize araması — turso 0.7 FTS5 uygulamıyor, sıralama yok)");
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
            println!("  Veritabanı  : {}", std::env::var("TURSO_DB_PATH").unwrap_or_else(|_| DB_PATH.into()));
            println!("  Yetenek     : {yetenek}");
            println!("  Tekmil      : {tekmil}");
            println!("  Kod hazinesi: {kod}");
            println!(
                "  FTS5 indeks : {} (tanımlı; turso 0.7 sorgulayamıyor)",
                if fts.is_empty() { "YOK".to_string() } else { fts.join(", ") }
            );
            println!("{}", "=".repeat(65));
        }
        Command::Tekmil { ajan, yetenek, puan, gerekce, hafta } => {
            tekmil_ver(&conn, hafta, &ajan, &yetenek, puan, &gerekce).await?;
            println!("TEKMİL KAYDEDİLDİ: {ajan} → yetenek {yetenek} (puan {puan}, hafta {hafta})");
        }
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Kapı düşebilmeli: eski `&s[..140]` bayt dilimi çok baytlı sınırda paniklerdi.
    #[test]
    fn kisalt_utf8_sinirinda_paniklemez() {
        // Her karakter 2 bayt; 5 karakterde kesmek 10. bayta denk gelir ama
        // 3 karakterde kesmek bayt ortasına düşerdi.
        let s = "şçğüöışçğüöı";
        for n in 0..s.chars().count() + 3 {
            let c = kisalt(s, n);
            assert!(std::str::from_utf8(c.as_bytes()).is_ok(), "n={n} geçersiz UTF-8");
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
    /// Tehlikeli olmasaydı `kisalt` gereksiz bir sarmalayıcı olurdu.
    #[test]
    fn bayt_dilimi_gercekten_tehlikeli() {
        let s = "şçğ"; // 6 bayt, 3 karakter
        assert_eq!(s.len(), 6, "fikstür varsayımı bozuldu");
        assert!(s.get(..3).is_none(), "3. bayt karakter sınırıymış — fikstür kötü seçilmiş");
        assert!(s.get(..2).is_some(), "2. bayt sınır olmalıydı");
    }
}
