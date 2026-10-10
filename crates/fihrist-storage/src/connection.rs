use fihrist_core::{FihristError, Result};
use std::path::{Path, PathBuf};
use turso::{Builder, Connection, Database};

/// Turso veritabanı yönetim ve bağlantı yapısı
pub struct TursoDb {
    db_path: PathBuf,
    db: Database,
}

impl TursoDb {
    /// Verilen yoldaki veritabanı dosyasını açar
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let db = Builder::new_local(p.to_string_lossy().as_ref())
            .build()
            .await
            .map_err(|e| FihristError::Database(format!("Turso build hatası: {e}")))?;

        Ok(Self { db_path: p, db })
    }

    /// Ortam değişkenlerinden veya bilinen aday konumlardan veritabanını bulur ve açar
    pub async fn open_default() -> Result<Self> {
        let path = find_database_path()?;
        Self::open(path).await
    }

    /// Yeni bir Turso bağlantısı üretir
    pub fn connect(&self) -> Result<Connection> {
        self.db
            .connect()
            .map_err(|e| FihristError::Database(format!("Turso connect hatası: {e}")))
    }

    /// Veritabanı dosya yolunu döndürür
    pub fn path(&self) -> &Path {
        &self.db_path
    }
}

/// Veritabanı dosya yolunu arar; sıra CLI ile aynıdır (`fihrist_core::konum::kutuphane_yolu`).
pub fn find_database_path() -> Result<PathBuf> {
    fihrist_core::konum::kutuphane_yolu().map_err(FihristError::Config)
}
