//! MCP kaynakları (resources) ve aboneliği (ADR 0004).
//!
//! Kaynak, dört yan dosyadan birindeki bir tablonun **bildirim günlüğüdür**
//! (`fihrist_degisiklik`, ADR 0003). URI `fihrist://<dosya kökü>/<tablo>`;
//! dosya, ana katalogla aynı dizindeki `<kök>.db`. Kök beyaz listededir.
//! Ana katalog (`kutup_kutuphane`) açık hatayla reddedilir: fts5'li dosyada
//! Turso şemayı keser, günlük kurulamaz.
//!
//! Abone geçicidir (CONTEXT.md): abonelik oturum belleğinde durur, imleç
//! dosyası yazılmaz, budamayı etkilemez. Her yoklama dosyayı taze açar; açık
//! bağlantı başka süreçlerin commit'ini görmez (ADR 0003, bulgu 4). Şema
//! yalnız `sqlite_master`'dan okunur: `pragma_table_info(...)` aynı
//! bağlantıdaki sonraki yazışları kaybettirir (bulgu 5), burada hiç yok.

use fihrist_canli::GUNLUK;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;
use turso::{params, Connection};

/// İzlenebilen dosyaların kökleri (`<kök>.db`). Başka kök reddedilir.
pub const DOSYALAR: [&str; 4] = [
    "kutup_kurallar",
    "kutup_ortak",
    "kutup_depolar",
    "kutup_kayitlar",
];
/// Ana katalog: fts5 sanal tablosu var, bildirim günlüğü kurulamaz (ADR 0003).
pub const ANA_KATALOG: &str = "kutup_kutuphane";
pub const SEMA: &str = "fihrist://";
/// Yoklama aralığı; ADR 0003'ün kabul ölçümleri de 250 ms'yle yapıldı.
pub const ARALIK: Duration = Duration::from_millis(250);
/// `resources/read`'in döndürdüğü en çok günlük kaydı.
pub const OKUMA_SINIRI: i64 = 50;
const MIME: &str = "application/json";

#[derive(Debug, thiserror::Error)]
pub enum Hata {
    #[error("geçersiz kaynak {uri:?}: {neden}")]
    Gecersiz { uri: String, neden: String },
    #[error("{uri}: {neden}")]
    Izlenemez { uri: String, neden: String },
    #[error("{islem} ({yol}): {kaynak}")]
    Canli {
        islem: &'static str,
        yol: String,
        kaynak: fihrist_canli::Hata,
    },
    #[error("{islem} ({yol}): {kaynak}")]
    Turso {
        islem: &'static str,
        yol: String,
        kaynak: turso::Error,
    },
    #[error("{islem} ({yol}): {neden}")]
    Bozuk {
        islem: &'static str,
        yol: String,
        neden: String,
    },
    #[error("kütüphane yolu çözülemedi: {0}")]
    Katalog(String),
    #[error("bilinmeyen yöntem: {0}")]
    Yontem(String),
}

impl Hata {
    /// JSON-RPC hata kodu. -32002, MCP'nin "kaynak bulunamadı"sı.
    pub fn kod(&self) -> i64 {
        match self {
            Hata::Gecersiz { .. } => -32602,
            Hata::Izlenemez { .. } => -32002,
            Hata::Yontem(_) => -32601,
            _ => -32603,
        }
    }
}

fn canli_hata(islem: &'static str, yol: &Path) -> impl FnOnce(fihrist_canli::Hata) -> Hata {
    let yol = yol.display().to_string();
    move |kaynak| Hata::Canli { islem, yol, kaynak }
}

fn turso_hata(islem: &'static str, yol: &Path) -> impl Fn(turso::Error) -> Hata {
    let yol = yol.display().to_string();
    move |kaynak| Hata::Turso {
        islem,
        yol: yol.clone(),
        kaynak,
    }
}

/// Ayrıştırılmış ve doğrulanmış kaynak URI'si.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri {
    pub kok: &'static str,
    pub tablo: String,
}

/// Tablo adı URI'ye ve dosya yoluna kaçamasın: yalnız harf, rakam ve `_`.
/// `.`, `/`, `\`, `%`, tırnak ve boşluk bu yüzden dışarıda.
fn tablo_adi_gecerli(tablo: &str) -> bool {
    !tablo.is_empty() && tablo.chars().all(|k| k.is_alphanumeric() || k == '_')
}

impl Uri {
    pub fn ayristir(uri: &str) -> Result<Uri, Hata> {
        let gecersiz = |neden: String| Hata::Gecersiz {
            uri: uri.to_string(),
            neden,
        };
        let govde = uri
            .strip_prefix(SEMA)
            .ok_or_else(|| gecersiz(format!("şema {SEMA} değil")))?;
        let (kok, tablo) = govde
            .split_once('/')
            .ok_or_else(|| gecersiz(format!("biçim {SEMA}<dosya>/<tablo>")))?;
        if kok == ANA_KATALOG {
            return Err(gecersiz(format!(
                "ana katalog ({ANA_KATALOG}) izlenmez: fts5 sanal tablosu olan dosyada \
                 Turso şemayı keser, bildirim günlüğü kurulamaz (ADR 0003)"
            )));
        }
        let kok = DOSYALAR.iter().find(|d| **d == kok).ok_or_else(|| {
            gecersiz(format!(
                "dosya kökü {kok:?} listede değil ({})",
                DOSYALAR.join(", ")
            ))
        })?;
        if !tablo_adi_gecerli(tablo) {
            return Err(gecersiz(format!(
                "tablo adı {tablo:?} geçersiz: yalnız harf, rakam ve _"
            )));
        }
        Ok(Uri {
            kok,
            tablo: tablo.to_string(),
        })
    }

    pub fn metin(&self) -> String {
        format!("{SEMA}{}/{}", self.kok, self.tablo)
    }

    pub fn dosya(&self, dizin: &Path) -> PathBuf {
        dizin.join(format!("{}.db", self.kok))
    }
}

fn uri_al(p: &Value) -> Result<Uri, Hata> {
    match p.get("uri").and_then(Value::as_str) {
        Some(u) => Uri::ayristir(u),
        None => Err(Hata::Gecersiz {
            uri: String::new(),
            neden: "'uri' alanı gerekli (metin)".into(),
        }),
    }
}

/// Dört dosyanın dizini: ana kataloğun yanı.
fn dizin(katalog: Result<PathBuf, String>) -> Result<PathBuf, Hata> {
    let k = katalog.map_err(Hata::Katalog)?;
    Ok(k.parent().map(Path::to_path_buf).unwrap_or_default())
}

/// Bir dosyanın tabloları ve tetikleyicileri (`tbl_name`, `name`).
struct Sema {
    tablolar: BTreeSet<String>,
    tetikleyiciler: BTreeSet<(String, String)>,
}

impl Sema {
    /// Günlük kurulu ve tablonun üç tetikleyicisi (`fd_<t>_e/g/s`, fihrist-canli) yerinde mi.
    fn izleniyor(&self, tablo: &str) -> bool {
        self.tablolar.contains(GUNLUK)
            && ["e", "g", "s"].iter().all(|ek| {
                self.tetikleyiciler
                    .contains(&(tablo.to_string(), format!("fd_{tablo}_{ek}")))
            })
    }
}

async fn sema(c: &Connection, yol: &Path) -> Result<Sema, Hata> {
    let h = turso_hata("şema okuma", yol);
    let mut r = c
        .query(
            "SELECT type, name, tbl_name FROM sqlite_master WHERE type IN ('table', 'trigger')",
            (),
        )
        .await
        .map_err(&h)?;
    let mut s = Sema {
        tablolar: BTreeSet::new(),
        tetikleyiciler: BTreeSet::new(),
    };
    while let Some(satir) = r.next().await.map_err(&h)? {
        let tur = satir.get::<String>(0).map_err(&h)?;
        let ad = satir.get::<String>(1).map_err(&h)?;
        if tur == "table" {
            s.tablolar.insert(ad);
        } else {
            s.tetikleyiciler
                .insert((satir.get::<String>(2).map_err(&h)?, ad));
        }
    }
    Ok(s)
}

/// Dosyayı taze açar ve kaynağın izlenebildiğini doğrular: dosya var, günlük
/// kurulu, tablo ve üç tetikleyicisi var. Hiçbir eksik sessizce "değişiklik
/// yok"a dönmez.
async fn ac_dogrula(dizin: &Path, u: &Uri) -> Result<(PathBuf, Connection), Hata> {
    let yol = u.dosya(dizin);
    let izlenemez = |neden: String| Hata::Izlenemez {
        uri: u.metin(),
        neden,
    };
    if !yol.is_file() {
        return Err(izlenemez(format!("dosya yok: {}", yol.display())));
    }
    let c = fihrist_canli::ac(&yol)
        .await
        .map_err(canli_hata("açılış", &yol))?;
    let s = sema(&c, &yol).await?;
    let kurulum = format!("kurulum: fihrist-izle {} --kur", yol.display());
    if !s.tablolar.contains(GUNLUK) {
        return Err(izlenemez(format!(
            "{} içinde bildirim günlüğü ({GUNLUK}) kurulu değil; {kurulum}",
            yol.display()
        )));
    }
    if !s.tablolar.contains(&u.tablo) {
        return Err(izlenemez(format!(
            "{} içinde {} tablosu yok",
            yol.display(),
            u.tablo
        )));
    }
    if !s.izleniyor(&u.tablo) {
        return Err(izlenemez(format!(
            "{} tablosunun tetikleyicileri (fd_{}_e/g/s) yok; tablo kurulumdan sonra \
             eklenmiş olabilir, {kurulum}",
            u.tablo, u.tablo
        )));
    }
    Ok((yol, c))
}

/// Dört dosyadaki izlenen tablolar. Olmayan ya da günlüğü kurulmamış dosya
/// listeye girmez (okunamaz, abone olunamaz; abonelikte aynı durum açık hata
/// verir). Açılamayan dosya bütün listeyi hatayla düşürür, adıyla.
async fn listele(dizin: &Path) -> Result<Value, Hata> {
    let mut kaynaklar = Vec::new();
    for kok in DOSYALAR {
        let yol = dizin.join(format!("{kok}.db"));
        if !yol.is_file() {
            continue;
        }
        let c = fihrist_canli::ac(&yol)
            .await
            .map_err(canli_hata("liste açılışı", &yol))?;
        let s = sema(&c, &yol).await?;
        // URI'ye yazılamayan tablo adı (yol karakteri vb.) listelenmez: abone de olunamaz.
        for tablo in s.tablolar.iter().filter(|t| tablo_adi_gecerli(t)) {
            if s.izleniyor(tablo) {
                kaynaklar.push(json!({
                    "uri": format!("{SEMA}{kok}/{tablo}"),
                    "name": format!("{kok}/{tablo}"),
                    "description": format!(
                        "{kok}.db içindeki {tablo} tablosunun bildirim günlüğü: son \
                         {OKUMA_SINIRI} değişiklik (no, zaman_ms, islem e/g/s, anahtar)"
                    ),
                    "mimeType": MIME,
                }));
            }
        }
    }
    Ok(json!({ "resources": kaynaklar }))
}

/// Tablonun günlükteki son `OKUMA_SINIRI` kaydı, eskiden yeniye, JSON metni olarak.
async fn oku(c: &Connection, u: &Uri, yol: &Path) -> Result<Value, Hata> {
    let h = turso_hata("günlük okuma", yol);
    let mut r = c
        .query(
            &format!(
                "SELECT no, zaman_ms, islem, anahtar FROM {GUNLUK} \
                 WHERE tablo = ?1 ORDER BY no DESC LIMIT ?2"
            ),
            params![u.tablo.as_str(), OKUMA_SINIRI],
        )
        .await
        .map_err(&h)?;
    let mut kayitlar = Vec::new();
    while let Some(s) = r.next().await.map_err(&h)? {
        let no = s.get::<i64>(0).map_err(&h)?;
        let ham = s.get::<String>(3).map_err(&h)?;
        let anahtar: Value = serde_json::from_str(&ham).map_err(|e| Hata::Bozuk {
            islem: "günlük okuma",
            yol: yol.display().to_string(),
            neden: format!("no {no}: anahtar JSON değil ({ham:?}): {e}"),
        })?;
        kayitlar.push(json!({
            "no": no,
            "zaman_ms": s.get::<i64>(1).map_err(&h)?,
            "islem": s.get::<String>(2).map_err(&h)?,
            "anahtar": anahtar,
        }));
    }
    kayitlar.reverse();
    let govde = json!({
        "uri": u.metin(),
        "dosya": format!("{}.db", u.kok),
        "tablo": u.tablo,
        "sinir": OKUMA_SINIRI,
        "kayitlar": kayitlar,
    });
    Ok(json!({"contents": [{"uri": u.metin(), "mimeType": MIME, "text": govde.to_string()}]}))
}

/// Günlüğün en küçük ve en büyük numarası; boşsa `(None, 0)`.
async fn sinirlar(c: &Connection, yol: &Path) -> Result<(Option<i64>, i64), Hata> {
    let h = turso_hata("günlük sınırları", yol);
    let mut r = c
        .query(
            &format!("SELECT min(no), coalesce(max(no), 0) FROM {GUNLUK}"),
            (),
        )
        .await
        .map_err(&h)?;
    match r.next().await.map_err(&h)? {
        Some(s) => Ok((
            s.get::<Option<i64>>(0).map_err(&h)?,
            s.get::<i64>(1).map_err(&h)?,
        )),
        None => Ok((None, 0)),
    }
}

/// `imlec`ten sonra bu tabloya düşmüş kayıt var mı.
async fn yeni_var(c: &Connection, yol: &Path, tablo: &str, imlec: i64) -> Result<bool, Hata> {
    let h = turso_hata("günlük yoklama", yol);
    let mut r = c
        .query(
            &format!("SELECT 1 FROM {GUNLUK} WHERE no > ?1 AND tablo = ?2 LIMIT 1"),
            params![imlec, tablo],
        )
        .await
        .map_err(&h)?;
    Ok(r.next().await.map_err(&h)?.is_some())
}

struct Abone {
    yol: PathBuf,
    tablo: String,
    /// Bu abonenin gördüğü son günlük numarası. Yalnız bellekte.
    imlec: i64,
}

/// Bir oturumun abonelikleri. Oturum düşünce iz bırakmaz.
pub struct Kaynaklar {
    /// URI → abone. Aynı URI'ye ikinci abonelik ilkini (imlecini) korur.
    aboneler: BTreeMap<String, Abone>,
    sonraki: Instant,
    yoklar: bool,
}

impl Kaynaklar {
    /// Yoklayan oturum (`konus` döngüsü).
    pub fn yeni() -> Self {
        Self {
            aboneler: BTreeMap::new(),
            sonraki: Instant::now(),
            yoklar: true,
        }
    }

    /// Yoklamayan yol (aktarıcının süreç içi yedeği). Abonelik orada açık
    /// hatayla reddedilir: kabul edilse bildirim hiç gitmezdi.
    pub fn yoklamasiz() -> Self {
        Self {
            yoklar: false,
            ..Self::yeni()
        }
    }

    pub fn abone_var(&self) -> bool {
        !self.aboneler.is_empty()
    }

    /// Sıradaki yoklama anına kadar uyur. İptale dayanıklı: an `yokla`da
    /// ilerler, `select!` bu geleceği düşürse de kayma olmaz.
    pub async fn bekle(&self) {
        tokio::time::sleep_until(self.sonraki).await
    }

    /// `resources/*` isteğini yanıtlar. `katalog`, ana kataloğun çözülmüş yolu
    /// (`kutuphane_yolu`); dört dosya onun dizininde aranır.
    pub async fn ele_al(
        &mut self,
        katalog: Result<PathBuf, String>,
        yontem: &str,
        p: &Value,
    ) -> Result<Value, Hata> {
        match yontem {
            "resources/list" => listele(&dizin(katalog)?).await,
            "resources/read" => {
                let u = uri_al(p)?;
                let (yol, c) = ac_dogrula(&dizin(katalog)?, &u).await?;
                oku(&c, &u, &yol).await
            }
            "resources/subscribe" => {
                let u = uri_al(p)?;
                if !self.yoklar {
                    return Err(Hata::Izlenemez {
                        uri: u.metin(),
                        neden: "hizmet koptu, süreç içi yedek yoklamaz: abonelik \
                                bildirimi verilemez; oturumu yeniden başlatın"
                            .into(),
                    });
                }
                if self.aboneler.contains_key(&u.metin()) {
                    return Ok(json!({}));
                }
                let (yol, c) = ac_dogrula(&dizin(katalog)?, &u).await?;
                // Abone geçmişi değil, bundan sonrasını izler.
                let imlec = fihrist_canli::son_no(&c)
                    .await
                    .map_err(canli_hata("son numara", &yol))?;
                self.aboneler.insert(
                    u.metin(),
                    Abone {
                        yol,
                        tablo: u.tablo,
                        imlec,
                    },
                );
                Ok(json!({}))
            }
            "resources/unsubscribe" => {
                let u = uri_al(p)?;
                match self.aboneler.remove(&u.metin()) {
                    Some(_) => Ok(json!({})),
                    None => Err(Hata::Gecersiz {
                        uri: u.metin(),
                        neden: "bu oturumda abonelik yok".into(),
                    }),
                }
            }
            _ => Err(Hata::Yontem(yontem.to_string())),
        }
    }

    /// Bütün abonelikleri yoklar; değişen her URI için TEK
    /// `notifications/resources/updated` iletisi döner. Bir dosyanın hatası
    /// (kilitli, silinmiş) öbürlerini durdurmaz ve stderr'e yazılır (stdout
    /// protokole aittir); o dosyanın imleçleri ilerlemez, sonraki başarılı
    /// yoklama aradakini yakalar.
    pub async fn yokla(&mut self) -> Vec<Value> {
        self.sonraki = Instant::now() + ARALIK;
        let mut dosyalar: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
        for (uri, a) in &self.aboneler {
            dosyalar.entry(a.yol.clone()).or_default().push(uri.clone());
        }
        let mut degisen = Vec::new();
        for (yol, uriler) in dosyalar {
            match self.dosya_yokla(&yol, &uriler).await {
                Ok(v) => degisen.extend(v),
                Err(e) => eprintln!("! kaynak yoklaması: {e}"),
            }
        }
        degisen
            .into_iter()
            .map(|uri| {
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/resources/updated",
                    "params": {"uri": uri}
                })
            })
            .collect()
    }

    async fn dosya_yokla(&mut self, yol: &Path, uriler: &[String]) -> Result<Vec<String>, Hata> {
        let c = fihrist_canli::ac(yol)
            .await
            .map_err(canli_hata("yoklama açılışı", yol))?;
        let (en_kucuk, en_buyuk) = sinirlar(&c, yol).await?;
        let imlecler: Vec<(&String, String, i64)> = uriler
            .iter()
            .filter_map(|u| self.aboneler.get(u).map(|a| (u, a.tablo.clone(), a.imlec)))
            .collect();
        let alt = imlecler
            .iter()
            .map(|(_, _, i)| *i)
            .min()
            .unwrap_or(en_buyuk);
        let ust = imlecler
            .iter()
            .map(|(_, _, i)| *i)
            .max()
            .unwrap_or(en_buyuk);
        // Budama boşluğu: imleçten sonraki satırlar silinmiş, hangi tablonun
        // değiştiği bilinemez; dosyadaki her abone koşulsuz bildirilir.
        // Numara geri dönmüşse (dosya değişmiş) de aynı.
        let bosluk = en_kucuk.is_some_and(|k| k > alt + 1) || en_buyuk < ust;
        let mut degisen = Vec::new();
        for (uri, tablo, imlec) in &imlecler {
            if bosluk || yeni_var(&c, yol, tablo, *imlec).await? {
                degisen.push((*uri).clone());
            }
        }
        // İmleçler ancak bütün sorgular bitince ilerler: yarım yoklama olay kaçırmaz.
        for u in uriler {
            if let Some(a) = self.aboneler.get_mut(u) {
                a.imlec = en_buyuk;
            }
        }
        Ok(degisen)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Her sınamaya ayrı dizin; içinde istenen dosyalar `kur` ile izlenir.
    async fn dizin_kur(ad: &str, dosyalar: &[(&str, &[&str], bool)]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fihrist-kaynak-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for (kok, sqller, kurulsun) in dosyalar {
            let yol = d.join(format!("{kok}.db"));
            let db = turso::Builder::new_local(&yol.to_string_lossy())
                .build()
                .await
                .unwrap();
            let c = db.connect().unwrap();
            for s in *sqller {
                c.execute(s, ()).await.unwrap();
            }
            if *kurulsun {
                fihrist_canli::kur(&c).await.unwrap();
            }
        }
        d
    }

    async fn yaz(d: &Path, kok: &str, sqller: &[&str]) {
        // Her yazış taze bağlantıdan: başka bir yazanın commit'i gibi.
        let c = fihrist_canli::ac(&d.join(format!("{kok}.db")))
            .await
            .unwrap();
        for s in sqller {
            c.execute(s, ()).await.unwrap();
        }
    }

    fn katalog(d: &Path) -> Result<PathBuf, String> {
        Ok(d.join("kutup_kutuphane.db"))
    }

    async fn abone(k: &mut Kaynaklar, d: &Path, uri: &str) -> Result<Value, Hata> {
        k.ele_al(katalog(d), "resources/subscribe", &json!({ "uri": uri }))
            .await
    }

    fn uriler(bildirimler: &[Value]) -> Vec<String> {
        bildirimler
            .iter()
            .map(|b| {
                assert_eq!(b["method"], "notifications/resources/updated");
                b["params"]["uri"].as_str().unwrap().to_string()
            })
            .collect()
    }

    const KAYIT: &str = "CREATE TABLE arac_cagrilari (kimlik TEXT PRIMARY KEY, arac TEXT)";
    const DIGER: &str = "CREATE TABLE diger (n INTEGER PRIMARY KEY)";

    #[test]
    fn uri_kabul_ve_ret() {
        let u = Uri::ayristir("fihrist://kutup_kayitlar/arac_cagrilari").unwrap();
        assert_eq!(
            (u.kok, u.tablo.as_str()),
            ("kutup_kayitlar", "arac_cagrilari")
        );
        assert_eq!(u.metin(), "fihrist://kutup_kayitlar/arac_cagrilari");
        assert_eq!(u.dosya(Path::new("/k")), Path::new("/k/kutup_kayitlar.db"));
        for kok in DOSYALAR {
            assert!(
                Uri::ayristir(&format!("fihrist://{kok}/t")).is_ok(),
                "{kok}"
            );
        }
        assert!(Uri::ayristir("fihrist://kutup_kurallar/çağrı_1").is_ok());

        let ana = Uri::ayristir("fihrist://kutup_kutuphane/yetenekler").unwrap_err();
        assert!(ana.to_string().contains("ana katalog"), "{ana}");
        assert_eq!(ana.kod(), -32602);

        for kotu in [
            "fihrist://kutup_baska/t",
            "fihrist://../kutup_kayitlar/t",
            "fihrist://kutup_kayitlar.db/t",
            "fihrist://KUTUP_KAYITLAR/t",
            "fihrist://kutup_kayitlar/../kutup_kutuphane",
            "fihrist://kutup_kayitlar/..",
            "fihrist://kutup_kayitlar/a/b",
            "fihrist://kutup_kayitlar/a\\b",
            "fihrist://kutup_kayitlar/a%2Fb",
            "fihrist://kutup_kayitlar/a b",
            "fihrist://kutup_kayitlar/",
            "fihrist://kutup_kayitlar",
            "file:///kutup_kayitlar/t",
        ] {
            let h = Uri::ayristir(kotu).unwrap_err();
            assert_eq!(h.kod(), -32602, "{kotu}: {h}");
        }
    }

    #[tokio::test]
    async fn gunluksuz_dosya_tetikleyicisiz_tablo_ve_olmayan_dosya_reddedilir() {
        let d = dizin_kur(
            "ret",
            &[
                ("kutup_kayitlar", &[KAYIT], false),
                ("kutup_depolar", &[DIGER], true),
            ],
        )
        .await;
        // Kurulumdan sonra eklenen tablonun tetikleyicisi yok.
        yaz(&d, "kutup_depolar", &["CREATE TABLE sonradan (x TEXT)"]).await;
        let mut k = Kaynaklar::yeni();

        let h = abone(&mut k, &d, "fihrist://kutup_kayitlar/arac_cagrilari")
            .await
            .unwrap_err();
        assert!(h.to_string().contains("bildirim günlüğü"), "{h}");
        assert_eq!(h.kod(), -32002);

        let h = abone(&mut k, &d, "fihrist://kutup_depolar/sonradan")
            .await
            .unwrap_err();
        assert!(h.to_string().contains("tetikleyici"), "{h}");

        let h = abone(&mut k, &d, "fihrist://kutup_depolar/yok")
            .await
            .unwrap_err();
        assert!(h.to_string().contains("tablosu yok"), "{h}");

        let h = abone(&mut k, &d, "fihrist://kutup_ortak/t")
            .await
            .unwrap_err();
        assert!(h.to_string().contains("dosya yok"), "{h}");

        let h = abone(&mut k, &d, "fihrist://kutup_kutuphane/yetenekler")
            .await
            .unwrap_err();
        assert!(h.to_string().contains("ana katalog"), "{h}");

        // Okuma da aynı doğrulamadan geçer: günlüksüz dosya boş liste değil, hata.
        let h = k
            .ele_al(
                katalog(&d),
                "resources/read",
                &json!({"uri": "fihrist://kutup_kayitlar/arac_cagrilari"}),
            )
            .await
            .unwrap_err();
        assert_eq!(h.kod(), -32002, "{h}");
        assert!(!k.abone_var(), "reddedilen abonelik kalmamalı");

        // Liste yalnız izlenen tabloları gösterir.
        let l = k
            .ele_al(katalog(&d), "resources/list", &json!({}))
            .await
            .unwrap();
        let adlar: Vec<&str> = l["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["uri"].as_str().unwrap())
            .collect();
        assert_eq!(adlar, ["fihrist://kutup_depolar/diger"]);
        assert_eq!(l["resources"][0]["mimeType"], MIME);
    }

    #[tokio::test]
    async fn okuma_json_doner() {
        let d = dizin_kur("oku", &[("kutup_kayitlar", &[KAYIT, DIGER], true)]).await;
        yaz(
            &d,
            "kutup_kayitlar",
            &[
                "INSERT INTO arac_cagrilari VALUES ('İğne-1', 'search_skills')",
                "INSERT INTO diger VALUES (7)",
                "UPDATE arac_cagrilari SET arac = 'info' WHERE kimlik = 'İğne-1'",
                "DELETE FROM arac_cagrilari WHERE kimlik = 'İğne-1'",
            ],
        )
        .await;
        let mut k = Kaynaklar::yeni();
        let uri = "fihrist://kutup_kayitlar/arac_cagrilari";
        let y = k
            .ele_al(katalog(&d), "resources/read", &json!({ "uri": uri }))
            .await
            .unwrap();
        let ic = &y["contents"][0];
        assert_eq!(
            (ic["uri"].as_str(), ic["mimeType"].as_str()),
            (Some(uri), Some(MIME))
        );
        let g: Value = serde_json::from_str(ic["text"].as_str().unwrap()).unwrap();
        assert_eq!(g["tablo"], "arac_cagrilari");
        assert_eq!(g["sinir"], OKUMA_SINIRI);
        let ozet: Vec<(String, Value)> = g["kayitlar"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r["islem"].as_str().unwrap().to_string(),
                    r["anahtar"].clone(),
                )
            })
            .collect();
        assert_eq!(
            ozet,
            [
                ("e".to_string(), json!(["İğne-1"])),
                ("g".to_string(), json!(["İğne-1"])),
                ("s".to_string(), json!(["İğne-1"])),
            ],
            "yalnız bu tablo, eskiden yeniye"
        );
        let kayitlar = g["kayitlar"].as_array().unwrap();
        assert!(kayitlar[0]["no"].as_i64() < kayitlar[2]["no"].as_i64());
        assert!(kayitlar[0]["zaman_ms"].as_i64().unwrap() > 1_700_000_000_000);
    }

    #[tokio::test]
    async fn okuma_son_elli_kaydi_verir() {
        let d = dizin_kur("sinir", &[("kutup_kurallar", &[DIGER], true)]).await;
        let sqller: Vec<String> = (1..=60)
            .map(|i| format!("INSERT INTO diger VALUES ({i})"))
            .collect();
        let sqller: Vec<&str> = sqller.iter().map(String::as_str).collect();
        yaz(&d, "kutup_kurallar", &sqller).await;
        let y = Kaynaklar::yeni()
            .ele_al(
                katalog(&d),
                "resources/read",
                &json!({"uri": "fihrist://kutup_kurallar/diger"}),
            )
            .await
            .unwrap();
        let g: Value = serde_json::from_str(y["contents"][0]["text"].as_str().unwrap()).unwrap();
        let k = g["kayitlar"].as_array().unwrap();
        assert_eq!(k.len(), 50);
        assert_eq!(
            (k[0]["anahtar"].clone(), k[49]["anahtar"].clone()),
            (json!([11]), json!([60]))
        );
    }

    #[tokio::test]
    async fn yoklama_yeni_kayitta_uri_bos_yoklamada_hic_bildirim_vermez() {
        let d = dizin_kur("yokla", &[("kutup_kayitlar", &[KAYIT, DIGER], true)]).await;
        // Abonelikten önceki yazış bildirilmez: abone bundan sonrasını izler.
        yaz(
            &d,
            "kutup_kayitlar",
            &["INSERT INTO arac_cagrilari VALUES ('once', 'x')"],
        )
        .await;
        let mut k = Kaynaklar::yeni();
        let uri = "fihrist://kutup_kayitlar/arac_cagrilari";
        abone(&mut k, &d, uri).await.unwrap();
        assert!(k.abone_var());
        assert!(k.yokla().await.is_empty(), "boş yoklamada bildirim olmaz");

        // Aynı yoklamaya düşen iki yazış: tek bildirim.
        yaz(
            &d,
            "kutup_kayitlar",
            &[
                "INSERT INTO arac_cagrilari VALUES ('a', 'x')",
                "INSERT INTO arac_cagrilari VALUES ('b', 'x')",
            ],
        )
        .await;
        assert_eq!(uriler(&k.yokla().await), [uri]);
        assert!(
            k.yokla().await.is_empty(),
            "aynı değişiklik ikinci kez bildirilmez"
        );

        // Abone olunmayan tablonun yazışı bildirilmez.
        yaz(&d, "kutup_kayitlar", &["INSERT INTO diger VALUES (1)"]).await;
        assert!(k.yokla().await.is_empty());

        // Abonelikten çıkınca yazış bildirilmez; ikinci çıkış açık hata.
        k.ele_al(katalog(&d), "resources/unsubscribe", &json!({ "uri": uri }))
            .await
            .unwrap();
        assert!(!k.abone_var());
        yaz(
            &d,
            "kutup_kayitlar",
            &["DELETE FROM arac_cagrilari WHERE kimlik = 'a'"],
        )
        .await;
        assert!(k.yokla().await.is_empty());
        let h = k
            .ele_al(katalog(&d), "resources/unsubscribe", &json!({ "uri": uri }))
            .await
            .unwrap_err();
        assert!(h.to_string().contains("abonelik yok"), "{h}");
    }

    #[tokio::test]
    async fn budama_boslugunda_dosyadaki_her_aboneye_kosulsuz_bildirim() {
        let d = dizin_kur(
            "bosluk",
            &[(
                "kutup_kayitlar",
                &[KAYIT, DIGER, "CREATE TABLE ucuncu (x TEXT)"],
                true,
            )],
        )
        .await;
        yaz(&d, "kutup_kayitlar", &["INSERT INTO ucuncu VALUES ('ilk')"]).await;
        let mut k = Kaynaklar::yeni();
        let a = "fihrist://kutup_kayitlar/arac_cagrilari";
        let b = "fihrist://kutup_kayitlar/diger";
        abone(&mut k, &d, a).await.unwrap();
        abone(&mut k, &d, b).await.unwrap();
        assert!(k.yokla().await.is_empty());

        // arac_cagrilari değişir, sonra başka tablo; budama (kalıcı tüketici
        // yok) son satır dışındakileri siler: arac_cagrilari'nin satırı gider.
        yaz(
            &d,
            "kutup_kayitlar",
            &[
                "INSERT INTO arac_cagrilari VALUES ('kayip', 'x')",
                "INSERT INTO ucuncu VALUES ('son')",
            ],
        )
        .await;
        let yol = d.join("kutup_kayitlar.db");
        let c = fihrist_canli::ac(&yol).await.unwrap();
        assert!(fihrist_canli::buda(&c, &yol).await.unwrap() >= 2);
        drop(c);

        // Günlükte artık arac_cagrilari satırı yok; yine de iki abone de bildirilir.
        let mut bildirilen = uriler(&k.yokla().await);
        bildirilen.sort();
        assert_eq!(bildirilen, [a, b]);
        assert!(k.yokla().await.is_empty(), "boşluk bir kez bildirilir");
    }

    #[tokio::test]
    async fn yoklamasiz_yol_aboneligi_reddeder() {
        let d = dizin_kur("yoklamasiz", &[("kutup_kayitlar", &[KAYIT], true)]).await;
        let mut k = Kaynaklar::yoklamasiz();
        let h = abone(&mut k, &d, "fihrist://kutup_kayitlar/arac_cagrilari")
            .await
            .unwrap_err();
        assert!(h.to_string().contains("yoklamaz"), "{h}");
        assert!(!k.abone_var());
        // Okuma ve liste yoklama istemez, çalışır.
        assert!(k
            .ele_al(katalog(&d), "resources/list", &json!({}))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn bilinmeyen_yontem_ve_eksik_uri() {
        let mut k = Kaynaklar::yeni();
        let d = std::env::temp_dir();
        let h = k
            .ele_al(katalog(&d), "resources/templates/list", &json!({}))
            .await
            .unwrap_err();
        assert_eq!(h.kod(), -32601);
        let h = k
            .ele_al(katalog(&d), "resources/read", &json!({}))
            .await
            .unwrap_err();
        assert_eq!(h.kod(), -32602);
        let h = k
            .ele_al(Err("bulunamadı".into()), "resources/list", &json!({}))
            .await
            .unwrap_err();
        assert!(h.to_string().contains("kütüphane yolu"), "{h}");
    }
}
