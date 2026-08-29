use std::fmt;

/// hermes-tools-core için birleşik hata enum yapısı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Ayrıştırma/Ayrıştırma mantığı hatası (örn. bozuk HTML/XML etiketleri)
    ParseError(String),
    /// Geçersiz girdi (boş dize, izin verilmeyen uzantı vb.)
    InvalidInput(String),
    /// Masa Döngüsü kapı doğrulaması veya iş kuralı ihlali
    ValidationFailed(String),
    /// Karakter kodlama veya dönüştürme hatası
    EncodingError(String),
    /// Azami girdi veya derinlik sınırı aşıldı (DoS/OOM koruması)
    MaxLimitExceeded(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::ParseError(s) => write!(f, "Ayrıştırma hatası: {}", s),
            ToolError::InvalidInput(s) => write!(f, "Geçersiz girdi: {}", s),
            ToolError::ValidationFailed(s) => write!(f, "Doğrulama başarısız: {}", s),
            ToolError::EncodingError(s) => write!(f, "Kodlama hatası: {}", s),
            ToolError::MaxLimitExceeded(s) => write!(f, "Azami sınır aşıldı: {}", s),
        }
    }
}

impl std::error::Error for ToolError {}

pub type ToolResult<T> = Result<T, ToolError>;
