//! # fihrist-gui
//!
//! el-Fihrist için masaüstü grafik arayüz dirmesi (`egui` / `eframe`).
//!
//! ## Render seçimi
//! - Windows ve diğerleri: WGPU (DirectX 12). Glow/OpenGL Windows'ta AMD/Intel'de
//!   pencereyi düz kara açar (Wgl sRGB swap chain).
//! - Linux/Pardus: Glow, açıkça. `wgpu` + Wayland'da gözlemlenen girdi kilitlenmesini
//!   önler ve Mesa `llvmpipe` yazılımsal render'ını destekler.

pub mod app;

pub use app::FihristApp;

#[cfg(target_os = "linux")]
pub const RENDER: eframe::Renderer = eframe::Renderer::Glow;
#[cfg(not(target_os = "linux"))]
pub const RENDER: eframe::Renderer = eframe::Renderer::Wgpu;

/// Pencere simgesi (kök dizindeki `icon.png`'nin 256 px'i; `.mcpb` paketi de onu taşır).
fn pencere_simgesi() -> eframe::egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../ikon-256.png"))
        .expect("ikon-256.png geçerli bir PNG")
}

/// Masaüstü uygulamasını başlatır
pub fn run_fihrist_gui() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_transparent(false)
            .with_title("el-Fihrist")
            .with_icon(std::sync::Arc::new(pencere_simgesi())),
        renderer: RENDER,
        // vsync: eframe 0.36'da alan yok; wgpu varsayılanı PresentMode::AutoVsync.
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    eframe::run_native(
        "el-Fihrist",
        native_options,
        Box::new(|cc| Ok(Box::new(FihristApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn pencere_simgesi_cozulur() {
        let s = super::pencere_simgesi();
        assert_eq!((s.width, s.height), (256, 256));
    }
}
