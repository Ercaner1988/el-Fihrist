//! Tekmil haftası: ISO 8601 yıl ve hafta, `YYYYWW` (örn. 202641), tarih sandığı olmadan.

/// Yılın 1 Ocak'ının Unix günü (1970-01-01 = 0).
fn yil_basi(yil: i64) -> i64 {
    let o = yil - 1;
    // 477: 1970'e dek geçen artık gün sayısı (1969/4 - 1969/100 + 1969/400).
    365 * (yil - 1970) + o.div_euclid(4) - o.div_euclid(100) + o.div_euclid(400) - 477
}

/// Unix gününün ISO haftası. Haftanın yılı, o haftanın perşembesinin yılıdır.
pub fn iso_hafta(unix_gun: i64) -> i64 {
    // 1970-01-01 perşembedir; (gün + 3) mod 7 pazartesiyi 0 yapar.
    let persembe = unix_gun - (unix_gun + 3).rem_euclid(7) + 3;
    let mut yil = 1970 + persembe.div_euclid(366);
    while yil_basi(yil + 1) <= persembe {
        yil += 1;
    }
    yil * 100 + (persembe - yil_basi(yil)) / 7 + 1
}

/// Çağrı anının ISO haftası (sistem saati, UTC günü).
pub fn bu_hafta() -> i64 {
    let sn = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    iso_hafta(sn.div_euclid(86_400))
}

#[cfg(test)]
mod testler {
    use super::iso_hafta;

    /// Takvimden elle doğrulanmış günler; yıl sınırı iki yönde de sınanır.
    #[test]
    fn bilinen_gunler() {
        assert_eq!(iso_hafta(0), 197001); // 1970-01-01 perşembe
        assert_eq!(iso_hafta(20454), 202601); // 2026-01-01 perşembe
        assert_eq!(iso_hafta(20736), 202641); // 2026-10-10 cumartesi
        assert_eq!(iso_hafta(20088), 202501); // 2024-12-31 salı: haftanın perşembesi 2025'te
        assert_eq!(iso_hafta(20089), 202501); // 2025-01-01 çarşamba
        assert_eq!(iso_hafta(20818), 202653); // 2026-12-31 perşembe: 2026'nın 53. haftası
        assert_eq!(iso_hafta(20821), 202653); // 2027-01-03 pazar: hâlâ 2026-W53
        assert_eq!(iso_hafta(20822), 202701); // 2027-01-04 pazartesi
    }
}
