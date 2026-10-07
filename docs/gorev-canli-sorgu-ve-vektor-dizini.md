# Görev: Canlı değişiklik bildirimi ve vektör dizini

İki görev, SurrealDB ile yapılan karşılaştırmadan (2026-09-30) çıktı. SurrealDB'ye
geçmeden, onun bize gerçekten yarayacak iki özelliğini Turso üstünde kazanmak
istiyoruz. Taşıma yok: Turso tek dosya, gömülü, MIT lisanslı kalır.

Turso durumu `turso` / `turso_core` **0.7.2** kaynağından okundu (Cargo.lock'taki sürüm).

## Görev 1: Canlı değişiklik bildirimi ve canlı sorgu

> **Durum (2026-10-02):** değişiklik bildirimi yapıldı, ama aşağıdaki CDC tasarımıyla
> değil, şemadaki tetikleyicilerle. CDC, Python `sqlite3` yazışlarını kaçırıyor.
> Gerekçe ve ölçümler: ADR 0003. Kod: `crates/fihrist-canli`, ikili `fihrist-izle`.
> Ana katalog (`kutup_kutuphane.db`, fts5) izlenmiyor.
> **Durum (2026-10-06):** canlı sorgu yapıldı (`fihrist_canli::abone_ol`,
> `fihrist-izle --sorgu`; abone geçici, fark durum tabanlı, CONTEXT.md "Canlı
> bildirim"). MCP `resources/subscribe` + `notifications/resources/updated`
> yapıldı (`crates/fihrist-kaynak`, ADR 0004; ana katalog yine dışarıda).
> **Kalan:** gerçek dosyalara tetikleyici kurulumu (Ercan'ın onayı);
> `ibnunnedim izle` adının `fihrist-izle --sorgu`ya ek olarak gerekip gerekmediği.

**Neden:** el-Fihrist MCP sunucusu, Paslı Beyin, graphify nöbetçisi ve Open Notebook
aktarımı veritabanını yoklamadan değişiklik haberi alsın. SurrealDB'deki
`LIVE SELECT`in karşılığı.

**Turso 0.7.2'de olan:** bağlantı başına açılan, deneysel değişiklik yakalama (CDC).
Değişiklikler `turso_cdc` tablosuna yazılır (`turso_core-0.7.2/connection.rs:385`,
`capture_data_changes`; pragma adı `unstable_` önekli). İtme yok: tablo okunmalı.

**Önerilen tasarım:**
- Yazan her bağlantıda CDC'yi aç.
- `turso_cdc` tablosunu son okunan değişiklik kimliğinden (imleç) kısa aralıkla oku.
  Olayları abonelere yay (`tokio::sync::broadcast`).
- **Canlı sorgu:** abone olunan bir sorgu, ilgili tablo değişince yeniden çalıştırılır.
  Aboneye yalnız fark gönderilir (eklenen / değişen / silinen satırlar).
- **Arayüzler:**
  - CLI: `ibnunnedim izle <tablo> [--sorgu ...]`.
  - MCP: `notifications/resources/updated` bildirimi.
- İmleç kalıcı tutulur: süreç yeniden başlayınca kaldığı yerden devam eder, olay kaybolmaz.
  `turso_cdc` tablosu, tüketilen kayıtlar için budanır.

**Kabul ölçütleri:**
- Ekleme, güncelleme ve silme 1 saniye içinde bildirilir.
- Süreç çöküp yeniden başlarsa olay kaybı yok (test: yaz, öldür, başlat, say).
- Ek sunucu ya da ek süreç yok; tek dosya düzeni korunur.
- `turso_cdc` boyutu sınırlı kalır.

**Açık sorular / riskler:**
- CDC bağlantıya özgü (`_conn`). Başka bir süreç CDC'yi açmadan yazarsa değişiklik
  kaçar. Yazan bütün araçlar el-Fihrist'in yazma yolundan mı geçmeli, yoksa tetikleyici
  (trigger) mı gerekir? Turso 0.7.2'de tetikleyici desteği denetlenecek.
- Pragma deneysel (`unstable_`). Turso sürümü sabitlenmeli; her Turso güncellemesinde
  bu görevin testleri koşulmalı.

## Görev 2: Yerleşik vektör dizini ve benzerlik fonksiyonları

> **Durum (2026-10-06):** Turso 0.8.2'de de yoğun vektör dizini yok (ADR 0005), karar
> sırasının 3. adımı geçerli. Karar (Ercan): HNSW yan dizini **bge-embed-rs (bugün ibnun-nedim) deposunda
> ayrı crate** olarak yazılır; el-Fihrist onu git rev ile alır. Gömmeleri bge-embed-rs
> üretir; doğruluk kaynağı Turso tabloları. Kabul ölçümü gerçek veriyle Ercan'ın makinesinde.

**Neden:**
- Open Notebook'tan Turso'ya aktarılacak yaklaşık 91 bin bge-m3 vektöründe
  (1024 boyut, yoğun) hızlı anlam araması.
- el-Fihrist beceri ve hafıza kayıtlarında Türkçe BM25 ile karma arama.

**Turso 0.7.2'de olan:**
- Benzerlik fonksiyonları: `vector_distance_cos` ve `vector_distance_l2`. Tam tarama yapar.
- Dizin olarak yalnız deneysel `toy_vector_sparse_ivf` (`index_method/mod.rs:17`).
  Bu seyrek vektörler içindir; bge-m3 yoğun vektörüne uygun değil.
- HNSW ya da DiskANN yok. DiskANN libSQL çatalında var, ama el-Fihrist `turso` kullanıyor.
- Ayrıca `fts` özelliği ile tantivy tabanlı tam metin arama var (`USING fts`,
  `fts_match`, `fts_score`). BM25 için değerlendirilebilir, ama Türkçe katlama
  (İ→i, I→ı) doğrulanmadan kullanılmamalı.

**Karar sırası ("daha iyisi yoksa"):**
1. Turso'nun güncel sürümünde yoğun vektör dizini (DiskANN, HNSW vb.) gelmiş mi bak.
   Gelmişse onu kullan, kendi dizinini yazma.
2. Gelmemişse ölç: 100 bin × 1024 üzerinde `vector_distance_cos` tam taraması kaç ms?
   p50 < 50 ms ise dizin gerekmez; yalnız karma aramayı kur.
3. Dizin gerekiyorsa saf Rust bir HNSW crate'iyle **yan dizin** kur:
   - Doğruluk kaynağı Turso tabloları; dizin dosyası her an yeniden kurulabilir.
   - Adaylar dizinden gelir, Turso'da `vector_distance_cos` ile yeniden sıralanır.
   - MTREE gerekmez: yoğun vektörde HNSW standart ve yeterli.
4. Görev 1 bittiyse dizin CDC olaylarıyla artımlı güncellenir; bitmediyse toplu yeniden kurulur.

**Arayüzler:**
- Rust API: `vektor_ara(tablo, sorgu, k)`.
- CLI: `ibnunnedim vektor-ara`.
- MCP aracı.
- Karma arama: BM25 ve vektör sonuçları RRF (reciprocal rank fusion) ile birleşir.
- SQL içinden tablo-değerli fonksiyon olarak çağrılabilir mi? `turso_ext` ile denetlenecek.

**Kabul ölçütleri:**
- 100 bin × 1024 vektörde, tam taramaya göre recall@10 ≥ 0,95.
- p50 gecikme < 20 ms.
- Dizin silinip yeniden kurulabiliyor.
- Karma aramada Türkçe katlama testleri geçiyor.

## İki görev için ortak kurallar

- Sandık sınırı: crate başına en fazla 1500 kod satırı. Yeni iş ayrı crate'lere gider,
  örneğin `fihrist-canli` ve `fihrist-vektor`.
- Türkçe metin: içeride UTF-8; karşılaştırmadan önce NFC ve ortak Türkçe katlama.
  Çıplak `to_lowercase()` kullanılmaz.
- Bun-first: yardımcı betik gerekirse Bun ile yazılır.
