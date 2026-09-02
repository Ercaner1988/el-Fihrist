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
//! ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Dış koşu geçti"
//! ```

mod arama;

use arama::{Belge, Indeks};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use turso::{params, Builder, Connection};

/// Varsayılan veritabanı yolu. `TURSO_DB_PATH` ile ezilebilir.
const DB_PATH: &str = "kutup_kutuphane.db";

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

/// Saf Rust BM25 ile bellek içi arama yapar.
///
/// Veritabanındaki tüm yetenek kayıtlarını tek sorgu ile okur, `Indeks::kur` ile
/// bellekte BM25 indeksini oluşturur ve `ara` ile en yüksek puanlı `limit` kaydı döndürür.
async fn search_skills(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<(Vec<Skill>, usize, std::time::Duration)> {
    let baslangic = Instant::now();

    let sql = r#"
        SELECT id, ad, aciklama, tam_metin_md, basari_puani_ort, kategori
        FROM yetenekler
    "#;
    let mut satirlar = conn.query(sql, ()).await?;
    let mut belgeler = Vec::new();
    let mut skill_map = Vec::new();

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

        skill_map.push(Skill {
            id,
            ad,
            aciklama,
            basari_puani_ort,
            kategori,
        });
    }

    let toplam_kayit = belgeler.len();
    let indeks = Indeks::kur(belgeler);
    let arama_sonuclari = indeks.ara(query, limit);

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
        Command::Search { query, limit } => {
            let (skills, toplam_kayit, sure) = search_skills(&conn, &query, limit).await?;
            println!("\nARAMA SONUÇLARI ('{query}') — {} kayıt", skills.len());
            #[cfg(debug_assertions)]
            let kip = " · DEBUG derlemesi, release ~12 kat hızlı";
            #[cfg(not(debug_assertions))]
            let kip = "";
            println!(
                "(saf Rust BM25 · {toplam_kayit} kayıt tarandı · {:.1} ms{kip})",
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
                std::env::var("TURSO_DB_PATH").unwrap_or_else(|_| DB_PATH.into())
            );
            println!("  Yetenek     : {yetenek}");
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
