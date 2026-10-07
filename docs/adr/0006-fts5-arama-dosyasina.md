# Ana katalogdaki fts5 tabloları ayrı, türetilmiş bir arama dosyasına taşınır (önerildi)

**Durum: önerildi, karar Ercan'ın.** Bu taslak seçenekleri ölçümle karşılaştırır; uygulama kabulden sonra başlar.

ADR 0005 bir sonucu açık bıraktı: Turso'nun bir sonraki sürümü `kutup_kutuphane.db`'yi açmayacak. Dosyada fts5 sanal tabloları var (`yetenekler_fts`, `kod_hazinesi_fts`). `main` (#9350) modülü bilinmeyen sanal tablo içeren dosyayı salt-okur kipte bile reddediyor. Bugünkü 0.8.2 dosyayı açıyor ama şemayı ilk fts5 satırında kesiyor (ADR 0003, bulgu 3). fts5'ten sonra gelen tablolar, indeksler ve tetikleyiciler Turso'ya görünmüyor; bu yüzden ana katalog izlenemiyor. Bu iki sorun aynı kökten geliyor. Python tarafı (`ibnunnedim_cli.py`, `ilkleme.py`) fts5 tablolarını kullanıyor (`ibnunnedim-cli/src/main.rs:823-826`, 2026-09-10); Rust tarafı aramayı bellekte BM25 ile yapıyor ve fts5'i okumuyor.

## Önerilen karar

- `yetenekler_fts` ve `kod_hazinesi_fts`, ana kataloğun yanındaki **arama dosyasına** (`kutup_arama.db`) taşınır. Ana katalogda sanal tablo kalmaz.
- **Arama dosyası türetilmiştir.** Doğruluk kaynağı ana katalogdur; dosya silinip her an yeniden kurulabilir (bge-dizin ile aynı ilke). Yalnız SQLite istemcileri açar; Turso onu hiç açmaz.
- **İçerikli fts5:** tablolar metnin kendi kopyasını tutar. Kaynağın `id`'si dizinlenmeyen sütun olarak (`id UNINDEXED`) saklanır ve ana tabloyla birleştirme bu `id` üzerinden yapılır, `rowid` üzerinden değil. Gerekçesi şu: `TEXT PRIMARY KEY`'li tabloda `rowid` kalıcı değildir.
  - Döküp yeniden yükleme (`.dump` → yeni dosya) 134 satırın 132'sinin rowid'ini değiştirdi (ölçüldü).
  - `VACUUM` 3.45.1'de değiştirmedi (134'te 0), ama SQLite belgesi değiştirebileceğini söylüyor.
  - Böyle bir durumda `rowid` birleştirmesi sessizce yanlış satırı verir; `id` birleştirmesi doğru kalır.
- **Tazelik denetimi okurken yapılır.** Arama dosyası bir damga saklar. Okur `ATTACH` eder ve damgayı karşılaştırır. Tutmuyorsa tek işlemde tam yeniden kurulum yapar (`DELETE` + `INSERT … SELECT`). Arka plan süreci ya da tüketici gerekmez.
  - Damga, dizinlenen satırların özetidir: satır sayısı ile sıralı `id` ve dizinlenen metinlerin sha256'sı.
  - `icerik_hash` dizinlenen bütün sütunları kapsıyorsa, metnin kendisi yerine `id:icerik_hash` dizisinin özeti yeter. Kapsayıp kapsamadığı görülmedi.
- **Göç SQLite ile, yedekten sonra, şu sırayla yapılır:**
  1. `.backup` ile yedek alınır.
  2. Arama dosyası kurulur, doldurulur ve fts5 `integrity-check` (`rank = 1`) çalıştırılır.
  3. fts5'e yazan tetikleyiciler düşürülür. **Bu adım tabloların düşürülmesinden önce gelmeli:** tetikleyici kalırsa ana tabloya her yazış `no such table: main.yetenekler_fts` ile düşer (ölçüldü).
  4. fts5 tabloları `DROP` edilir.
  5. `PRAGMA integrity_check` çalıştırılır.
  6. Python okurları `ATTACH`'a çevrilir.

  Betik tablo, sütun ve tetikleyici adlarını dosyanın `sqlite_master`'ından okur; aşağıdaki sentetik şemayı varsaymaz. Kip düzeni G-1 kurulum betiğindekiyle aynıdır: önce kopyada PROVA, yeşilse GERÇEK.

## Ölçülen (2026-10-07, sentetik dosya)

Sentetik katalog şunları içeriyordu:
- 145 `yetenekler` satırı (ortalama 5 983 karakter; gerçek ortalama 8 289).
- 11 `kod_hazinesi` satırı.
- Dış içerikli `yetenekler_fts`; onu senkron tutan üç tetikleyici.
- İçerikli `kod_hazinesi_fts`.
- fts5'ten sonra kurulmuş iki tablo.

SQLite 3.45.1, Turso 0.8.2 ve Turso `main` (`331b1f9`) kullanıldı. Linux bulut konteyneri, debug derleme.

| | Göçten önce | Göçten sonra |
|---|---|---|
| Turso 0.8.2, fts5'ten sonraki tablo | `no such table: ajan_tekmilleri` | okunuyor |
| Turso `main`, açılış (`read_only` false/true) | `Virtual table module not found: fts5` | açılıyor |
| `fihrist-izle --kur` | reddedildi (sanal tablo var) | 4 tablo izleniyor; Turso'nun ve SQLite'ın `UPDATE`'i günlüğe düştü |
| `PRAGMA integrity_check` | | `ok` |

Arama dosyasının kendisi:
- `ATTACH` sonrası **nitelemesiz** ad çözülüyor: `yetenekler_fts MATCH …` ve `snippet(…)` sorgu metni değişmeden çalıştı. `id` üzerinden birleştirme de çalıştı.
- Damga hesabının ortancası:
  - dizinlenen bütün metin (871 bin karakter): 9,6 ms;
  - yalnız `id:icerik_hash`: 0,16 ms.

  İçerik değişince damga değişti.
- Tam yeniden kurulum (145 satır) beş koşuda 36–43 ms, ortanca 39 ms.
- Arama dosyası 1,4 MB. Metnin ikinci kopyası olduğu için ana kataloğun metin hacmi kadar büyür.

## Reddedilen seçenekler

- **Turso'nun kendi fts dizini** (`CREATE INDEX … USING fts`, tantivy). Ölçüldü:
  - Dizin bir kez kurulunca SQLite dosyayı **hiç açamıyor**: `malformed database schema (__turso_internal_fts_dir_…) - near "USING": syntax error`. Python sqlite3 de sqlite3 CLI de aynı hatayı verdi.
  - Kataloğa SQLite ile yazanlar var (`ilkleme.py`, `toplu_aktarim.py`; ADR 0003, bulgu 1). Hepsi kırılırdı.
  - Özellik deneysel; `experimental_index_method` bayrağı ister.
  - Türkçe katlama yok: `ışık` araması `IŞIK`'ı ve `Işık`'ı bulmuyor; `istanbul` araması `İstanbul`'u bulmuyor.
  
  Python tarafı emekliye ayrılırsa yeniden bakılabilir; ama o durumda aşağıdaki seçenek daha sadedir.
- **fts5'i tamamen silmek.** En sade yol: ikinci dosya da senkron da gerekmez. Ancak Python tarafı bu tabloları kullanıyor. **Ercan Python tarafının fts5'e artık ihtiyacı olmadığını söylerse önerilen kararın yerine bu geçer:** göçte 2. ve 6. adımlar atlanır.
- **Arama dosyasında dış içerikli fts5.** `content=` tabloyu fts5 tablosunun kendi dosyasında arıyor: `rebuild` komutu `no such table: ara.yetenekler` ile düştü (ölçüldü).
- **Dosyalar arası tetikleyiciyle senkron.** SQLite nitelemeli adı reddediyor: `qualified table names are not allowed on INSERT, UPDATE, and DELETE statements within triggers`. Nitelemesiz ad `main`'e çözülüyor ve yazış düşüyor (ölçüldü). `TEMP` tetikleyici yalnız kendi bağlantısında yaşar, öteki yazanları kaçırır.
- **0.8.x'te beklemek.** Bugünkü kesilme sürer: Turso'nun ana kataloğa yaptığı yazışlarda fts5'ten sonraki indeksler ve tetikleyiciler atlanabilir (ADR 0005). Upstream düzeltmeleri de gelmez.
- **Upstream'den salt-okur açılış istemek** (gönderilecek 2 numaralı taslak). Yalnız okumayı kurtarır; `ibnunnedim` ise ana kataloğa yazıyor (`tekmil`, `gomme`). #9350'nin gerekçesi de bu yazışların fts dizinini bayatlatması. Taslak kendi başına gönderilebilir, bu kararı değiştirmez.

## Sonuçlar (kabul edilirse)

- Ana katalog Turso'nun her sürümüyle açılır ve kesilmeden okunur. ADR 0003, 0004 ve 0005'teki "ana katalog izlenmez" kısıtı kalkar:
  - G-1 kurulum betiğindeki katalog reddi kaldırılır.
  - MCP kaynak beyaz listesine `kutup_kutuphane` eklenebilir (ayrı iş, ADR 0004 güncellenir).
  - Tetikleyiciler kurulunca Python'un yazışı da `json_array` ister (SQLite ≥ 3.38; ADR 0003).
- Bir sonraki Turso yükseltmesi ancak bu göç gerçek dosyada yapıldıktan sonra yapılır.
- `ibnunnedim info` ve MCP çıktısındaki fts satırı (`fts_indeksleri`) ana dosyaya bakıyor; göçten sonra "yok" der. Ya arama dosyasına bakacak biçimde değişir ya da satır kaldırılır.
- CONTEXT.md'ye **Arama dosyası** terimi girer, 179. satır güncellenir.
- Türkçe katlama bu kararın konusu değil. fts5 `unicode61` bugün de `ışık` ile `IŞIK`'ı eşlemiyor (ölçüldü); göç bu durumu değiştirmez.

## Ölçülmedi ya da görülmedi

- Gerçek `kutup_kutuphane.db`'nin şeması. fts5'in dış içerikli olup olmadığı, tetikleyicileri ve sütunları görülmedi; yukarıdaki sentetik dosya bir tahmindir. Göç betiği bu yüzden şemayı dosyadan okur.
- Python okurlarının fts5 sorguları (`ibnunnedim_cli.py`, `ilkleme.py`) görülmedi. `rowid` ile birleştiriyorlarsa `id`'ye çevrilmeleri gerekir.
- Yeniden kurulum süresi gerçek metinle ve Windows'ta ölçülmedi.
- Bugünkü olası indeks bozulmasının (2026-09-10'daki `wrong # of entries in index`) göçten önce SQLite `REINDEX` ile onarılıp onarılamayacağı ölçülmedi. G-1 provası katalog sağlığını raporluyor; göç ancak sağlıklı bir kopyadan başlar.
