//! Canlı sorgu: abone olunan sorgu, izlenen tablolardan biri değişince yeniden
//! koşulur; aboneye yalnız fark (eklenen, değişen, silinen) gider.
//!
//! Fark durum tabanlıdır. Günlük (`fihrist_degisiklik`) yalnız "abone olunan
//! tablolardan biri değişti" işaretidir; neyin değiştiğini yeni sonucun
//! öncekiyle karşılaştırılması söyler. Günlükteki anahtar okunmaz: birleşimden
//! (JOIN) ya da süzgeçten geçen sonuç da doğru farklanır.
//!
//! Abone geçicidir: bellekte yaşar, imleç dosyası yazmaz, [`crate::buda`]yı
//! etkilemez. Başka bir tüketici abonenin imlecinden sonrasını budamışsa
//! (günlüğün en küçük numarası imleç + 1'den büyük) değişiklik kaçmış olabilir;
//! o zaman sorgu koşulsuz yeniden koşulur. Sessiz kayıp yok.
//!
//! Fark anahtarı açıktır, yoksa abonelik reddedilir: anahtar sütunu sonuçta
//! yoksa, birden çok tablo verilip anahtar verilmemişse, sonuçta aynı anahtar
//! iki kez geçiyorsa. Sessiz birleştirme yapılmaz.

use crate::{ac, anahtar_sutunlari, son_no, tamsayi, Hata, Sonuc, GUNLUK};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use turso::{Connection, Value};

/// Sonuç kümesinin bir satırı, sorgudaki sütun sırasıyla.
pub type Satir = Vec<Value>;

/// Abone olunacak sorgu.
#[derive(Debug, Clone)]
pub struct CanliSorgu {
    /// Koşulacak, yalnız okuyan sorgu (`SELECT`, `WITH` ya da `VALUES`);
    /// yazan deyim reddedilir, bkz. `kos`.
    pub sorgu: String,
    /// İzlenen tablolar. Sorgudan çıkarılmaz, açıkça verilir.
    pub tablolar: Vec<String>,
    /// Fark anahtarının sütunları (sonuçtaki adlarıyla). `None` ise tek
    /// tablonun birincil anahtarı, o da yoksa `rowid` kullanılır.
    pub anahtar: Option<Vec<String>>,
}

/// İki sonuç kümesi arasındaki fark.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fark {
    pub eklenen: Vec<Satir>,
    /// `(önceki, yeni)`: aynı anahtar, farklı satır.
    pub degisen: Vec<(Satir, Satir)>,
    pub silinen: Vec<Satir>,
}

impl Fark {
    pub fn bos_mu(&self) -> bool {
        self.eklenen.is_empty() && self.degisen.is_empty() && self.silinen.is_empty()
    }
}

/// Bellekte yaşayan canlı sorgu. Her [`Abone::yokla`] veritabanını taze açar.
pub struct Abone {
    db: PathBuf,
    tanim: CanliSorgu,
    sutunlar: Vec<String>,
    /// Anahtar sütunlarının sonuçtaki yerleri.
    anahtar: Vec<usize>,
    /// Görülen son günlük numarası. Yalnız bellekte; budamayı etkilemez.
    imlec: i64,
    /// Son sonuç, sorgunun sırasıyla: (anahtar kodu, satır).
    sonuc: Vec<(String, Satir)>,
}

impl Abone {
    /// Aboneliği kurar ve sorgunun tam sonucunu ilk görüntü olarak alır
    /// ([`Abone::goruntu`]). Anahtar belirsizse reddeder.
    pub async fn baslat(db: &Path, tanim: CanliSorgu) -> Sonuc<Abone> {
        if tanim.tablolar.is_empty() {
            return Err(Hata::Sorgu("izlenecek tablo verilmedi".into()));
        }
        let c = ac(db)
            .await
            .map_err(baglam(format!("açılış {}", db.display())))?;
        for t in &tanim.tablolar {
            tetikleyici_denetle(&c, t).await?;
        }
        let adlar = match (&tanim.anahtar, tanim.tablolar.as_slice()) {
            (Some(a), _) if a.is_empty() => {
                return Err(Hata::Sorgu("fark anahtarı boş verildi".into()))
            }
            (Some(a), _) => a.clone(),
            (None, [t]) => {
                let pk = anahtar_sutunlari(&c, t)
                    .await
                    .map_err(baglam(format!("birincil anahtar ({t})")))?;
                if pk.is_empty() {
                    vec!["rowid".to_string()]
                } else {
                    pk
                }
            }
            (None, _) => {
                return Err(Hata::Sorgu(format!(
                    "birden çok tablo ({}) verildi ama fark anahtarı verilmedi",
                    tanim.tablolar.join(", ")
                )))
            }
        };
        // İmleç sorgudan ÖNCE alınır: arada gelen yazış sonraki yoklamada
        // sorguyu yeniden koşturur (fark boşsa bildirilmez), kaçmaz.
        let imlec = son_no(&c).await.map_err(baglam("günlük son numarası"))?;
        let (sutunlar, satirlar) = kos(&c, &tanim.sorgu).await?;
        let anahtar = adlar
            .iter()
            .map(|a| sutun_yeri(&sutunlar, a))
            .collect::<Sonuc<Vec<_>>>()?;
        let sonuc = anahtarla(satirlar, &anahtar)?;
        Ok(Abone {
            db: db.to_path_buf(),
            tanim,
            sutunlar,
            anahtar,
            imlec,
            sonuc,
        })
    }

    pub fn sutunlar(&self) -> &[String] {
        &self.sutunlar
    }

    /// Bilinen son sonuç; [`Abone::baslat`]tan hemen sonra ilk görüntü.
    pub fn goruntu(&self) -> impl Iterator<Item = &Satir> {
        self.sonuc.iter().map(|(_, s)| s)
    }

    /// Bir kez yoklar. İzlenen tablolarda değişiklik yoksa ya da yeni sonuç
    /// öncekiyle aynıysa `None`. Hata olursa ne sonuç ne imleç ilerler:
    /// sonraki yoklama aynı aralığı yeniden dener.
    pub async fn yokla(&mut self) -> Sonuc<Option<Fark>> {
        let c = ac(&self.db)
            .await
            .map_err(baglam(format!("açılış {}", self.db.display())))?;
        let (en_kucuk, en_buyuk, ilgili) = self.gunluk_durumu(&c).await?;
        // Budanmış aralık: imleçten sonraki satırlar silinmiş, değişiklik
        // kaçmış olabilir. Numara geri gitmişse (günlük yeniden kurulmuş) de.
        let bosluk = en_kucuk.is_some_and(|n| n > self.imlec + 1) || en_buyuk < self.imlec;
        if !bosluk && !ilgili {
            self.imlec = en_buyuk;
            return Ok(None);
        }
        let (sutunlar, satirlar) = kos(&c, &self.tanim.sorgu).await?;
        if sutunlar != self.sutunlar {
            return Err(Hata::Sorgu(format!(
                "sorgunun sütunları değişti ({} → {}); yeniden abone olunmalı",
                self.sutunlar.join(", "),
                sutunlar.join(", ")
            )));
        }
        let yeni = anahtarla(satirlar, &self.anahtar)?;
        let f = fark(&self.sonuc, &yeni);
        self.sonuc = yeni;
        self.imlec = en_buyuk;
        Ok((!f.bos_mu()).then_some(f))
    }

    /// (en küçük numara, en büyük numara ya da 0, imleçten sonra izlenen
    /// tablolara satır var mı). Tek deyim, tek anlık görüntü: ayrı sorgular
    /// arasına giren bir budama boşluğu da ilgili satırı da gizleyebilirdi.
    async fn gunluk_durumu(&self, c: &Connection) -> Sonuc<(Option<i64>, i64, bool)> {
        let yerler: Vec<String> = (0..self.tanim.tablolar.len())
            .map(|i| format!("?{}", i + 2))
            .collect();
        let sql = format!(
            "SELECT (SELECT min(no) FROM {GUNLUK}), (SELECT coalesce(max(no), 0) FROM {GUNLUK}), \
             EXISTS (SELECT 1 FROM {GUNLUK} WHERE no > ?1 AND tablo IN ({}))",
            yerler.join(", ")
        );
        let mut p = vec![Value::Integer(self.imlec)];
        p.extend(self.tanim.tablolar.iter().cloned().map(Value::Text));
        let mut r = c.query(&sql, p).await.map_err(baglam("günlük durumu"))?;
        let s = r
            .next()
            .await
            .map_err(baglam("günlük durumu"))?
            .ok_or_else(|| Hata::Sorgu("günlük durumu satır döndürmedi".into()))?;
        let en_kucuk = match s.get_value(0).map_err(baglam("günlük durumu"))? {
            Value::Integer(n) => Some(n),
            _ => None,
        };
        Ok((
            en_kucuk,
            tamsayi(s.get_value(1).map_err(baglam("günlük durumu"))?),
            tamsayi(s.get_value(2).map_err(baglam("günlük durumu"))?) != 0,
        ))
    }
}

/// Arka planda yoklayan abonelik. Düşürülünce yoklama görevi iptal edilir;
/// aynı sorguyu izleyen öteki abonelikler etkilenmez (durum paylaşılmaz).
pub struct Abonelik {
    sutunlar: Vec<String>,
    ilk: Vec<Satir>,
    alici: mpsc::Receiver<Sonuc<Fark>>,
    gorev: JoinHandle<()>,
}

impl Abonelik {
    pub fn sutunlar(&self) -> &[String] {
        &self.sutunlar
    }

    pub fn ilk_goruntu(&self) -> &[Satir] {
        &self.ilk
    }

    /// Sıradaki fark ya da yoklama hatası (hata aboneliği bitirmez).
    pub async fn sonraki(&mut self) -> Option<Sonuc<Fark>> {
        self.alici.recv().await
    }
}

impl Drop for Abonelik {
    fn drop(&mut self) {
        self.gorev.abort();
    }
}

/// Aboneliği kurar, ilk görüntüyü alır ve her `aralik`ta yoklayan bir tokio
/// görevi başlatır.
pub async fn abone_ol(db: &Path, tanim: CanliSorgu, aralik: Duration) -> Sonuc<Abonelik> {
    let mut abone = Abone::baslat(db, tanim).await?;
    let sutunlar = abone.sutunlar().to_vec();
    let ilk = abone.goruntu().cloned().collect();
    // Sınırlı kanal: tüketici yavaşsa görev gönderimde bekler. Fark durum
    // tabanlı olduğu için bu arada gelen değişiklik sonraki farka girer.
    let (verici, alici) = mpsc::channel(16);
    let gorev = tokio::spawn(async move {
        loop {
            tokio::time::sleep(aralik).await;
            if let Some(s) = abone.yokla().await.transpose() {
                if verici.send(s).await.is_err() {
                    return; // alıcı düşürüldü: abonelik bitti
                }
            }
        }
    });
    Ok(Abonelik {
        sutunlar,
        ilk,
        alici,
        gorev,
    })
}

/// Durum tabanlı fark: anahtarı yalnız yenide olan eklenen, yalnız öncekinde
/// olan silinen, ikisinde olup satırı farklı olan değişen. Sıra sorgununki.
fn fark(onceki: &[(String, Satir)], yeni: &[(String, Satir)]) -> Fark {
    let eski: HashMap<&str, &Satir> = onceki.iter().map(|(k, s)| (k.as_str(), s)).collect();
    let taze: HashSet<&str> = yeni.iter().map(|(k, _)| k.as_str()).collect();
    let mut f = Fark::default();
    for (k, s) in yeni {
        match eski.get(k.as_str()) {
            None => f.eklenen.push(s.clone()),
            Some(o) if *o != s => f.degisen.push(((*o).clone(), s.clone())),
            Some(_) => {}
        }
    }
    for (k, s) in onceki {
        if !taze.contains(k.as_str()) {
            f.silinen.push(s.clone());
        }
    }
    f
}

/// Sonucu anahtar koduyla eşler. Aynı anahtar ikinci kez geçerse ret: hangi
/// satırın "önceki" sayılacağı belirsiz kalırdı.
fn anahtarla(satirlar: Vec<Satir>, anahtar: &[usize]) -> Sonuc<Vec<(String, Satir)>> {
    let mut gorulen = HashSet::new();
    let mut v = Vec::with_capacity(satirlar.len());
    for s in satirlar {
        let k = anahtar_kodu(&s, anahtar);
        if !gorulen.insert(k.clone()) {
            let degerler: Vec<_> = anahtar.iter().filter_map(|&i| s.get(i)).collect();
            return Err(Hata::Sorgu(format!(
                "fark anahtarı sonuçta yineleniyor: {degerler:?}"
            )));
        }
        v.push((k, s));
    }
    Ok(v)
}

/// Anahtar sütunlarının tür etiketli, uzunluk önekli kodu. `Value` ne `Eq`
/// ne `Hash`; bu kod ikisini de sağlar ve farklı anahtarlar çakışmaz.
fn anahtar_kodu(satir: &[Value], anahtar: &[usize]) -> String {
    let mut k = String::new();
    for &i in anahtar {
        k.push_str(&match satir.get(i) {
            Some(Value::Integer(n)) => format!("i{n};"),
            Some(Value::Real(f)) => format!("r{};", f.to_bits()),
            Some(Value::Text(s)) => format!("t{}:{s};", s.len()),
            Some(Value::Blob(b)) => format!("b{}:{b:?};", b.len()),
            Some(Value::Null) | None => "n;".to_string(),
        });
    }
    k
}

/// Anahtar sütununun sonuçtaki yeri. Ad, SQL tanımlayıcısı gibi ASCII'de
/// büyük/küçük harf duyarsız eşlenir; iki sütun eşleşirse belirsizdir, ret.
fn sutun_yeri(sutunlar: &[String], ad: &str) -> Sonuc<usize> {
    let mut eslesen = sutunlar
        .iter()
        .enumerate()
        .filter(|(_, s)| s.eq_ignore_ascii_case(ad))
        .map(|(i, _)| i);
    match (eslesen.next(), eslesen.next()) {
        (Some(i), None) => Ok(i),
        (None, _) => Err(Hata::Sorgu(format!(
            "fark anahtarı `{ad}` sonuçta yok (sütunlar: {}); sorgu onu seçmeli",
            sutunlar.join(", ")
        ))),
        (Some(_), Some(_)) => Err(Hata::Sorgu(format!(
            "fark anahtarı `{ad}` sonuçta birden çok kez geçiyor; takma adla ayırın"
        ))),
    }
}

/// Tablonun üç tetikleyicisi kurulu mu? Değilse o tablonun değişikliği
/// günlüğe düşmez ve abone sessizce bayat kalırdı: ret.
async fn tetikleyici_denetle(c: &Connection, tablo: &str) -> Sonuc<()> {
    let adlar: Vec<Value> = ["e", "g", "s"]
        .iter()
        .map(|ek| Value::Text(format!("fd_{tablo}_{ek}")))
        .collect();
    let mut r = c
        .query(
            "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND name IN (?1, ?2, ?3)",
            adlar,
        )
        .await
        .map_err(baglam(format!("tetikleyici denetimi ({tablo})")))?;
    let n = match r
        .next()
        .await
        .map_err(baglam(format!("tetikleyici denetimi ({tablo})")))?
    {
        Some(s) => tamsayi(
            s.get_value(0)
                .map_err(baglam(format!("tetikleyici denetimi ({tablo})")))?,
        ),
        None => 0,
    };
    if n == 3 {
        Ok(())
    } else {
        Err(Hata::Sorgu(format!(
            "`{tablo}` izlenmiyor ({n}/3 tetikleyici): tablo yok ya da `kur` koşulmadı"
        )))
    }
}

/// Sorguyu koşar: (sütun adları, satırlar). Abone sorgusu yalnız okur ve her
/// yoklamada yeniden koşulduğu için yazan bir deyim her seferinde yazardı.
/// İki kat: biçim denetimi, sonra açık bir işlemde koşu ve her durumda geri
/// alma; deyim yine de yazdıysa (ör. `WITH … DELETE`) ret.
async fn kos(c: &Connection, sorgu: &str) -> Sonuc<(Vec<String>, Vec<Satir>)> {
    salt_okur_bicim(sorgu)?;
    c.execute("BEGIN", ())
        .await
        .map_err(baglam("salt-okur işlem açılışı"))?;
    let sonuc = islemde_kos(c, sorgu).await;
    let geri = c.execute("ROLLBACK", ()).await;
    match (sonuc, geri) {
        (Ok(s), Ok(_)) => Ok(s),
        (Err(e), Ok(_)) => Err(e),
        (Ok(_), Err(g)) => Err(baglam("salt-okur işlem geri alımı")(g)),
        (Err(e), Err(g)) => Err(Hata::Sorgu(format!(
            "{e}; ardından geri alma da başarısız: {g}"
        ))),
    }
}

async fn islemde_kos(c: &Connection, sorgu: &str) -> Sonuc<(Vec<String>, Vec<Satir>)> {
    let mut d = c
        .prepare(sorgu)
        .await
        .map_err(baglam(format!("sorgu koşulamadı ({sorgu})")))?;
    let mut r = d
        .query(())
        .await
        .map_err(baglam(format!("sorgu koşulamadı ({sorgu})")))?;
    let sutunlar = r.column_names();
    let mut v = Vec::new();
    while let Some(s) = r.next().await.map_err(baglam("sorgu satırı"))? {
        let satir = (0..s.column_count())
            .map(|i| s.get_value(i))
            .collect::<Result<Satir, _>>()
            .map_err(baglam("sorgu değeri"))?;
        v.push(satir);
    }
    let n = d.n_change();
    if n > 0 {
        return Err(Hata::Sorgu(format!(
            "abone sorgusu {n} satır yazdı (geri alındı); yalnız okuyan sorgu verilmeli"
        )));
    }
    Ok((sutunlar, v))
}

/// Sorgunun ilk sözcüğü (boşluk ve yorumlar atlanarak) `SELECT`, `WITH` ya da
/// `VALUES` olmalı. SQL anahtar sözcükleri ASCII'dir; Türkçe katlama gerekmez.
fn salt_okur_bicim(sorgu: &str) -> Sonuc<()> {
    let mut kalan = sorgu.trim_start();
    loop {
        if let Some(s) = kalan.strip_prefix("--") {
            kalan = s.split_once('\n').map_or("", |(_, k)| k).trim_start();
        } else if let Some(s) = kalan.strip_prefix("/*") {
            kalan = s.split_once("*/").map_or("", |(_, k)| k).trim_start();
        } else {
            break;
        }
    }
    let ilk: String = kalan
        .chars()
        .take_while(|k| k.is_ascii_alphabetic())
        .collect();
    if ["SELECT", "WITH", "VALUES"]
        .iter()
        .any(|a| ilk.eq_ignore_ascii_case(a))
    {
        Ok(())
    } else {
        Err(Hata::Sorgu(format!(
            "abone sorgusu yalnız okuyabilir (SELECT, WITH ya da VALUES ile başlamalı); \
             ilk sözcük: `{ilk}`"
        )))
    }
}

/// Hatayı hangi işlemde çıktığıyla sarar.
fn baglam<E: Display>(islem: impl Display) -> impl FnOnce(E) -> Hata {
    move |e| Hata::Sorgu(format!("{islem}: {e}"))
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::testler::{gecici_db, yaz};
    use crate::{buda, imlec_yaz, kur, oku};
    use std::time::Instant;

    fn t(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    fn tanim(sorgu: &str, tablolar: &[&str], anahtar: Option<&[&str]>) -> CanliSorgu {
        CanliSorgu {
            sorgu: sorgu.to_string(),
            tablolar: tablolar.iter().map(|s| s.to_string()).collect(),
            anahtar: anahtar.map(|a| a.iter().map(|s| s.to_string()).collect()),
        }
    }

    async fn kurulu_db(ad: &str, kurulum: &[&str]) -> PathBuf {
        let yol = gecici_db(ad, kurulum).await;
        kur(&ac(&yol).await.unwrap()).await.unwrap();
        yol
    }

    async fn ret_mesaji(yol: &Path, tn: CanliSorgu) -> String {
        match Abone::baslat(yol, tn).await {
            Err(Hata::Sorgu(m)) => m,
            Err(e) => panic!("beklenmeyen hata türü: {e}"),
            Ok(_) => panic!("abonelik reddedilmeliydi"),
        }
    }

    #[tokio::test]
    async fn k1_ekleme_guncelleme_silme_siniflanir() {
        let yol = kurulu_db(
            "sorgu-k1",
            &[
                "CREATE TABLE k (id TEXT PRIMARY KEY, ad TEXT, v INTEGER)",
                "INSERT INTO k VALUES ('a', 'Işık', 1)",
            ],
        )
        .await;
        let mut ab = Abone::baslat(
            &yol,
            tanim("SELECT id, ad, v FROM k ORDER BY id", &["k"], None),
        )
        .await
        .unwrap();
        assert_eq!(ab.sutunlar(), ["id", "ad", "v"]);
        let a1 = vec![t("a"), t("Işık"), Value::Integer(1)];
        assert_eq!(ab.goruntu().collect::<Vec<_>>(), [&a1], "ilk görüntü");

        yaz(&yol, &["INSERT INTO k VALUES ('İğne', 'çöğüş', 5)"]).await;
        let igne = vec![t("İğne"), t("çöğüş"), Value::Integer(5)];
        assert_eq!(
            ab.yokla().await.unwrap(),
            Some(Fark {
                eklenen: vec![igne.clone()],
                ..Fark::default()
            })
        );

        yaz(&yol, &["UPDATE k SET v = 2 WHERE id = 'a'"]).await;
        let a2 = vec![t("a"), t("Işık"), Value::Integer(2)];
        assert_eq!(
            ab.yokla().await.unwrap(),
            Some(Fark {
                degisen: vec![(a1, a2)],
                ..Fark::default()
            })
        );

        yaz(&yol, &["DELETE FROM k WHERE id = 'İğne'"]).await;
        assert_eq!(
            ab.yokla().await.unwrap(),
            Some(Fark {
                silinen: vec![igne],
                ..Fark::default()
            })
        );
    }

    #[tokio::test]
    async fn bilesik_anahtar_ve_rowid_kendiliginden_secilir() {
        let yol = kurulu_db(
            "sorgu-bilesik",
            &[
                "CREATE TABLE b (a TEXT NOT NULL, n INTEGER NOT NULL, x TEXT, PRIMARY KEY (a, n))",
                "CREATE TABLE r (x TEXT)",
                "INSERT INTO b VALUES ('p', 1, 'eski'), ('p', 2, 'öteki')",
                "INSERT INTO r VALUES ('ilk')",
            ],
        )
        .await;
        let mut b = Abone::baslat(&yol, tanim("SELECT x, n, a FROM b", &["b"], None))
            .await
            .unwrap();
        let mut r = Abone::baslat(&yol, tanim("SELECT rowid, x FROM r", &["r"], None))
            .await
            .unwrap();
        yaz(
            &yol,
            &[
                "UPDATE b SET x = 'yeni' WHERE a = 'p' AND n = 1",
                "UPDATE r SET x = 'son' WHERE x = 'ilk'",
            ],
        )
        .await;
        let fb = b.yokla().await.unwrap().unwrap();
        assert_eq!(fb.degisen.len(), 1, "bileşik anahtar: {fb:?}");
        assert!(fb.eklenen.is_empty() && fb.silinen.is_empty());
        assert_eq!(fb.degisen[0].1, [t("yeni"), Value::Integer(1), t("p")]);
        let fr = r.yokla().await.unwrap().unwrap();
        assert_eq!(
            fr.degisen,
            [(
                vec![Value::Integer(1), t("ilk")],
                vec![Value::Integer(1), t("son")]
            )]
        );
    }

    #[tokio::test]
    async fn k2_fark_bir_saniyeden_once_gelir() {
        let yol = kurulu_db(
            "sorgu-k2",
            &["CREATE TABLE k (id INTEGER PRIMARY KEY, ad TEXT)"],
        )
        .await;
        let mut ab = abone_ol(
            &yol,
            tanim("SELECT id, ad FROM k", &["k"], None),
            Duration::from_millis(250),
        )
        .await
        .unwrap();
        assert!(ab.ilk_goruntu().is_empty());
        let mut en_kotu = Duration::ZERO;
        for (sql, beklenen) in [
            ("INSERT INTO k VALUES (1, 'bir')", (1, 0, 0)),
            ("UPDATE k SET ad = 'Bİr' WHERE id = 1", (0, 1, 0)),
            ("DELETE FROM k WHERE id = 1", (0, 0, 1)),
        ] {
            let bas = Instant::now();
            yaz(&yol, &[sql]).await;
            let f = tokio::time::timeout(Duration::from_secs(1), ab.sonraki())
                .await
                .expect("1 sn içinde fark gelmedi")
                .unwrap()
                .unwrap();
            let gecen = bas.elapsed();
            en_kotu = en_kotu.max(gecen);
            assert_eq!(
                (f.eklenen.len(), f.degisen.len(), f.silinen.len()),
                beklenen
            );
            println!("K2 gecikme: {sql}: {} ms", gecen.as_millis());
        }
        println!("K2 en kötü gecikme: {} ms", en_kotu.as_millis());
        assert!(en_kotu < Duration::from_secs(1), "{en_kotu:?}");
    }

    #[tokio::test]
    async fn k3_dusen_abone_otekini_etkilemez() {
        let yol = kurulu_db(
            "sorgu-k3",
            &["CREATE TABLE k (id INTEGER PRIMARY KEY, ad TEXT)"],
        )
        .await;
        let tn = tanim("SELECT id, ad FROM k", &["k"], None);
        let aralik = Duration::from_millis(250);
        let mut a1 = abone_ol(&yol, tn.clone(), aralik).await.unwrap();
        let mut a2 = abone_ol(&yol, tn, aralik).await.unwrap();
        let bekle = Duration::from_secs(2);

        yaz(&yol, &["INSERT INTO k VALUES (1, 'bir')"]).await;
        for ab in [&mut a1, &mut a2] {
            let f = tokio::time::timeout(bekle, ab.sonraki()).await.unwrap();
            assert_eq!(
                f.unwrap().unwrap().eklenen,
                [vec![Value::Integer(1), t("bir")]]
            );
        }

        // Düşürme yoklama görevini iptal eder; öteki abone sürer.
        drop(a1);
        yaz(&yol, &["INSERT INTO k VALUES (2, 'iki')"]).await;
        let f = tokio::time::timeout(bekle, a2.sonraki()).await.unwrap();
        assert_eq!(
            f.unwrap().unwrap().eklenen,
            [vec![Value::Integer(2), t("iki")]]
        );
        yaz(&yol, &["DELETE FROM k WHERE id = 1"]).await;
        let f = tokio::time::timeout(bekle, a2.sonraki()).await.unwrap();
        assert_eq!(
            f.unwrap().unwrap().silinen,
            [vec![Value::Integer(1), t("bir")]]
        );
    }

    #[tokio::test]
    async fn k4_sonuc_degismezse_fark_donmez() {
        let yol = kurulu_db(
            "sorgu-k4",
            &[
                "CREATE TABLE k (id INTEGER PRIMARY KEY, ad TEXT, v INTEGER)",
                "CREATE TABLE d (x TEXT)",
                "INSERT INTO k VALUES (1, 'içeride', 1), (2, 'dışarıda', 50)",
            ],
        )
        .await;
        let mut ab = Abone::baslat(
            &yol,
            tanim("SELECT id, ad FROM k WHERE v < 10", &["k"], None),
        )
        .await
        .unwrap();

        // Hiç yazış yok.
        assert_eq!(ab.yokla().await.unwrap(), None);
        assert_eq!(ab.yokla().await.unwrap(), None);

        // Günlükte k satırı var ama sorgu sonucu aynı: süzgeç dışı satır ve
        // seçilmeyen sütun değişti.
        let once = ab.imlec;
        yaz(
            &yol,
            &[
                "UPDATE k SET v = 60 WHERE id = 2",
                "UPDATE k SET v = 2 WHERE id = 1",
                "INSERT INTO d VALUES ('başka tablo')",
            ],
        )
        .await;
        let gunluk = oku(&ac(&yol).await.unwrap(), once, 100).await.unwrap();
        assert_eq!(gunluk.iter().filter(|d| d.tablo == "k").count(), 2);
        assert_eq!(ab.yokla().await.unwrap(), None);
        assert_eq!(ab.imlec, gunluk[2].no, "imleç ilerlemeli");

        // Gerçek değişiklik yine görülür.
        yaz(&yol, &["UPDATE k SET v = 3 WHERE id = 2"]).await;
        let f = ab.yokla().await.unwrap().unwrap();
        assert_eq!(f.eklenen, [vec![Value::Integer(2), t("dışarıda")]]);
    }

    #[tokio::test]
    async fn k5_belirsiz_anahtar_reddedilir() {
        let yol = kurulu_db(
            "sorgu-k5",
            &[
                "CREATE TABLE k (id TEXT PRIMARY KEY, ad TEXT)",
                "CREATE TABLE r (x TEXT)",
                "INSERT INTO k VALUES ('a', 'aynı'), ('b', 'aynı')",
                "INSERT INTO r VALUES ('x')",
            ],
        )
        .await;

        // Anahtar sütunu sonuçta yok: birincil anahtar ve rowid.
        let m = ret_mesaji(&yol, tanim("SELECT ad FROM k", &["k"], None)).await;
        assert!(m.contains("`id` sonuçta yok"), "{m}");
        let m = ret_mesaji(&yol, tanim("SELECT x FROM r", &["r"], None)).await;
        assert!(m.contains("`rowid` sonuçta yok"), "{m}");
        let m = ret_mesaji(&yol, tanim("SELECT id FROM k", &["k"], Some(&["yok"]))).await;
        assert!(m.contains("`yok` sonuçta yok"), "{m}");

        // Çok tablo, anahtar yok.
        let m = ret_mesaji(&yol, tanim("SELECT id, x FROM k, r", &["k", "r"], None)).await;
        assert!(m.contains("birden çok tablo"), "{m}");

        // Yinelenen anahtar.
        let m = ret_mesaji(&yol, tanim("SELECT id, ad FROM k", &["k"], Some(&["ad"]))).await;
        assert!(m.contains("yineleniyor"), "{m}");

        // İzlenmeyen tablo ve boş anahtar listesi.
        let m = ret_mesaji(&yol, tanim("SELECT 1", &["yok"], None)).await;
        assert!(m.contains("izlenmiyor"), "{m}");
        let m = ret_mesaji(&yol, tanim("SELECT id FROM k", &["k"], Some(&[]))).await;
        assert!(m.contains("boş"), "{m}");

        // Abonelik sırasında yinelenen anahtar: hata, durum ilerlemez;
        // düzelince fark yine doğru.
        let mut ab = Abone::baslat(&yol, tanim("SELECT x FROM r", &["r"], Some(&["x"])))
            .await
            .unwrap();
        yaz(&yol, &["INSERT INTO r VALUES ('x')"]).await;
        let e = ab.yokla().await.unwrap_err();
        assert!(e.to_string().contains("yineleniyor"), "{e}");
        assert!(
            ab.yokla().await.is_err(),
            "imleç ilerlememeli, hata sürmeli"
        );
        yaz(&yol, &["UPDATE r SET x = 'y' WHERE rowid = 2"]).await;
        let f = ab.yokla().await.unwrap().unwrap();
        assert_eq!(f.eklenen, [vec![t("y")]]);
    }

    /// Abone sorgusu her yoklamada yeniden koşulur: yazan deyim reddedilmeli
    /// ve hiçbir yolla kalıcı yazış bırakmamalı.
    #[tokio::test]
    async fn k7_yazan_sorgu_reddedilir_ve_iz_birakmaz() {
        let yol = kurulu_db(
            "sorgu-k7",
            &[
                "CREATE TABLE w (id INTEGER PRIMARY KEY, ad TEXT)",
                "INSERT INTO w VALUES (1, 'bir'), (2, 'iki')",
            ],
        )
        .await;
        async fn sayi(yol: &Path) -> i64 {
            let c = ac(yol).await.unwrap();
            let mut r = c.query("SELECT count(*) FROM w", ()).await.unwrap();
            tamsayi(r.next().await.unwrap().unwrap().get_value(0).unwrap())
        }

        // Biçim katı: ilk sözcük yazan deyim.
        for s in [
            "DELETE FROM w",
            "  /* yorum */ insert into w VALUES (3, 'üç') RETURNING id",
            "UPDATE w SET ad = 'x' RETURNING id",
        ] {
            let m = ret_mesaji(&yol, tanim(s, &["w"], None)).await;
            assert!(m.contains("yalnız okuyabilir"), "{s}: {m}");
        }
        // İşlem katı: biçimi geçen ama yazan deyim; yazış geri alınır.
        let m = ret_mesaji(
            &yol,
            tanim(
                "WITH x AS (SELECT 1) DELETE FROM w RETURNING id",
                &["w"],
                None,
            ),
        )
        .await;
        assert!(m.contains("geri alındı"), "{m}");
        // Birden çok deyim: ikincisi yazamamalı.
        Abone::baslat(&yol, tanim("SELECT id FROM w; DELETE FROM w", &["w"], None))
            .await
            .ok();
        assert_eq!(sayi(&yol).await, 2, "hiçbir yol satır silmemeli");
        assert!(oku(&ac(&yol).await.unwrap(), 0, 100)
            .await
            .unwrap()
            .is_empty());

        // Yorumla ve küçük harfle başlayan okuma kabul edilir.
        let ab = Abone::baslat(
            &yol,
            tanim("-- ilk görüntü\n  select id, ad FROM w", &["w"], None),
        )
        .await
        .unwrap();
        assert_eq!(ab.goruntu().count(), 2);
    }

    #[tokio::test]
    async fn k6_budanmis_aralikta_degisiklik_kacmaz() {
        let yol = kurulu_db(
            "sorgu-k6",
            &[
                "CREATE TABLE k (id INTEGER PRIMARY KEY, ad TEXT)",
                "CREATE TABLE d (x TEXT)",
            ],
        )
        .await;
        let mut ab = Abone::baslat(&yol, tanim("SELECT id, ad FROM k", &["k"], None))
            .await
            .unwrap();

        // k'ye yazış, ardından başka tabloya yazış. Kalıcı bir tüketici ikisini
        // de geçmiş, budama k satırını siliyor; günlükte yalnız d satırı kalıyor.
        yaz(
            &yol,
            &[
                "INSERT INTO k VALUES (7, 'yedi')",
                "INSERT INTO d VALUES ('z')",
            ],
        )
        .await;
        let c = ac(&yol).await.unwrap();
        let son = crate::son_no(&c).await.unwrap();
        imlec_yaz(&yol, "kalici", son).unwrap();
        assert_eq!(buda(&c, &yol).await.unwrap(), 1);
        let kalan = oku(&c, 0, 100).await.unwrap();
        assert_eq!(kalan.len(), 1);
        assert_eq!(kalan[0].tablo, "d");
        assert!(kalan[0].no > ab.imlec + 1, "boşluk kurulmalı");

        let f = ab.yokla().await.unwrap();
        assert_eq!(
            f.map(|f| f.eklenen),
            Some(vec![vec![Value::Integer(7), t("yedi")]]),
            "budanmış aralıktaki değişiklik kaçtı"
        );
        assert_eq!(ab.imlec, son);
    }
}
