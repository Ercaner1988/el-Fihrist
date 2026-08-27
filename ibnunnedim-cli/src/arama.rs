use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Belge {
    #[allow(dead_code)]
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
/// Standard Rust `.to_lowercase()` fonksiyonu `İ` karakterini `i` + `U+0307` (birleşen nokta)
/// olarak ayrıştırır ve `I` karakterini `ı` yerine `i` yapar. Bu yüzden Türkçe katlama
/// elle yapılır: `İ` -> `i`, `I` -> `ı`, ardından `.to_lowercase()`.
pub fn belirtecle(metin: &str) -> Vec<String> {
    let mut katlanmis = String::with_capacity(metin.len());
    for c in metin.chars() {
        match c {
            'İ' => katlanmis.push('i'),
            'I' => katlanmis.push('ı'),
            other => katlanmis.extend(other.to_lowercase().chars()),
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

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn belirtec_turkce_katlama() {
        // Rust'ın varsayılan to_lowercase()'inin 'İ' için 'i' dönmediğini kanıtla
        assert_ne!("İ".to_lowercase(), "i");

        let res_istanbul = belirtecle("İSTANBUL");
        assert_eq!(res_istanbul, vec!["istanbul"]);

        let res_isik = belirtecle("IŞIK");
        assert_eq!(res_isik, vec!["ışık"]);
    }

    #[test]
    fn bm25_ilgiliyi_one_alir() {
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
        let (ilk_idx, ilk_puan) = sonuclar[0];
        let (_ikinci_idx, ikinci_puan) = sonuclar[1];

        assert_eq!(ilk_idx, 0, "Rust başlığında geçen belge ilk sırada olmalı");
        assert!(
            ilk_puan > ikinci_puan,
            "Terimi çok içeren belgenin puanı kesin büyük olmalı ({ilk_puan} > {ikinci_puan})"
        );
    }

    #[test]
    fn bm25_gecmeyeni_dondurmez() {
        let b1 = Belge {
            id: "b1".into(),
            ad: "Python Kütüphanesi".into(),
            aciklama: "Python projeleri için araçlar".into(),
            tam_metin_md: "kodlar".into(),
        };

        let indeks = Indeks::kur(vec![b1]);
        let sonuclar = indeks.ara("rust", 10);

        assert!(
            sonuclar.is_empty(),
            "Derlemde bulunmayan terim için sonuç boş dönmeli"
        );
    }
}
