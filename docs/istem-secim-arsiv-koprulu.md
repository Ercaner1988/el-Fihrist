# İstem: Seçim günlüğü → Oturum arşivi → Köprülü

> Bu dosyayı yeni bir Claude Code oturumuna olduğu gibi ver. Çalışma dizini: `C:\Users\buzbe\Desktop\Github\rust\el-Fihrist` (Nazar işi için `..\Nazar` de gerekir).

## Bağlam

2026-10-13/14 tarihli bir grill oturumunda kararlar alındı. Önce şunları oku, terimleri oradaki anlamlarıyla kullan:

- el-Fihrist `CONTEXT.md`: **Oturum arşivi**, **Mahremiyet** bölümü (Mahremiyet katmanı, Bulut model, Yerel model, Köprülü, Arka MCP, Kapsam dışı bağlayıcı, Takma adlandırma) ve İlişkiler'deki iki katman kuralı.
- el-Fihrist `docs/adr/0007-koprulu-tek-mcp-kapisi.md`: Köprülü kararı, sınırları, iş sırası, gömme modeli kapısı.
- Nazar `CONTEXT.md`: **Sıralama işlevi**, **Seçim**, **Seçim günlüğü**.

Kararları yeniden tartışma. Bir karar kodla çelişiyorsa ya da uygulanamıyorsa dur ve Ercan'a sor.

## İş sırası

Her adımın sonunda dur ve Ercan'a göster: ne yapıldı, testlerin ham çıktısı, açık kalan ne. Bir sonraki adıma onay almadan geçme.

### Adım 1: Nazar'da Seçim günlüğü (öncelikli, hemen)

Neden önce: Sıralama işlevini (BM25 puanlaması + Harmanlama ağırlıkları + "eğer…" kuralları, örn. commit f31aa33) yeniden kurmak için veri gerekiyor. 2026-09-27'de "bir ay veri biriksin" denmişti ama hiç günlük tutulmamış; `%APPDATA%\nazar\masaustu-gecmis.txt` yalnız son 50 sorgu metnini tutuyor. Her gün gecikme bir günlük veri kaybı.

- Her arama için kaydet: sorgu, kavram silsilesi, gösterilen sonuçlar (sırası ile), her sonucun kol puanları (BM25, vektör, ad) ve harmanlanmış puanı, kanal (`masaustu` / `mcp`), zaman, varsa oturum kimliği.
- **Masaüstü Seçimi** anında yazılır: kişi bir sonucu açınca o aramaya bağlanır.
- **MCP Seçimi** canlı izlenmez. Adım 2'deki Oturum arşivinden sonradan çıkarılır (bkz. aşağı). Bu adımda yalnız MCP aramalarının kendisi kaydedilir ve sonradan eşleme için oturum kimliği tutulur.
- Kayıt Nazar'ın kendi veritabanında (Turso) durur. Önce Nazar'da hangi veritabanı/şema olduğunu kodda bul; varsayma.
- Masaüstündeki son 50 sorgu listesi ("arama geçmişi") ayrı şeydir; ona dokunma.

### Adım 2: Oturum arşivi (`ibnunnedim arsivle`)

- Kaynak: Claude Code dökümleri, `~/.claude/projects/**/*.jsonl`. Dökümler özetleme öncesi konuşmanın tamamını `compact_boundary` işaretleriyle saklar; yani ayrı bir bağlam sıkıştırma aracı gerekmez.
- Arşive girer: Ercan'ın mesajları, modelin cevapları, araç çağrılarının adı ve girdisi, sınırdaki özet. **Girmez:** araç çıktısının gövdesi; yalnız izi ve dökümdeki yerine işaret.
- Mahremiyet katmanı: **Kişisel**.
- Tetik: `PreCompact` kancası `ibnunnedim arsivle <döküm>` çağırır; `SessionStart` kancası arşivlenmemiş dökümleri tamamlar. Arşivleme artımlıdır (aynı bölüm iki kez girmez).
- Arşiv aranabilir olmalı (önce el-Fihrist'te mevcut arama yolunu kullan; ADR 0005/0006'daki Turso fts5 tuzağına dikkat).
- **MCP Seçimi eşlemesi:** bir `nazar_ara` sonucundaki yollardan biri aynı oturumda sonraki herhangi bir araç çağrısının (`Read`, `Edit`, `Grep`…) girdisinde geçiyorsa, o sonuç Seçim sayılır ve Nazar Seçim günlüğüne `kanal = mcp` olarak yazılır. Toplu iş olarak koşar. "Okundu ama işe yaramadı" da Seçim sayılır; bilinen gürültü, kabul edildi.

### Adım 3: Köprülü ilk sürümü

ADR 0007'ye göre: tek MCP kapısı; her arka araç kendi adıyla yansıtılır (`mcp__kopru__nazar_ara` gibi); Arka MCP ilk çağrıda başlar, boşta durur; yazan/silen araçlar **onaylı** işaretlenir. Bu sürümde mahremiyet süzgeci **yok**. `fihrist-santral` yerinde kalır. Başlamadan önce Ercan'la arka MCP listesini ve boşta durdurma süresini netleştir.

### Sonraki adımlar (bu oturumda başlama)

4. Takma adlandırma + yerel ad bulucu model (Kişi kayıtları sözlük).
5. Gömme adaylarının Kaggle'da ölçümü (e5-large-instruct, arctic-embed-l-v2, Qwen3-Embedding-0.6B, EG2); kapı ADR 0007 ekinde. Türkçe çiftler kurulacak. Veri: `C:\Users\buzbe\Desktop\gomme-paket`.
6. EmbeddingGemma 2 ayrı görsel/ses kanalı (candle 0.9.2'de gemma4 yok; yükseltme gerekir).
- Toplu özetleyici (Yerel ön işleme) 4'ten sonra.
- Xenolith'in Rust'a çevrilmesi tezden sonra.

## Kesin kısıtlar

- CLAUDE.md kuralları geçerli: Bun-first; egui + WGPU (Vulkan önce); rkyv (bincode yasak); crate başına ≤1500 kod satırı (`altin-kapi\taban\sandik-siniri.toml`); Türkçe UTF-8, çıplak `to_lowercase` yok.
- Gerçek veritabanlarına yazma yok; kopya yalnız `sqlite3 "file:<yol>?mode=ro" ".backup '<hedef>'"` ile.
- Bu makinede `bili plugin install claude` (billion-context) **kurulmaz**: Masaüstü Code sekmesi vekili atlıyor ama `DISABLE_AUTO_COMPACT=1` yine uygulanıyor, oturum hiç sıkıştırılmaz.
- zopay'a dokunma (başka oturumlar koşuyor). `--admin` yok. GPG imzası Ercan'ın işi. Yıkıcı işlem yok. Push ve GitHub işlemi Ercan sormadan yok.
- Her davranış değişikliği bir test ile gelir; "çalışıyor" demeden önce ham çıktıyı göster.
