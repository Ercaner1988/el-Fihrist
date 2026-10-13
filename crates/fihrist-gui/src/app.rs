use eframe::egui;
use fihrist_core::{QueryFilter, Skill};
use fihrist_storage::{SkillStore, TursoDb, TursoSkillStore};
use kilim_tema::{Rol, RolVer};
use std::sync::mpsc::{channel, Receiver, Sender};

/// el-Fihrist masaüstü grafik arayüz uygulaması
pub struct FihristApp {
    pub search_query: String,
    pub skills: Vec<Skill>,
    pub selected_skill: Option<Skill>,
    pub status_message: String,
    tx: Sender<Vec<Skill>>,
    rx: Receiver<Vec<Skill>>,
}

impl FihristApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let app = Self::bos();
        let tx = app.tx.clone();

        // Başlangıçta veritabanından yetenekleri arka planda yükle
        let tx_initial = tx.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };

            rt.block_on(async move {
                if let Ok(db) = TursoDb::open_default().await {
                    if let Ok(conn) = db.connect() {
                        let store = TursoSkillStore::new(conn);
                        if let Ok(skills) = store.list_skills(500, 0).await {
                            let _ = tx_initial.send(skills);
                        }
                    }
                }
            });
        });

        app
    }

    /// Arka plan işi başlatmayan boş durum (`new` ve sınamalar).
    fn bos() -> Self {
        let (tx, rx) = channel();
        Self {
            search_query: String::new(),
            skills: Vec::new(),
            selected_skill: None,
            status_message: "Veritabanı bağlanıyor...".to_string(),
            tx,
            rx,
        }
    }

    fn trigger_search(&mut self) {
        let term = self.search_query.trim().to_string();
        self.status_message = format!("Aranıyor: {term}...");
        let tx_search = self.tx.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };

            rt.block_on(async move {
                if let Ok(db) = TursoDb::open_default().await {
                    if let Ok(conn) = db.connect() {
                        let store = TursoSkillStore::new(conn);
                        if term.is_empty() {
                            if let Ok(skills) = store.list_skills(500, 0).await {
                                let _ = tx_search.send(skills);
                            }
                        } else {
                            let filter = QueryFilter {
                                terim: term,
                                limit: 100,
                                ..Default::default()
                            };
                            if let Ok(results) = store.search_skills(&filter).await {
                                let skills = results.into_iter().map(|r| r.skill).collect();
                                let _ = tx_search.send(skills);
                            }
                        }
                    }
                }
            });
        });
    }
}

impl eframe::App for FihristApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ciz(ui);
    }
}

impl FihristApp {
    /// Bir kare: bölgeler `.rol(..)` ile işaretli (çerçeve/sarma/sabit genişlik roldan gelir).
    fn ciz(&mut self, ui: &mut egui::Ui) {
        // Arka plan iş parçacığından gelen verileri kontrol et
        if let Ok(loaded) = self.rx.try_recv() {
            self.status_message = format!("{} yetenek listelendi (Turso DB)", loaded.len());
            self.skills = loaded;
        }

        // Üst arama paneli
        egui::Panel::top("top_panel")
            .rol(Rol::UstCubuk)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("📖 el-Fihrist Yetenek Kütüphanesi");
                    ui.separator();
                    ui.label("Arama:");
                    let text_edit = ui.text_edit_singleline(&mut self.search_query);
                    if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.trigger_search();
                    }
                    if ui.button("Ara").clicked() {
                        self.trigger_search();
                    }
                    if ui.button("Sıfırla").clicked() {
                        self.search_query.clear();
                        self.trigger_search();
                    }
                });
            });

        // Alt durum çubuğu
        egui::Panel::bottom("bottom_panel")
            .rol(Rol::DurumCubugu)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Render: {}", crate::RENDER));
                    });
                });
            });

        // Sol panel: Yetenek Listesi
        egui::Panel::left("left_panel")
            .resizable(true)
            .default_size(280.0)
            .rol(Rol::YanPanel)
            .show(ui, |ui| {
                ui.heading("Yetenek Listesi");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.skills.is_empty() {
                        ui.label("Kayıtlı yetenek bulunamadı.");
                    } else {
                        for skill in &self.skills {
                            let is_selected = self
                                .selected_skill
                                .as_ref()
                                .map(|s| s.id == skill.id)
                                .unwrap_or(false);

                            if ui.selectable_label(is_selected, &skill.ad).clicked() {
                                self.selected_skill = Some(skill.clone());
                            }
                        }
                    }
                });
            });

        // Ana panel: Seçili Yetenek Detayları
        egui::CentralPanel::default()
            .rol(Rol::Zemin)
            .show(ui, |ui| {
                if let Some(skill) = &self.selected_skill {
                    ui.heading(&skill.ad);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(format!("Kimlik: {}", skill.id));
                        ui.separator();
                        ui.label(format!("Kategori: {}", skill.kategori));
                    });
                    if !skill.etiketler.is_empty() {
                        ui.label(format!("Etiketler: {}", skill.etiketler.join(", ")));
                    }
                    ui.separator();
                    ui.heading("Açıklama / Yönerge");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(&skill.aciklama);
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label("Detayları görüntülemek için soldan bir yetenek seçin.");
                    });
                }
            });
    }
}

#[cfg(test)]
mod sinama {
    use super::*;
    use kilim_tema::{acik, koyu, rol_kayitlari, Aydinlik, TemaTercihi, Varyant};

    /// `giydir` + bir kare; dönüş: bağlam (kayıtları ve görselleri okumak için).
    fn kare(aydinlik: Aydinlik) -> egui::Context {
        let ctx = egui::Context::default();
        let mut tercih = TemaTercihi::varsayilan(Varyant::CamGobegiAltin);
        tercih.aydinlik = aydinlik;
        kilim_tema::giydir(&ctx, &tercih);
        let mut app = FihristApp::bos();
        let girdi = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 600.0),
            )),
            ..Default::default()
        };
        // ilk kare yerleşimi oturtur; kayıt ikinci karede de aynıdır
        for _ in 0..2 {
            ctx.run_ui(girdi.clone(), |ui| app.ciz(ui))
                .textures_delta
                .clear();
        }
        ctx
    }

    #[test]
    fn bolgeler_kilim_defterinde_dogru_rolle() {
        let ctx = kare(Aydinlik::Acik);
        let k = rol_kayitlari(&ctx);
        let roller: Vec<Rol> = k.iter().map(|k| k.rol).collect();
        for r in [Rol::UstCubuk, Rol::DurumCubugu, Rol::YanPanel, Rol::Zemin] {
            assert_eq!(
                roller.iter().filter(|x| **x == r).count(),
                1,
                "{r:?}: {roller:?}"
            );
        }
        assert_eq!(roller.len(), 4, "beklenmeyen ek bölge: {roller:?}");
        assert!(k.iter().all(|k| k.uyumlu), "uyumsuz rol: {k:?}");
        let yan = k.iter().find(|k| k.rol == Rol::YanPanel).unwrap();
        assert!(yan.sabit_genislik, "yan panel sabit genişlikte olmalı");
    }

    #[test]
    fn giydir_sonrasi_gorsel_kilim_paletinden() {
        for (a, p, koyu_mu) in [
            (Aydinlik::Acik, acik(), false),
            (Aydinlik::Koyu, koyu(), true),
        ] {
            let ctx = kare(a);
            let v = ctx.global_style().visuals.clone();
            assert_eq!(v.dark_mode, koyu_mu);
            assert_eq!(v.panel_fill, p.zem, "panel_fill palet zemi değil");
            assert_eq!(v.window_fill, p.yuzey);
            assert_eq!(v.override_text_color, Some(p.murekkep));
            assert_ne!(v.error_fg_color, egui::Visuals::light().error_fg_color);
        }
        // sarma_kur giydir'in içinde: yatay satırda da sarma açık
        let ctx = kare(Aydinlik::Acik);
        assert_eq!(ctx.global_style().wrap_mode, Some(egui::TextWrapMode::Wrap));
    }
}
