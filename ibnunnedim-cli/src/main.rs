//! İbnünnedîm CLI — Saf Rust Turso SQLite Yetenek Kütüphanesi Arayüzü
//!
//! Bu crate, Turso SQLite `kutup_kutuphane.db` veritabanına erişim sağlar
//! ve FTS5 arama, yetenek listeleme, tekmil puanlama gibi işlevleri sunar.
//! 
//! Kullanım:
//! ```bash
//! ibnunnedim search "rust"
//! ibnunnedim list --kategori "software-development"
//! ibnunnedim info
//! ibnunnedim tekmil --ajan "Kassam" --yetenek "1" --puan 100 --gerekce "Canlı dış koşu geçti"
//! ```

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use turso::{Connection, Params};

/// Varsayılan Turso SQLite veritabanı yolu
const DB_PATH: &str = r"C:\Users\buzbe\OneDrive\Masaüstü\hermes yazılım\kutuphane\kutup_kutuphane.db";

/// CLI Argümanları
#[derive(Parser, Debug)]
#[command(name = "ibnunnedim")]
#[command(about = "İbnünnedîm Kütüphaneci CLI - Saf Rust Turso SQLite Arayüzü")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// FTS5 araması yapar
    Search {
        /// Arama sorgusu
        query: String,
        /// Maksimum sonuç sayısı (varsayılan: 10)
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Tüm yetenekleri listeler
    List {
        /// Kategori filtresi (opsiyonel)
        #[arg(short, long)]
        kategori: Option<String>,
    },
    /// Veritabanı istatistiklerini gösterir
    Info,
    /// Tekmil puanı ekler
    Tekmil {
        /// Ajan adı
        #[arg(long)]
        ajan: String,
        /// Yetenek ID
        #[arg(long)]
        yetenek: String,
        /// Verilen puan (0-100)
        #[arg(long)]
        puan: f64,
        /// Değerlendirme gerekçesi
        #[arg(long)]
        gerekce: String,
    },
}

#[derive(Error, Debug)]
enum CliError {
    #[error("Veritabanı hatası: {0}")]
    Database(#[from] turso::Error),
    #[error("SQL hatası: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("IO hatası: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Deserialize, Serialize)]
struct Skill {
    id: i64,
    ad: String,
    aciklama: String,
    basari_puani_ort: Option<f64>,
    kategori: Option<String>,
}

fn get_db_connection() -> Result<Connection> {
    let db_path = std::env::var("TURSO_DB_PATH")
        .unwrap_or_else(|_| DB_PATH.to_string());
    let conn = Connection::open(&db_path)?;
    Ok(conn)
}

async fn search_skills(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Skill>> {
    let sql = r#"
        SELECT id, ad, aciklama, basari_puani_ort, kategori
        FROM yetenekler
        WHERE rowid IN (
            SELECT rowid FROM yetenekler_fts WHERE yetenekler_fts MATCH ?
        )
        LIMIT ?
    "#;
    
    let skills = conn.query(sql, params![query, limit])?
        .map(|row| {
            Ok(Skill {
                id: row.get::<_, i64>(0)?,
                ad: row.get::<_, String>(1)?,
                aciklama: row.get::<_, String>(2)?,
                basari_puani_ort: row.get::<_, Option<f64>>(3)?,
                kategori: row.get::<_, Option<String>>(4)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    
    Ok(skills)
}

async fn list_all_skills(conn: &Connection, kategori: Option<&str>) -> Result<Vec<Skill>> {
    let (sql, params): (&str, Params) = if let Some(kat) = kategori {
        (r#"SELECT id, ad, kategori, basari_puani_ort FROM yetenekler WHERE kategori = ? ORDER BY ad ASC"#, params![kat])
    } else {
        (r#"SELECT id, ad, kategori, basari_puani_ort FROM yetenekler ORDER BY kategori, ad ASC"#, Params::Empty)
    };

    let skills = conn.query(sql, params)?
        .map(|row| {
            Ok(Skill {
                id: row.get::<_, i64>(0)?,
                ad: row.get::<_, String>(1)?,
                kategori: row.get::<_, Option<String>>(2)?,
                basari_puani_ort: row.get::<_, Option<f64>>(3)?,
                aciklama: String::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    
    Ok(skills)
}

async fn show_info(conn: &Connection) -> Result<(usize, usize, usize)> {
    let tot_yetenek: usize = conn.query_row(r#"SELECT COUNT(*) FROM yetenekler"#, Params::Empty, |r| r.get(0))?;
    let tot_tekmil: usize = conn.query_row(r#"SELECT COUNT(*) FROM ajan_tekmilleri"#, Params::Empty, |r| r.get(0))?;
    let tot_kod: usize = conn.query_row(r#"SELECT COUNT(*) FROM kod_hazinesi"#, Params::Empty, |r| r.get(0))?;
    
    Ok((tot_yetenek, tot_tekmil, tot_kod))
}

async fn tekmil_ver(conn: &Connection, ajan: &str, yetenek_id: i64, puan: f64, gerekce: &str) -> Result<()> {
    let sql = r#"
        INSERT INTO ajan_tekmilleri (hafta_no, ajan_adi, yetenek_id, verilen_puan, degerlendirme_gerekcesi)
        VALUES (202634, ?, ?, ?, ?)
    "#;
    conn.execute(sql, params![ajan, yetenek_id, puan, gerekce])?;

    // Ortalama ve sayacı güncelle
    let (avg_puan, count): (f64, usize) = conn.query_row(
        r#"SELECT AVG(verilen_puan), COUNT(*) FROM ajan_tekmilleri WHERE yetenek_id = ?"#,
        params![yetenek_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    conn.execute(
        r#"UPDATE yetenekler SET basari_puani_ort = ?, puanlayan_ajan_sayisi = ? WHERE id = ?"#,
        params![avg_puan, count, yetenek_id],
    )?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let conn = get_db_connection()?;

    match args.command {
        Command::Search { query, limit } => {
            let skills = search_skills(&conn, &query, limit).await?;
            println!("\n🔍 İBNÜNNEDÎM KÜTÜPHANE ARAMA SONUÇLARI ('{}'):", query);
            println!("{}", "-".repeat(70));
            for s in skills {
                println!("📌 [{}] {} | Kategori: {} | Başarı: {:?}", 
                    s.id, s.ad, s.kategori.unwrap_or_else(|| "-".to_string()),
                    s.basari_puani_ort.unwrap_or(-1.0));
                println!("   Açıklama: {}...", &s.aciklama[..s.aciklama.len().min(140)]);
            }
        }
        Command::List { kategori } => {
            let skills = list_all_skills(&conn, kategori.as_deref()).await?;
            println!("\n📚 İBNÜNNEDÎM KÜTÜPHANESİ TAM YETENEK DÖKÜMÜ ({} Yetenek):", skills.len());
            println!("{}", "=".repeat(75));
            let mut current_cat: Option<String> = None;
            for s in skills {
                let cat = s.kategori.unwrap_or_else(|| "GENEL".to_string());
                if Some(&cat) != current_cat.as_ref() {
                    println!("\n🗂️  KATEGORİ: {}", cat);
                    println!("{}", "-".repeat(50));
                    current_cat = Some(cat);
                }
                println!("  • [{}] {} (Başarı: {:?}/100)", s.id, s.ad, s.basari_puani_ort.unwrap_or(-1.0));
            }
        }
        Command::Info => {
            let (yetenek, tekmil, kod) = show_info(&conn).await?;
            println!("\n📊 İBNÜNNEDÎM KÜTÜPHANESİ DİZİN & İSTATİSTİK RAPORU:");
            println!("{}", "=".repeat(65));
            println!("  📍 Veritabanı Konumu: {}", DB_PATH);
            println!("  📖 Kayıtlı Yetenek Sayısı : {}", yetenek);
            println!("  📝 Ajan Tekmil Kaydı Sayısı: {}", tekmil);
            println!("  💻 Kod Hazinesi Sayısı   : {}", kod);
            println!("  ⚡ Tam Metin İndeks (FTS5): ETKİN (yetenekler_fts & kod_hazinesi_fts)");
            println!("{}", "=".repeat(65));
        }
        Command::Tekmil { ajan, yetenek, puan, gerekce } => {
            let yetenek_id: i64 = yetenek.parse().unwrap_or(0);
            tekmil_ver(&conn, &ajan, yetenek_id, puan, &gerekce).await?;
            println!("✅ TEKMİL KAYDEDİLDİ: {} -> {} (Puan: {})", ajan, yetenek_id, puan);
        }
    }

    Ok(())
}
