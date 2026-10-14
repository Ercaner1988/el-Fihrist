//! `ibnunnedim arsivle | arsiv-ara | secim-esle` komutları (ibnunnedim-cli yalnız bunu çağırır: sandık sınırı).

use crate::dokum::{dokumleri_bul, varsayilan_kok};
use crate::{esle, Depo};
use clap::{Args, Subcommand};
use fihrist_nazar::SecimBilgisi;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub enum Komutlar {
    /// Claude Code dökümlerini Oturum arşivine ekler (artımlı, yinelemez). Kancalar çağırır:
    /// PreCompact → `arsivle --kanca`, SessionStart → `arsivle --kanca` (kalanları tamamlar).
    Arsivle(ArsivleArgs),
    /// Oturum arşivinde arar (Kişisel veri: yalnız bu CLI; MCP'ye bağlı DEĞİL).
    ArsivAra(AraArgs),
    /// MCP Seçimini çıkarır: bir nazar_ara sonucundaki yol sonra bir araç çağrısında geçtiyse Seçimdir;
    /// nazard'ın Seçim günlüğüne `mcp` kanalıyla yazdırır (yinelenebilir).
    SecimEsle(EsleArgs),
}

#[derive(Args, Debug)]
pub struct ArsivleArgs {
    /// Tek döküm (.jsonl).
    pub dokum: Option<PathBuf>,
    /// Kökteki bütün dökümler (alt ajanlar dahil).
    #[arg(long)]
    pub tumu: bool,
    /// Döküm kökü. Varsayılan: `CLAUDE_CONFIG_DIR` ya da `~/.claude`, altında `projects`.
    #[arg(long)]
    pub kok: Option<PathBuf>,
    /// Claude Code kancası: stdin'deki JSON'dan okur (`transcript_path`; SessionStart'ta hepsi). Hata
    /// olsa da çıkış 0: PreCompact'ta sıfır olmayan çıkış sıkıştırmayı ENGELLER.
    #[arg(long)]
    pub kanca: bool,
}

#[derive(Args, Debug)]
pub struct AraArgs {
    /// Aranacak sözcükler.
    pub sorgu: Vec<String>,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    /// Yalnız bu tür: ercan | model | arac | ozet | sinir.
    #[arg(long)]
    pub tur: Option<String>,
}

#[derive(Args, Debug)]
pub struct EsleArgs {
    /// Nazar'ın Seçim günlüğü. Varsayılan: `%LOCALAPPDATA%\nazar\secim-gunlugu.db`.
    #[arg(long)]
    pub nazar_gunlugu: Option<PathBuf>,
    /// Yalnız göster, nazard'a yazma.
    #[arg(long)]
    pub kuru: bool,
}

/// `kutuphane`: katalog dosyası (`kutup_kutuphane.db`); arşiv yanına `kutup_arsiv.db` olarak konur. Katalog
/// açılmaz (Turso dosyayı süreç ömrünce kilitler), yalnız yolu kullanılır.
pub async fn calistir(k: Komutlar, kutuphane: &Path) -> Result<(), String> {
    let arsiv_db = kutuphane.with_file_name("kutup_arsiv.db");
    let arsiv_db = arsiv_db.as_path();
    match k {
        Komutlar::Arsivle(a) => {
            let kanca = a.kanca;
            let s = arsivle(a, arsiv_db).await;
            match (s, kanca) {
                (Err(e), true) => {
                    eprintln!("arsivle (kanca): {e}");
                    Ok(()) // sıkıştırmayı engelleme
                }
                (s, _) => s,
            }
        }
        Komutlar::ArsivAra(a) => ara(a, arsiv_db).await,
        Komutlar::SecimEsle(a) => esle_calistir(a, arsiv_db).await,
    }
}

/// Kanca stdin'inden `(transcript_path, tümü_mü)`.
fn kanca_girdisi() -> Result<(Option<PathBuf>, bool), String> {
    let mut s = String::new();
    std::io::stdin()
        .read_to_string(&mut s)
        .map_err(|e| e.to_string())?;
    let d: serde_json::Value =
        serde_json::from_str(&s).map_err(|e| format!("kanca JSON'u okunamadı: {e}"))?;
    let tumu = d.get("hook_event_name").and_then(|v| v.as_str()) == Some("SessionStart");
    let yol = d
        .get("transcript_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    Ok((yol, tumu))
}

async fn arsivle(a: ArsivleArgs, arsiv_db: &Path) -> Result<(), String> {
    let (mut dokum, mut tumu) = (a.dokum, a.tumu);
    if a.kanca {
        let (y, t) = kanca_girdisi()?;
        (dokum, tumu) = (dokum.or(y), tumu || t);
    }
    let dokumler = match (&dokum, tumu) {
        (Some(y), false) => vec![y.clone()],
        (_, true) => {
            let kok = a
                .kok
                .or_else(varsayilan_kok)
                .ok_or("döküm kökü bulunamadı; --kok ile ver")?;
            dokumleri_bul(&kok)
        }
        (None, false) => return Err("döküm yolu ya da --tumu ya da --kanca ver".into()),
    };
    let basla = std::time::Instant::now();
    let depo = Depo::ac(arsiv_db).await?;
    let toplam = dokumler.len();
    let dokumler = depo.degisenler(&dokumler).await?;
    let (mut yeni, mut satir, mut bozuk, mut bastan, mut hatali) = (0, 0, 0, 0, 0);
    for d in &dokumler {
        match depo.ice_aktar(d).await {
            Ok(o) => {
                yeni += o.yeni;
                satir += o.satir;
                bozuk += o.bozuk;
                bastan += o.bastan as usize;
            }
            // Biri okunamazsa (silinmiş, kilitli) ötekiler sürer; sayı sonda söylenir.
            Err(e) => {
                hatali += 1;
                eprintln!("  atlandı: {e}");
            }
        }
    }
    println!(
        "ARSİV {toplam} döküm ({} değişmiş) · {satir} yeni satır okundu · {yeni} kayıt eklendi · {bozuk} bozuk satır · {bastan} baştan okundu · {hatali} hatalı · {:.1} sn\n  dosya: {}",
        dokumler.len(),
        basla.elapsed().as_secs_f64(),
        arsiv_db.display()
    );
    if hatali > 0 {
        return Err(format!("{hatali} döküm işlenemedi"));
    }
    Ok(())
}

async fn ara(a: AraArgs, arsiv_db: &Path) -> Result<(), String> {
    if !arsiv_db.is_file() {
        return Err(format!(
            "arşiv yok: {} (önce `arsivle --tumu`)",
            arsiv_db.display()
        ));
    }
    let depo = Depo::ac(arsiv_db).await?;
    let sorgu = a.sorgu.join(" ");
    let r = depo.ara(&sorgu, a.limit, a.tur.as_deref()).await?;
    if r.is_empty() {
        println!("Sonuç yok.");
    }
    for (i, s) in r.iter().enumerate() {
        println!(
            "{}. [{:.2} {}{}] {} · oturum {}\n   {}\n   {}",
            i + 1,
            s.skor,
            s.tur,
            s.arac
                .as_deref()
                .map(|a| format!(" {a}"))
                .unwrap_or_default(),
            s.zaman,
            s.oturum.chars().take(8).collect::<String>(),
            s.yer,
            s.kesit.replace('\n', " ")
        );
    }
    Ok(())
}

fn varsayilan_nazar_gunlugu() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|d| PathBuf::from(d).join("nazar").join("secim-gunlugu.db"))
}

async fn esle_calistir(a: EsleArgs, arsiv_db: &Path) -> Result<(), String> {
    let gunluk = a
        .nazar_gunlugu
        .or_else(varsayilan_nazar_gunlugu)
        .ok_or("Nazar günlüğü bulunamadı; --nazar-gunlugu ile ver")?;
    if !gunluk.is_file() {
        return Err(format!(
            "Nazar Seçim günlüğü yok: {} (nazard yeni sürümle çalışmadı mı?)",
            gunluk.display()
        ));
    }
    if !arsiv_db.is_file() {
        return Err(format!(
            "arşiv yok: {} (önce `arsivle --tumu`)",
            arsiv_db.display()
        ));
    }
    let depo = Depo::ac(arsiv_db).await?;
    let (secimler, r) = esle::esle(&depo, &gunluk).await?;
    println!(
        "EŞLEME {} MCP araması · {} oturumsuz (eşlenemez) · {} arşivde yok · {} Seçim bulundu",
        r.arama,
        r.oturumsuz,
        r.arsivsiz,
        secimler.len()
    );
    if a.kuru {
        for s in &secimler {
            println!("  arama {} → {} ({})", s.arama_id, s.yol, s.arac);
        }
        return Ok(());
    }
    let gonderilecek: Vec<SecimBilgisi> = secimler
        .into_iter()
        .map(|s| SecimBilgisi {
            arama_id: s.arama_id,
            yol: s.yol.into(),
            yaprak: s.yaprak,
            satir_no: s.satir,
            gosterilen_sira: None,
            eylem: s.arac,
            kanal: Some("mcp".into()),
            zaman_ms: Some(s.zaman_ms),
        })
        .collect();
    let n = fihrist_nazar::secim_gonder(gonderilecek).await?;
    println!("  nazard'a {n} Seçim gönderildi (nazard daha önce yazılanları tekrar yazmaz)");
    Ok(())
}
