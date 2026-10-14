//! MCP Seçimi eşlemesi (Nazar CONTEXT.md "Seçim"): bir `nazar_ara` sonucundaki yollardan biri, aynı oturumda
//! SONRAKİ herhangi bir araç çağrısının girdisinde geçiyorsa o sonuç Seçim sayılır. Okunup işe yaramayan yol da
//! Seçim sayılır (bilinen gürültü, kabul edildi); bu yüzden masaüstü Seçimi daha güçlü sinyaldir.
//!
//! Toplu iş: Nazar'ın Seçim günlüğünün KOPYASINI okur (Turso aynı dosyada çok süreçli erişimi desteklemez;
//! kopya yalnız `sqlite3 "file:<yol>?mode=ro" ".backup '<hedef>'"` ile alınır), Seçimleri nazard'a boru
//! üzerinden yazdırır. nazard aynı (arama, yol) Seçimini ikinci kez yazmaz: koşu yinelenebilir.

use crate::depo::{hata, Depo};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use turso::{Builder, Connection};

/// Nazar'ın kendi araçları sonraki çağrı sayılmaz (bir sonraki `nazar_ara` sorgusunda yol geçebilir).
const NAZAR_ARACI: &str = "nazar_";

/// Eşlenen tek Seçim.
#[derive(Debug, Clone, PartialEq)]
pub struct Secim {
    pub arama_id: i64,
    pub yol: String,
    pub yaprak: Option<u32>,
    pub satir: Option<usize>,
    /// Yolu kullanan çağrının anı (unix ms).
    pub zaman_ms: i64,
    /// Kullanan aracın adı (`Read`, `Edit`…).
    pub arac: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Rapor {
    /// Günlükteki MCP aramaları.
    pub arama: usize,
    /// Oturum kimliği olmayan (eşlenemeyen) MCP araması.
    pub oturumsuz: usize,
    /// Arşivde hiç kaydı olmayan oturumun aramaları (arşivlenmemiş döküm olabilir).
    pub arsivsiz: usize,
}

/// Karşılaştırma biçimi: küçük harf, `\` → `/`, ardışık `/` tek. JSON'daki kaçışlı yol (`C:\\a`) ve
/// düz yol aynı biçime iner. (Türkçe harf katlaması gerekmez: yollar iki yanda da aynı kaynaktan.)
pub fn normallestir(s: &str) -> String {
    let mut cikti = String::with_capacity(s.len());
    for c in katla::kucult(s).chars() {
        let c = if c == '\\' { '/' } else { c };
        if c == '/' && cikti.ends_with('/') {
            continue;
        }
        cikti.push(c);
    }
    cikti
}

/// `c:/x/y` → ayrıca `/c/x/y` (Git Bash'in `Bash` aracına yazdığı biçim).
fn varyantlar(norm: &str) -> Vec<String> {
    let b = norm.as_bytes();
    if b.len() > 2 && b[1] == b':' && b[2] == b'/' && b[0].is_ascii_alphabetic() {
        return vec![norm.to_string(), format!("/{}{}", b[0] as char, &norm[2..])];
    }
    vec![norm.to_string()]
}

struct Cagri {
    zaman_ms: i64,
    arac: String,
    metin: String,
}

/// Nazar günlüğünü `sqlite3 .backup` ile kopyalar (salt okunur kaynak).
pub fn kopyala(kaynak: &Path, hedef: &Path) -> Result<(), String> {
    let kaynak = kaynak
        .to_str()
        .ok_or("kaynak yolu UTF-8 değil")?
        .replace('\\', "/")
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('?', "%3f")
        .replace('#', "%23");
    let hedef_s = hedef
        .to_str()
        .ok_or("hedef yolu UTF-8 değil")?
        .replace('\\', "/");
    if hedef_s.contains('\'') {
        return Err(format!("hedef yolunda tek tırnak olamaz: {hedef_s}"));
    }
    let uri = if kaynak.starts_with('/') {
        format!("file:{kaynak}?mode=ro")
    } else {
        format!("file:///{kaynak}?mode=ro")
    };
    let c = std::process::Command::new("sqlite3")
        .arg(&uri)
        .arg(format!(".backup '{hedef_s}'"))
        .output()
        .map_err(|e| format!("sqlite3 çalıştırılamadı (PATH'te olmalı): {e}"))?;
    if !c.status.success() || !hedef.is_file() {
        return Err(format!(
            "sqlite3 .backup başarısız ({uri}): {}",
            String::from_utf8_lossy(&c.stderr).trim()
        ));
    }
    Ok(())
}

async fn cagrilar(arsiv: &Depo, oturum: &str) -> Result<Vec<Cagri>, String> {
    let mut r = arsiv
        .conn
        .query(
            "SELECT zaman_ms, arac, metin FROM kayitlar \
             WHERE tur = 'arac' AND oturum = ?1 AND zaman_ms IS NOT NULL ORDER BY zaman_ms",
            [oturum.to_string()],
        )
        .await
        .map_err(hata)?;
    let mut cikti = Vec::new();
    while let Some(s) = r.next().await.map_err(hata)? {
        let arac: String = s.get(1).map_err(hata)?;
        if arac.contains(NAZAR_ARACI) {
            continue;
        }
        cikti.push(Cagri {
            zaman_ms: s.get(0).map_err(hata)?,
            metin: normallestir(&s.get::<String>(2).map_err(hata)?),
            arac,
        });
    }
    Ok(cikti)
}

/// Nazar günlüğünün (kopya) MCP aramalarını arşivle eşler.
pub async fn esle_kopyadan(
    arsiv: &Depo,
    nazar_kopya: &Path,
) -> Result<(Vec<Secim>, Rapor), String> {
    let db = Builder::new_local(nazar_kopya.to_string_lossy().as_ref())
        .build()
        .await
        .map_err(hata)?;
    let nazar: Connection = db.connect().map_err(hata)?;
    let mut r = nazar
        .query(
            "SELECT a.id, a.zaman_ms, a.oturum, s.yol, s.yaprak, s.satir FROM aramalar a \
             JOIN sonuclar s ON s.arama_id = a.id WHERE a.kanal = 'mcp' ORDER BY a.id, s.sira",
            (),
        )
        .await
        .map_err(hata)?;
    // arama → (oturum, zaman, sonuçlar); sıra korunur.
    type Sonuclar = Vec<(String, Option<u32>, Option<usize>)>;
    let mut aramalar: Vec<(i64, i64, Option<String>, Sonuclar)> = Vec::new();
    while let Some(s) = r.next().await.map_err(hata)? {
        let id: i64 = s.get(0).map_err(hata)?;
        if aramalar.last().map(|a| a.0) != Some(id) {
            aramalar.push((
                id,
                s.get(1).map_err(hata)?,
                s.get(2).map_err(hata)?,
                Vec::new(),
            ));
        }
        let yaprak: Option<i64> = s.get(4).map_err(hata)?;
        let satir: Option<i64> = s.get(5).map_err(hata)?;
        let sonuc = (
            s.get(3).map_err(hata)?,
            yaprak.map(|y| y as u32),
            satir.map(|n| n as usize),
        );
        if let Some(son) = aramalar.last_mut() {
            son.3.push(sonuc);
        }
    }
    drop(r);
    // Sonuçsuz aramalar JOIN'de düşer; sayıyı ayrıca al.
    let mut r = nazar
        .query(
            "SELECT count(*), count(oturum) FROM aramalar WHERE kanal = 'mcp'",
            (),
        )
        .await
        .map_err(hata)?;
    let mut rapor = Rapor::default();
    if let Some(s) = r.next().await.map_err(hata)? {
        let (toplam, oturumlu): (i64, i64) = (s.get(0).map_err(hata)?, s.get(1).map_err(hata)?);
        (rapor.arama, rapor.oturumsuz) = (toplam as usize, (toplam - oturumlu) as usize);
    }
    drop(r);

    let mut onbellek: HashMap<String, Vec<Cagri>> = HashMap::new();
    let mut secimler = Vec::new();
    for (arama_id, zaman, oturum, sonuclar) in aramalar {
        let Some(oturum) = oturum else { continue };
        if !onbellek.contains_key(&oturum) {
            let c = cagrilar(arsiv, &oturum).await?;
            onbellek.insert(oturum.clone(), c);
        }
        let c = &onbellek[&oturum];
        if c.is_empty() {
            rapor.arsivsiz += 1;
            continue;
        }
        let mut gorulen = HashSet::new();
        for (yol, yaprak, satir) in sonuclar {
            if !gorulen.insert(yol.clone()) {
                continue; // aynı yolun öteki yaprakları: en iyi sıradaki satır yeter
            }
            let hedefler = varyantlar(&normallestir(&yol));
            let bulunan = c
                .iter()
                .filter(|k| k.zaman_ms > zaman)
                .find(|k| hedefler.iter().any(|h| k.metin.contains(h.as_str())));
            if let Some(k) = bulunan {
                secimler.push(Secim {
                    arama_id,
                    yol,
                    yaprak,
                    satir,
                    zaman_ms: k.zaman_ms,
                    arac: k.arac.clone(),
                });
            }
        }
    }
    Ok((secimler, rapor))
}

/// Kaynak günlüğü kopyalayıp eşler.
pub async fn esle(arsiv: &Depo, nazar_gunlugu: &Path) -> Result<(Vec<Secim>, Rapor), String> {
    let gecici = std::env::temp_dir().join(format!("nazar-secim-kopya-{}.db", std::process::id()));
    std::fs::remove_file(&gecici).ok();
    kopyala(nazar_gunlugu, &gecici)?;
    let sonuc = esle_kopyadan(arsiv, &gecici).await;
    std::fs::remove_file(&gecici).ok();
    sonuc
}
