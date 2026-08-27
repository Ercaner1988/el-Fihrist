# KÜTÜPHANE KURALI · SİSTEME ENTRİYE ZORUNLULUK

**Yazan:** İbnünnedîm · **Güncelleme Tarihi:** 2026-08-27

---

## Kurallar

1. **Kütüphaneden Çağırma Zorunluluğu:** Kütüphanede bulunan her yetenek, kod parçası veya prompt şablonu **mutlaka** `kutup_kutuphane.db` üzerinden çağrılmalıdır.

2. **Açık İstisna:** Sadece ve sadece kullanıcı açıkça "dışarıdaki kopyayı kullan" dediğinde kütüphane dışındaki dosyalar doğrudan kullanılabilir.

3. **Sistem Kontrolü:** Sistem otomatik olarak kütüphane referanslarını denetler ve doğrulama yapar.

---

## Uygulama

| Senaryo | Kütüphaneden mi çağrılmalı? | Açıklama |
|---|---|---|
| `rust-multilingual-readme` yeteneği | **EVET** | Kütüphanede var, mutlak zorunluluk |
| `codebase-inspection` kodu | **EVET** | `kod_hazinesi` tablosunda SHA256 ile kayıtlı |
| `grounded-citations` doğrulama motoru | **EVET** | `rust-grounded-citations` modülü olarak kütüphanede |
| `9router` (pathfinder) | **EVET** | `rust-pathfinder-router` modülü olarak kütüphanede |

---

## İstisna Durumları

Kütüphane dışındaki kaynakların kullanılabilmesi için **açık onay** gerekir:

```bash
# Geçersiz: Sistem varsayılan olarak kütüphaneyi kullanır
ibnunnedim search "rust"

# Geçerli (açık onay varsa): Dışarıdaki kopyayı kullan
user: "Lütfen dışarıdaki hermes-tools-core klasöründeki kodu doğrudan çalıştır"
```

---

## Kütüphane Yapısı

| Tablo | İçerik |
|---|---|
| `yetenekler` | 137 yetenek (129 başlangıç + 5 ilk grup + 3 ikinci grup) |
| `kod_hazinesi` | 9 Rust kod parçacığı (SHA256 özetleri ile) |
| `prompt_hazinesi` | Şablon promptlar |
| `ajan_raporlari` | Ajan performans raporları |
| `ajan_tekmilleri` | Tekmil puanları |
| `skillopt_kuyrugu` | İyileştirme sırası bekleyen beceriler |
| `statik_hafiza_kisayollari` | `user.md`, `soul.md` kısayolları |

---

## Kontrol Komutları

```bash
# Kütüphanedeki mevcut yetenekleri göster
ibnunnedim info

# Kütüphanedeki kod parçacıklarını ara
ibnunnedim search "rust"

# Tekmil puanlarını görüntüle
ibnunnedim tekmil --ajan "ibnunnedim" --yetenek "<yetenek-id>" --puan <0-100> --gerekce "<açıklama>"
```

---

## İhale Kuralları

| İhlal | Cezası |
|---|---|
| Kütüphanede olanı kütüphaneden çağırmazsan | Commit reddedilir |
| SHA256 özet eşleşmiyorsa | Kütüphanedeki sürüm geçerlidir |
| Tekmil eksikse | `skillopt_kuyrugu`'na eklenir |
| Kütüphanede olup dışarıda aynı kod varsa | Dışarıdaki silinir / taşınır |

---

## Cezeri/SkillOpt Entegrasyon Prosedürü

**Amaç:** Kütüphanede olan bir yetenek/dosya ile dışarıdaki (daha gelişmiş) bir sürüm aynı işlevi görebiliyorsa, dışardaki sürümün avantajları kütüphaneye aktarılmalı ve Cezeri'ye iletilerek skillopt edilmeli.

### Adım Adım Prosedür

1. **Tespit:** Dışarıda aynı işlevi gören bir dosya/bileşen bulunur.
2. **Analiz:** Kütüphanedeki sürümle dışardakini karşılaştır:
   - Farklı işlevler var mı?
   - Dışardaki sürümde hangi üstünlükler var? (hız, doğruluk, özellik, test coverage)
3. **Cezeri'ye Gönderim:** `skillopt_kuyrugu` tablosuna yeni bir kayıt ekle:
   ```sql
   INSERT INTO skillopt_kuyrugu (yetenek_id, sebep, ozellikler, oncelik) 
   VALUES ('<yetenek-id>', 'Dışardaki sürümde X üstünlüğü mevcut', 'X, Y, Z', 1);
   ```
4. **Cezeri'nin Görevi:** Cezeri, bu talebi işleyip kütüphanedeki yeteneği geliştirir.
5. **Doğrulama:** Yeni sürüm `kutup_kutuphane.db`'ye işlendikten sonra `tekmil` puanı 100 olana kadar tekrar test edilir.

### Örnek Senaryo

| Durum | Kütüphanede | Dışarda | İşlem |
|---|---|---|---|
| `rust-docx-openxml` | Temel XML okuma | OpenXML parsing + styles + metadata | `skillopt_kuyrugu`'na ekle, Cezeri'ye gönder |

---

## Geçerli Olan

> **"Açıkça istenmedikçe kütüphaneden çağrılmalı"** kuralı **aktiftir**.

---

*İbnünnedîm · Kütüphaneci Ajan*
