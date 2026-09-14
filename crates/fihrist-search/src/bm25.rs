use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Belge {
    pub id: String,
    pub ad: String,
    pub aciklama: String,
    pub tam_metin_md: String,
}

pub struct Indeks {
    pub belgeler: Vec<Belge>,
    // term -> list of (doc_index, term_frequency_in_doc)
    ters_indeks: HashMap<String, Vec<(usize, usize)>>,
    belge_uzunluklari: Vec<usize>,
    ort_belge_uzunlugu: f64,
}

/// Türkçe katlamalı belirteçleyici.
///
/// Türkçe büyük 'İ'/'I' ve 'ç ş ğ ü ö ı' harflerini standart ASCII izdüşümlerine
/// katlayarak sorgu-belge arama tutarlılığı sağlar.
pub fn belirtecle(metin: &str) -> Vec<String> {
    let mut katlanmis = String::with_capacity(metin.len());
    for c in metin.chars() {
        match c {
            'İ' | 'I' => katlanmis.push('i'),
            other => {
                for k in other.to_lowercase() {
                    katlanmis.push(match k {
                        'ı' => 'i',
                        'ş' => 's',
                        'ğ' => 'g',
                        'ü' => 'u',
                        'ö' => 'o',
                        'ç' => 'c',
                        _ => k,
                    });
                }
            }
        }
    }
    katlanmis
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

impl Indeks {
    /// Belgelerden bellekte ters indeks ve uzunluk istatistiklerini kurar.
    /// Başlık (`ad`) alanı arama ağırlığında daha değerli olduğu için 3 kez belirteçlenir.
    pub fn kur(belgeler: Vec<Belge>) -> Self {
        let mut ters_indeks: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut belge_uzunluklari = Vec::with_capacity(belgeler.len());
        let mut toplam_uzunluk = 0usize;

        for (idx, b) in belgeler.iter().enumerate() {
            let mut doc_tokens = Vec::new();
            let ad_tokens = belirtecle(&b.ad);
            for _ in 0..3 {
                doc_tokens.extend(ad_tokens.clone());
            }
            doc_tokens.extend(belirtecle(&b.aciklama));
            doc_tokens.extend(belirtecle(&b.tam_metin_md));

            let len = doc_tokens.len();
            belge_uzunluklari.push(len);
            toplam_uzunluk += len;

            let mut frekanslar: HashMap<String, usize> = HashMap::new();
            for token in doc_tokens {
                *frekanslar.entry(token).or_insert(0) += 1;
            }

            for (term, count) in frekanslar {
                ters_indeks.entry(term).or_default().push((idx, count));
            }
        }

        let ort_belge_uzunlugu = if belgeler.is_empty() {
            0.0
        } else {
            toplam_uzunluk as f64 / belgeler.len() as f64
        };

        Self {
            belgeler,
            ters_indeks,
            belge_uzunluklari,
            ort_belge_uzunlugu,
        }
    }

    /// BM25 algoritması ile sorguyu puanlar (k1 = 1.2, b = 0.75).
    /// Azalan puana göre sıralanmış `(belge_indeksi, puan)` çiftlerini döndürür.
    pub fn ara(&self, sorgu: &str, limit: usize) -> Vec<(usize, f64)> {
        let n_docs = self.belgeler.len();
        let sorgu_tokenlar = belirtecle(sorgu);

        if n_docs == 0 || sorgu_tokenlar.is_empty() {
            return Vec::new();
        }

        let k1 = 1.2f64;
        let b = 0.75f64;
        let mut puanlar = vec![0.0f64; n_docs];

        for term in &sorgu_tokenlar {
            if let Some(postings) = self.ters_indeks.get(term) {
                let df = postings.len();
                let idf = (1.0 + (n_docs as f64 - df as f64 + 0.5) / (df as f64 + 0.5)).ln();
                for &(doc_idx, tf) in postings {
                    let doc_len = self.belge_uzunluklari[doc_idx] as f64;
                    let tf_bileseni = (tf as f64 * (k1 + 1.0))
                        / (tf as f64 + k1 * (1.0 - b + b * (doc_len / self.ort_belge_uzunlugu)));
                    puanlar[doc_idx] += idf * tf_bileseni;
                }
            }
        }

        let mut sonuclar: Vec<(usize, f64)> = puanlar
            .into_iter()
            .enumerate()
            .filter(|&(_, score)| score > 0.0)
            .collect();

        sonuclar.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sonuclar.truncate(limit);
        sonuclar
    }
}

/// İki kanalı birleştirir: her kanal KENDİ maksimumuna bölünüp ağırlıklı toplanır.
pub fn harmanla(
    bm: &[(usize, f64)],
    kos: &[(usize, f64)],
    agirlik: f64,
    limit: usize,
) -> Vec<(usize, f64)> {
    let en = |v: &[(usize, f64)]| v.iter().map(|(_, p)| *p).fold(0.0f64, f64::max);
    let (bm_en, kos_en) = (en(bm), en(kos));

    let mut birlesik: HashMap<usize, f64> = HashMap::new();
    for (i, p) in bm {
        let n = if bm_en > 0.0 { p / bm_en } else { 0.0 };
        *birlesik.entry(*i).or_default() += agirlik * n;
    }
    for (i, p) in kos {
        let n = if kos_en > 0.0 { p / kos_en } else { 0.0 };
        *birlesik.entry(*i).or_default() += (1.0 - agirlik) * n;
    }

    let mut liste: Vec<(usize, f64)> = birlesik.into_iter().collect();
    liste.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    liste.truncate(limit);
    liste
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belirtec_turkce_katlama() {
        assert_eq!(belirtecle("İSTANBUL"), vec!["istanbul"]);
        assert_eq!(belirtecle("kaynakça"), vec!["kaynakca"]);
        assert_eq!(belirtecle("ağaç şüphe örgü"), vec!["agac", "suphe", "orgu"]);
    }

    #[test]
    fn test_bm25_siralama() {
        let b1 = Belge {
            id: "b1".into(),
            ad: "Rust Kodlama Kılavuzu".into(),
            aciklama: "Rust dili ile hızlı ve güvenli kodlama rust kuralları".into(),
            tam_metin_md: "rust rust rust".into(),
        };

        let b2 = Belge {
            id: "b2".into(),
            ad: "Genel Sistem".into(),
            aciklama: "Bu belgede sadece bir kez rust kelimesi geçmektedir.".into(),
            tam_metin_md: "detaylar".into(),
        };

        let indeks = Indeks::kur(vec![b1, b2]);
        let sonuclar = indeks.ara("rust", 10);

        assert_eq!(sonuclar.len(), 2);
        assert_eq!(sonuclar[0].0, 0);
        assert!(sonuclar[0].1 > sonuclar[1].1);
    }
}
