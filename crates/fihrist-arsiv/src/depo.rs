//! Arşiv veritabanı (Turso, ayrı dosya `kutup_arsiv.db`) ve artımlı içe aktarma.
//!
//! fts5 YOK (ADR 0005/0006: Turso fts5'li dosyayı kesiyor ya da açmıyor); arama `ara.rs`'te bellekte BM25.
//! Artımlılık iki katman: dökümün işlenen bayt konumu (`dokumler`) ve `(uuid, blok)` tekilliği. Bir oturum
//! devam ettirilince Claude Code eski satırları yeni dosyaya kopyalıyor (ölçüldü: satırların ~%18'i); tekillik
//! onları da ikinci kez almaz. Konum kayıtla AYNI işlemde ilerler: yarım kalan koşu ne kayıp ne yineleme bırakır.

use crate::dokum::ayristir;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use turso::transaction::TransactionBehavior;
use turso::{params_from_iter, BatchStatement, Builder, Connection, Database, Value};

/// Şema sürümü; daha yenisi açılmaz (sessiz boş sonuç yok).
const SURUM: i64 = 1;
/// İşlem başına en çok bu kadar kayıt (büyük döküm tek işlemde şişmesin).
const PARCA: usize = 400;

const SEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (anahtar TEXT PRIMARY KEY, deger TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS dokumler (
    yol TEXT PRIMARY KEY, bayt INTEGER NOT NULL, satir INTEGER NOT NULL, guncelleme_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS kayitlar (
    id INTEGER PRIMARY KEY, uuid TEXT NOT NULL, blok INTEGER NOT NULL, oturum TEXT NOT NULL,
    tur TEXT NOT NULL, arac TEXT, yan INTEGER NOT NULL, zaman TEXT NOT NULL, zaman_ms INTEGER,
    metin TEXT NOT NULL, dokum TEXT NOT NULL, satir INTEGER NOT NULL,
    UNIQUE (uuid, blok)
);
CREATE INDEX IF NOT EXISTS kayitlar_oturum ON kayitlar (oturum, zaman_ms);
";

/// Bir içe aktarmanın sayıları.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ozet {
    /// Arşive yeni giren kayıt.
    pub yeni: usize,
    /// Okunan döküm satırı.
    pub satir: usize,
    /// JSON olmayan satır (sayılır, atlanır).
    pub bozuk: usize,
    /// Döküm baştan okundu (dosya küçülmüş: yeniden yazılmış).
    pub bastan: bool,
}

pub struct Depo {
    pub(crate) conn: Connection,
    _db: Database,
}

pub(crate) fn hata(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub(crate) fn v(d: impl Into<Value>) -> Value {
    d.into()
}

impl Depo {
    /// Dosyayı açar (yoksa kurar). Turso dosyayı süreç ömrünce kilitler; kancalar üst üste binerse kısa süre bekler.
    pub async fn ac(yol: &Path) -> Result<Self, String> {
        let mut son = String::new();
        for deneme in 0..8 {
            if deneme > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            }
            match Self::ac_bir(yol).await {
                Ok(d) => return Ok(d),
                Err(e) if e.contains("şema sürümü") => return Err(e),
                Err(e) => son = e,
            }
        }
        Err(format!("arşiv açılamadı ({}): {son}", yol.display()))
    }

    async fn ac_bir(yol: &Path) -> Result<Self, String> {
        if let Some(d) = yol.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d).map_err(hata)?;
        }
        let db = Builder::new_local(yol.to_string_lossy().as_ref())
            .build()
            .await
            .map_err(hata)?;
        let conn = db.connect().map_err(hata)?;
        conn.execute_batch(SEMA).await.map_err(hata)?;
        let mut r = conn
            .query("SELECT deger FROM meta WHERE anahtar = 'surum'", ())
            .await
            .map_err(hata)?;
        match r.next().await.map_err(hata)? {
            None => {
                conn.execute(
                    "INSERT INTO meta (anahtar, deger) VALUES ('surum', ?1)",
                    [SURUM.to_string()],
                )
                .await
                .map_err(hata)?;
            }
            Some(s) => {
                let d: String = s.get(0).map_err(hata)?;
                if d.parse::<i64>().ok() != Some(SURUM) {
                    return Err(format!("arşiv şema sürümü {d}, bu yapı {SURUM} bekliyor"));
                }
            }
        }
        drop(r);
        Ok(Self { conn, _db: db })
    }

    async fn konum(&self, yol: &str) -> Result<(i64, i64), String> {
        let mut r = self
            .conn
            .query(
                "SELECT bayt, satir FROM dokumler WHERE yol = ?1",
                [yol.to_string()],
            )
            .await
            .map_err(hata)?;
        match r.next().await.map_err(hata)? {
            Some(s) => Ok((s.get(0).map_err(hata)?, s.get(1).map_err(hata)?)),
            None => Ok((0, 0)),
        }
    }

    /// Dökümü kaldığı yerden okur ve kayıtları arşive ekler. Yarım yazılmış son satır (Claude Code
    /// dökümü sürerken yazıyor) sonraki koşuya kalır.
    pub async fn ice_aktar(&self, dokum: &Path) -> Result<Ozet, String> {
        let yol = dokum.to_string_lossy().replace('\\', "/");
        let (mut bayt, mut satir_sayisi) = self.konum(&yol).await?;
        let mut f = std::fs::File::open(dokum).map_err(|e| format!("{}: {e}", dokum.display()))?;
        let boy = f.metadata().map_err(hata)?.len() as i64;
        let mut ozet = Ozet::default();
        if boy < bayt {
            (bayt, satir_sayisi, ozet.bastan) = (0, 0, true); // yeniden yazılmış; tekillik yinelemeyi önler
        }
        f.seek(SeekFrom::Start(bayt as u64)).map_err(hata)?;
        let mut tampon = Vec::new();
        f.read_to_end(&mut tampon).map_err(hata)?;
        let Some(son_yeni_satir) = tampon.iter().rposition(|&b| b == b'\n') else {
            return Ok(ozet);
        };
        let metin = String::from_utf8_lossy(&tampon[..=son_yeni_satir]);
        let mut grup: Vec<BatchStatement> = Vec::new();
        let mut grup_kayit = 0;
        let mut okunan = 0usize; // bu koşuda tüketilen bayt
        for satir in metin.split_inclusive('\n') {
            okunan += satir.len();
            satir_sayisi += 1;
            ozet.satir += 1;
            match ayristir(satir.trim_end()) {
                None => ozet.bozuk += 1,
                Some(kayitlar) => {
                    for k in kayitlar {
                        grup.push(
                            BatchStatement::new(
                                "INSERT OR IGNORE INTO kayitlar (uuid, blok, oturum, tur, arac, yan, zaman, zaman_ms, metin, dokum, satir) \
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                                params_from_iter([
                                    v(k.uuid),
                                    v(k.blok),
                                    v(k.oturum),
                                    v(k.tur),
                                    v(k.arac),
                                    v(k.yan as i64),
                                    v(k.zaman),
                                    v(k.zaman_ms),
                                    v(k.metin),
                                    v(yol.clone()),
                                    v(satir_sayisi),
                                ]),
                            )
                            .map_err(hata)?,
                        );
                        grup_kayit += 1;
                    }
                }
            }
            if grup_kayit >= PARCA {
                ozet.yeni += self
                    .yaz(&mut grup, &yol, bayt + okunan as i64, satir_sayisi)
                    .await?;
                grup_kayit = 0;
            }
        }
        ozet.yeni += self
            .yaz(&mut grup, &yol, bayt + okunan as i64, satir_sayisi)
            .await?;
        Ok(ozet)
    }

    /// Kayıtları ve ilerleyen konumu tek işlemde yazar; yeni giren kayıt sayısını döner.
    async fn yaz(
        &self,
        grup: &mut Vec<BatchStatement>,
        yol: &str,
        bayt: i64,
        satir: i64,
    ) -> Result<usize, String> {
        let kayit_sayisi = grup.len();
        grup.push(
            BatchStatement::new(
                "INSERT OR REPLACE INTO dokumler (yol, bayt, satir, guncelleme_ms) VALUES (?1, ?2, ?3, ?4)",
                params_from_iter([v(yol), v(bayt), v(satir), v(simdi_ms())]),
            )
            .map_err(hata)?,
        );
        let sonuc = self
            .conn
            .transactional_batch(std::mem::take(grup), TransactionBehavior::Immediate)
            .await
            .map_err(hata)?;
        Ok(sonuc
            .iter()
            .take(kayit_sayisi)
            .map(|r| r.rows_affected() as usize)
            .sum())
    }

    /// Dökümlerden hangileri işlenecek: boyutu kayıtlı konumdan farklı olanlar (yeni, büyümüş ya da küçülmüş).
    /// Tek sorguyla bütün konumlar okunur; değişmeyen dökümler dosya açılmadan, veritabanına sorulmadan atlanır
    /// (SessionStart kancası binlerce dökümde saniyeler yemesin).
    pub async fn degisenler(
        &self,
        dokumler: &[std::path::PathBuf],
    ) -> Result<Vec<std::path::PathBuf>, String> {
        let mut r = self
            .conn
            .query("SELECT yol, bayt FROM dokumler", ())
            .await
            .map_err(hata)?;
        let mut konum = std::collections::HashMap::new();
        while let Some(s) = r.next().await.map_err(hata)? {
            konum.insert(
                s.get::<String>(0).map_err(hata)?,
                s.get::<i64>(1).map_err(hata)?,
            );
        }
        Ok(dokumler
            .iter()
            .filter(|d| {
                let yol = d.to_string_lossy().replace('\\', "/");
                // Boyutu okunamayan döküm de işlenir: hata `ice_aktar`ta açıkça çıksın.
                std::fs::metadata(d)
                    .map_or(true, |m| konum.get(&yol).copied() != Some(m.len() as i64))
            })
            .cloned()
            .collect())
    }

    /// Türe göre kayıt sayıları.
    pub async fn sayim(&self) -> Result<Vec<(String, i64)>, String> {
        let mut r = self
            .conn
            .query(
                "SELECT tur, count(*) FROM kayitlar GROUP BY tur ORDER BY tur",
                (),
            )
            .await
            .map_err(hata)?;
        let mut cikti = Vec::new();
        while let Some(s) = r.next().await.map_err(hata)? {
            cikti.push((s.get(0).map_err(hata)?, s.get(1).map_err(hata)?));
        }
        Ok(cikti)
    }
}

fn simdi_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}
