//! Arşiv araması: Türkçe katlamalı BM25, bellekte (fts5 değil: ADR 0005/0006; ayrıca fts5 `ışık`/`IŞIK`'ı eşlemez).
//!
//! Her aramada kayıtlar baştan okunup indekslenir (ölçülen arşiv ~60 MB metin: saniyeler). Hizmet süreci
//! kalıcı indeks isterse bu işlev oraya taşınır; şimdilik sıfır kalıcı durum, bayatlama yok.
//! ponytail: `ibnunnedim_cli::arama::Indeks` aynı BM25'i yapıyor ama ibnunnedim-cli bu crate'e bağımlı
//! (döngü); ortak crate'e çıkarılınca ikisi birleşir.

use crate::depo::{hata, Depo};
use std::collections::HashMap;

const K1: f64 = 1.2;
const B: f64 = 0.75;

#[derive(Debug, Clone, PartialEq)]
pub struct Sonuc {
    pub skor: f64,
    pub oturum: String,
    pub tur: String,
    pub arac: Option<String>,
    pub zaman: String,
    /// Asıl dökümdeki yer: `<döküm>:<satır>`.
    pub yer: String,
    pub kesit: String,
}

/// Arşive girmeyen ama iz bırakan `sonuc` türü aranmaz (kimlik ve sayıdan ibaret).
const ARANAN: &str = "'ercan', 'model', 'arac', 'ozet', 'sinir'";

fn kesit(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    if s.chars().count() > n {
        format!("{t}…")
    } else {
        t
    }
}

impl Depo {
    /// `tur`: boşsa hepsi (`ercan`, `model`, `arac`, `ozet`, `sinir`); doluysa yalnız o tür.
    pub async fn ara(
        &self,
        sorgu: &str,
        limit: usize,
        tur: Option<&str>,
    ) -> Result<Vec<Sonuc>, String> {
        let mut terimler = katla::belirtecle(sorgu);
        terimler.sort();
        terimler.dedup();
        if terimler.is_empty() {
            return Ok(Vec::new());
        }
        let mut r = match tur {
            Some(t) => {
                let sql = "SELECT id, metin, arac FROM kayitlar WHERE tur = ?1";
                self.conn.query(sql, [t.to_string()]).await
            }
            None => {
                let sql = format!("SELECT id, metin, arac FROM kayitlar WHERE tur IN ({ARANAN})");
                self.conn.query(&sql, ()).await
            }
        }
        .map_err(hata)?;
        // terim → (kayıt sırası, tf); yalnız sorgu terimleri tutulur.
        let mut ters: HashMap<&str, Vec<(usize, u32)>> =
            terimler.iter().map(|t| (t.as_str(), Vec::new())).collect();
        let (mut idler, mut uzunluk): (Vec<i64>, Vec<u32>) = (Vec::new(), Vec::new());
        while let Some(s) = r.next().await.map_err(hata)? {
            let id: i64 = s.get(0).map_err(hata)?;
            let metin: String = s.get(1).map_err(hata)?;
            let arac: Option<String> = s.get(2).map_err(hata)?;
            let mut belirtecler = katla::belirtecle(&metin);
            belirtecler.extend(arac.iter().flat_map(|a| katla::belirtecle(a)));
            let sira = idler.len();
            idler.push(id);
            uzunluk.push(belirtecler.len() as u32);
            let mut say: HashMap<&str, u32> = HashMap::new();
            for b in &belirtecler {
                if let Some((anahtar, _)) = ters.get_key_value(b.as_str()) {
                    *say.entry(*anahtar).or_default() += 1;
                }
            }
            for (t, n) in say {
                if let Some(l) = ters.get_mut(t) {
                    l.push((sira, n));
                }
            }
        }
        drop(r);
        let n = idler.len() as f64;
        if n == 0.0 {
            return Ok(Vec::new());
        }
        let ort = uzunluk.iter().map(|&u| u as f64).sum::<f64>() / n;
        let mut skorlar: HashMap<usize, f64> = HashMap::new();
        for liste in ters.values() {
            let df = liste.len() as f64;
            let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
            for &(sira, tf) in liste {
                let tf = tf as f64;
                let u = uzunluk[sira] as f64;
                *skorlar.entry(sira).or_default() +=
                    idf * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * u / ort.max(1.0)));
            }
        }
        let mut sirali: Vec<(usize, f64)> = skorlar.into_iter().collect();
        // Eşit skorda kayıt sırası: aynı sorgu aynı sırayı versin.
        sirali.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        let mut cikti = Vec::new();
        for (sira, skor) in sirali.into_iter().take(limit) {
            let mut r = self
                .conn
                .query(
                    "SELECT oturum, tur, arac, zaman, dokum, satir, metin FROM kayitlar WHERE id = ?1",
                    [idler[sira]],
                )
                .await
                .map_err(hata)?;
            if let Some(s) = r.next().await.map_err(hata)? {
                let (dokum, satir): (String, i64) =
                    (s.get(4).map_err(hata)?, s.get(5).map_err(hata)?);
                cikti.push(Sonuc {
                    skor,
                    oturum: s.get(0).map_err(hata)?,
                    tur: s.get(1).map_err(hata)?,
                    arac: s.get(2).map_err(hata)?,
                    zaman: s.get(3).map_err(hata)?,
                    yer: format!("{dokum}:{satir}"),
                    kesit: kesit(&s.get::<String>(6).map_err(hata)?, 240),
                });
            }
        }
        Ok(cikti)
    }
}
