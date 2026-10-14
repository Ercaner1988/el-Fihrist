# Yerel MCP'ler tek kapıdan, Köprülü'den açılır

**Durum: kabul edildi** (Ercan, 2026-10-14; grill oturumu). Kod henüz yok.

Bir YZ arayüzü açıldığında nazar, kesfuzzunun, archify-graft, agent-reach, shamela gibi her yerel MCP ayrı bir süreç olarak kalkıyor. Birden çok oturum açıkken aynı sunucunun birden çok kopyası bellekte duruyor. Her yeni sağlayıcı ya da istemci için yetenekleri, hafızayı ve kapıları yeniden kurmak gerekiyor. Ayrıca bilgisayarın dışına giden veriyi tek yerden denetleyecek bir nokta yok. Mahremiyet katmanları (Genel, İlgilisine özel, Kişisel; bkz. CONTEXT.md, Mahremiyet) uygulanacaksa buna bir uygulama noktası gerekiyor.

## Karar

- **Köprülü**, el-Fihrist'in tek MCP kapısıdır. YZ arayüzlerine yalnız o kaydedilir; Arka MCP'ler arayüze ayrıca kaydedilmez.
- Arka MCP'ler yalnız Ercan'ın kendi yerel MCP'leridir. Başkalarının MCP'leri ancak bir Hüküm ile arkaya alınır. Gmail, Drive, Calendar gibi bulut bağlayıcıları kapsam dışıdır; katmanlar onları korumaz.
- Köprülü bir Arka MCP'yi **ilk çağrıda başlatır**, boşta kalınca **durdurur**. Böylece bellekte yalnız kullanılanlar durur.
- **Her arka araç kendi adıyla yansıtılır** (örn. `mcp__kopru__nazar_ara`). Tek bir `cagir(arac, girdi)` aracı seçilmedi, çünkü:
  - arayüzün araç başına izinleri (Claude Code `permissions.allow/deny`) korunur;
  - model araçları ad ve şemayla görür, ayrı bir yönlendirme ya da karar modeli gerekmez. Seçimi oturumun modeli yapar; araç sayısı büyürse `search_skills` daraltır.
- Yazan ya da silen araçlar Köprülü'de ayrıca **onaylı** diye işaretlenir.
- Köprülü dönen sonucu kaynağın **Mahremiyet katmanı**na göre süzer. Kişisel ya da İlgilisine özel veri Bulut modele **Takma adlandırma**dan geçmeden gitmez. Katmanı bilinmeyen kaynak Kişisel sayılır.
- `fihrist-santral` (oturumlar arası mesaj kutuları) yerinde kalır; Köprülü onun üstüne kurulur.

## Sınırlar

- Köprülü yalnız kendi içinden geçeni görür. Arayüzün kendi araçları (`Read`, `Bash`, `Grep`…) onu atlar. Bulut model bu araçlarla okuduğu her şeyi görür. Hafifletme: hassas klasörler için arayüzde `permissions.deny`. Tam mahremiyet ancak Yerel modelle sağlanır.
- Tek arıza noktası: Köprülü düşerse bütün yerel araçlar düşer.

## Sıra

1. Nazar Seçim günlüğü.
2. Oturum arşivi.
3. Köprülü'nün ilk sürümü: tek kapı, yansıtma, isteğe bağlı başlatma ve boşta durdurma; süzgeç yok.
4. Takma adlandırma ve yerel ad bulucu model.
5. Gömme adaylarının ölçümü.
6. EG2 görsel ve ses kanalı.

Xenolith'in Rust'a çevrilmesi tezden sonradır.

## Ek: gömme modeli kapısı

Aynı oturumda verildi. Yeni bir gömme modeli İbnü'n-Nedîm Gömme'nin **metin** kanalına ancak şu iki koşulu sağlarsa girer:

- ölçüm setinde bge-m3'ü MRR'de en az **+0,05** geçer;
- hiçbir sınama kitabında ilk 5'te bulma oranı düşmez.

Ölçüm seti bugün 175 Arapça sorgudan oluşuyor (`gomme-paket`, kitaplar 12320, 10614, 1455). Türkçe sorgular eklenecek.

Gerekçe:

- 2026-09-27 ince ayarı MRR'yi yalnız +0,037 artırdı (0,312 → 0,349).
- Aynı ince ayar 10614 kitabında ilk 5'te bulma oranını %60,8'den %51,0'a düşürdü.

EmbeddingGemma 2 ayrı bir görsel ve ses kanalı olarak eklenir; bge-m3'ün yerine geçmez, bu kapıya tabi değildir. Metin kanalına aday olursa o da aynı kapıdan geçer.
