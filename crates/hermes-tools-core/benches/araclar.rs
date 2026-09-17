//! `hermes-tools-core` yeteneklerinin CodSpeed benchmark'ları.
//!
//! KAPSAM: crate'in dışa verdiği SAF işlevler (dosya sistemi yok, ağ yok).
//! Hepsi tek bir metin/gövde alıp rapor üretir; ajanlar bu işlevleri araç
//! çağrısı başına bir kez koşturur, yani ölçülmesi gereken şey bir çağrının
//! tamamıdır — parçaları değil.
//!
//! GİRDİ BOYUTLARI gerçek kullanıma göre seçildi:
//!   html_ayikla        → orta boy bir web sayfası (~40 bağlantılı 200 blok)
//!   verify_citations   → tez uzunluğunda markdown (1.200 paragraf)
//!   docx XML           → 400 paragraflık bir Word gövdesi
//!   kod denetimi       → 2.000 satırlık Rust dosyası
//!   kod tabanı özeti   → 600 dosyalık istatistik yığını
//!   oturum arama       → 1.000 turluk döküm
//!   nobet_tut          → 400 satırlık YAML'ın iki sürümü
//!   graf               → 32×32 ve 64×64 ızgara (1.024 / 4.096 düğüm)
//!
//! Gövdeler ölçülen bölgenin DIŞINDA kurulur (`with_inputs` ya da closure
//! dışı `let`); yoksa sayılar veri üretimini ölçer.

use hermes_tools_core::{
    analyze_file_content, check_rust_code_rules, extract_paragraphs_from_xml,
    generate_multilingual_readme, html_ayikla, nobet_tut, parse_cargo_toml_content, search_session,
    summarize_codebase, update_dynamic_sections, verify_citations, Graph, LanguageStat,
    ReadmeUpdates, RoadmapItem,
};

fn main() {
    divan::main();
}

/// Tohumlu LCG: bağımlılıksız ve her koşuda aynı gövde. Değişken veri
/// benchmark'lar arası karşılaştırmayı anlamsızlaştırır.
struct Lcg(u64);

impl Lcg {
    fn sonraki(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }

    fn sec<'a>(&mut self, secenekler: &[&'a str]) -> &'a str {
        secenekler[self.sonraki() % secenekler.len()]
    }
}

/// Türkçe katlama yolunu ve markdown noktalamasını çalıştıran sözlük.
/// Rastgele ASCII, `to_lowercase`/`trim` yollarının asıl işini atlardı.
const KELIMELER: &[&str] = &[
    "yetenek",
    "kaynakça",
    "İstanbul",
    "ağaç",
    "şüphe",
    "çözümleme",
    "belge",
    "rasyonelleşme",
    "hukuk",
    "düzenleme",
    "gömme",
    "vektör",
    "ölçüm",
    "kapı",
    "Rust",
    "tokio",
    "ayrıştırıcı",
    "önbellek",
    "güvenlik",
    "doğrulama",
];

fn cumle(rng: &mut Lcg, kelime: usize) -> String {
    let mut s = String::with_capacity(kelime * 10);
    for i in 0..kelime {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(rng.sec(KELIMELER));
    }
    s
}

// --- html_ayikla ---

/// Orta boy bir sayfa: başlık, paragraflar, listeler ve bağlantılar.
/// `<a href>` dalı ayrı bir ayrıştırma yolu (href soyma + bağlantı toplama),
/// o yüzden bloklardan beşte biri bağlantı içeriyor.
fn html_govde(blok: usize) -> String {
    let mut rng = Lcg(0x8741_0000);
    let mut s = String::with_capacity(blok * 220);
    s.push_str("<html><head><title>");
    s.push_str(&cumle(&mut rng, 6));
    s.push_str("</title></head><body>\n");
    for i in 0..blok {
        if i % 5 == 0 {
            s.push_str(&format!(
                "<p>{} <a href=\"https://ornek.org/yol/{i}?q={}\">{}</a> {}</p>\n",
                cumle(&mut rng, 8),
                i,
                cumle(&mut rng, 3),
                cumle(&mut rng, 8)
            ));
        } else if i % 7 == 0 {
            s.push_str(&format!("<h2>{}</h2>\n", cumle(&mut rng, 5)));
        } else {
            s.push_str(&format!("<p>{}</p>\n", cumle(&mut rng, 24)));
        }
    }
    s.push_str("</body></html>");
    s
}

#[divan::bench]
fn html_ayiklama(bencher: divan::Bencher) {
    let html = html_govde(200);
    bencher.bench(|| html_ayikla(divan::black_box(&html)).expect("gövde geçerli HTML"));
}

// --- verify_citations ---

/// Tez uzunluğunda markdown; paragrafların yarısı `[[ATIF: ...]]` taşır.
fn atifli_belge(paragraf: usize) -> String {
    let mut rng = Lcg(0x4717_0000);
    let mut s = String::with_capacity(paragraf * 180);
    for i in 0..paragraf {
        s.push_str(&cumle(&mut rng, 18));
        if i % 2 == 0 {
            s.push_str(&format!(
                " [[ATIF: {}, {}, s. {}]]",
                cumle(&mut rng, 2),
                1990 + (i % 35),
                10 + (i % 300)
            ));
        }
        s.push_str("\n\n");
    }
    s
}

#[divan::bench]
fn atif_dogrulama(bencher: divan::Bencher) {
    let belge = atifli_belge(1_200);
    bencher.bench(|| verify_citations(divan::black_box(&belge)).expect("belge geçerli"));
}

// --- docx ---

/// OpenXML `document.xml` gövdesi: paragraf başına birkaç `w:t` koşusu.
fn docx_xml(paragraf: usize) -> String {
    let mut rng = Lcg(0xD0C_0001);
    let mut s = String::with_capacity(paragraf * 260);
    s.push_str("<?xml version=\"1.0\"?><w:document><w:body>");
    for _ in 0..paragraf {
        s.push_str("<w:p><w:pPr><w:pStyle w:val=\"Normal\"/></w:pPr>");
        for _ in 0..3 {
            s.push_str("<w:r><w:t xml:space=\"preserve\">");
            s.push_str(&cumle(&mut rng, 9));
            s.push_str(" </w:t></w:r>");
        }
        s.push_str("</w:p>");
    }
    s.push_str("</w:body></w:document>");
    s
}

#[divan::bench]
fn docx_paragraf_ayiklama(bencher: divan::Bencher) {
    let xml = docx_xml(400);
    bencher.bench(|| extract_paragraphs_from_xml(divan::black_box(&xml)).expect("XML geçerli"));
}

// --- rules_checker ---

/// 2.000 satırlık Rust dosyası; satırların bir kısmı bilerek ihlal taşır
/// (ihlal dalı `String` ayırır, temiz satırdan pahalıdır).
fn rust_kaynagi(satir: usize) -> String {
    let mut rng = Lcg(0xC0DE_4001);
    let mut s = String::with_capacity(satir * 44);
    for i in 0..satir {
        match i % 13 {
            0 => s.push_str("    // yorum satırı: denetim bunu atlar\n"),
            3 => s.push_str("    let deger = sonuc.unwrap();\n"),
            7 => s.push_str("    panic!(\"beklenmeyen durum\");\n"),
            9 => s.push_str("    unsafe { std::ptr::read(p) };\n"),
            11 => s.push_str("    todo!(\"sonra\");\n"),
            _ => {
                s.push_str("    let x = ");
                s.push_str(rng.sec(KELIMELER));
                s.push_str("_hesapla(girdi);\n");
            }
        }
    }
    s
}

#[divan::bench]
fn kod_kurallari_denetimi(bencher: divan::Bencher) {
    let kod = rust_kaynagi(2_000);
    bencher.bench(|| check_rust_code_rules(divan::black_box(&kod)).expect("kod boş değil"));
}

// --- codebase ---

/// Dosya başına bir `analyze_file_content` + tek bir `summarize_codebase`:
/// `ibnunnedim kod-tabani` çağrısının şekli tam olarak bu.
#[divan::bench]
fn kod_tabani_ozeti(bencher: divan::Bencher) {
    let uzantilar = ["rs", "py", "ts", "md", "toml", "json"];
    let dosyalar: Vec<(String, String)> = (0..600)
        .map(|i| {
            (
                format!("src/modul_{i}.{}", uzantilar[i % uzantilar.len()]),
                rust_kaynagi(60),
            )
        })
        .collect();

    bencher.bench(|| {
        let istatistikler: Vec<LanguageStat> = dosyalar
            .iter()
            .map(|(ad, icerik)| {
                analyze_file_content(divan::black_box(ad), divan::black_box(icerik))
                    .expect("içerik dolu, uzantı var")
            })
            .collect();
        summarize_codebase(istatistikler).expect("özet üretilebilir")
    });
}

// --- session ---

/// 1.000 turluk oturum dökümü. `search_session` her çağrıda dökümü BAŞTAN
/// ayrıştırır ve `to_lowercase` ile tarar; ölçülen maliyet budur.
fn oturum_dokumu(tur: usize) -> String {
    let mut rng = Lcg(0x5E55_1000);
    let mut s = String::with_capacity(tur * 200);
    for i in 0..tur {
        let rol = if i % 2 == 0 { "[user]" } else { "[assistant]" };
        s.push_str(rol);
        s.push(' ');
        s.push_str(&cumle(&mut rng, 22));
        if i % 37 == 0 {
            s.push_str(" FTS5 dizini yeniden kuruldu");
        }
        s.push('\n');
    }
    s
}

#[divan::bench(args = [None, Some("user")])]
fn oturum_arama(bencher: divan::Bencher, rol: Option<&str>) {
    let dokum = oturum_dokumu(1_000);
    bencher
        .bench(|| search_session(divan::black_box(&dokum), "FTS5", rol).expect("sorgu boş değil"));
}

// --- config_nobetci ---

/// 400 satırlık YAML'ın iki sürümü: bir anahtar kaybolmuş, bir bayrak
/// sessizce kapanmış, satırların %5'i değişmiş. `nobet_tut`'un pahalı
/// kısmı (çokluk sayımlı satır farkı + bayrak taraması) böyle görünür.
fn yapilandirma_cifti(satir: usize) -> (String, String) {
    let mut rng = Lcg(0x900B_ACCE);
    let mut onceki = String::from("model:\n  default: hermes-beyin\ntools:\n  enabled: true\n");
    let mut guncel = String::from("model:\n  default: hermes-beyin\ntools:\n  enabled: false\n");
    for i in 0..satir {
        let anahtar = format!("anahtar_{i}");
        let deger = rng.sec(KELIMELER);
        onceki.push_str(&format!("{anahtar}:\n  deger: {deger}\n"));
        if i % 20 == 0 {
            // Kaybolan anahtar: güncel sürümde hiç yok.
            continue;
        }
        if i % 7 == 0 {
            guncel.push_str(&format!("{anahtar}:\n  deger: {}\n", rng.sec(KELIMELER)));
        } else {
            guncel.push_str(&format!("{anahtar}:\n  deger: {deger}\n"));
        }
    }
    (onceki, guncel)
}

#[divan::bench]
fn yapilandirma_nobeti(bencher: divan::Bencher) {
    let (onceki, guncel) = yapilandirma_cifti(400);
    bencher.bench(|| {
        nobet_tut(
            divan::black_box(&onceki),
            divan::black_box(&guncel),
            divan::black_box(200),
        )
        .expect("güncel sürüm dolu")
    });
}

// --- multilingual ---

const CARGO_ORNEK: &str = r#"
[workspace.package]
name = "el-fihrist"
version = "0.1.0"
license = "MIT"
description = "Yapay zekâ ajanları için yetenek, kod ve hafıza kütüphanesi"
authors = ["Ercan Er <ornek@ornek.org>"]
repository = "https://github.com/Ercaner1988/el-Fihrist"
homepage = "https://github.com/Ercaner1988/el-Fihrist"
documentation = "https://docs.rs/el-fihrist"
"#;

#[divan::bench]
fn cargo_toml_ayristirma() -> hermes_tools_core::ProjectMeta {
    parse_cargo_toml_content(divan::black_box(CARGO_ORNEK))
}

/// Yedi dilde README üretimi: crate'in en büyük tek çıktısı (dil başına
/// binlerce satır `format!`). `ibnunnedim yolbulucu` bunu her koşuda üretir.
#[divan::bench]
fn coklu_dil_readme(bencher: divan::Bencher) {
    let mut meta = parse_cargo_toml_content(CARGO_ORNEK);
    meta.modules = (0u64..12)
        .map(|i| hermes_tools_core::ModuleInfo {
            name: format!("modul_{i}"),
            description: cumle(&mut Lcg(i + 1), 12),
        })
        .collect();
    meta.roadmap = (0u64..10)
        .map(|i| RoadmapItem {
            completed: i % 3 == 0,
            title: cumle(&mut Lcg(100 + i), 6),
        })
        .collect();
    meta.co_authors = vec!["Hermes Agent".into(), "Claude".into()];

    bencher.bench(|| generate_multilingual_readme(divan::black_box(&meta)));
}

/// Mevcut README'nin dinamik bölümlerini değiştirme: satır satır tarama +
/// yeniden birleştirme. Girdi, üretilen Türkçe README'nin kendisi.
#[divan::bench]
fn readme_bolum_guncelleme(bencher: divan::Bencher) {
    let meta = parse_cargo_toml_content(CARGO_ORNEK);
    let readme = generate_multilingual_readme(&meta).tr;
    let updates = ReadmeUpdates {
        features: Some("* Yeni: ölçülen gömme çekirdeği\n* Yeni: CodSpeed kapısı".to_string()),
        roadmap: Some(vec![
            RoadmapItem {
                completed: true,
                title: "BM25 çekirdeği".into(),
            },
            RoadmapItem {
                completed: false,
                title: "Vektör füzyonu".into(),
            },
        ]),
        release_updates: None,
    };

    bencher
        .bench(|| update_dynamic_sections(divan::black_box(&readme), divan::black_box(&updates)));
}

// --- router ---

/// `kenar` düğümlü ızgara graf: her düğüm sağına ve altına birer kenar.
/// Dijkstra'nın yığın davranışı ancak bu ölçekte görünür.
fn izgara(kenar: usize) -> Graph {
    let mut g = Graph::new();
    for y in 0..kenar {
        for x in 0..kenar {
            let bu = format!("{x}:{y}");
            if x + 1 < kenar {
                g.add_edge(&bu, &format!("{}:{y}", x + 1), 1.0 + ((x + y) % 7) as f64)
                    .expect("ağırlık pozitif");
            }
            if y + 1 < kenar {
                g.add_edge(&bu, &format!("{x}:{}", y + 1), 1.0 + ((x * y) % 5) as f64)
                    .expect("ağırlık pozitif");
            }
        }
    }
    g
}

#[divan::bench(args = [32, 64])]
fn en_kisa_yol(bencher: divan::Bencher, kenar: usize) {
    let g = izgara(kenar);
    let hedef = format!("{}:{}", kenar - 1, kenar - 1);
    bencher.bench(|| {
        g.shortest_path(divan::black_box("0:0"), divan::black_box(&hedef))
            .expect("graf sınır içinde")
    });
}
