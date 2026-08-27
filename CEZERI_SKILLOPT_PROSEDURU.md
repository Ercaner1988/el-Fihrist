# Cezeri/SkillOpt Entegrasyon Prosedürü

**Yazan:** İbnünnedîm · **Güncelleme Tarihi:** 2026-08-27

---

## 1. Amacı

Kütüphanede bulunan bir yetenek/dosya ile **dışarıdaki** (daha gelişmiş/üstün) bir sürüm aynı işlevi görebiliyorsa, dışardaki sürümün avantajları kütüphaneye aktarılmalı ve Cezeri'ye iletilerek `skillopt` edilmeli.

---

## 2. Prosedür Akışı

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. Dışarıda aynı işlevi gören bir dosya/bileşen bulunur             │
└────────────────────┬────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 2. Kütüphanedeki sürümle dışardakini karşılaştır:                   │
│    - Farklı işlevler var mı?                                         │
│    - Dışardaki sürümde hangi üstünlükler var?                        │
│      (hız, doğruluk, özellik, test coverage, Türkçe destek)         │
└────────────────────┬────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 3. skillopt_kuyrugu tablosuna kayıt ekle:                          │
│    INSERT INTO skillopt_kuyrugu (                                   │
│      yetenek_id, sebep, ozellikler, oncelik                         │
│    ) VALUES (                                                        │
│      '<yetenek-id>',                                                 │
│      'Dışardaki sürümde X üstünlüğü mevcut',                        │
│      'X, Y, Z',                                                      │
│      1                                                               │
│    );                                                                │
└────────────────────┬────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 4. Cezeri bu talebi işler ve kütüphanedeki yeteneği geliştirir      │
└────────────────────┬────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 5. Yeni sürüm kutup_kutuphane.db'ye işlendi                        │
│    → tekmil puanı 100 olana kadar test tekrarı                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Örnek Senaryolar

| Durum | Kütüphanede | Dışarda | İşlem |
|---|---|---|---|
| `rust-docx-openxml` | Temel XML okuma | OpenXML + styles + metadata | `skillopt_kuyrugu`'na ekle |
| `rust-web-extract` | Basit HTML → Markdown | BeautifulSoup + CSS selector desteği | `skillopt_kuyrugu`'na ekle |
| `rust-multilingual-readme` | 4 dil şablon | GitHub badge support + CI/CD badge | `skillopt_kuyrugu`'na ekle |
| `rust-rules-checker` | unwrap/panic tespiti | Unsafe blocks detection + Clippy integration | `skillopt_kuyrugu`'na ekle |

---

## 4. Cezeri'nin Görevi

1. **Talebi Oku:** `skillopt_kuyrugu` tablosundan yeni talepleri çek
2. **Analiz Et:** Dışardaki üstünlükleri kütüphanedeki sürümle karşılaştır
3. **Kodla:** Kütüphanedeki modülü geliştir (Rust → Rust veya Python → Rust)
4. **Test Et:** `cargo test --workspace` ile tüm testleri geçtiğinden emin ol
5. **Tekmil Al:** `tekmil` komutu ile 100 puan alana kadar iterasyon yap
6. **DB'ye İşle:** Yeni sürümü `kutup_kutuphane.db`'ye SHA256 hash'iyle kaydet

---

## 5. Kütüphane Kontrol Mekanizması (`ibnunnedim-cli`)

### `check-duplicates` Komutu (Öneri)

```bash
# Dışardaki dosyaları tespit et ve kütüphanedeki sürümle karşılaştır
ibnunnedim check-duplicates --path "crates/hermes-tools-core"

# Sonuç: skillopt_kuyrugu'na eklenecekler listesi
```

### `skillopt-status` Komutu (Öneri)

```bash
# Kuyrukta bekleyen skillopt taleplerini göster
ibnunnedim skillopt-status

# Talep detayları ve önceliği
```

---

## 6. İhale Kuralları (skillopt için)

| Durum | Cezası |
|---|---|
| Aynı işlev dışarda var, kütüphanede yok | Kütüphaneye ekle (yeni yetenek) |
| Aynı işlev kütüphanede ve dışarda var, dışarda daha iyi | `skillopt_kuyrugu`'na ekle |
| Kütüphanede olanı kütüphaneden çağırmazsan | Commit reddedilir |
| SHA256 özet eşleşmiyorsa (dışardaki farklı) | Kütüphanedeki sürüm geçerlidir |
| Tekmil < 85 (kullanıcı memnun değil) | `skillopt_kuyrugu`'na otomatik ekle |

---

## 7. SQL Schema (skillopt_kuyrugu)

```sql
CREATE TABLE skillopt_kuyrugu (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    yetenek_id TEXT NOT NULL,
    sebep TEXT NOT NULL,
    ozellikler TEXT,
    oncelik INTEGER DEFAULT 1,  -- 1: Yüksek, 2: Orta, 3: Düşük
    eklenme_tarihi DATETIME DEFAULT CURRENT_TIMESTAMP,
    durum TEXT DEFAULT 'bekliyor'  -- bekliyor, iyiye_alındı, reddedildi
);
```

---

## 8. Cezeri'ye Gönderim Örneği

```rust
// ibnunnedim-cli/src/skillopt.rs
pub fn cezeriye_gonder(yetenek_id: &str, sebep: &str, ozellikler: &[&str]) {
    let db = baglan().unwrap();
    let ozellikler_str = ozellikler.join(", ");
    db.execute(
        "INSERT INTO skillopt_kuyrugu (yetenek_id, sebep, ozellikler, oncelik, durum)
         VALUES (?, ?, ?, 1, 'bekliyor')",
        params![yetenek_id, sebep, ozellikler_str],
    ).unwrap();
}
```

---

## 9. Son Kontrol Listesi

- [ ] Dışarıdaki dosya aynı işlevi görebiliyor mu?
- [ ] Dışardaki sürümün hangi üstünlükleri var?
- [ ] Kütüphanedeki sürümde bu üstünlükler eksik mi?
- [ ] `skillopt_kuyrugu`'na kayıt yapıldı mı?
- [ ] Cezeri talebi işledi mi?
- [ ] Yeni sürüm `cargo test`'ten geçti mi?
- [ ] `tekmil` puanı 100 oldu mu?

---

*İbnünnedîm · Kütüphaneci Ajan*
