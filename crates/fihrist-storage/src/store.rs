use fihrist_core::{FihristError, MatchType, QueryFilter, Result, SearchResult, Skill};
use turso::{params, Connection};

/// Yetenek veri depolama arayüzü
pub trait SkillStore {
    fn count_skills(&self) -> impl std::future::Future<Output = Result<usize>> + Send;
    fn list_skills(
        &self,
        limit: usize,
        offset: usize,
    ) -> impl std::future::Future<Output = Result<Vec<Skill>>> + Send;
    fn get_skill(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<Skill>>> + Send;
    fn search_skills(
        &self,
        filter: &QueryFilter,
    ) -> impl std::future::Future<Output = Result<Vec<SearchResult>>> + Send;
}

/// Turso bağlantısı üzerinden çalışan SkillStore implementasyonu
pub struct TursoSkillStore {
    conn: Connection,
}

impl TursoSkillStore {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

impl SkillStore for TursoSkillStore {
    async fn count_skills(&self) -> Result<usize> {
        let mut rows = self
            .conn
            .query("SELECT count(*) FROM yetenekler", ())
            .await
            .map_err(|e| FihristError::Database(format!("Sorgu hatası: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| FihristError::Database(format!("Satır okuma hatası: {e}")))?
        {
            let count: i64 = row
                .get(0)
                .map_err(|e| FihristError::Database(format!("Sütun hatası: {e}")))?;
            Ok(count as usize)
        } else {
            Ok(0)
        }
    }

    async fn list_skills(&self, limit: usize, offset: usize) -> Result<Vec<Skill>> {
        let sql = "SELECT id, ad, aciklama, kategori, etiketler FROM yetenekler ORDER BY id LIMIT ? OFFSET ?";
        let mut rows = self
            .conn
            .query(sql, params![limit as i64, offset as i64])
            .await
            .map_err(|e| FihristError::Database(format!("Sorgu hatası: {e}")))?;

        let mut skills = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| FihristError::Database(format!("Satır okuma hatası: {e}")))?
        {
            let id: String = row.get(0).unwrap_or_default();
            let ad: String = row.get(1).unwrap_or_default();
            let aciklama: String = row.get(2).unwrap_or_default();
            let kategori: String = row.get(3).unwrap_or_default();
            let etiketler_raw: String = row.get(4).unwrap_or_default();
            let etiketler: Vec<String> = etiketler_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            skills.push(Skill {
                id,
                ad,
                aciklama,
                kategori,
                etiketler,
                dosya_yolu: None,
                olusturma_tarihi: None,
                guncelleme_tarihi: None,
            });
        }

        Ok(skills)
    }

    async fn get_skill(&self, id: &str) -> Result<Option<Skill>> {
        let sql = "SELECT id, ad, aciklama, kategori, etiketler FROM yetenekler WHERE id = ?";
        let mut rows = self
            .conn
            .query(sql, params![id])
            .await
            .map_err(|e| FihristError::Database(format!("Sorgu hatası: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| FihristError::Database(format!("Satır okuma hatası: {e}")))?
        {
            let id: String = row.get(0).unwrap_or_default();
            let ad: String = row.get(1).unwrap_or_default();
            let aciklama: String = row.get(2).unwrap_or_default();
            let kategori: String = row.get(3).unwrap_or_default();
            let etiketler_raw: String = row.get(4).unwrap_or_default();
            let etiketler: Vec<String> = etiketler_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            Ok(Some(Skill {
                id,
                ad,
                aciklama,
                kategori,
                etiketler,
                dosya_yolu: None,
                olusturma_tarihi: None,
                guncelleme_tarihi: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn search_skills(&self, filter: &QueryFilter) -> Result<Vec<SearchResult>> {
        let limit = if filter.limit == 0 { 20 } else { filter.limit };
        let pattern = format!("%{}%", filter.terim);

        let sql = "SELECT id, ad, aciklama, kategori, etiketler FROM yetenekler \
                   WHERE ad LIKE ? OR aciklama LIKE ? OR etiketler LIKE ? \
                   LIMIT ?";

        let mut rows = self
            .conn
            .query(
                sql,
                params![pattern.clone(), pattern.clone(), pattern, limit as i64],
            )
            .await
            .map_err(|e| FihristError::Database(format!("Arama hatası: {e}")))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| FihristError::Database(format!("Satır okuma hatası: {e}")))?
        {
            let id: String = row.get(0).unwrap_or_default();
            let ad: String = row.get(1).unwrap_or_default();
            let aciklama: String = row.get(2).unwrap_or_default();
            let kategori: String = row.get(3).unwrap_or_default();
            let etiketler_raw: String = row.get(4).unwrap_or_default();
            let etiketler: Vec<String> = etiketler_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let skill = Skill {
                id,
                ad,
                aciklama,
                kategori,
                etiketler,
                dosya_yolu: None,
                olusturma_tarihi: None,
                guncelleme_tarihi: None,
            };

            results.push(SearchResult {
                skill,
                skor: 1.0,
                eslesme_tipi: MatchType::FullText,
            });
        }

        Ok(results)
    }
}
