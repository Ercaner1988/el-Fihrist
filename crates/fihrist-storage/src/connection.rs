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

/// Veritabanı dosya yolunu arar:
/// 1. `TURSO_DB_PATH` ortam değişkeni
/// 2. Yerel dosya `kutup_kutuphane.db`
/// 3. Bilinen standart dizinler (`kutuphane/`, `hermes yazılım/kutuphane/`)
pub fn find_database_path() -> Result<PathBuf> {
    if let Ok(env_path) = std::env::var("TURSO_DB_PATH") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
    }

    let candidates = [
        PathBuf::from("kutup_kutuphane.db"),
        PathBuf::from("../kutuphane/kutup_kutuphane.db"),
        PathBuf::from("kutuphane/kutup_kutuphane.db"),
    ];

    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }

    // Windows OneDrive / Desktop fallback
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(userprofile)
            .join("Desktop")
            .join("hermes yazılım")
            .join("kutuphane")
            .join("kutup_kutuphane.db");
        if p.exists() {
            return Ok(p);
        }
    }

    Err(FihristError::Config(
        "kutup_kutuphane.db dosyası bulunamadı. Lütfen TURSO_DB_PATH ortam değişkenini ayarlayın."
            .into(),
    ))
}
