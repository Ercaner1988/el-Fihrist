//! `config-nobetci`: iki yapılandırma sürümünü karşılaştıran küçük araç.
//!
//! Hermes shell hook'undan çağrılır; karşılaştırma mantığı
//! [`hermes_tools_core::config_nobetci`] modülündedir. Bu ikili yalnızca dosya
//! okur ve raporu basar.
//!
//! ```text
//! config-nobetci <onceki.yaml> <guncel.yaml> [--json] [--azami-fark N]
//! ```
//!
//! Çıkış kodu: 0 = dikkat gerekmiyor, 1 = dikkat gerekiyor, 2 = araç hatası.

use hermes_tools_core::{nobet_tut, NobetRaporu, SuphePuani};
use std::process::ExitCode;

const AZAMI_FARK_VARSAYILAN: usize = 40;

fn main() -> ExitCode {
    let argvler: Vec<String> = std::env::args().skip(1).collect();

    if argvler.iter().any(|a| a == "-h" || a == "--help") {
        yardim();
        return ExitCode::SUCCESS;
    }

    let mut konumsal = Vec::new();
    let mut json_cikti = false;
    let mut azami_fark = AZAMI_FARK_VARSAYILAN;
    let mut i = 0;
    while i < argvler.len() {
        match argvler[i].as_str() {
            "--json" => json_cikti = true,
            "--azami-fark" => {
                i += 1;
                match argvler.get(i).and_then(|s| s.parse::<usize>().ok()) {
                    Some(n) => azami_fark = n,
                    None => {
                        eprintln!("Hata: --azami-fark bir sayı bekler");
                        return ExitCode::from(2);
                    }
                }
            }
            diger if diger.starts_with('-') => {
                eprintln!("Hata: bilinmeyen seçenek {diger}");
                return ExitCode::from(2);
            }
            diger => konumsal.push(diger.to_string()),
        }
        i += 1;
    }

    if konumsal.len() != 2 {
        eprintln!("Hata: iki dosya yolu gerekir (önceki, güncel)");
        yardim();
        return ExitCode::from(2);
    }

    let onceki = match std::fs::read_to_string(&konumsal[0]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Hata: önceki sürüm okunamadı ({}): {e}", konumsal[0]);
            return ExitCode::from(2);
        }
    };
    let guncel = match std::fs::read_to_string(&konumsal[1]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Hata: güncel sürüm okunamadı ({}): {e}", konumsal[1]);
            return ExitCode::from(2);
        }
    };

    let rapor = match nobet_tut(&onceki, &guncel, azami_fark) {
        Ok(r) => r,
        Err(e) => {
            // Doğrulama hatası da bir bulgudur; hook görebilsin diye JSON basılır.
            if json_cikti {
                println!(
                    r#"{{"suphe":"Hata","ozet":{},"gerekceler":[{}],"kaybolan_anahtarlar":[]}}"#,
                    kacisli(&e.to_string()),
                    kacisli(&e.to_string())
                );
            } else {
                eprintln!("Doğrulama: {e}");
            }
            return ExitCode::from(1);
        }
    };

    if json_cikti {
        println!("{}", json_uret(&rapor));
    } else {
        metin_bas(&rapor);
    }

    if rapor.suphe.dikkat_ister() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn yardim() {
    println!(
        "config-nobetci — iki yapılandırma sürümünü karşılaştırır\n\n\
         KULLANIM:\n    config-nobetci <onceki> <guncel> [--json] [--azami-fark N]\n\n\
         SEÇENEKLER:\n    \
         --json            Raporu JSON olarak bas\n    \
         --azami-fark N    En çok N satır farkı göster (varsayılan {AZAMI_FARK_VARSAYILAN})\n    \
         -h, --help        Bu yardımı göster\n\n\
         ÇIKIŞ KODU:\n    0 = dikkat gerekmiyor\n    1 = dikkat gerekiyor\n    2 = araç hatası"
    );
}

fn metin_bas(r: &NobetRaporu) {
    println!("{}", r.ozet());
    if r.suphe == SuphePuani::Degismemis {
        return;
    }
    for g in &r.gerekceler {
        println!("  - {g}");
    }
    if !r.farklar.is_empty() {
        println!("\nSatır farkları:");
        for f in &r.farklar {
            println!("  {} {}: {}", f.durum.etiket(), f.satir_no, f.icerik.trim());
        }
        if r.farklar_kirpildi {
            println!("  ... (liste kırpıldı)");
        }
    }
}

/// Raporu JSON'a çevirir. `serde_json` zaten bağımlılıkta.
fn json_uret(r: &NobetRaporu) -> String {
    let govde = serde_json::json!({
        "suphe": format!("{:?}", r.suphe),
        "ozet": r.ozet(),
        "dikkat_ister": r.suphe.dikkat_ister(),
        "onceki_satir": r.onceki_satir,
        "guncel_satir": r.guncel_satir,
        "onceki_boyut": r.onceki_boyut,
        "guncel_boyut": r.guncel_boyut,
        "kaybolan_anahtarlar": r.kaybolan_anahtarlar,
        "eklenen_anahtarlar": r.eklenen_anahtarlar,
        "gerekceler": r.gerekceler,
        "farklar_kirpildi": r.farklar_kirpildi,
        "farklar": r.farklar.iter().map(|f| serde_json::json!({
            "satir_no": f.satir_no,
            "durum": f.durum.etiket(),
            "icerik": f.icerik,
        })).collect::<Vec<_>>(),
    });
    govde.to_string()
}

/// Serde devreye giremediğinde (hata yolunda) güvenli dizgi kaçışı.
fn kacisli(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}
