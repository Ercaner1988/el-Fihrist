//! Ortak hata tipi — hem `main.rs` (ikili) hem `lib.rs` (kütüphane) kök
//! bağlamında `crate::CliError` / `crate::Result` olarak erişilebilir olması
//! için ayrı dosyada tutulur (gomme.rs bu isimlere `crate::` üzerinden bakar).

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Veritabanı hatası: {0}")]
    Database(#[from] turso::Error),
    #[error("IO hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("Girdi hatası: {0}")]
    Girdi(String),
}

pub type Result<T> = std::result::Result<T, CliError>;
