//! Santral: oturumlar arası mesaj kutuları ve onları açan üç MCP aracı.
//!
//! Kutular bellektedir; kalıcı değildir. Oturumların MCP süreçleri tek el-Fihrist
//! hizmetine aktardığında kutular o hizmettedir ve oturumlar birbirini görür.
//! Hizmete ulaşamayan oturumda kutular o oturumun sürecindedir: yalnız kendi
//! bıraktığını okuyabilir. Hizmet ya da süreç kapanınca bekleyen mesajlar gider.
//! Nazar'a mesaj iletilmez; `santral_durum` yalnız Nazar'ın açık olup olmadığını söyler.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// `gonderen` verilmezse yazılan ad.
const ADSIZ: &str = "adsız";

/// Santral üzerinden iletilen mesaj paketi.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantralMesaji {
    pub id: String,
    pub gonderen: String,
    pub hedef: String,
    pub konu: String,
    pub icerik: String,
    pub zaman_ms: u64,
}

impl SantralMesaji {
    pub fn yeni(
        gonderen: impl Into<String>,
        hedef: impl Into<String>,
        konu: impl Into<String>,
        icerik: impl Into<String>,
    ) -> Self {
        let zaman = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let gonderen = gonderen.into();
        Self {
            id: format!("{zaman}-{gonderen}"),
            gonderen,
            hedef: hedef.into(),
            konu: konu.into(),
            icerik: icerik.into(),
            zaman_ms: zaman,
        }
    }
}

/// Bellekteki kutular: kutu adı → bekleyen mesajlar. Klonlar aynı kutuları paylaşır.
#[derive(Debug, Default, Clone)]
pub struct Santral {
    kutular: Arc<Mutex<HashMap<String, Vec<SantralMesaji>>>>,
}

impl Santral {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Kutulara kilit. Başka iş parçacığı kilit altında çöktüyse (zehirli kilit) içerik yine
    /// kullanılır: kutular düz bir eşlemdir, yarım kalmış bir değişmez yoktur.
    fn kilit(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<SantralMesaji>>> {
        self.kutular.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Mesajı hedefin kutusuna bırakır; kutudaki bekleyen sayısını döndürür.
    pub fn ilet(&self, mesaj: SantralMesaji) -> usize {
        let mut k = self.kilit();
        let kuyruk = k.entry(mesaj.hedef.clone()).or_default();
        kuyruk.push(mesaj);
        kuyruk.len()
    }

    /// Kutudaki mesajları çeker ve kutuyu boşaltır.
    pub fn yokla(&self, kutu: &str) -> Vec<SantralMesaji> {
        self.kilit().remove(kutu).unwrap_or_default()
    }

    pub fn bekleyen_sayisi(&self, kutu: &str) -> usize {
        self.kilit().get(kutu).map_or(0, Vec::len)
    }

    /// Mesaj bekleyen kutular ve sayıları, ada göre sıralı.
    pub fn aktif_kutular(&self) -> Vec<(String, usize)> {
        let mut v: Vec<_> = self
            .kilit()
            .iter()
            .map(|(ad, m)| (ad.clone(), m.len()))
            .collect();
        v.sort();
        v
    }
}

/// Santral'ın MCP araç tanımları.
pub fn araclar() -> Vec<Value> {
    vec![
        json!({
            "name": "santral_gonder",
            "description": "Başka bir oturuma mesaj, durum ya da iş devri bırakır: mesaj `hedef` adlı kutuya düşer, alıcı santral_yokla'ya aynı adı vererek okur. Kutular bellektedir: oturumlar ortak el-Fihrist hizmetine bağlıyken birbirini görür; hizmete ulaşamayan oturumda kutu yalnız o oturumun içindedir. Hizmet ya da oturum kapanınca bekleyen mesajlar kaybolur. Nazar'a iletilmez, toplu yayın yoktur. Yanıt bekliyorsan kendi kutu adını `gonderen` olarak ver.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hedef": {"type": "string", "description": "Kutu adı: alıcının yoklarken vereceği ad (oturum kimliği ya da iki tarafın anlaştığı bir rol adı, örn. 'antigravity')"},
                    "konu": {"type": "string", "description": "Kısa konu ya da tür (örn. 'is-devri', 'bilgi', 'soru')"},
                    "icerik": {"type": "string", "description": "İletilecek metin ya da JSON yükü"},
                    "gonderen": {"type": "string", "description": "Yanıtın bırakılacağı kendi kutu adın; boşsa 'adsız' yazılır"}
                },
                "required": ["hedef", "konu", "icerik"]
            }
        }),
        json!({
            "name": "santral_yokla",
            "description": "`hedef` kutusundaki bekleyen mesajları (gönderen, konu, içerik, zaman) eskiden yeniye döndürür ve kutuyu boşaltır; okunan mesaj bir daha gelmez. Kutu boşsa boş liste döner.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hedef": {"type": "string", "description": "Yoklanacak kutu adı (santral_gonder'deki hedef)"}
                },
                "required": ["hedef"]
            }
        }),
        json!({
            "name": "santral_durum",
            "description": "Mesaj bekleyen kutuları sayılarıyla ve Nazar daemon'unun açık olup olmadığını gösterir. Mesajları okumaz, kutuları boşaltmaz.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
    ]
}

fn metin(p: &Value, ad: &str) -> Result<String, String> {
    p.get(ad)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("'{ad}' alanı gerekli (boş olmayan metin)"))
}

/// Santral aracını koşturur. Ad santral'ın değilse `None`.
pub async fn calistir(ad: &str, p: &Value, s: &Santral) -> Option<Result<String, String>> {
    Some(match ad {
        "santral_gonder" => gonder(p, s),
        "santral_yokla" => metin(p, "hedef")
            .and_then(|h| serde_json::to_string_pretty(&s.yokla(&h)).map_err(|e| e.to_string())),
        "santral_durum" => Ok(durum(s).await),
        _ => return None,
    })
}

fn gonder(p: &Value, s: &Santral) -> Result<String, String> {
    let hedef = metin(p, "hedef")?;
    let gonderen = metin(p, "gonderen").unwrap_or_else(|_| ADSIZ.to_string());
    let mesaj = SantralMesaji::yeni(
        gonderen,
        hedef.clone(),
        metin(p, "konu")?,
        metin(p, "icerik")?,
    );
    let n = s.ilet(mesaj);
    Ok(format!(
        "mesaj '{hedef}' kutusuna bırakıldı; kutuda bekleyen: {n}"
    ))
}

async fn durum(s: &Santral) -> String {
    let kutular = s.aktif_kutular();
    let kutular = if kutular.is_empty() {
        "yok (bekleyen mesaj yok)".to_string()
    } else {
        kutular
            .iter()
            .map(|(ad, n)| format!("{ad} ({n})"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let nazar = match nazar_istemci::Istemci::baglan_varsayilan().await {
        Ok(mut i) => match i.durum().await {
            Ok(d) => format!(
                "açık (belge: {}, parça: {}, ocr kuyruğu: {})",
                d.belge_sayisi, d.parca_sayisi, d.ocr_kuyrugu
            ),
            Err(e) => format!("bağlandı ama durum alınamadı: {e}"),
        },
        Err(e) => format!("kapalı ({e})"),
    };
    format!("Mesaj bekleyen kutular: {kutular}\nNazar: {nazar}")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn santral_mesaj_ilet_ve_yokla() {
        let s = Santral::yeni();
        assert_eq!(
            s.ilet(SantralMesaji::yeni(
                "oturum-a",
                "ajan-1",
                "gorev-devri",
                "testleri kos"
            )),
            1
        );
        assert_eq!(
            s.ilet(SantralMesaji::yeni(
                "oturum-b",
                "ajan-1",
                "bilgi",
                "graf hazir"
            )),
            2
        );
        assert_eq!(s.bekleyen_sayisi("ajan-1"), 2);
        assert_eq!(s.bekleyen_sayisi("oturum-a"), 0);
        assert_eq!(s.aktif_kutular(), vec![("ajan-1".to_string(), 2)]);

        let alinan = s.yokla("ajan-1");
        assert_eq!(alinan.len(), 2);
        assert_eq!(alinan[0].konu, "gorev-devri");
        assert_eq!(alinan[1].konu, "bilgi");
        // İkinci yoklamada kutu boş.
        assert!(s.yokla("ajan-1").is_empty());
    }

    /// Araçlar üzerinden gidiş-dönüş: gönderen adı alıcıya ulaşır (yanıt verebilsin).
    #[tokio::test]
    async fn arac_gidis_donus_gonderen_tasinir() {
        let s = Santral::yeni();
        let r = calistir(
            "santral_gonder",
            &json!({"hedef": "b", "konu": "soru", "icerik": "x", "gonderen": "a"}),
            &s,
        )
        .await;
        assert!(matches!(r, Some(Ok(_))), "{r:?}");
        let Some(Ok(okunan)) = calistir("santral_yokla", &json!({"hedef": "b"}), &s).await else {
            panic!("yoklama başarısız");
        };
        let v: Vec<SantralMesaji> = serde_json::from_str(&okunan).unwrap();
        assert_eq!((v.len(), v[0].gonderen.as_str()), (1, "a"));
    }

    /// Boş hedefle yoklama hata verir: eskiden "" kutusunu okuyup hep boş liste dönüyordu.
    #[tokio::test]
    async fn bos_hedefle_yoklama_hata() {
        let s = Santral::yeni();
        let r = calistir("santral_yokla", &json!({}), &s).await;
        assert!(matches!(r, Some(Err(_))), "{r:?}");
        let r = calistir("santral_yokla", &json!({"hedef": "  "}), &s).await;
        assert!(matches!(r, Some(Err(_))), "{r:?}");
    }

    #[tokio::test]
    async fn baska_arac_none() {
        assert!(calistir("search_skills", &json!({}), &Santral::yeni())
            .await
            .is_none());
    }

    #[test]
    fn arac_semalari_zorunlulari_tanimli() {
        for t in araclar() {
            let ozellik = &t["inputSchema"]["properties"];
            for z in t["inputSchema"]["required"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let z = z.as_str().unwrap();
                assert!(
                    ozellik[z]["description"].is_string(),
                    "{}: {z} açıklamasız",
                    t["name"]
                );
            }
        }
    }
}
