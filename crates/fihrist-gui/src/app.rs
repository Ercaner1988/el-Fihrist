use eframe::egui;
use fihrist_core::{QueryFilter, Skill};
use fihrist_storage::{SkillStore, TursoDb, TursoSkillStore};
use kilim_tema::{alan_satiri, Rol, RolVer};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Arka plandan gelen sonuç: liste ya da durum çubuğunda gösterilecek hata iletisi.
type Sonuc = Result<Vec<Skill>, String>;
/// Kanaldaki zarf: (yükleme sıra numarası, sonuç). Yalnız en yeni sıra kabul edilir.
type Zarf = (u64, Sonuc);

/// el-Fihrist masaüstü grafik arayüz uygulaması
pub struct FihristApp {
    pub search_query: String,
    pub skills: Vec<Skill>,
    pub selected_skill: Option<Skill>,
    pub status_message: String,
    /// Son başlatılan yüklemenin sıra numarası: eski aramanın geç gelen sonucu yenisini ezmesin.
    sira: u64,
    tx: Sender<Zarf>,
    rx: Receiver<Zarf>,
    /// Sonuç gelince çizimi uyandırır (boşta yük sıfır: çizim yalnız girdi ya da veri gelince).
    ctx: Option<egui::Context>,
}

impl FihristApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::kur(cc, gercek_getir)
    }

    /// `new`in dikişi: bağlamı saklar, başlangıçta yetenekleri arka planda yükler (`getir` sınamada sahte).
    fn kur(
        cc: &eframe::CreationContext<'_>,
        getir: impl FnOnce(String) -> Sonuc + Send + 'static,
    ) -> Self {
        let mut app = Self::bos(Some(cc.egui_ctx.clone()));
        drop(app.yukle_ile(String::new(), getir));
        app
    }

    /// Arka plan işi başlatmayan boş durum (`new` ve sınamalar).
    fn bos(ctx: Option<egui::Context>) -> Self {
        let (tx, rx) = channel();
        Self {
            search_query: String::new(),
            skills: Vec::new(),
            selected_skill: None,
            status_message: "Veritabanı bağlanıyor...".to_string(),
            sira: 0,
            tx,
            rx,
            ctx,
        }
    }

    fn trigger_search(&mut self) {
        let term = self.search_query.trim().to_string();
        self.status_message = format!("Aranıyor: {term}...");
        self.yukle(term);
    }

    /// Boş terim: ilk 500 yetenek; doluysa arama. Sonuç ya da hata kanaldan gelir.
    fn yukle(&mut self, term: String) {
        drop(self.yukle_ile(term, gercek_getir));
    }

    /// `yukle`nin dikişi: `getir` arka plan iş parçacığında koşar (sınamada sahte verilir). Her çağrı
    /// yeni bir sıra numarası alır; yalnız en yenisinin sonucu `ciz`de kabul edilir.
    /// Tutamaç yalnız sınamanın iş parçacığını beklemesi içindir (üretim bırakır, iş kendi biter).
    fn yukle_ile(
        &mut self,
        term: String,
        getir: impl FnOnce(String) -> Sonuc + Send + 'static,
    ) -> std::thread::JoinHandle<()> {
        self.sira = self.sira.wrapping_add(1);
        let sira = self.sira;
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || gonder(&tx, ctx.as_ref(), sira, getir(term)))
    }
}

/// Gerçek veritabanı yükleyicisi: kendi tek iş parçacıklı çalışma zamanında `getir`i koşar.
fn gercek_getir(term: String) -> Sonuc {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Çalışma zamanı kurulamadı: {e}"))
        .and_then(|rt| rt.block_on(getir(term)))
}

/// Sonucu kanala yazar, sonra (bağlam varsa) çizimi uyandırır.
fn gonder(tx: &Sender<Zarf>, ctx: Option<&egui::Context>, sira: u64, sonuc: Sonuc) {
    let _ = tx.send((sira, sonuc));
    if let Some(ctx) = ctx {
        ctx.request_repaint();
    }
}

async fn getir(term: String) -> Sonuc {
    let db = TursoDb::open_default()
        .await
        .map_err(|e| format!("Veritabanı açılamadı: {e}"))?;
    let conn = db
        .connect()
        .map_err(|e| format!("Veritabanı bağlantısı kurulamadı: {e}"))?;
    let store = TursoSkillStore::new(conn);
    if term.is_empty() {
        return store
            .list_skills(500, 0)
            .await
            .map_err(|e| format!("Liste okunamadı: {e}"));
    }
    let filter = QueryFilter {
        terim: term,
        limit: 100,
        ..Default::default()
    };
    store
        .search_skills(&filter)
        .await
        .map(|r| r.into_iter().map(|r| r.skill).collect())
        .map_err(|e| format!("Arama başarısız: {e}"))
}

/// Sol sütun genişliği: en uzun adın tek satır genişliği, tavan kullanılabilir genişliğin %45'i.
fn anahtar_genisligi(ui: &egui::Ui, adlar: &[&str]) -> f32 {
    let yazi = egui::TextStyle::Body.resolve(ui.style());
    let en_uzun = adlar
        .iter()
        .map(|a| {
            ui.painter()
                .layout_no_wrap(a.to_string(), yazi.clone(), egui::Color32::WHITE)
                .size()
                .x
        })
        .fold(0.0, f32::max);
    (en_uzun + ui.spacing().item_spacing.x).min(ui.available_width() * 0.45)
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
        while let Ok((sira, sonuc)) = self.rx.try_recv() {
            if sira != self.sira {
                continue; // eski aramanın geç gelen sonucu yenisini ezmesin
            }
            match sonuc {
                Ok(loaded) => {
                    self.status_message = format!("{} yetenek listelendi (Turso DB)", loaded.len());
                    self.skills = loaded;
                }
                Err(hata) => self.status_message = hata,
            }
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
                    // Uzun kimlik dar merkezde yan yana sığmaz: sabit sol sütunlu satır (Grid/horizontal değil).
                    let etiketler = skill.etiketler.join(", ");
                    let satirlar = [
                        ("Kimlik", skill.id.as_str()),
                        ("Kategori", skill.kategori.as_str()),
                        ("Etiketler", etiketler.as_str()),
                    ];
                    let sol = anahtar_genisligi(ui, &satirlar.map(|(a, _)| a));
                    for (anahtar, deger) in satirlar {
                        if !deger.is_empty() {
                            alan_satiri(ui, sol, anahtar, |ui| ui.label(deger));
                        }
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// `giydir` + bir kare; dönüş: bağlam (kayıtları ve görselleri okumak için).
    fn kare(aydinlik: Aydinlik) -> egui::Context {
        let ctx = egui::Context::default();
        let mut tercih = TemaTercihi::varsayilan(Varyant::CamGobegiAltin);
        tercih.aydinlik = aydinlik;
        kilim_tema::giydir(&ctx, &tercih);
        let mut app = FihristApp::bos(None);
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
    fn arka_plan_hatasi_durum_cubuguna_duser_sessiz_kalmaz() {
        let mut app = FihristApp::bos(None);
        gonder(&app.tx, None, 0, Err("Veritabanı açılamadı: deneme".into()));
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| app.ciz(ui))
            .textures_delta
            .clear();
        assert_eq!(app.status_message, "Veritabanı açılamadı: deneme");
        gonder(&app.tx, None, 0, Ok(Vec::new()));
        ctx.run_ui(egui::RawInput::default(), |ui| app.ciz(ui))
            .textures_delta
            .clear();
        assert!(
            app.status_message.contains("0 yetenek"),
            "{}",
            app.status_message
        );
    }

    #[test]
    fn sonuc_gelince_cizim_uyandirilir() {
        let (tx, rx) = channel();
        let (ctx, sayac) = sayacli_baglam();
        gonder(&tx, Some(&ctx), 1, Ok(Vec::new()));
        assert_eq!(sayac.load(Ordering::SeqCst), 1, "tam bir kez istenmeli");
        assert!(rx.try_recv().is_ok());
    }

    /// Gerçek bağ 1: `new`in kurduğu uygulama bağlamı saklar ve ilk yükleme sonucu çizimi uyandırır.
    #[test]
    fn new_baglami_saklar_ilk_yukleme_cizimi_uyandirir() {
        let (ctx, sayac) = sayacli_baglam();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = FihristApp::kur(&cc, |_| Ok(Vec::new()));
        bekle(&sayac, 1);
        kare_ciz(&ctx, &mut app, 900.0);
        assert!(app.status_message.contains("0 yetenek"));
    }

    /// Gerçek bağ 2: `yukle` bağlamı arka plana taşır (arama sonucu da çizimi uyandırır).
    #[test]
    fn yukle_baglami_arka_plana_tasir() {
        let (ctx, sayac) = sayacli_baglam();
        let mut app = FihristApp::bos(Some(ctx));
        app.yukle_ile("x".into(), |_| Ok(Vec::new()));
        bekle(&sayac, 1);
    }

    /// Yarış: eski aramanın geç gelen sonucu yeni aramayı (ve durum çubuğunu) ezmez.
    #[test]
    fn eski_arama_sonucu_yeniyi_ezmez() {
        let (ctx, sayac) = sayacli_baglam();
        let mut app = FihristApp::bos(Some(ctx.clone()));
        let (serbest, bekle_eski) = channel::<()>();
        // İş parçacıkları tutamaçla beklenir: yeniden çizim isteklerini saymak güvenilmez (bekleyen bir
        // istek varken egui geri çağrıyı yeniden tetiklemez).
        let eski = app.yukle_ile(String::new(), move |_| {
            bekle_eski.recv().unwrap_or_default(); // yavaş liste yüklemesi
            Ok(vec![beceri("eski", "x")])
        });
        app.yukle_ile("yeni".into(), |_| Ok(Vec::new()))
            .join()
            .unwrap();
        kare_ciz(&ctx, &mut app, 900.0);
        assert!(
            app.status_message.contains("0 yetenek"),
            "{}",
            app.status_message
        );
        serbest.send(()).unwrap();
        eski.join().unwrap(); // eski sonuç kanalda
        drop(sayac);
        kare_ciz(&ctx, &mut app, 900.0);
        assert!(app.skills.is_empty(), "eski sonuç yeniyi ezdi");
        assert!(
            app.status_message.contains("0 yetenek"),
            "{}",
            app.status_message
        );
    }

    /// Dar pencere (400 px) + uzun, boşluksuz kimlik: hiçbir öğe pencere dışına taşmaz.
    #[test]
    fn dar_pencerede_uzun_kimlik_tasmaz() {
        let ctx = egui::Context::default();
        kilim_tema::giydir(&ctx, &TemaTercihi::varsayilan(Varyant::CamGobegiAltin));
        let mut app = FihristApp::bos(None);
        app.selected_skill = Some(beceri(&"k".repeat(90), &"kategori-".repeat(8)));
        for _ in 0..3 {
            ctx.run_ui(girdi(400.0), |ui| app.ciz(ui))
                .textures_delta
                .clear();
        }
        let tasan: Vec<egui::Rect> = ctx.viewport(|v| {
            v.prev_pass
                .widgets
                .layers()
                .flat_map(|(_, rs)| rs.iter().map(|r| r.rect))
                .filter(|r| r.right() > 400.5 || r.left() < -0.5)
                .collect()
        });
        assert!(tasan.is_empty(), "pencere dışına taşan öğeler: {tasan:?}");
    }

    fn beceri(id: &str, kategori: &str) -> Skill {
        Skill {
            id: id.into(),
            ad: "ad".into(),
            aciklama: String::new(),
            kategori: kategori.into(),
            etiketler: vec![],
            dosya_yolu: None,
            olusturma_tarihi: None,
            guncelleme_tarihi: None,
        }
    }

    fn girdi(genislik: f32) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(genislik, 600.0),
            )),
            ..Default::default()
        }
    }

    fn kare_ciz(ctx: &egui::Context, app: &mut FihristApp, genislik: f32) {
        ctx.run_ui(girdi(genislik), |ui| app.ciz(ui))
            .textures_delta
            .clear();
    }

    /// Çizim isteklerini sayan bağlam.
    fn sayacli_baglam() -> (egui::Context, Arc<AtomicUsize>) {
        let ctx = egui::Context::default();
        let sayac = Arc::new(AtomicUsize::new(0));
        let s = sayac.clone();
        ctx.set_request_repaint_callback(move |_| {
            s.fetch_add(1, Ordering::SeqCst);
        });
        (ctx, sayac)
    }

    /// Arka plan iş parçacığının çizim isteğini (en az `n` kez) bekler.
    fn bekle(sayac: &AtomicUsize, n: usize) {
        let son = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while sayac.load(Ordering::SeqCst) < n {
            assert!(std::time::Instant::now() < son, "çizim uyandırılmadı");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
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
