# el-Fihrist

Ercan'ın araç ailesinde (zopay, zopay-word, agent-reach-rs, pasli-beyin, kervan) ortak yeteneklerin tek kaynaktan dağıtıldığı merkez depo.

## Dil

### Ödünç alma biçimleri

**Merkez ödünç alma**:
Bir kaynak kütüphanenin yalnız el-Fihrist'e bağlanması ve diğer araçlara el-Fihrist'in isteğe bağlı bir feature'ı olarak sunulması (pasli-beyin'in `fihrist` feature'ı gibi); kapalıyken aracın kendi yolu çalışır.
_Kaçın_: hub, köprü, "el-Fihrist'ten çekmek"

**Parça ödünç alma**:
Bir kod parçasının `kutup_kutuphane.db`'den alınıp hedef depoya kopyalanması.
_Kaçın_: vendor, yapıştırma

**Yöntem ödünç alma**:
Kod alınmadan yalnız yaklaşımın alınması; iki depo ayrı kalır.
_Kaçın_: esinlenme, port

### Veri

**Araç veritabanı**:
Her hedef aracın kendi ürettiği veriyi tuttuğu, el-Fihrist'siz de çalışan turso veritabanı.
_Kaçın_: yerel DB, önbellek

**Veri sınıfı**:
Araçlar arasında aynı anlamı taşıyan kayıt türü; aynı sınıf her araçta aynı şemayla kodlanır.
_Kaçın_: tablo, model, tip

**Ortak crate**:
Kendi deposunda duran, hiçbir hedef araca (el-Fihrist dahil) bağımlı olmayan, izin verici lisanslı küçük kütüphane; sınıf sözleşmesi bunun bir örneğidir.
_Kaçın_: merkez crate, el-Fihrist crate'i

**Sınıf sözleşmesi**:
Veri sınıflarının şemasını, değişiklik günlüğü biçimini ve dal kimliğini tanımlayan, bütün araçların ve el-Fihrist'in bağlandığı izin verici lisanslı ortak crate.
_Kaçın_: ortak model, shared types

**Eşgüdüm**:
Birlikte kurulu araçların veritabanları arasında aynı veri sınıfındaki kayıtları el-Fihrist üzerinden uyumlu tutma.
_Kaçın_: senkron, replikasyon

**Veritabanı ödünç alma**:
Bir aracın kendi araç veritabanını kapatıp el-Fihrist'in veritabanını kullanması.
_Kaçın_: bağlanma, birleştirme

**Dal**:
Bir öznenin aynı veri sınıfı üzerinde, ana veriye dokunmadan çalıştığı geçici ya da kalıcı kopya; iş bitince birleştirilir, sonra silinir ya da eşgüdüme alınır.
_Kaçın_: fork, sandbox, kopya

**Dal kimliği**:
Bir dalın adı; YZ oturumu AgentGit'le yönetiliyorsa `AGIT_SESSION`'dır, değilse oturumun kendi kimliğidir.
_Kaçın_: dal no, oturum adı

**Değişiklik günlüğü**:
Bir Dal'daki her değişikliği, yalnız eklenerek ve durumdan önce yazılan kayıtlar olarak tutan, Dal başına tek dosya.
_Kaçın_: log, geçmiş, olay tablosu

**Birleştirme**:
Bir dalda yapılan değişikliklerin el-Fihrist'teki ana veriye geri konması.
_Kaçın_: merge, senkron

**Yabancı veritabanı**:
Başka bir uygulamanın yazdığı, bizim yalnız okuduğumuz veritabanı (ör. `zotero.sqlite`); araç veritabanı değildir.

### Canlı bildirim

**Bildirim günlüğü**:
Bir veritabanı dosyasında izlenen tabloların her ekleme, güncelleme ve silmesinin, tetikleyicilerle aynı işlem içinde yazıldığı tablo (`fihrist_degisiklik`, ADR 0003). Dosya başına birdir ve budanır. Değişikliğin içeriğini değil, hangi satırın değiştiğini (tablo, işlem, anahtar) söyler.
_Kaçın_: değişiklik günlüğü (Dal'ın günlüğüdür), CDC

**Tüketici**:
Bildirim günlüğünü kalıcı bir imleçle (`<db>.imlec/<ad>`) okuyan süreç. Teslimi en az bir kez alır; budama onun imlecini geçemez.
_Kaçın_: abone

**Canlı sorgu**:
İzlenen tablolarından biri değişince yeniden koşulan ve sonucu bir öncekiyle karşılaştırılan sorgu; tablolar sorgudan çıkarılmaz, açıkça verilir.

**Abone**:
Bir canlı sorguyu bellekte tutan geçici dinleyici. İmleç yazmaz, budamayı etkilemez; başlarken sorgunun ilk görüntüsünü alır, düşünce iz bırakmaz.
_Kaçın_: tüketici (kalıcı imleç tutar)

**Fark**:
Bir canlı sorgunun iki sonucu arasındaki eklenen, değişen ve silinen satırlar. Satırın kimliği abonenin verdiği anahtar sütunlarıdır; verilmezse tek tablonun birincil anahtarı. Kimlik kurulamıyorsa abonelik reddedilir. Boş fark bildirilmez.

### Tema

**Tema paketi**:
Paleti, vurguyu ve kenarlık dokusu seçimini taşıyan küçük metin dosyası (`.tema`, şema `kilim-tema` ortak crate'inde); araç yalnız etkin paketi tutar.
_Kaçın_: skin, stil dosyası

**Tema kataloğu**:
Bütün tema paketlerinin el-Fihrist'te durduğu yer (`temalar/` tohumu + kullanıcı dizini, `fihrist-tema`); araçlar ona yalnız kullanıcı tema değiştirirken uzanır, açılışta ve çizim döngüsünde asla; kapalıyken aracın yerleşik teması çalışır.
_Kaçın_: tema deposu, tema sunucusu

### Hafıza

**Hafıza grafı**:
Kod tabanı yönetimi için agit (oturum geçmişi), graft (kod grafı), graphify (kavram grafı) ve archify'ın (görünüm) çıktılarını birbirine bağlayan, el-Fihrist'in bir modülü olarak yaşayan graf; görünümü şimdilik Logseq'tir.
_Kaçın_: ikinci beyin, bilgi tabanı, wiki

### Aktörler

**Kaynak kütüphane**:
Hedef araçlara yetenek sağlayan dış ya da iç depo (turso, agentfs, graphify-rs, archify-graft-rs, laya, regex, rank_bm25, bge-embed-rs).

**Hedef araç**:
Ercan'a ait, kaynak kütüphaneleri tüketen araç (zopay, zopay-word, agent-reach-rs, el-Fihrist, pasli-beyin, kervan).

**YZ arayüzü**:
Hedef araçları MCP ya da CLI üzerinden çağıran çıkarım ortamı (Claude Code, Claude Desktop, Antigravity).

**Özne**:
Araç verisine yazan taraf; özneler YZ oturumlarıdır, araçlar değil.
_Kaçın_: yazar (bir eserin yazarıyla karışır), fail (test çıktısındaki FAILED ile karışır)

**Okur**:
Veriyi yalnız okuyan araç (agent-reach-rs araştırmada, pasli-beyin depo izlemede).

**Atölye**:
Başka bir aracın verisini onunla birlikte işleyen araç; zopay-word, zopay'ın atölyesidir.

**Çıkarım ayarı**:
Hangi YZ'nin hangi uç ve yolla çalışacağının belirlenmesi; Kervan'ın işidir.

### Kişi ve roller

**Kişi**:
Araçlar arasında ortak tutulan gerçek kişi kimliği; bir eserdeki ya da davadaki konumu ona bağlı roller belirtir. Aynı Kişi aynı anda birden çok rol taşıyabilir.
_Kaçın_: yazar, taraf (bunlar Kişi'nin rolleridir, Kişi'nin kendisi değil)

**Kişi eşlemesi**:
İki aracın kendi Kişi kayıtlarının aynı gerçek kişiyi gösterdiğinin el-Fihrist'te açıkça yazılması. İsteğe bağlıdır; aynı ad tek başına eşleme sayılmaz.
_Kaçın_: birleştirme (Dal'ın Birleştirme'siyle karışır), kimlik çözümleme

**Yazar**:
Bir eserin (kitap, makale, bölüm) yazarı ya da eşdeğer katkı sahibi (editör, çevirmen) olan Kişi rolü. **Özne ile karıştırılmaz**: Yazar bir eserle ilgilidir, Özne veriye yazan YZ oturumudur.

**Taraf**:
Bir uyuşmazlık dosyasının iki yanından biri olan Kişi rolü: Başvurucu ya da Diğer Taraf.
_Kaçın_: vekil, arabulucu (bunlar Taraf değil, ayrı rollerdir)

**Vekil**:
Bir Taraf'ı temsil eden Kişi rolü; hangi Taraf'ı temsil ettiğini bilir.

**Arabulucu**:
Uyuşmazlık dosyasını yürüten, iki yandan da bağımsız Kişi rolü.

### Kürasyon

**Çakışma**:
İki yeteneğin (ya da MCP aracının) aynı işi görmesi, öyle ki biri ötekinin yerine geçebilir. Konu yakınlığı çakışma değildir: aynı alanda duran iki yetenek farklı iş görebilir.
_Kaçın_: benzerlik, örtüşme (ikisi de ölçülen yakınlığı anlatır, hükmü değil)

**Hüküm**:
Bir yetenek çiftinin ilişkisi üzerine, bir ajanın gerekçeli taslağı yazıp Ercan'ın onayladığı karar: **kopya**, **kardeş**, **çakışma** ya da **bağımsız**. Hükmü yalnız Ercan verir; el-Fihrist öneri yapar, hiçbir yeteneği kendisi kaldırmaz.
_Kaçın_: karar, skor (skor yalnız adayı çıkarır)

**Kopya**:
Aynı yeteneğin birden çok YZ arayüzünde ya da yolda durması; kataloğa ayrı kayıt olarak girer ama tek yetenektir.
_Kaçın_: çakışma (kopyalar çakışmadan önce ayıklanır)

**Kardeş**:
Aynı ailenin (ör. aynı MCP sunucusunun araçları) birbirini tamamlayan parçaları; birbiriyle çakışmaz.
_Kaçın_: çakışma

## İlişkiler

- Bir **Hüküm** bir çifte verilir; **Kopya** ve **Kardeş** çakışma sayımına girmez.
- Bir **Merkez ödünç alma** tam olarak bir **Kaynak kütüphane**yi el-Fihrist'in bir crate'i ardından bir **Hedef araç**a taşır.
- Bir **Hedef araç**, **YZ arayüzü**ne kendi MCP sunucusuyla görünür; ödünç aldığı kütüphaneyi ayrıca göstermez.

- Her **Hedef araç**ın en fazla bir **Araç veritabanı** vardır; **Veritabanı ödünç alma** yapan aracın sıfırdır.
- Hiçbir **Hedef araç** el-Fihrist'e bağımlı değildir: ne kurulum, ne çalışma zamanı, ne kaynak kodu düzeyinde. Arama yetenekleri (BM25, gömme istemcisi) **Merkez ödünç alma** ile dağıtılmaz; her araç kendi yolunu taşır ya da el-Fihrist'ten bağımsız bir **Ortak crate**'e bağlanır. el-Fihrist bütün **Araç veritabanı**larına erişir; tersi gerekmez.
- **Eşgüdüm** yalnız aynı **Veri sınıfı**ndaki kayıtlar arasında olur.
- Bir **Kişi** aynı anda birden çok rol taşıyabilir (**Yazar**, **Taraf**, **Vekil**, **Arabulucu**); rol, Kişi'nin kimliğini değiştirmez.
- Bir **Vekil** tam bir **Taraf**'ı temsil eder; Vekil'in kendisi Taraf sayılmaz.
- Bir **Abone** bildirim günlüğünü yalnız "değişti" işareti olarak okur; doğruluğu yeniden koşulan sorgudan gelir, günlüğün eksiksizliğinden değil. Günlükte budanmış bir aralık görürse sorguyu koşulsuz yeniden koşar.
- fts5 sanal tablosu olan dosyanın **Bildirim günlüğü** olmaz (ADR 0003); ana katalog bu yüzden izlenmez.
- **Yabancı veritabanı** turso kuralının dışındadır (turso, aynı dosyada SQLite ile karışık çok süreçli erişimi desteklemez).

## Örnek konuşma

> **Geliştirici:** "Pasli-beyin bge-m3 gömmesini doğrudan bge-embed-rs'den mi alıyor?"
> **Ercan:** "Hayır — **Merkez ödünç alma**: pasli-beyin `ibnunnedim-cli`'ye bağlı, bge-embed-rs'yi yalnız el-Fihrist tanır."

## İşaretlenen belirsizlikler

- "el-Fihrist üzerinden ödünç alma" üç anlamda kullanılıyordu — çözüldü: varsayılan anlam **Merkez ödünç alma**; öteki ikisi ayrı adlarla anılır.
- Merkez bugün `ibnunnedim-cli` (lib+bin) crate'idir, `fihrist-core` değil — ince bir kütüphaneye taşınıp taşınmayacağı açık.
- Merkez ödünç alma ile **Veritabanı ödünç alma** yön olarak zıttır: ilkinde araç el-Fihrist'in crate'ine derleme zamanında bağlanır, ikincisinde araç çalışma zamanında el-Fihrist'in verisini kullanır ve derleme zamanı bağımlılığı yoktur.
- **Eşgüdüm** mekanizması: aynı veriye aynı anda erişim, **Dal** + **Birleştirme** ile çözülür (turso'nun çok süreçli WAL'ı deneysel; Windows'ta `experimental_win_iocp` gerekiyor). **Okur** ve **Atölye** ilişkileri dal gerektirmez.
- "senkron" iki anlamda kullanılıyordu — graft'ın grafı kaynakla güncel tutması ve araç veritabanları arası uyum. Çözüm: ilki **tazelik**, ikincisi **Eşgüdüm**; archify-graft'ın el-Fihrist'e gömülmesi tazelik içindir.
- pasli-beyin'deki "Ollama bge-m3" ifadeleri eskimiş: gömme arka ucu bge-embed-rs'dir (127.0.0.1:11434, Open Notebook'la paylaşılır; `FIHRIST_BGE_URL` ile değişir).
- "Değişiklik günlüğü" iki anlamda kullanılıyordu: Dal'ın JSONL günlüğü (ADR 0002) ve tetikleyicilerin yazdığı `fihrist_degisiklik` tablosu (ADR 0003). Çözüm: ilki **Değişiklik günlüğü**, ikincisi **Bildirim günlüğü**. Tablo adı değişmedi.
- "Yazar" iki anlamda kullanılıyordu — veriye yazan YZ oturumu ve bir eserin yazarı. Çözüm: ilki **Özne**, ikincisi **Yazar** (Kişi'nin rolü). "Kişi = yazar mı, taraf mı?" sorusu da böylece kapandı: ikisi de Kişi'nin rolüdür, biri ötekinin yerini almaz.
