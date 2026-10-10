//! Santral Operatörü (İbnünnedîm ↔ Nazar ↔ Oturumlar Arası İletişim).
//!
//! Oturumlar arası doğrudan mesajlaşma, iş devretme (handoff) ve
//! anlık kanal yönlendirme mekanizması.
//!
//! Fiziksel taşıyıcı:
//! 1. Oturumlar arası TCP santral kuyruğu (hizmet içi tampon).
//! 2. Nazar IPC borusu (`\\.\pipe\nazar-v1` / `nazard` erişimi).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

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
    pub fn yeni(gonderen: impl Into<String>, hedef: impl Into<String>, konu: impl Into<String>, icerik: impl Into<String>) -> Self {
        let zaman = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let gonderen_str = gonderen.into();
        let id = format!("{}-{}", zaman, gonderen_str);
        Self {
            id,
            gonderen: gonderen_str,
            hedef: hedef.into(),
            konu: konu.into(),
            icerik: icerik.into(),
            zaman_ms: zaman,
        }
    }
}

/// Bellekteki Santral Durumu: Oturum bazlı gelen kutusu (inbox).
#[derive(Debug, Default, Clone)]
pub struct Santral {
    // hedef_oturum -> mesajlar
    kutular: Arc<Mutex<HashMap<String, Vec<SantralMesaji>>>>,
}

impl Santral {
    pub fn yeni() -> Self {
        Self {
            kutular: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Mesajı hedefin gelen kutusuna bırakır.
    pub fn ilet(&self, mesaj: SantralMesaji) -> usize {
        let mut k = self.kutular.lock().unwrap();
        let kuyruk = k.entry(mesaj.hedef.clone()).or_default();
        kuyruk.push(mesaj);
        kuyruk.len()
    }

    /// Bir oturuma ait birikmiş mesajları çeker ve kutuyu boşaltır.
    pub fn yokla(&self, oturum: &str) -> Vec<SantralMesaji> {
        let mut k = self.kutular.lock().unwrap();
        k.remove(oturum).unwrap_or_default()
    }

    /// Bekleyen toplam mesaj sayısı.
    pub fn bekleyen_sayisi(&self, oturum: &str) -> usize {
        let k = self.kutular.lock().unwrap();
        k.get(oturum).map(|v| v.len()).unwrap_or(0)
    }

    /// Aktif kutusu olan tüm oturumları listeler.
    pub fn aktif_hedefler(&self) -> Vec<String> {
        let k = self.kutular.lock().unwrap();
        k.keys().cloned().collect()
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn santral_mesaj_ilet_ve_yokla() {
        let s = Santral::yeni();
        let m1 = SantralMesaji {
            id: "1".into(),
            gonderen: "oturum-a".into(),
            hedef: "hermes".into(),
            konu: "gorev-devri".into(),
            icerik: "testleri kos".into(),
            zaman_ms: 100,
        };
        let m2 = SantralMesaji {
            id: "2".into(),
            gonderen: "oturum-b".into(),
            hedef: "hermes".into(),
            konu: "bilgi".into(),
            icerik: "graf hazir".into(),
            zaman_ms: 200,
        };

        assert_eq!(s.ilet(m1.clone()), 1);
        assert_eq!(s.ilet(m2.clone()), 2);
        assert_eq!(s.bekleyen_sayisi("hermes"), 2);
        assert_eq!(s.bekleyen_sayisi("oturum-a"), 0);

        let alinan = s.yokla("hermes");
        assert_eq!(alinan.len(), 2);
        assert_eq!(alinan[0].konu, "gorev-devri");
        assert_eq!(alinan[1].konu, "bilgi");

        // İkinci yoklamada kutu boş olmalı
        assert_eq!(s.bekleyen_sayisi("hermes"), 0);
        assert!(s.yokla("hermes").is_empty());
    }
}
