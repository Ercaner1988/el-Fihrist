//! Katalog konumu: `kutup_kutuphane.db` nerede. CLI, MCP ve GUI aynı sırayla arar.

use std::path::PathBuf;

/// Kütüphane dosyasının adı; yan dosyalar (`kutup_*.db`) aynı klasörde durur.
pub const DB_ADI: &str = "kutup_kutuphane.db";

/// Kurulum klasörü (`kur.ps1` hedefi), `USERPROFILE`'a göre; katalog altında `kutuphane\`.
const KURULUM: &str = r"Desktop\mcp-tools\el-fihrist";

/// Kütüphaneyi bul: `TURSO_DB_PATH`, çalışma dizini, ikilinin yanındaki `kutuphane\`,
/// en son kurulum klasöründeki `kutuphane\` (depodan, `target\release`'ten çalışınca).
///
/// Bulunamazsa yaratmaz, nereye baktığını söyleyip durur: `Builder::new_local` olmayan
/// dosyayı yaratır, eskiden boş bir DB açılıp "no such table: yetenekler" diye düşülüyor
/// ve geride çöp dosya kalıyordu — kütüphane bozukmuş gibi görünüyordu.
pub fn kutuphane_yolu() -> Result<PathBuf, String> {
    // Boş değer verilmemiş sayılır: Desktop eklentisi boş ayarı "" diye geçirir.
    if let Some(v) = std::env::var("TURSO_DB_PATH")
        .ok()
        .filter(|v| !v.is_empty())
    {
        // Açıkça verilmişse tahmin yürütme: ya odur ya hata.
        let p = PathBuf::from(&v);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("TURSO_DB_PATH bir dosyayı göstermiyor: {v}"))
        };
    }
    let exe = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from));
    let kurulum = std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(KURULUM));
    let adaylar = std::iter::once(PathBuf::from(DB_ADI)).chain(
        exe.into_iter()
            .chain(kurulum)
            .map(|k| k.join("kutuphane").join(DB_ADI)),
    );
    let mut denenen = Vec::new();
    for p in adaylar {
        if p.is_file() {
            return Ok(p);
        }
        denenen.push(p.display().to_string());
    }
    Err(format!(
        "kütüphane bulunamadı. Bakılan yerler:\n  {}\n\
         TURSO_DB_PATH ile açıkça gösterebilirsin.",
        denenen.join("\n  ")
    ))
}
