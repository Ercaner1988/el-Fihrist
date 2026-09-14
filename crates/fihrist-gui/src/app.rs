use eframe::egui;
use fihrist_core::{QueryFilter, Skill};
use fihrist_storage::{SkillStore, TursoDb, TursoSkillStore};
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
        let (tx, rx) = channel();

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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Arka plan iş parçacığından gelen verileri kontrol et
        if let Ok(loaded) = self.rx.try_recv() {
            self.status_message = format!("{} yetenek listelendi (Turso DB)", loaded.len());
            self.skills = loaded;
        }

        // Üst arama paneli
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
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
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Backend: Glow (Pardus / Linux Uyumlu)");
                });
            });
        });

        // Sol panel: Yetenek Listesi
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
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
        egui::CentralPanel::default().show(ctx, |ui| {
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
