//! `temalar/*.tema` tohumunu kilim-tema'nın yerleşik temasından yeniden yazar.
//! Yerleşik palet değişince bir kez çalıştırın: `cargo run -p fihrist-tema --example tohum_yaz`.
use kilim_tema::{TemaPaketi, Varyant};

fn main() {
    let dizin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../temalar");
    std::fs::create_dir_all(&dizin).unwrap();
    for (id, ad, kenarlik) in [
        ("kilim-cam-gobegi", "Kilim: cam göbeği + altın", Varyant::CamGobegiAltin),
        ("kilim-kirmizi-yesil", "Kilim: kırmızı + yeşil", Varyant::KirmiziYesil),
    ] {
        std::fs::write(dizin.join(format!("{id}.tema")), TemaPaketi::yerlesik(id, ad, kenarlik).kod()).unwrap();
    }
}
