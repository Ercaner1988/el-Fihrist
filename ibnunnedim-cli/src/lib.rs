//! ibnunnedim-cli'nin kütüphane yüzeyi.
//!
//! Not (pasli-beyin entegrasyonu, 2026-08-29): BM25 arama (arama.rs) yalnız
//! ikili içinden erişilebilirdi; kütüphane tüketicileri için dışa açıldı.
//! main.rs'teki `mod arama` aynen kalır — ikili davranışı değişmez.
pub mod arama;
