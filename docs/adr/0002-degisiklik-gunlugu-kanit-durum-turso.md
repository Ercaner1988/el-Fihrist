# Değişiklik günlüğü dal başına JSONL dosyasıdır ve durumdan önce yazılır; durum turso'da kalır

ADR 0001 değişikliklerin "yalnız ekleme yapılan bir günlükte" tutulmasını söyledi, günlüğün nerede duracağını söylemedi. Karar:

- **Günlük** (kanıt ve ön-yazma): her **Dal** için ayrı bir JSON Lines dosyası; **Sınıf sözleşmesi** crate'i (`sinif-sozlesmesi`) biçimini ve yazma kuralını taşır.
- **Durum**: her aracın kendi **Araç veritabanı**nda (turso), ADR 0001'deki gibi.
- **Yazma sırası: önce günlük, sonra durum.** Günlük kaydı `sync_data` ile diske işlendikten sonra durum yazılır. Araya çökme girerse günlük durumun önünde kalır ve yeniden oynatılarak onarılır; tersi sırada iz kaybolurdu.
- Dal başına tek yazar olduğu için günlük dosyasında kilit sorunu yoktur. Dosya adı dal kimliğinden birebir (geri çevrilebilir) kodlanır; iki dal asla aynı dosyaya düşmez.
- Yalnız `\n` ile kapanmış satır kayıttır. Çökmeden kalan yarım son satır kayıt sayılmaz, okuyana bildirilir ve bir sonraki yazmada budanır. Ortadaki bozuk satır hatadır, atlanmaz.

## Reddedilen seçenekler

- **Günlüğü turso tablosunda tutmak:** durum ve günlük tek işlemde yazılır, atomiklik kazanılırdı. Ama sınıf sözleşmesi crate'i turso ve tokio'ya bağımlı olurdu; her araç bunu bu küçük, izin verici lisanslı crate üzerinden taşımak zorunda kalırdı. Atomiklik yerine, yeniden oynatılabilir ön-yazma kabul edildi. Turso çok süreçli yazmayı kararlı sunduğunda yeniden bakılabilir.
- **Bozuk satırı sessizce atlamak:** kaybolan kaydı görünmez kılar.

## Sonuçlar

- Günlük ile durum arasındaki tutarlılık bir **kural**a (yazma sırası) dayanır, işlem garantisine değil; onarım yeniden oynatmayla yapılır ve bu, tüketici araçların işidir (yol haritası 2. adım).
- Günlük dosyaları küçük kalmalıdır (dal başına); büyürse sondan tarayan bir budama gerekir.
