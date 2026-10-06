# MCP kaynak bildirimi dört yan dosyanın bildirim günlüğünü yoklar; ana katalog yok, içerik JSON

ADR 0003 bildirim günlüğünü ve `fihrist-izle` tüketicisini kurdu. YZ arayüzleri değişikliği MCP üzerinden görmek istiyor: `resources/subscribe` ve `notifications/resources/updated` (MCP 2024-11-05). Karar:

- **URI:** `fihrist://<dosya kökü>/<tablo>`, ör. `fihrist://kutup_kayitlar/arac_cagrilari`. Dosya, ana katalogla aynı dizindeki `<kök>.db`. Kök beyaz listededir; tablo adında yalnız harf, rakam ve `_` olabilir. Liste dışı kök, `..`, `/` ve başka yol karakterleri `-32602` ile reddedilir.
- **Kapsam dört dosya:** `kutup_kurallar`, `kutup_ortak`, `kutup_depolar`, `kutup_kayitlar`. Ana katalog (`kutup_kutuphane`) için abonelik **açık hatayla reddedilir**: fts5'li dosyada Turso şemayı keser, günlük kurulamaz (ADR 0003, bulgu 3).
- **Sessiz "değişiklik yok" yok:** dosya yoksa, günlük kurulu değilse ya da tablonun üç tetikleyicisi yoksa abonelik de okuma da `-32002` ile ve kurulum komutuyla döner. `resources/list` yalnız izlenen tabloları gösterir.
- **`resources/read` JSON döner** (`application/json`, `text`): o tablonun günlükteki son 50 kaydı, eskiden yeniye: `no`, `zaman_ms`, `islem`, `anahtar`. Alıcılar YZ arayüzleridir, Rust habitatının dışındadır. altin-kapi ADR 0005 madde 4 JSON'u tam bu durum için bırakır. Rust↔Rust aktarım (elçi) bu kararın konusu değil.
- **Abone geçicidir** (CONTEXT.md): abonelik oturum belleğinde durur, imleç dosyası yazılmaz, budamayı etkilemez. Abone olununca imleç o anki `son_no`'dur. Yoklamada günlüğün en küçük numarası imleç+1'den büyükse araya budama girmiştir: o dosyadaki **bütün** abone URI'lerine koşulsuz bildirim gider. En büyük numara imlecin gerisine düşmüşse (dosya değişmiş) de aynısı olur. Kaçırma yok, fazladan bildirim olabilir.
- **Yoklama 250 ms**, her yoklama dosyayı `fihrist_canli::ac` ile **taze açar** (bulgu 4). Şema yalnız `sqlite_master`'dan okunur, `pragma_table_info(...)` hiç yok (bulgu 5). Aynı yoklamada aynı URI için tek bildirim. Yoklama hatası stderr'e yazılır, imleç ilerlemez; sonraki başarılı yoklama aradakini yakalar.
- `initialize` `capabilities.resources = {"subscribe": true, "listChanged": false}` bildirir.
- Kod `crates/fihrist-kaynak`'ta, `ibnunnedim-cli/src/mcp.rs` yalnız bağlar. Hizmet yolunda yoklama hizmetin `konus` döngüsünde koşar, bildirim aktarıcıdan geçer. Aktarıcının süreç içi yedeği yoklamaz; abonelik orada açık hatayla reddedilir.
- `ibnunnedim-cli` sandık sınırının üstünde (taban 3363). mcp.rs'e eklenen 16 satırın ve sonraki `ibnunnedim izle` bağlaması için 10 satır payın karşılığında `olcum.rs` (49 satır) `crates/fihrist-olcum`'a taşındı.

## Reddedilen seçenekler

- **Ana katalog için dosya zamanına dayalı kaba sinyal:** hangi tablonun değiştiğini söylemez. WAL'lı dosyada yazış önce `-wal`'a düşer, ana dosyanın zamanı checkpoint'e kadar değişmeyebilir. Bildirim gelmezse "değişmedi" mi, "göremedik" mi ayırt edilemez.
- **fts tablolarını ayrı dosyaya taşımak:** ana katalog izlenebilir olurdu, ama şema göçü ve arama yolu değişir. Ercan şimdilik istemedi.
- **Aboneyi kalıcı tüketici yapmak** (imleç dosyası): oturum düşünce imleç kalır, budama onu bekleyip durur.
- **Okumada ikili biçim:** alıcı Rust değil; altin-kapi ADR 0005 madde 3–4.

## Sonuçlar

- Oturum betiğinde (`crates/fihrist-kaynak/deneme/oturum.ts`) yazıştan bildirime süreç içinde 237 ve 234 ms, hizmet yolunda 231 ve 231 ms ölçüldü (ikişer koşu).
- Budama boşluğu ve numara geri dönmesi fazladan bildirim üretir; alıcı yeniden okur, zararı yoktur.
- Hizmet koparsa hizmetteki abonelikler gider ve istemci bunu bilmez. Yeniden abone olmak gerekir.
- `resources/templates/list` yok (`-32601`). Dört kök ve tablo adları listeden bulunur.
