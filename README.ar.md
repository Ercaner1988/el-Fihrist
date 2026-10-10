# el-Fihrist (الفهرست)

> بسم الله الرحمن الرحيم. الحمد لله رب العالمين، والصلاة والسلام على رسوله. يستمد هذا المشروع اسمه وروحه من الببليوغرافي العظيم أبو الفرج محمد بن إسحاق النديم الذي عاش في بغداد في القرن العاشر، ومن عمله الخالد *الفهرست*. استلهاماً من تراث أول أمين مكتبة في حضارتنا؛ قمنا ببناء مكتبة للمهارات، والرموز البرمجية، والذاكرة مخصصة للوكلاء الأذكياء (AI agents)، تعتمد على لغة Rust الخالصة وقاعدة بيانات Turso SQLite.

---

## 🌍 Dil Seçenekleri / Languages
🇹🇷 [Türkçe](README.md) | 🇬🇧 [English](README.en.md) | 🇸🇦 [العربية](README.ar.md) | 🇯🇵 [日本語](README.ja.md) | 🇨🇳 [中文](README.zh.md) | 🇷🇺 [Русский](README.ru.md) | 🇪🇸 [Español](README.es.md)

---

### 🇸🇦 العربية

#### 📌 المحتويات
- [📖 الوصف والميزات](#-الوصف-والميزات)
    - [🧰 المهارات المتاحة ووحدات النظام](#-المهارات-المتاحة-ووحدات-النظام)
- [⚙️ المتطلبات والتثبيت](#️-المتطلبات-والتثبيت)
- [🏗️ البنية التحتية وآلية العمل](#️-البنية-التحتية-وآلية-العمل)
- [🗺️ خارطة الطريق](#️-خارطة-الطريق)
- [📝 تحديثات الإصدار](#-تحديثات-الإصدار)
- [👥 المساهمون (Co-Authors)](#-المساهمون-co-authors)
- [📜 الترخيص](#-الترخيص)

---

#### 📖 الوصف والميزات
**الفهرست (el-Fihrist)** هو محرك محمول للمهارات، ومقاطع الأكواد، ومكتبة الذاكرة مخصص للوكلاء الأذكياء (مثل Hermes Agent، Claude Desktop MCP، DeepSeek Harness، OpenCode، Codex).

* **محرك بحث BM25 بلغة Rust الخالصة:** محرك بحث في الذاكرة عالي السرعة للملاءمة، مع دعم طي الأحرف التركية (مثل `İ→i`, `I→ı`, `Ş→ş`).
* **نواة Turso / SQLite:** قاعدة بيانات محمولة (`kutup_kutuphane.db`) تحتوي على أكثر من 140 مهارة خارجية وأدوات Rust مدمجة.
* **بنية متعددة الوحدات (Multi-Crate):** هيكل Rust معياري: واجهة الأوامر (`ibnunnedim-cli`) وتسع وحدات مكتبية ضمن `crates/` (مذكورة أدناه).

---

#### ⚙️ المتطلبات والتثبيت

##### التبعيات والوحدات (Crates)
* **Rust 1.63+** (الإصدار 2021)
* وحدات مساحة العمل: `ibnunnedim-cli`, `crates/fihrist-core`, `crates/fihrist-storage`, `crates/fihrist-gui`, `crates/fihrist-canli`, `crates/fihrist-nazar`, `crates/fihrist-kaynak`, `crates/fihrist-olcum`, `crates/fihrist-tema`, `crates/terim`
* قاعدة بيانات النظام: `Turso SQLite 0.8.2` (`kutup_kutuphane.db`)

##### البناء والتشغيل
```bash
# بناء نسخة الإصدار لمساحة العمل
cargo build --release --workspace

# البحث عن المهارات باستخدام BM25
./target/release/ibnunnedim search "rust"

# سرد المهارات
./target/release/ibnunnedim list --kategori "software-development"

# إرسال درجة التقرير (Tekmil)
./target/release/ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Passed outer gauntlet"

# التشغيل كخادم MCP عبر stdio (يتصل به الوكلاء/العملاء)
./target/release/ibnunnedim mcp
```

---

#### 🏗️ البنية التحتية وآلية العمل
* **فهرسة BM25:** يتم قراءة جميع المهارات في قاعدة البيانات في استعلام واحد وتحميلها إلى فهرس BM25 في الذاكرة. يتم اقتصاص النصوص بأمان باستخدام دالة `kisalt` المتوافقة مع UTF-8.
* **تكامل قاعدة البيانات:** يتم إنشاء اتصال SQLite متزامن محلياً عن بُعد وبدون نسخ (zero-copy) باستخدام محرك `turso::Builder::new_local`.
* **أدوات الطبقات:** `tokio` (بيئة تشغيل غير متزامنة)، `clap` (محلل وسيط CLI)، `serde_json` (بروتوكول MCP وعمليات البيان).

---

#### 🗺️ خارطة الطريق
- [x] محرك بحث BM25 بلغة Rust الخالصة مع طي الأحرف التركية.
- [x] تكامل Turso SQLite `kutup_kutuphane.db` وآلية تحديد نقاط Tekmil.
- [x] خادم MCP عبر stdio (JSON-RPC 2.0) للوكلاء — `ibnunnedim mcp`، 4 أدوات.
- [ ] الانتقال إلى هياكل تصنيف قواعد بيانات/مهارات دلالية وتفاعلية (استدلالية، هيرمينوطيقية، إبستمولوجية، أخلاقية، إلخ) لبنية استدعاء أدوات مستقلة وسريعة ودقيقة.
- [ ] بناء جسر MCP (بروتوكول سياق النموذج) GraphQL/gRPC محلي للوكلاء.
- [ ] التعديل المباشر لـ Turso عبر محرك النوم المستقل SkillOpt.
- [ ] بنية تحتية لفهرسة ذاكرة الوكلاء متعددة المستأجرين (Multi-tenant).
- [ ] إشعارات التغييرات الحية والاستعلامات الحية عبر Turso CDC (ما يقابل `LIVE SELECT` في SurrealDB). ([gorev-canli-sorgu-ve-vektor-dizini.md](docs/gorev-canli-sorgu-ve-vektor-dizini.md))
- [ ] فهرس متجهات مدمج للمتجهات الكثيفة (فهرس HNSW جانبي إن لم يتوفر في Turso) وبحث تشابه هجين مع BM25. ([gorev-canli-sorgu-ve-vektor-dizini.md](docs/gorev-canli-sorgu-ve-vektor-dizini.md))

---

#### 📝 تحديثات الإصدار
* **v0.2.0 (الحالي):**
  - تمت إضافة بوابات فحص النظافة Maşa Döngüsü D1-D3 & R2.
  - تم دمج بنية README الهجينة بـ 7 لغات (معيار `readme-yolbulucu`).
  - تم اعتماد الهيكل المعياري للوحدات الأساسية (`codebase`، `docx`، `multilingual`، إلخ).
* **v0.1.0:** أول مساحة عمل Rust قابلة للتجميع، و`ibnunnedim-cli`، والتكامل الأساسي لقاعدة بيانات Turso (البنية الأولية).

---

#### 👥 المساهمون (Co-Authors)
المساهمون في بنية المشروع وقاعدة التعليمات البرمجية:
* **Ercan ER** `<ercan.er@medeniyet.edu.tr>` *(مالك المشروع والمهندس المعماري الرئيسي)*
* **İbnünnedîm (hermes-agent)** `<291384122+hermes-agent@users.noreply.github.com>` *(الوكيل أمين المكتبة)*
* **Mihenk (Claude Opus 5)** `<noreply@anthropic.com>` *(حكم النظافة، جودة الكود، والتدقيق)*
* **Cezeri (Demirci)** *(مراقبة صحة البرمجيات والنظام المستقل - مهندس واجهة الوكيل)*

---

#### 📜 الترخيص
هذا المشروع مرخص بموجب [ترخيص MIT](LICENSE).