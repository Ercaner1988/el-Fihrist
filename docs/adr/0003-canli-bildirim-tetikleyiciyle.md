# Canlı değişiklik bildirimi şemadaki tetikleyicilerle yapılır, Turso'nun CDC'siyle değil

Yol haritası Görev 1 (`docs/gorev-canli-sorgu-ve-vektor-dizini.md`), Turso'nun bağlantı başına CDC'sini önermişti. 2026-10-02'de gerçek veritabanlarının kopyaları üzerinde denendi. Karar:

- Her izlenen tabloya üç tetikleyici kurulur (ekleme `e`, güncelleme `g`, silme `s`). Tetikleyiciler `fihrist_degisiklik` günlüğüne satır yazar: numara, zaman (ms), tablo, işlem, birincil anahtarın JSON dizisi.
- Tüketici günlüğü kısa aralıkla yoklar ve **her yoklamada veritabanını taze açar**. İmleci, veritabanının yanındaki `<db>.imlec/<tüketici>` dosyasında tutar. Teslim en az bir kez olur: imleç, olaylar işlendikten sonra yazılır.
- Budama, en yavaş tüketicinin geçtiği satırları siler, **son satırı hep bırakır**. Tablo boşalırsa `INTEGER PRIMARY KEY` numarayı 1'den yeniden verirdi ve imleci ileride kalan tüketici yeni olayları atlardı.
- fts5 sanal tablosu olan dosya (`kutup_kutuphane.db`) **izlenmez**: kurulum reddedilir (bkz. bulgu 3).
- Kod `crates/fihrist-canli`'de, ikili `fihrist-izle`. `ibnunnedim-cli` sandık sınırının üstünde olduğu için büyütülmedi.

## Ölçülen bulgular (Turso 0.7.2)

1. **CDC yalnız kendi bağlantısını görür.** `unstable_capture_data_changes_conn('full')` Turso'nun yazışını yakaladı, aynı dosyaya Python `sqlite3` ile yapılan yazışı kaçırdı. Kütüphaneyi Python betikleri de besliyor (`ilkleme.py`, `toplu_aktarim.py`). Ayrıca CDC birincil anahtarı değil rowid'i kaydediyor.
2. **Tetikleyici her yazanı yakalar.** Turso, Python `sqlite3` ve sqlite3 CLI'nin yazışlarının hepsi, gerçek anahtarla günlüğe düştü. `integrity_check` sonucu `ok`.
3. **fts5 şemayı keser.** Turso'da fts5 modülü yok. Şema yüklemesi ilk fts5 satırında hata alıyor, hata yutuluyor ve sonraki her şey yok sayılıyor. Sonradan eklenen günlük ve tetikleyiciler Turso'ya görünmüyor; Turso'nun o dosyaya yaptığı yazış tetikleyiciyi hatasız atlıyor. (2026-09-10'da `depolar` tablosunda görülen kesilmeyle aynı kök.)
4. **Açık bağlantı bayat kalır.** Uzun süre açık tutulan bir Turso bağlantısı başka süreçlerin commit'lerini görmedi; taze açılan hemen gördü. Yeniden açmanın maliyeti 56 MB'lık `kutup_kayitlar.db`'de ~5 ms.
5. **`pragma_table_info(...)` yazış kaybettirir.** Bu tablo fonksiyonu aynı bağlantıda bir kez çalışınca, sonraki otomatik-commit yazışlar hatasız ama kalıcı olmadan kayboluyor. Açık `BEGIN/COMMIT` kurtarıyor; düz `PRAGMA table_info` zararsız. Ayrıca Turso bileşik anahtarda bütün `pk` değerlerini 1 veriyor (SQLite 1, 2, 3, … verir).

3 ve 5 Turso'ya hata bildirimi olarak gönderilmeli.

## Reddedilen seçenekler

- **Turso CDC:** Turso dışından yapılan yazışları kaçırır (bulgu 1). Pragma da deneysel (`unstable_`).
- **Günlüğü tüketiciye itmek (süreç içi yayın):** ek süreç ya da sunucu ister. Yoklama, tek dosya düzenini korur; ölçülen gecikme 1 saniyenin altında.
- **İmleci veritabanında tutmak:** yoklama döngüsü her öbekte veritabanına yazardı; dosya ile döngü salt-okur kalır.

## Sonuçlar

- Tetikleyici şemada durduğu için, o veritabanına yazan **her araç** (zopay'ın rusqlite'ı dahil) her yazışında bir günlük satırı da yazar. Tetikleyiciler `json_array` kullanır; SQLite 3.38'den eskisi bu tablolara yazamaz.
- Tetikleyiciler kurulum anındaki tabloları kapsar. Sonradan eklenen tablo için kurulum yeniden koşulur; iki kez koşmak zararsız.
- Ana katalog, Turso fts5'i desteklene ya da fts tabloları ayrı dosyaya taşınana kadar izlenmez.
- Kabul ölçümleri (kopyalarda, 250 ms yoklama): en kötü gecikme 411 ms; öldür, yaz, başlat sonrası kayıp ve tekrar yok; ana katalogda kurulum reddi, dosya sha256 değişmedi.
