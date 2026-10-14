//! Döküm zaman damgası: tarih kütüphanesi yok (Claude Code tek biçim yazıyor).

/// `1970-01-01`den bu yana gün (Hinnant'ın `days_from_civil`i).
fn gun(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-09-09T16:28:32.381Z` → Unix milisaniyesi.
///
/// Tarih kütüphanesi YOK: Claude Code tek bir biçim yazıyor (UTC, `Z`,
/// milisaniye). `Z` ile bitmeyen damga `None` — saat dilimi kaymasını
/// sessizce yanlış süreye çevirmektense reddeder.
pub fn iso_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 || !s.ends_with('Z') {
        return None;
    }
    if [4, 7].iter().any(|&i| b[i] != b'-') || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let n = |a: usize, z: usize| s.get(a..z).and_then(|p| p.parse::<i64>().ok());
    let (y, mo, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (h, mi, se) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    let ms = if b[19] == b'.' {
        let kesir = &s[20..s.len() - 1];
        let uc: String = kesir.chars().chain("000".chars()).take(3).collect();
        uc.parse::<i64>().ok()?
    } else {
        0
    };
    Some(((gun(y, mo, d) * 24 + h) * 60 + mi) * 60_000 + se * 1000 + ms)
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Eşleme bu damgalardan yapılıyor; biri kayarsa Seçimler yanlış aramaya bağlanır.
    #[test]
    fn iso_ms_bilinen_anlar() {
        assert_eq!(iso_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(iso_ms("2000-01-01T00:00:00.000Z"), Some(946_684_800_000));
        assert_eq!(iso_ms("2024-02-29T12:00:00.500Z"), Some(1_709_208_000_500));
        assert_eq!(iso_ms("2000-01-01T00:00:01Z"), Some(946_684_801_000));
        assert_eq!(iso_ms("2000-01-01T00:00:00.5Z"), Some(946_684_800_500));
    }

    /// Saat dilimli ya da bozuk damga sessizce yanlış an üretmemeli.
    #[test]
    fn iso_ms_bicim_disini_reddeder() {
        assert_eq!(iso_ms("2024-02-29T12:00:00.500+03:00"), None);
        assert_eq!(iso_ms("2024-02-29 12:00:00Z"), None);
        assert_eq!(iso_ms(""), None);
        assert_eq!(iso_ms("çöp"), None);
    }
}
