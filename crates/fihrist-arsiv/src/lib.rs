//! Oturum arşivi (CONTEXT.md "Oturum arşivi"): Claude Code dökümlerinin özetleme sınırları arasında kalan
//! bölümlerinin aranabilir kaydı. Mahremiyet katmanı **Kişisel**: Köprülü'nün süzgeci gelene dek
//! yalnız CLI'dan açılır, MCP'ye BAĞLANMAZ (bulut model okurdu).

pub mod ara;
pub mod depo;
pub mod dokum;
pub mod esle;
pub mod komut;
pub mod zaman;

pub use depo::{Depo, Ozet};

#[cfg(test)]
mod sinama;
