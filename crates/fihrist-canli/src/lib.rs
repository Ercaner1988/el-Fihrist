//! Canlı değişiklik bildirimi (yol haritası, Görev 1).
//!
//! Mekanizma Turso'nun bağlantı başına CDC'si DEĞİL, şemaya yazılan
//! tetikleyiciler: her tablonun ekleme/güncelleme/silmesi `fihrist_degisiklik`
//! günlüğüne bir satır düşer. Tetikleyici dosyada durur, hangi motor yazarsa
//! yazsın ateşlenir.
//!
//! Turso 0.7.2 ölçümleri (2026-10-02, gerçek DB'lerin kopyalarında):
//! - CDC (`unstable_capture_data_changes_conn`) yalnız Turso'nun kendi yazışını
//!   görür; aynı dosyaya Python `sqlite3` ile yapılan yazışı kaçırır. Ayrıca
//!   anahtarı değil rowid'i kaydeder.
//! - fts5 sanal tablosu olan dosyada Turso şemayı fts5 satırında keser: sonradan
//!   eklenen günlük ve tetikleyiciler Turso'ya görünmez, Turso'nun yazışı onları
//!   ateşlemez. [`kur`] böyle dosyayı reddeder (`kutup_kutuphane.db`).
//! - Açık kalan bağlantı başka süreçlerin yazışını görmez (bayat anlık görüntü).
//!   Bu yüzden her yoklama [`ac`] ile taze açar; maliyeti ~5 ms.
//! - `pragma_table_info(...)` tablo fonksiyonu aynı bağlantıdaki sonraki
//!   otomatik-commit yazışları hatasız kaybettirir. Burada yalnız
//!   `PRAGMA table_info` kullanılır.
//!
//! Teslim en az bir kez: imleç olaylar işlendikten sonra yazılır; arada çökme
//! olursa son öbek yeniden gelir, kaybolmaz.
//!
//! Canlı sorgu ([`Abone`], [`abone_ol`]) bu günlüğü yalnız işaret olarak okur:
//! izlenen tablo değişince sorgu yeniden koşulur, aboneye fark gider.

mod sorgu;

pub use sorgu::{abone_ol, Abone, Abonelik, CanliSorgu, Fark, Satir};
use std::path::{Path, PathBuf};
use turso::{params, Builder, Connection, Value};

/// Değişiklik günlüğü tablosu.
pub const GUNLUK: &str = "fihrist_degisiklik";

#[derive(Debug, thiserror::Error)]
pub enum Hata {
    #[error("turso: {0}")]
    Turso(#[from] turso::Error),
    #[error("imleç dosyası: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Kurulamaz(String),
    #[error("canlı sorgu: {0}")]
    Sorgu(String),
}

pub type Sonuc<T> = Result<T, Hata>;

/// Günlükteki bir satır. `islem`: `e` ekleme, `g` güncelleme, `s` silme.
/// `anahtar`: birincil anahtar sütunlarının JSON dizisi (`["9router"]`),
/// birincil anahtarı olmayan tabloda `[rowid]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Degisiklik {
    pub no: i64,
    pub zaman_ms: i64,
    pub tablo: String,
    pub islem: String,
    pub anahtar: String,
}

/// Veritabanını taze açar. Olmayan dosyayı YARATMAZ (`Builder::new_local`
/// yaratırdı ve izleyici boş bir dosyayı sessizce "değişiklik yok" diye izlerdi).
pub async fn ac(yol: &Path) -> Sonuc<Connection> {
    if !yol.is_file() {
        return Err(Hata::Kurulamaz(format!(
            "veritabanı yok: {}",
            yol.display()
        )));
    }
    let db = Builder::new_local(&yol.to_string_lossy()).build().await?;
    Ok(db.connect()?)
}

/// Günlüğü ve her tablonun üç tetikleyicisini kurar; izlenen tabloları döner.
/// İki kez koşmak zararsızdır; sonradan eklenen tablolar için yeniden koşulur.
pub async fn kur(c: &Connection) -> Sonuc<Vec<String>> {
    let sanal = metinler(
        c,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND sql LIKE 'CREATE VIRTUAL TABLE%'",
    )
    .await?;
    if !sanal.is_empty() {
        return Err(Hata::Kurulamaz(format!(
            "sanal tablo var ({}). Turso 0.7.2 şemayı bunlarda keser: günlük ve \
             tetikleyiciler Turso'ya görünmez kalır. Bu dosya izlenemez.",
            sanal.join(", ")
        )));
    }

    let tablolar: Vec<String> = metinler(
        c,
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
    )
    .await?
    .into_iter()
    .filter(|t| {
        !["sqlite_", "fihrist_", "__turso"]
            .iter()
            .any(|o| t.starts_with(o))
    })
    .collect();

    let mut ddl = vec![format!(
        "CREATE TABLE IF NOT EXISTS {GUNLUK} (\
         no INTEGER PRIMARY KEY, \
         zaman_ms INTEGER NOT NULL DEFAULT (CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)), \
         tablo TEXT NOT NULL, \
         islem TEXT NOT NULL CHECK (islem IN ('e', 'g', 's')), \
         anahtar TEXT NOT NULL)"
    )];
    for t in &tablolar {
        let pk = anahtar_sutunlari(c, t).await?;
        for (ek, olay, satir) in [
            ("e", "INSERT", "NEW"),
            ("g", "UPDATE", "NEW"),
            ("s", "DELETE", "OLD"),
        ] {
            let anahtar = if pk.is_empty() {
                format!("{satir}.rowid")
            } else {
                pk.iter()
                    .map(|s| format!("{satir}.{}", kimlik(s)))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            ddl.push(format!(
                "CREATE TRIGGER IF NOT EXISTS {} AFTER {olay} ON {} BEGIN \
                 INSERT INTO {GUNLUK} (tablo, islem, anahtar) VALUES ({}, '{ek}', json_array({anahtar})); END",
                kimlik(&format!("fd_{t}_{ek}")),
                kimlik(t),
                metin(t),
            ));
        }
    }

    // Tek işlem: yarım kurulum (günlük var, tetikleyicilerin yarısı yok) kalmaz.
    c.execute("BEGIN", ()).await?;
    for s in &ddl {
        if let Err(e) = c.execute(s, ()).await {
            if let Err(r) = c.execute("ROLLBACK", ()).await {
                eprintln!("geri alma başarısız: {r}");
            }
            return Err(e.into());
        }
    }
    c.execute("COMMIT", ()).await?;
    Ok(tablolar)
}

/// `sonra`dan büyük numaralı en çok `sinir` değişikliği sırayla okur.
pub async fn oku(c: &Connection, sonra: i64, sinir: i64) -> Sonuc<Vec<Degisiklik>> {
    let mut r = c
        .query(
            &format!(
                "SELECT no, zaman_ms, tablo, islem, anahtar FROM {GUNLUK} \
                 WHERE no > ?1 ORDER BY no LIMIT ?2"
            ),
            params![sonra, sinir],
        )
        .await?;
    let mut v = Vec::new();
    while let Some(s) = r.next().await? {
        v.push(Degisiklik {
            no: tamsayi(s.get_value(0)?),
            zaman_ms: tamsayi(s.get_value(1)?),
            tablo: yazi(s.get_value(2)?),
            islem: yazi(s.get_value(3)?),
            anahtar: yazi(s.get_value(4)?),
        });
    }
    Ok(v)
}

/// Günlüğün son numarası (boşsa 0). Yeni tüketici buradan başlar.
pub async fn son_no(c: &Connection) -> Sonuc<i64> {
    let mut r = c
        .query(&format!("SELECT coalesce(max(no), 0) FROM {GUNLUK}"), ())
        .await?;
    Ok(match r.next().await? {
        Some(s) => tamsayi(s.get_value(0)?),
        None => 0,
    })
}

/// Bütün tüketicilerin geçtiği satırları siler. Son satır HEP kalır: tablo
/// boşalırsa `INTEGER PRIMARY KEY` numarayı 1'den yeniden verir ve imleci
/// ileride olan tüketici yeni olayları atlardı. Tüketici yoksa son satır
/// dışında hepsi gider.
pub async fn buda(c: &Connection, db: &Path) -> Sonuc<u64> {
    let tuketiciler = imlecler(db)?;
    let esik = match tuketiciler.iter().min() {
        Some(&n) => n,
        None => son_no(c).await?,
    };
    Ok(c.execute(
        &format!("DELETE FROM {GUNLUK} WHERE no < ?1 AND no < (SELECT max(no) FROM {GUNLUK})"),
        params![esik],
    )
    .await?)
}

/// İmleç dosyaları veritabanının yanında `<db>.imlec/<tüketici>`.
/// DB'ye değil dosyaya yazılır: yoklama döngüsü veritabanına yazmaz.
fn imlec_dizini(db: &Path) -> PathBuf {
    let mut s = db.as_os_str().to_owned();
    s.push(".imlec");
    s.into()
}

fn imlec_yolu(db: &Path, tuketici: &str) -> Sonuc<PathBuf> {
    let gecerli = !tuketici.is_empty()
        && tuketici
            .chars()
            .all(|k| k.is_ascii_alphanumeric() || k == '-' || k == '_');
    if !gecerli {
        return Err(Hata::Kurulamaz(format!(
            "tüketici adı yalnız harf, rakam, - ve _ içerebilir: {tuketici:?}"
        )));
    }
    Ok(imlec_dizini(db).join(tuketici))
}

/// Tüketicinin imleci; ilk kez geliyorsa `None`.
pub fn imlec_oku(db: &Path, tuketici: &str) -> Sonuc<Option<i64>> {
    match std::fs::read_to_string(imlec_yolu(db, tuketici)?) {
        Ok(s) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|e| Hata::Kurulamaz(format!("bozuk imleç ({tuketici}): {s:?}: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// İmleci geçici dosya + yeniden adlandırma ile yazar: çökme yarım dosya bırakmaz.
pub fn imlec_yaz(db: &Path, tuketici: &str, no: i64) -> Sonuc<()> {
    let yol = imlec_yolu(db, tuketici)?;
    std::fs::create_dir_all(imlec_dizini(db))?;
    let gecici = yol.with_extension("yaziliyor");
    std::fs::write(&gecici, no.to_string())?;
    std::fs::rename(&gecici, &yol)?;
    Ok(())
}

fn imlecler(db: &Path) -> Sonuc<Vec<i64>> {
    let dizin = match std::fs::read_dir(imlec_dizini(db)) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut v = Vec::new();
    for g in dizin {
        let ad = g?.file_name().to_string_lossy().into_owned();
        if ad.ends_with(".yaziliyor") {
            continue;
        }
        if let Some(n) = imlec_oku(db, &ad)? {
            v.push(n);
        }
    }
    Ok(v)
}

/// Birincil anahtar sütunları, tablodaki sırasıyla. Turso'nun `pk` değeri
/// bileşik anahtarda hep 1 (SQLite 1,2,3 verir); sıra bu yüzden `cid`'den.
pub(crate) async fn anahtar_sutunlari(c: &Connection, tablo: &str) -> Sonuc<Vec<String>> {
    let mut r = c
        .query(&format!("PRAGMA table_info({})", kimlik(tablo)), ())
        .await?;
    let mut v = Vec::new();
    while let Some(s) = r.next().await? {
        if tamsayi(s.get_value(5)?) > 0 {
            v.push(yazi(s.get_value(1)?));
        }
    }
    Ok(v)
}

async fn metinler(c: &Connection, sql: &str) -> Sonuc<Vec<String>> {
    let mut r = c.query(sql, ()).await?;
    let mut v = Vec::new();
    while let Some(s) = r.next().await? {
        v.push(yazi(s.get_value(0)?));
    }
    Ok(v)
}

fn kimlik(ad: &str) -> String {
    format!("\"{}\"", ad.replace('"', "\"\""))
}

fn metin(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub(crate) fn tamsayi(v: Value) -> i64 {
    match v {
        Value::Integer(n) => n,
        _ => 0,
    }
}

fn yazi(v: Value) -> String {
    match v {
        Value::Text(s) => s,
        Value::Integer(n) => n.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
pub(crate) mod testler {
    use super::*;

    pub(crate) async fn gecici_db(ad: &str, kurulum: &[&str]) -> PathBuf {
        let yol =
            std::env::temp_dir().join(format!("fihrist-canli-{ad}-{}.db", std::process::id()));
        for ek in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ek}", yol.display()));
        }
        let _ = std::fs::remove_dir_all(imlec_dizini(&yol));
        let db = Builder::new_local(&yol.to_string_lossy())
            .build()
            .await
            .unwrap();
        let c = db.connect().unwrap();
        for s in kurulum {
            c.execute(s, ()).await.unwrap();
        }
        yol
    }

    pub(crate) async fn yaz(yol: &Path, sqller: &[&str]) {
        // Her yazış taze bağlantıdan: tetikleyicilerin dosyaya işlendiğini sınar.
        let c = ac(yol).await.unwrap();
        for s in sqller {
            c.execute(s, ()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn uc_islem_anahtariyla_yakalanir() {
        let yol = gecici_db(
            "uc",
            &[
                "CREATE TABLE k (id TEXT PRIMARY KEY, v INTEGER)",
                "CREATE TABLE b (a TEXT NOT NULL, n INTEGER NOT NULL, x TEXT, PRIMARY KEY (a, n))",
                "CREATE TABLE r (x TEXT)",
            ],
        )
        .await;
        let izlenen = kur(&ac(&yol).await.unwrap()).await.unwrap();
        assert_eq!(izlenen, ["b", "k", "r"]);

        yaz(
            &yol,
            &[
                "INSERT INTO k VALUES ('İğne', 1)",
                "UPDATE k SET v = 2 WHERE id = 'İğne'",
                "DELETE FROM k WHERE id = 'İğne'",
                "INSERT INTO b VALUES ('x', 7, 'y')",
                "INSERT INTO r VALUES ('z')",
            ],
        )
        .await;

        let v = oku(&ac(&yol).await.unwrap(), 0, 100).await.unwrap();
        let ozet: Vec<_> = v
            .iter()
            .map(|d| (d.tablo.as_str(), d.islem.as_str(), d.anahtar.as_str()))
            .collect();
        assert_eq!(
            ozet,
            [
                ("k", "e", r#"["İğne"]"#),
                ("k", "g", r#"["İğne"]"#),
                ("k", "s", r#"["İğne"]"#),
                ("b", "e", r#"["x",7]"#),
                ("r", "e", "[1]"),
            ]
        );
        assert!(v.windows(2).all(|w| w[0].no < w[1].no));
        assert!(
            v[0].zaman_ms > 1_700_000_000_000,
            "zaman ms cinsinden olmalı"
        );

        // İkinci kurulum zararsız ve kopya tetikleyici kurmaz.
        kur(&ac(&yol).await.unwrap()).await.unwrap();
        yaz(&yol, &["INSERT INTO r VALUES ('w')"]).await;
        assert_eq!(
            oku(&ac(&yol).await.unwrap(), v[4].no, 100)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn yeniden_baslayan_tuketici_kaldigi_yerden_surer() {
        let yol = gecici_db("imlec", &["CREATE TABLE k (id TEXT PRIMARY KEY)"]).await;
        kur(&ac(&yol).await.unwrap()).await.unwrap();
        yaz(&yol, &["INSERT INTO k VALUES ('a')"]).await;

        // İlk kez gelen tüketici geçmişi değil, bundan sonrasını izler.
        assert_eq!(imlec_oku(&yol, "t1").unwrap(), None);
        let bas = son_no(&ac(&yol).await.unwrap()).await.unwrap();
        imlec_yaz(&yol, "t1", bas).unwrap();

        yaz(
            &yol,
            &[
                "INSERT INTO k VALUES ('b')",
                "INSERT INTO k VALUES ('c')",
                "INSERT INTO k VALUES ('d')",
            ],
        )
        .await;
        let ilk = oku(&ac(&yol).await.unwrap(), bas, 2).await.unwrap();
        assert_eq!(ilk.len(), 2);
        imlec_yaz(&yol, "t1", ilk[1].no).unwrap();

        // "Çöküş": süreç durumu gider, yalnız dosyalar kalır. Arada yazış olur.
        yaz(&yol, &["INSERT INTO k VALUES ('e')"]).await;
        let imlec = imlec_oku(&yol, "t1").unwrap().unwrap();
        let kalan: Vec<_> = oku(&ac(&yol).await.unwrap(), imlec, 100)
            .await
            .unwrap()
            .into_iter()
            .map(|d| d.anahtar)
            .collect();
        assert_eq!(kalan, [r#"["d"]"#, r#"["e"]"#], "olay kaybı ya da tekrar");

        assert!(imlec_yaz(&yol, "../kacak", 1).is_err());
    }

    #[tokio::test]
    async fn budama_son_satiri_tutar_numara_geri_donmez() {
        let yol = gecici_db("buda", &["CREATE TABLE k (id TEXT PRIMARY KEY)"]).await;
        kur(&ac(&yol).await.unwrap()).await.unwrap();
        yaz(
            &yol,
            &[
                "INSERT INTO k VALUES ('a')",
                "INSERT INTO k VALUES ('b')",
                "INSERT INTO k VALUES ('c')",
            ],
        )
        .await;

        let c = ac(&yol).await.unwrap();
        let son = son_no(&c).await.unwrap();
        imlec_yaz(&yol, "yavas", son - 1).unwrap();
        imlec_yaz(&yol, "hizli", son).unwrap();
        // En yavaş tüketicinin henüz okumadığı satır silinmez.
        assert_eq!(buda(&c, &yol).await.unwrap(), 1);
        assert_eq!(oku(&c, son - 2, 100).await.unwrap().len(), 2);

        // Günlüğün sonunu geçmiş imleç (bozuk ya da başka dosyadan kalma)
        // son satırı yine de sildirmemeli.
        imlec_yaz(&yol, "yavas", son + 100).unwrap();
        imlec_yaz(&yol, "hizli", son + 100).unwrap();
        buda(&c, &yol).await.unwrap();
        assert_eq!(oku(&c, 0, 100).await.unwrap().len(), 1, "son satır kalmalı");

        yaz(&yol, &["INSERT INTO k VALUES ('d')"]).await;
        let yeni = oku(&ac(&yol).await.unwrap(), son, 100).await.unwrap();
        assert_eq!(yeni.len(), 1);
        assert_eq!(yeni[0].no, son + 1, "numara geri dönmemeli");
    }

    #[tokio::test]
    async fn olmayan_dosya_yaratilmaz() {
        let yol = std::env::temp_dir().join("fihrist-canli-olmayan.db");
        let _ = std::fs::remove_file(&yol);
        assert!(ac(&yol).await.is_err());
        assert!(!yol.exists());
    }
}
