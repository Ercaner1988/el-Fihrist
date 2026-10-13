//! # fihrist-gui
//!
//! el-Fihrist için masaüstü grafik arayüz dirmesi (`egui` / `eframe`).
//!
//! ## Render seçimi
//! - Windows ve diğerleri: WGPU; arka uç Vulkan önce, sürücü yoksa DX12 (AMD DX12 gölgelendirici
//!   derleyicisi her açılışta ~140 sn CPU yer). Glow/OpenGL Windows'ta AMD/Intel'de pencereyi düz
//!   kara açar (Wgl sRGB swap chain).
//! - Linux/Pardus: Glow, açıkça. `wgpu` + Wayland'da gözlemlenen girdi kilitlenmesini
//!   önler ve Mesa `llvmpipe` yazılımsal render'ını destekler.

pub mod app;

pub use app::FihristApp;

#[cfg(target_os = "linux")]
pub const RENDER: eframe::Renderer = eframe::Renderer::Glow;
#[cfg(not(target_os = "linux"))]
pub const RENDER: eframe::Renderer = eframe::Renderer::Wgpu;

/// Pencere simgesi (kök dizindeki `icon.png`'nin 256 px'i; `.mcpb` paketi de onu taşır).
/// Çözülemezse None: pencere simgesiz açılır, çökmez (geçerliliği `testler` denetler).
fn pencere_simgesi() -> Option<eframe::egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../ikon-256.png")).ok()
}

/// Bu makinede Vulkan bağdaştırıcısı var mı (CPU/yazılım bağdaştırıcısı sayılmaz).
/// İkisi birden açılırsa DX12 de yüklenir (+64 MB), o yüzden önce yalnız Vulkan yoklanır.
#[cfg(not(target_os = "linux"))]
fn vulkan_var() -> bool {
    use eframe::egui_wgpu::wgpu;
    let mut tanim = wgpu::InstanceDescriptor::new_without_display_handle();
    tanim.backends = wgpu::Backends::VULKAN;
    let ornek = wgpu::Instance::new(tanim);
    pollster::block_on(ornek.enumerate_adapters(wgpu::Backends::VULKAN))
        .iter()
        .any(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
}

/// wgpu ayarı: Windows/macOS'ta arka uç Vulkan önce, yoksa DX12. vsync: wgpu varsayılanı AutoVsync
/// (eframe 0.36'da `NativeOptions`'ta alan yok). Linux Glow kullanır, wgpu ayarı varsayılan.
#[cfg(not(target_os = "linux"))]
fn wgpu_ayari() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{wgpu::Backends, WgpuConfiguration, WgpuSetup};
    let mut ayar = WgpuConfiguration::default();
    if let WgpuSetup::CreateNew(yeni) = &mut ayar.wgpu_setup {
        yeni.instance_descriptor.backends = if vulkan_var() {
            Backends::VULKAN
        } else {
            Backends::DX12
        };
    }
    ayar
}

#[cfg(target_os = "linux")]
fn wgpu_ayari() -> eframe::egui_wgpu::WgpuConfiguration {
    eframe::egui_wgpu::WgpuConfiguration::default()
}

/// Masaüstü uygulamasını başlatır
pub fn run_fihrist_gui() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([900.0, 600.0])
        .with_min_inner_size([400.0, 300.0])
        .with_transparent(false)
        .with_title("el-Fihrist");
    if let Some(simge) = pencere_simgesi() {
        viewport = viewport.with_icon(std::sync::Arc::new(simge));
    }
    let native_options = eframe::NativeOptions {
        viewport,
        renderer: RENDER,
        wgpu_options: wgpu_ayari(),
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    eframe::run_native(
        "el-Fihrist",
        native_options,
        Box::new(|cc| {
            // giydir = iki tema + sarma_kur + tercih; roller `app.rs`te `.rol(..)` ile işaretli.
            let tercih = kilim_tema::TemaTercihi::yukle(
                "el-fihrist",
                kilim_tema::TemaTercihi::varsayilan(kilim_tema::Varyant::CamGobegiAltin),
            );
            kilim_tema::giydir(&cc.egui_ctx, &tercih);
            Ok(Box::new(FihristApp::new(cc)))
        }),
    )
}

#[cfg(test)]
mod tests {
    /// Windows kuralı: yalnız bir arka uç açılır (Vulkan ya da DX12), ikisi birden değil.
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn wgpu_arka_ucu_tek_ve_vulkan_ya_da_dx12() {
        use eframe::egui_wgpu::{wgpu::Backends, WgpuSetup};
        let WgpuSetup::CreateNew(y) = super::wgpu_ayari().wgpu_setup else {
            panic!("CreateNew bekleniyordu");
        };
        let b = y.instance_descriptor.backends;
        assert!(b == Backends::VULKAN || b == Backends::DX12, "{b:?}");
    }

    #[test]
    fn pencere_simgesi_cozulur() {
        let Some(s) = super::pencere_simgesi() else {
            panic!("ikon-256.png çözülemedi: pencere simgesiz açılır");
        };
        assert_eq!((s.width, s.height), (256, 256));
    }
}
