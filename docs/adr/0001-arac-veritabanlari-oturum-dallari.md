# Her aracın kendi turso veritabanı var; aynı anda yazma oturum dallarıyla, eşgüdüm el-Fihrist üzerinden yürür

Ercan'ın araçları (zopay, zopay-word, agent-reach-rs, pasli-beyin, kervan) tek başına kurulabilmeli, ama birlikte kurulduklarında verilerini paylaşabilmeli. Karar:

- Her araç kendi ürettiği veriyi kendi turso **araç veritabanı**nda tutar ve el-Fihrist'e derleme zamanında bağımlı değildir.
- Aynı anlamı taşıyan kayıtlar her araçta aynı **veri sınıfı** şemasıyla kodlanır.
- el-Fihrist bütün araç veritabanlarına erişir ve aynı sınıftaki kayıtları **eşgüdüm**le uyumlu tutar.
- Bir araç isterse kendi veritabanını kapatıp el-Fihrist'inkini kullanır (**veritabanı ödünç alma**). Böylece disk ve eşgüdüm maliyeti ortadan kalkar.

Aynı veriye aynı anda erişim, turso'nun deneysel çok süreçli WAL'ıyla (Windows'ta `experimental_win_iocp` gerekiyor) değil, **YZ oturumu başına dal**la çözülür. Yazarlar YZ oturumlarıdır; aynı araçta birden çok oturum açılabildiği için dal birimi oturumdur. Yöntem AgentGit'ten ödünç alınır:

- dal ana veriden açılır;
- değişiklikler yalnız ekleme yapılan bir günlükte tutulur;
- birleştirmeyi bir birleştirme ajanı zorunlu bir özetle önerir, çakışmada insan karar verir;
- eşgüdüm yalnız ileri sarar;
- hassas sınıflar eşgüdümden önce taranır.

Dal kimliği, varsa `AGIT_SESSION`'dan, yoksa oturumun kendi kimliğinden gelir. Böylece her veri değişikliği onu üreten konuşmaya bağlanır; AgentGit'e derleme zamanı bağımlılığı yoktur.

## Reddedilen seçenekler

- **Paylaşılan dosyaya çok süreçli yazma:** turso bunu henüz kararlı sunmuyor. Kararlı hâle geldiğinde dal modeli korunarak buna geçilebilir.
- **İş başına dal:** aynı işe açılan iki oturum yine çakışır.
- **AgentGit'i veri deposu olarak kullanmak:** git satır düzeyinde birleştirme yapmaz.

## Sonuçlar

- **Okur** (agent-reach-rs, pasli-beyin) ve **Atölye** (zopay-word → zopay) ilişkileri dal gerektirmez; çakışma yalnız iki yazar aynı sınıfa yazdığında doğar.
- `zotero.sqlite` gibi **yabancı veritabanları** bu kuralın dışındadır: turso, SQLite ile karışık çok süreçli erişimi desteklemez (COMPAT.md, 4. garanti). zopay onlar için rusqlite'ı korur.
- Veri sınıfı şemaları, değişiklik günlüğü biçimi ve dal kimliği ayrı, küçük, izin verici lisanslı (MIT/Apache-2.0) bir **sınıf sözleşmesi** crate'inde durur. Araçlar ve el-Fihrist yalnız ona bağlanır. el-Fihrist'in içinde olsaydı araçlar el-Fihrist'e bağımlı olurdu; zopay-word'ün içinde olsaydı AGPL bulaşırdı; her araçta bir kopya olsaydı şemalar sessizce ayrışırdı.
