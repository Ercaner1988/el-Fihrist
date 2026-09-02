// Canlı doğrulama: gerçek config.yaml üzerinde nöbet modülü
use hermes_tools_core::{nobet_tut, SuphePuani};

fn main() {
    let yol = r"C:\Users\buzbe\AppData\Local\hermes\config.yaml";
    let guncel = std::fs::read_to_string(yol).expect("config.yaml okunamadı");

    println!("=== SENARYO 1: aynı dosya (değişim yok) ===");
    let r = nobet_tut(&guncel, &guncel, 20).unwrap();
    println!("  {}", r.ozet());
    println!(
        "  bildirilmeli={} dikkat={}",
        r.bildirilmeli(),
        r.suphe.dikkat_ister()
    );
    assert_eq!(r.suphe, SuphePuani::Degismemis);

    println!("\n=== SENARYO 2: providers bölümü silinmiş (gerçek bozulma taklidi) ===");
    let bozuk: String = guncel
        .lines()
        .scan(false, |atla, s| {
            if !s.starts_with(' ') && !s.trim().is_empty() {
                *atla = s.starts_with("providers:");
            }
            Some(if *atla { None } else { Some(s) })
        })
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    let r = nobet_tut(&guncel, &bozuk, 5).unwrap();
    println!("  {}", r.ozet());
    for g in &r.gerekceler {
        println!("  GEREKÇE: {g}");
    }
    println!("  kayıp anahtarlar: {:?}", r.kaybolan_anahtarlar);
    println!("  fark örneği (ilk 3):");
    for f in r.farklar.iter().take(3) {
        println!(
            "    {} satır {}: {}",
            f.durum.etiket(),
            f.satir_no,
            f.icerik.trim()
        );
    }
    assert!(r.suphe.dikkat_ister(), "providers kaybı dikkat istemeliydi");

    println!("\n=== SENARYO 3: sekme ile girinti (YAML'ı kıran hata) ===");
    let sekmeli = guncel.replacen("\n  ", "\n\t", 1);
    let r = nobet_tut(&guncel, &sekmeli, 5).unwrap();
    println!("  {}", r.ozet());
    for g in &r.gerekceler {
        println!("  GEREKÇE: {g}");
    }
    assert_eq!(r.suphe, SuphePuani::SozDizimiSuphesi);

    println!("\n=== SENARYO 4: dosya sıfırlanmış ===");
    match nobet_tut(&guncel, "", 5) {
        Err(e) => println!("  Beklenen hata: {e}"),
        Ok(_) => panic!("boş dosya hata vermeliydi"),
    }

    println!("\n=== SENARYO 5: model.default değişmiş (olağan) ===");
    let model_degisti = guncel.replacen("hermes-beyin", "gpt-5-codex", 1);
    let r = nobet_tut(&guncel, &model_degisti, 10).unwrap();
    println!("  {}", r.ozet());
    println!("  dikkat ister mi: {}", r.suphe.dikkat_ister());
    for f in r.farklar.iter().take(2) {
        println!(
            "    {} satır {}: {}",
            f.durum.etiket(),
            f.satir_no,
            f.icerik.trim()
        );
    }

    println!("\nTÜM SENARYOLAR GEÇTİ ✔");
}
