# Ana katalogdaki fts5 tabloları ayrı, türetilmiş bir arama dosyasına taşınır

**Durum: kabul edildi** (Ercan, 2026-10-07). Göç betiği: `crates/fihrist-canli/deneme/arama-gocu.ts`.

ADR 0005 bir sonucu açık bıraktı: Turso'nun bir sonraki sürümü `kutup_kutuphane.db`'yi açmayacak. Dosyada fts5 sanal tabloları var (`yetenekler_fts`, `kod_hazinesi_fts`). `main` (#9350) modülü bilinmeyen sanal tablo içeren dosyayı salt-okur kipte bile reddediyor. Bugünkü 0.8.2 dosyayı açıyor ama şemayı ilk fts5 satırında kesiyor (ADR 0003, bulgu 3). fts5'ten sonra gelen tablolar, indeksler ve tetikleyiciler Turso'ya görünmüyor; bu yüzden ana katalog izlenemiyor. Bu iki sorun aynı kökten geliyor. Python tarafı (`ibnunnedim_cli.py`, `ilkleme.py`) fts5 tablolarını kullanıyor (`ibnunnedim-cli/src/main.rs:823-826`, 2026-09-10); Rust tarafı aramayı bellekte BM25 ile yapıyor ve fts5'i okumuyor.

## Karar

- `yetenekler_fts` ve `kod_hazinesi_fts`, ana kataloğun yanındaki **arama dosyasına** (`kutup_arama.db`) taşınır. Ana katalogda sanal tablo kalmaz.
- **Arama dosyası türetilmiştir.** Doğruluk kaynağı ana katalogdur (bge-dizin ile aynı ilke). Arama dosyasını yalnız SQLite istemcileri açar, `ATTACH` ile; Turso onu hiç açmaz.
- Betik her fts5 tablosunu `sqlite_master`'dan okur ve türüne göre taşır; adları varsaymaz:
  - **Dış içerikli** (`content=T`): arama dosyasında içerikli tablo olarak T'den kurulur, rowid korunur. T'nin birincil anahtarı sona `UNINDEXED` sütun olarak eklenir. Sona eklenir ki `snippet`, `highlight` ve `bm25`'in sütun sıraları değişmesin. Tokenizer ve öteki seçenekler aynen kalır. `--tazele` bu tabloyu T'den yeniden kurabilir.
  - **İçerikli** (`content` yok): kaynak tablosu tanımsızdır; satırlar rowid'leriyle aynen taşınır. Onu yazan araç `ATTACH` ile yazmayı sürdürür (nitelemesiz ad `INSERT`'te de çözülür). `--tazele` dokunmaz.
  - **Durdurur:** içeriksiz fts5 (`content=''`, kaynak metin yok), fts5 dışında sanal tablo, fts5'e dayanan görünüm (görünüm başka dosyadaki tabloya bakamaz), bileşik anahtarlı kaynak.
- **Birleştirme `id` ile önerilir, `rowid` ile değil.** `TEXT PRIMARY KEY`'li tabloda `rowid` kalıcı değildir:
  - Döküp yeniden yükleme (`.dump` → yeni dosya) 134 satırın 132'sinin rowid'ini değiştirdi (ölçüldü).
  - `VACUUM` 3.45.1'de değiştirmedi (134'te 0), ama SQLite belgesi değiştirebileceğini söylüyor.
  - Böyle bir durumda `rowid` birleştirmesi sessizce yanlış satırı verir; `id` birleştirmesi doğru kalır. Rowid yine de korunur; bugünkü `rowid` birleştirmeleri göçten hemen sonra çalışır.
- **Tazelik `--tazele` ile.** Arama dosyasındaki `arama_meta` tablosu her dış içerikli tablonun damgasını saklar. `--tazele` damgayı kaynaktan yeniden hesaplar; tutmuyorsa o tabloyu tek işlemde baştan kurar. Arka plan süreci ya da tüketici gerekmez. Python okuru aramadan önce `bun arama-gocu.ts --dizin … --tazele` çağırır.
  - Damga: kaynak satırları rowid sırasıyla; satır `[rowid, sütunlar…, anahtar]`, boşluksuz JSON dizisi + `"\n"`, UTF-8, sha256 onaltılık.
  - Python aynı damgayı `json.dumps(satir, ensure_ascii=False, separators=(",", ":"))` ile üretir. Tırnak, ters bölü, denetim karakteri, U+2028, emoji, Arapça hareke ve NULL içeren satırlarda iki dilin özeti aynı çıktı (ölçüldü). Ondalık sayı denenmedi; fts5 sütunları metin.
- **Göç** (`arama-gocu.ts`, düzeni G-1 kurulum betiğiyle aynı):
  - Önce yedek alınır (`sqlite3 .backup`, salt-okunur). PROVA yedeğin kopyasında koşar. GERÇEK (`--gercek`) yalnız prova yeşilse ve dosya yedekten beri değişmediyse koşar.
  - Arama dosyasının kurulumu ve ana kataloğun temizliği tek bağlantıda, tek işlemdedir (`ATTACH`, `BEGIN IMMEDIATE`).
  - fts5'e dokunan bütün tetikleyiciler düşer. Biri kalırsa ana tabloya her yazış `no such table: main.yetenekler_fts` ile düşer (ölçüldü). Betik bunu, tetikleyicisi düşen her tabloya geri alınan bir deneme yazışıyla doğrular.
  - Commit'ten önce şunlar doğrulanır:
    - satır sayısı ve içerik özeti kaynakla aynı;
    - seçenekler korunmuş;
    - fts5 `integrity-check` geçiyor;
    - örnek bir sözcük `MATCH` ile kendi satırını buluyor.

    Herhangi biri tutmazsa işlem geri alınır, katalog değişmez.
  - Commit'ten sonra iki dosyada `integrity_check` çalışır. Arama dosyası `ATTACH` edilip nitelemesiz `MATCH` denenir. `--ikili` verilmişse Turso'nun bütün tabloları gördüğü de ölçülür.
  - WAL kipinde çok dosyalı commit çökmeye karşı atomik değildir. Yarım kalan göç yeniden koşulunca algılanır:
    - katalogda fts5 varken bulunan arama dosyası silinmez, kenara alınır;
    - katalogda fts5 yokken arama dosyası tanınmıyorsa betik açık hatayla durur.
  - Geri dönüş yedekten yapılır; yolu raporda yazılıdır.

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

Göç betiği (sentetik, Linux, Bun 1.4.2):
- Altı fikstürde 32 denetimin 32'si geçti:
  - ana senaryo (WAL kipi, rowid boşlukları, ilgisiz bir tetikleyici, Arapça metin): PROVA, GERÇEK, yeniden koşu ve `--tazele`;
  - içeriksiz fts5, görünüm ve rtree: durdu, dosya değişmedi;
  - anahtarı fts5'te zaten olan tablo;
  - yarım kalmış göçün artığı;
  - işlem ortasında hata: geri alındı, katalog değişmedi.
- Dokuz mutasyonun dokuzu yakalandı. Her biri betiğin kendi adımında düştü:
  - tetikleyiciyi düşürmemek: deneme yazışı düştü;
  - rowid'i korumamak: içerik özeti tutmadı;
  - seçenekleri düşürmek: seçenek karşılaştırması tutmadı;
  - `--tazele`'yi bozmak: yeniden kurulum sınaması düştü.
- Göçten sonra G-1 kurulum betiği kataloğu öteki dosyalar gibi kurdu. Üç koşuda 78 adımın 78'i geçti; ekleme, güncelleme ve silme yaklaşık 250 ms'de bildirildi.
- **Kilit çakışması** (göçten bağımsız): Turso dosyayı bloklamayan `fcntl` kilidiyle açıyor (`turso_core-0.8.2/io/unix.rs:278-300`). `fihrist-izle` 250 ms'de bir yoklarken Turso'nun 200 yazışından 5'i `Locking error` aldı; izleyici kapalıyken 100 yazışta 0. Kurulum betiği bunun için sınırlı yeniden deneme yapar ve sayısını rapora yazar. İzleyici açıkken `ibnunnedim`'in kendi yazışlarına etkisi ayrı bir iştir.

## Reddedilen seçenekler

- **Turso'nun kendi fts dizini** (`CREATE INDEX … USING fts`, tantivy). Ölçüldü:
  - Dizin bir kez kurulunca SQLite dosyayı **hiç açamıyor**: `malformed database schema (__turso_internal_fts_dir_…) - near "USING": syntax error`. Python sqlite3 de sqlite3 CLI de aynı hatayı verdi.
  - Kataloğa SQLite ile yazanlar var (`ilkleme.py`, `toplu_aktarim.py`; ADR 0003, bulgu 1). Hepsi kırılırdı.
  - Özellik deneysel; `experimental_index_method` bayrağı ister.
  - Türkçe katlama yok: `ışık` araması `IŞIK`'ı ve `Işık`'ı bulmuyor; `istanbul` araması `İstanbul`'u bulmuyor.

  Python tarafı emekliye ayrılırsa yeniden bakılabilir; ama o durumda aşağıdaki seçenek daha sadedir.
- **fts5'i tamamen silmek.** En sade yol: ikinci dosya da senkron da gerekmez. Ancak Python tarafı bu tabloları kullanıyor; Ercan taşımayı seçti (2026-10-07). Python tarafı emekliye ayrılırsa arama dosyası silinir, başka bir şey gerekmez.
- **Arama dosyasında dış içerikli fts5.** `content=` tabloyu fts5 tablosunun kendi dosyasında arıyor: `rebuild` komutu `no such table: ara.yetenekler` ile düştü (ölçüldü).
- **Dosyalar arası tetikleyiciyle senkron.** SQLite nitelemeli adı reddediyor: `qualified table names are not allowed on INSERT, UPDATE, and DELETE statements within triggers`. Nitelemesiz ad `main`'e çözülüyor ve yazış düşüyor (ölçüldü). `TEMP` tetikleyici yalnız kendi bağlantısında yaşar, öteki yazanları kaçırır.
- **0.8.x'te beklemek.** Bugünkü kesilme sürer: Turso'nun ana kataloğa yaptığı yazışlarda fts5'ten sonraki indeksler ve tetikleyiciler atlanabilir (ADR 0005). Upstream düzeltmeleri de gelmez.
- **Upstream'den salt-okur açılış istemek** (gönderilecek 2 numaralı taslak). Yalnız okumayı kurtarır; `ibnunnedim` ise ana kataloğa yazıyor (`tekmil`, `gomme`). #9350'nin gerekçesi de bu yazışların fts dizinini bayatlatması. Taslak kendi başına gönderilebilir, bu kararı değiştirmez.

## Sonuçlar

- Ana katalog Turso'nun her sürümüyle açılır ve kesilmeden okunur. ADR 0003, 0004 ve 0005'teki "ana katalog izlenmez" kısıtı kalkar:
  - G-1 kurulum betiği, katalogda sanal tablo yoksa onu öteki dosyalar gibi kurar (yapıldı); göçten önce reddetmeyi sürdürür.
  - MCP kaynak beyaz listesine `kutup_kutuphane` eklenebilir (ayrı iş, ADR 0004 güncellenir).
  - Tetikleyiciler kurulunca Python'un yazışı da `json_array` ister (SQLite ≥ 3.38; ADR 0003).
- Bir sonraki Turso yükseltmesi ancak bu göç gerçek dosyada yapıldıktan sonra yapılır.
- `ibnunnedim info` ve MCP çıktısındaki fts satırı (`fts_indeksleri`) ana dosyaya bakıyor; göçten sonra "yok" der. Ya arama dosyasına bakacak biçimde değişir ya da satır kaldırılır.
- CONTEXT.md'ye **Arama dosyası** terimi girdi.
- Python okurları `ATTACH`'a çevrilir ve aramadan önce `--tazele` çağırır (Python kodu bu depoda değil; Ercan).
- Türkçe katlama bu kararın konusu değil. fts5 `unicode61` bugün de `ışık` ile `IŞIK`'ı eşlemiyor (ölçüldü). Arapça harekeleri de kaldırmıyor: `remove_diacritics` yalnız Latin harflerine işler. Harekesiz `كتاب` sorgusu harekeli metinde 0 satır, harekeli `كِتَابُ` 21 satır buldu (ölçüldü). Göç tokenizer'ı aynen taşır, bu durumu değiştirmez.

## Ölçülmedi ya da görülmedi

- Gerçek `kutup_kutuphane.db`'nin şeması. fts5'in dış içerikli olup olmadığı, tetikleyicileri ve sütunları görülmedi; yukarıdaki sentetik dosya bir tahmindir. Göç betiği bu yüzden şemayı dosyadan okur.
- Python okurlarının fts5 sorguları (`ibnunnedim_cli.py`, `ilkleme.py`) görülmedi. Rowid korunduğu için `rowid` birleştirmeleri göçten sonra da çalışır; kalıcı olması için `id`'ye çevrilmeleri önerilir.
- Yeniden kurulum süresi gerçek metinle ve Windows'ta ölçülmedi. Göç betiği Windows'ta koşulmadı; `bun:sqlite`'ın orada fts5 taşıdığını betik ön koşulda sınar.
- Bugünkü olası indeks bozulmasının (2026-09-10'daki `wrong # of entries in index`) göçten önce SQLite `REINDEX` ile onarılıp onarılamayacağı ölçülmedi. G-1 provası katalog sağlığını raporluyor; göç ancak sağlıklı bir kopyadan başlar.

## Uygulama (2026-10-09, gerçek dosyada)

Göç Windows'ta, gerçek `hermes yazılım\kutuphane` dizininde prova + `--gercek` ile koştu: kırmızı 0, uyarı 2. Rapor ve yedek: `kutuphane\arama-gocu-20261009\`.

- Gerçek şema: iki tablo da **dış içerikli** (`yetenekler_fts` ← `yetenekler`, 147 satır; `kod_hazinesi_fts` ← `kod_hazinesi`, 11 satır, `id` UNINDEXED eklendi). fts5'e dokunan tetikleyici **yoktu**: eski dizin hiçbir yazışla güncellenmiyordu.
- Uyarılar bunu doğruladı: `kod_hazinesi_fts`'te "altin" eski dizinde 0, kaynaktan kurulan yenide 1 satır (eski dizin bayattı).
- İki dosyada `integrity_check` ok; `ATTACH` ile nitelemesiz `MATCH` çalışıyor; Turso 0.8.2 göçten sonra 7/7 tabloyu görüyor.
- `ibnunnedim info` artık "FTS5 indeks: YOK (tanımlı; arama saf Rust BM25 ile yapılıyor)" diyor (yukarıdaki açık nokta).
- Python okuru `ibnunnedim_cli.py` `kutup_arama.db`'yi `ATTACH` ediyor (fts sorgusu "rust" için 33 satır, `database_list` = main, arama). `--tazele` çağrısı eklenmedi; tetikleyici olmadığından eski dizin de hiç taze değildi, bayatlık göçle gelmedi.
- `ilkleme.py` ve `toplu_aktarim.py` kataloğu `os.remove` ile silip `kutup_kutuphane.sql`'deki fts5'li şemayı kurar, yani bu göçü geri alır ve veriyi yok eder. İkisinin başına koşulsuz `SystemExit` kondu (yolları zaten ölü OneDrive yoluydu; kimse çağırmıyor).
