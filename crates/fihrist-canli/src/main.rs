//! `fihrist-izle`: bir veritabanının değişikliklerini yoklayıp satır satır basar.
//!
//! Çıktı satırı: `no<TAB>zaman_ms<TAB>tablo<TAB>işlem<TAB>anahtar`.
//! İmleç her öbekten SONRA yazılır (en az bir kez teslim).

use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "fihrist-izle",
    about = "Veritabanı değişikliklerini canlı izler"
)]
struct Arg {
    /// İzlenecek veritabanı dosyası.
    db: PathBuf,
    /// Önce günlüğü ve tetikleyicileri kur (iki kez koşmak zararsız).
    #[arg(long)]
    kur: bool,
    /// Tüketici adı; imleç bu adla saklanır.
    #[arg(long, default_value = "izle")]
    tuketici: String,
    /// Yoklama aralığı (ms).
    #[arg(long, default_value_t = 250)]
    aralik: u64,
    /// Bir öbek basılınca çık (betik ve sınama için).
    #[arg(long)]
    bir_kez: bool,
}

// ponytail: budama her 1000 olayda bir; günlük ondan büyümez.
const BUDAMA_ESIGI: usize = 1000;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = calis(Arg::parse()).await {
        eprintln!("fihrist-izle: {e}");
        std::process::exit(1);
    }
}

async fn calis(a: Arg) -> fihrist_canli::Sonuc<()> {
    let c = fihrist_canli::ac(&a.db).await?;
    if a.kur {
        let t = fihrist_canli::kur(&c).await?;
        eprintln!("kuruldu: {} tablo izleniyor ({})", t.len(), t.join(", "));
    }
    let mut imlec = match fihrist_canli::imlec_oku(&a.db, &a.tuketici)? {
        Some(n) => n,
        None => {
            let n = fihrist_canli::son_no(&c).await?;
            fihrist_canli::imlec_yaz(&a.db, &a.tuketici, n)?;
            n
        }
    };
    drop(c);
    eprintln!(
        "izleniyor: {} (tüketici {}, imleç {imlec})",
        a.db.display(),
        a.tuketici
    );

    let mut budamadan_beri = 0;
    loop {
        // Yoklama başına taze bağlantı: açık bağlantı başka süreçlerin yazışını görmez.
        // Geçici hata (dosya kilitli vb.) izlemeyi durdurmaz, bildirilir.
        match fihrist_canli::ac(&a.db).await {
            Ok(c) => match fihrist_canli::oku(&c, imlec, 500).await {
                Ok(v) if !v.is_empty() => {
                    let mut o = std::io::stdout().lock();
                    for d in &v {
                        writeln!(
                            o,
                            "{}\t{}\t{}\t{}\t{}",
                            d.no, d.zaman_ms, d.tablo, d.islem, d.anahtar
                        )
                        .ok();
                    }
                    o.flush().ok();
                    imlec = v[v.len() - 1].no;
                    fihrist_canli::imlec_yaz(&a.db, &a.tuketici, imlec)?;
                    budamadan_beri += v.len();
                    if budamadan_beri >= BUDAMA_ESIGI {
                        match fihrist_canli::buda(&c, &a.db).await {
                            Ok(n) => eprintln!("budandı: {n} satır"),
                            Err(e) => eprintln!("! budama: {e}"),
                        }
                        budamadan_beri = 0;
                    }
                    if a.bir_kez {
                        return Ok(());
                    }
                }
                Ok(_) => {}
                Err(e) => eprintln!("! okuma: {e}"),
            },
            Err(e) => eprintln!("! açılış: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(a.aralik)).await;
    }
}
