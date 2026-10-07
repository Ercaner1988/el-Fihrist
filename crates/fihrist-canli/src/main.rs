//! `fihrist-izle`: bir veritabanının değişikliklerini yoklayıp satır satır basar.
//!
//! Çıktı satırı: `no<TAB>zaman_ms<TAB>tablo<TAB>işlem<TAB>anahtar`.
//! İmleç her öbekten SONRA yazılır (en az bir kez teslim).
//!
//! `--sorgu` kipi canlı sorgudur: imleç dosyası yazılmaz. Çıktı satırı bir
//! önek ve sekmeyle ayrılmış değerlerdir: `=` ilk görüntü, `+` eklenen,
//! `~` değişen (yeni hali), `-` silinen.
//!
//! Belirlenimlilik sözleşmesi: çıktıda zaman ve rastgelelik yok; aynı ilk
//! durum ve aynı görülen anlık görüntüler aynı baytları verir. Satır sırası
//! sorgununkidir (tam belirlenimli sıra için tekil anahtar üzerinde
//! `ORDER BY`). Fark durum tabanlıdır: bir yoklama aralığına düşen yazışlar
//! tek farkta birleşir; ilk görüntüye bütün farklar uygulanınca her zaman
//! güncel sonuç çıkar. Her değişikliği sırasıyla görmek için tüketici kipi.

use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use turso::Value;

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
    /// Bir öbek (sorgu kipinde ilk fark) basılınca çık (betik ve sınama için).
    #[arg(long)]
    bir_kez: bool,
    /// Canlı sorgu: bu SELECT'i izle, yalnız farkı bas.
    #[arg(long, requires = "tablo")]
    sorgu: Option<String>,
    /// Canlı sorguda izlenen tablo (birden çok kez verilebilir).
    #[arg(long)]
    tablo: Vec<String>,
    /// Canlı sorguda fark anahtarı sütunu (birden çok kez verilebilir). Yoksa
    /// tek tablonun birincil anahtarı, o da yoksa rowid.
    #[arg(long)]
    anahtar: Vec<String>,
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
    if let Some(sorgu) = &a.sorgu {
        drop(c);
        return sorgu_izle(&a, sorgu).await;
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
                        )?;
                    }
                    o.flush()?;
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

async fn sorgu_izle(a: &Arg, sorgu: &str) -> fihrist_canli::Sonuc<()> {
    let tanim = fihrist_canli::CanliSorgu {
        sorgu: sorgu.to_string(),
        tablolar: a.tablo.clone(),
        anahtar: (!a.anahtar.is_empty()).then(|| a.anahtar.clone()),
    };
    let mut abone = fihrist_canli::Abone::baslat(&a.db, tanim).await?;
    eprintln!(
        "canlı sorgu: {} (sütunlar: {})",
        a.db.display(),
        abone.sutunlar().join(", ")
    );
    bas(abone.goruntu().map(|s| ('=', s)))?;
    loop {
        tokio::time::sleep(Duration::from_millis(a.aralik)).await;
        match abone.yokla().await {
            Ok(Some(f)) => {
                bas(f
                    .eklenen
                    .iter()
                    .map(|s| ('+', s))
                    .chain(f.degisen.iter().map(|(_, s)| ('~', s)))
                    .chain(f.silinen.iter().map(|s| ('-', s))))?;
                if a.bir_kez {
                    return Ok(());
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("! sorgu: {e}"),
        }
    }
}

fn bas<'a>(
    satirlar: impl Iterator<Item = (char, &'a fihrist_canli::Satir)>,
) -> std::io::Result<()> {
    let mut o = std::io::stdout().lock();
    for (onek, s) in satirlar {
        let hucreler: Vec<String> = s.iter().map(hucre).collect();
        writeln!(o, "{onek}\t{}", hucreler.join("\t"))?;
    }
    o.flush()
}

/// Sekme ve satır sonları (`\n`, `\r`) kaçışlanır: bir değer bir hücre, bir
/// satır bir satır kalır; `\r\n` ile bölen okuyucu değerin sonunu kırpmaz.
fn hucre(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Integer(n) => n.to_string(),
        Value::Real(f) => f.to_string(),
        Value::Text(s) => s
            .replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r"),
        Value::Blob(b) => format!(
            "x'{}'",
            b.iter().map(|x| format!("{x:02x}")).collect::<String>()
        ),
    }
}
