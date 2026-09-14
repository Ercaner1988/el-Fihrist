use crate::bm25::{harmanla, Belge, Indeks};
use fihrist_core::{MatchType, QueryFilter, SearchResult, Skill};

/// Hibrit Arama Pipeline Yapısı
pub struct SearchPipeline {
    indeks: Indeks,
    skills: Vec<Skill>,
}

impl SearchPipeline {
    /// Yetenek listesinden arama hattını başlatır
    pub fn new(skills: Vec<Skill>) -> Self {
        let belgeler = skills
            .iter()
            .map(|s| Belge {
                id: s.id.clone(),
                ad: s.ad.clone(),
                aciklama: s.aciklama.clone(),
                tam_metin_md: format!("{} {} {}", s.ad, s.aciklama, s.etiketler.join(" ")),
            })
            .collect();

        let indeks = Indeks::kur(belgeler);
        Self { indeks, skills }
    }

    /// BM25 arama sorgusu yürütür ve SearchResult listesi döner
    pub fn search(&self, filter: &QueryFilter) -> Vec<SearchResult> {
        let limit = if filter.limit == 0 { 20 } else { filter.limit };
        let sonuclar = self.indeks.ara(&filter.terim, limit);

        sonuclar
            .into_iter()
            .filter_map(|(idx, skor)| {
                self.skills.get(idx).map(|skill| SearchResult {
                    skill: skill.clone(),
                    skor,
                    eslesme_tipi: MatchType::FullText,
                })
            })
            .collect()
    }

    /// BM25 ve Vektör skorlarını harmanlayarak arama yapar
    pub fn search_hybrid(
        &self,
        terim: &str,
        vektor_puanlari: &[(usize, f64)],
        agirlik: f64,
        limit: usize,
    ) -> Vec<SearchResult> {
        let bm_puanlari = self.indeks.ara(terim, limit * 2);
        let birlesik = harmanla(&bm_puanlari, vektor_puanlari, agirlik, limit);

        birlesik
            .into_iter()
            .filter_map(|(idx, skor)| {
                self.skills.get(idx).map(|skill| SearchResult {
                    skill: skill.clone(),
                    skor,
                    eslesme_tipi: MatchType::Exact,
                })
            })
            .collect()
    }
}
