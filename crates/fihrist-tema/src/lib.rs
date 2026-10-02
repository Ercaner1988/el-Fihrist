//! el-Fihrist tema kataloğu. Bütün tema paketleri burada durur; araçlar katalogla
//! yalnız kullanıcı tema değiştirirken konuşur (`kilim_tema::tema_secici_kaynakli`),
//! seçilen paketi kendi `<uygulama>.tema` dosyasına yazıp yalnız onu kullanır.
//!
//! Katalog = depodaki `temalar/*.tema` (derlemeye gömülü tohum) + kullanıcı dizini
//! (`FIHRIST_TEMALAR` ya da `<yapılandırma>/el-fihrist/temalar`). Aynı `id` varsa
//! kullanıcı dizini kazanır; bozuk dosya sessizce atlanır.

use std::path::PathBuf;

use kilim_tema::{TemaKaynagi, TemaPaketi};

const TOHUM: [&str; 2] = [
    include_str!("../../../temalar/kilim-cam-gobegi.tema"),
    include_str!("../../../temalar/kilim-kirmizi-yesil.tema"),
];

pub struct FihristTemalari {
    dizin: Option<PathBuf>,
}

impl FihristTemalari {
    /// Tohum + varsayılan kullanıcı dizini.
    pub fn bul() -> Self {
        Self {
            dizin: kullanici_dizini(),
        }
    }

    /// Tohum + verilen dizin (sınama ve özel kurulumlar için).
    pub fn dizinde(dizin: PathBuf) -> Self {
        Self { dizin: Some(dizin) }
    }

    fn paketler(&self) -> Vec<TemaPaketi> {
        let mut hepsi: Vec<TemaPaketi> = TOHUM
            .iter()
            .filter_map(|m| TemaPaketi::coz(m).ok())
            .collect();
        let dosyalar = self.dizin.as_ref().and_then(|d| std::fs::read_dir(d).ok());
        for e in dosyalar.into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "tema") {
                if let Some(p) = std::fs::read_to_string(e.path())
                    .ok()
                    .and_then(|m| TemaPaketi::coz(&m).ok())
                {
                    hepsi.retain(|q| q.id != p.id);
                    hepsi.push(p);
                }
            }
        }
        hepsi.sort_by(|a, b| a.ad.cmp(&b.ad));
        hepsi
    }
}

impl TemaKaynagi for FihristTemalari {
    fn liste(&self) -> Vec<(String, String)> {
        self.paketler().into_iter().map(|p| (p.id, p.ad)).collect()
    }

    fn getir(&self, id: &str) -> Option<TemaPaketi> {
        self.paketler().into_iter().find(|p| p.id == id)
    }
}

fn kullanici_dizini() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FIHRIST_TEMALAR") {
        return Some(PathBuf::from(d));
    }
    let kok = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }?;
    Some(kok.join("el-fihrist").join("temalar"))
}

#[cfg(test)]
mod sinama {
    use super::*;
    use kilim_tema::Varyant;

    fn bos_dizin(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fihrist-tema-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn tohum_yerlesik_temayla_ayni() {
        let k = FihristTemalari::dizinde(bos_dizin("tohum"));
        assert_eq!(
            k.getir("kilim-cam-gobegi").unwrap(),
            TemaPaketi::yerlesik(
                "kilim-cam-gobegi",
                "Kilim: cam göbeği + altın",
                Varyant::CamGobegiAltin
            )
        );
        assert_eq!(
            k.getir("kilim-kirmizi-yesil").unwrap().kenarlik,
            Varyant::KirmiziYesil
        );
        assert_eq!(k.liste().len(), 2);
    }

    #[test]
    fn kullanici_paketi_eklenir_ayni_id_ezer_bozuk_atlanir() {
        let d = bos_dizin("kullanici");
        let mut yeni = TemaPaketi::yerlesik("gece", "Gece", Varyant::CamGobegiAltin);
        yeni.koyu.zem = egui_renk(1, 1, 1);
        std::fs::write(d.join("gece.tema"), yeni.kod()).unwrap();
        let mut ezen = TemaPaketi::yerlesik(
            "kilim-cam-gobegi",
            "Benim cam göbeğim",
            Varyant::CamGobegiAltin,
        );
        ezen.acik.zem = egui_renk(9, 9, 9);
        std::fs::write(d.join("ezen.tema"), ezen.kod()).unwrap();
        std::fs::write(d.join("bozuk.tema"), "sema=1\nid=Büyük\n").unwrap();
        std::fs::write(d.join("not.txt"), "tema değil").unwrap();

        let k = FihristTemalari::dizinde(d);
        let liste = k.liste();
        assert_eq!(liste.len(), 3, "{liste:?}");
        assert_eq!(k.getir("gece").unwrap(), yeni);
        assert_eq!(k.getir("kilim-cam-gobegi").unwrap(), ezen);
        assert!(k.getir("yok").is_none());
    }

    fn egui_renk(r: u8, g: u8, b: u8) -> egui::Color32 {
        egui::Color32::from_rgb(r, g, b)
    }
}
