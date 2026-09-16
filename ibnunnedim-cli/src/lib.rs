//! ibnunnedim-cli'nin kütüphane yüzeyi.
//!
//! Not (pasli-beyin entegrasyonu, 2026-08-29): BM25 arama (arama.rs) yalnız
//! ikili içinden erişilebilirdi; kütüphane tüketicileri için dışa açıldı.
//! main.rs'teki `mod arama` aynen kalır — ikili davranışı değişmez.
//!
//! Not (pasli-beyin gömme köprüsü): gomme.rs `crate::Result`/`crate::CliError`
//! kullanır; bu isimler kök bağlamda bulunsun diye error.rs ayrı dosyada
//! tutulur ve hem burada hem main.rs'te `mod error` ile içeri alınıp kökte
//! yeniden dışa verilir (`pub use`). main.rs'teki `mod gomme` aynen kalır.
pub mod arama;
mod error;
pub mod gomme;
pub use error::{CliError, Result};
