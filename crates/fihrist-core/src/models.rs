use serde::{Deserialize, Serialize};

/// el-Fihrist ana yetenek (skill) modeli
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub ad: String,
    pub aciklama: String,
    pub kategori: String,
    pub etiketler: Vec<String>,
    pub dosya_yolu: Option<String>,
    pub olusturma_tarihi: Option<String>,
    pub guncelleme_tarihi: Option<String>,
}

/// Arama sorgusu ve filtreleme parametreleri
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilter {
    pub terim: String,
    pub kategori: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

/// Arama sonucu kaydı
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub skill: Skill,
    pub skor: f64,
    pub eslesme_tipi: MatchType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Exact,
    FullText,
    Vector,
}
