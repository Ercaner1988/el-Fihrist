//! # fihrist-gui
//!
//! el-Fihrist için masaüstü grafik arayüz dirmesi (`egui` / `eframe`).
//!
//! ## Pardus / Linux ve Donanım Uyumluluk İlkesi:
//! Linux ve Pardus sistemlerinde `wgpu` + Wayland kombinasyonunda gözlemlenen
//! girdi (fare/klavye) kilitlenme sorunlarını önlemek ve eski donanımlardaki
//! (Mesa `llvmpipe`) yazılımsal render uyumluluğunu garantilemek amacıyla
//! Linux derlemelerinde açıkça `eframe::Renderer::Glow` (`Renderer::Glow`) seçilir.

pub mod app;

pub use app::FihristApp;

/// Masaüstü uygulamasını başlatır
pub fn run_fihrist_gui() -> eframe::Result {
    let mut native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("el-Fihrist"),
        ..Default::default()
    };

    // Linux ve Pardus sistemlerinde wgpu Wayland girdi kilitlenmelerini önlemek
    // ve Mesa llvmpipe yazılımsal render'ı desteklemek için Glow açıkça atanır.
    #[cfg(target_os = "linux")]
    {
        native_options.renderer = eframe::Renderer::Glow;
    }

    // Windows ve diğer platformlarda da güvenli ve hafif render için Glow tercih edilebilir.
    #[cfg(not(target_os = "linux"))]
    {
        native_options.renderer = eframe::Renderer::Glow;
    }

    eframe::run_native(
        "el-Fihrist",
        native_options,
        Box::new(|cc| Ok(Box::new(FihristApp::new(cc)))),
    )
}
