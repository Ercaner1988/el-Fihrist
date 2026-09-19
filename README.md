# el-Fihrist (الفهرست)

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/Ercaner1988/el-Fihrist?utm_source=badge)

> Bismillahirrahmanirrahim. Rahmân ve Rahîm olan Allah'ın adıyla. Hamd âlemlerin Rabbine, salât ve selâm O'nun Resûlü'ne olsun. Bu proje adını ve ruhunu, 10. yüzyılda Bağdat'ta yaşamış büyük bibliyograf Ebü’l-Ferec Muhammed b. İshâk en-Nedîm ve onun ölümsüz eseri *el-Fihrist*'ten almaktadır. Medeniyetimizin ilk kütüphanecisinin mirasıyla; yapay zekâ ajanları için saf Rust ve Turso SQLite tabanlı yetenek, kod ve hafıza kütüphanesini inşa ettik.

---

## 🌍 Dil Seçenekleri / Languages
🇹🇷 [Türkçe](README.md) | 🇬🇧 [English](README.en.md) | 🇸🇦 [العربية](README.ar.md) | 🇯🇵 [日本語](README.ja.md) | 🇨🇳 [中文](README.zh.md) | 🇷🇺 [Русский](README.ru.md) | 🇪🇸 [Español](README.es.md)

---

### 🇹🇷 Türkçe

#### 📌 İçindekiler
- [📖 Ne Nedir ve Özellikler](#-ne-nedir-ve-özellikler)
    - [🧰 Mevcut Yetenekler ve Sistem Modülleri](#-mevcut-yetenekler-ve-sistem-modülleri)
- [⚙️ Gereksinimler ve Kurulum](#️-gereksinimler-ve-kurulum)
- [🏗️ Altyapı ve Çalışma Mantığı](#️-altyapı-ve-çalışma-mantığı)
- [🗺️ Yol Haritası](#️-yol-haritası)
- [📝 Sürüm Güncellemeleri](#-sürüm-güncellemeleri)
- [👥 Katkıda Bulunanlar (Co-Authors)](#-katkıda-bulunanlar-co-authors)
- [📜 Lisans](#-lisans)

---

#### 📖 Ne Nedir ve Özellikler
**el-Fihrist**, yapay zekâ ajanları (Hermes Agent, Claude Desktop MCP, DeepSeek Harness, OpenCode, Codex) için taşınabilir yetenek, kod parçası ve hafıza kütüphanesi motorudur.

* **Saf Rust BM25 Arama Motoru:** Bellek içi, Türkçe karakter katlamalı (`İ→i`, `I→ı`, `Ş→ş`) yüksek hızlı alaka arama motoru.
* **Turso / SQLite Çekirdeği:** 140+ dış yetenek verisi, yerleşik Rust araçları ve taşınabilir SQLite veritabanı (`kutup_kutuphane.db`).
* **Çoklu Crate Mimarisi:** CLI (`ibnunnedim-cli`) ve Core (`hermes-tools-core`) olmak üzere iki crate'lik modüler Rust yapısı.
* **Katı Doğrulama (Maşa Döngüsü):** D1-D3 ve R2 Altın hygiene denetim kapılarıyla güvenli kod ve tekmil denetimi.

##### 🧰 Mevcut Yetenekler ve Sistem Modülleri
`hermes-tools-core` KÜTÜPHANESİNDE 10 modül bulunur. Bunlar Rust API'sidir; `ibnunnedim` ikilisi bu crate'e bağlı değildir — ikilinin komutları için aşağıdaki Kullanım bölümüne bakın.
1. **citation:** Tez ve akademik metinler için Zopay altyapısıyla atıf doğrulama ve raporlama (`verify_citations`).
2. **codebase:** Kaynak kod özetleme, dosya içerik analizi ve kalite tespiti (`analyze_file_content`).
3. **docx:** MS Word XML katmanı ayrıştırma ve düşük form faktörlü okuma (`extract_paragraphs_from_xml`).
4. **extract:** Akıllı, gürültüden arındırılmış temiz HTML ve içerik ayıklama motoru (`html_ayikla`).
5. **masa_dongusu:** 4 kapılı doğrulama (D1, D2, D3, R2) ve kapı geçiş raporu (`validate_masa_dongusu`).
6. **multilingual:** Standartlara oturtulmuş, paralel çok dilli README motoru ve kalite denetleyicisi.
7. **router:** Ajan alt ağları yönlendirmesi, çizge üzerinde Dijkstra ile en kısa yol ve Edge/Route yönetimi (`RouteResult`).
8. **rules_checker:** Gelişmiş Rust kaynak kod kuralları, kalite standartları denetçisi (`check_rust_code_rules`).
9. **session:** İletişim oturumları ve hafıza verileri için alt dize (substring) eşleşmeli arama modülü (`search_session`).
10. **skillopt:** Ajan yeteneklerinin otonom evrimi, puan matrisli doğrulama (Soft/Hard gates) komut altyapısı.

---

#### ⚙️ Gereksinimler ve Kurulum

##### Bağımlılıklar & Crate'ler
* **Rust 1.63+** (Edition 2021)
* Workspace Crate'leri: `ibnunnedim-cli`, `crates/hermes-tools-core`
* Sistem Veritabanı: `Turso SQLite 0.7.2` (`kutup_kutuphane.db`)

##### Derleme ve Çalıştırma
```bash
# Workspace release derlemesi
cargo build --release --workspace

# BM25 ile yetenek araması
./target/release/ibnunnedim search "rust"

# Yetenek listeleme
./target/release/ibnunnedim list --kategori "software-development"

# Tekmil puanı verme
./target/release/ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Dış koşu geçti"

# Stdio MCP sunucusu olarak koş (ajan/istemci bağlanır)
./target/release/ibnunnedim mcp
```

---

#### 🏗️ Altyapı ve Çalışma Mantığı
* **BM25 İndeksleme:** Veritabanındaki tüm yetenekler tek sorguda okunup bellek içinde BM25 indeksine alınır. UTF-8 kararlı `kisalt` fonksiyonu ile güvenle sınırlandırılır.
* **Veritabanı Entegrasyonu:** `turso::Builder::new_local` sürücüsüyle zero-copy uzak & yerel eşitlemeli SQLite bağlantısı kurulur.
* **Katman Araçları:** `tokio` (asenkron çalışma zamanı), `clap` (CLI argüman ayrıştırıcı), `serde_json` (MCP protokolü ve manifest işlemleri).

---

#### 🗺️ Yol Haritası
- [x] Saf Rust BM25 arama motoru ve Türkçe karakter katlaması.
- [x] Turso SQLite `kutup_kutuphane.db` entegrasyonu ve tekmil scoring mekanizması.
- [x] Ajanlar için stdio MCP (JSON-RPC 2.0) sunucusu — `ibnunnedim mcp`, 4 araç.
- [ ] Anlambilimsel (heuristic, hermeneutic, epistemologic, moral vb.) canlı ve değişken veritabanı/yetenek sınıflandırma yapısının kurularak, otonom ve doğrudan nokta atışı hedef bulan, çok daha hızlı araç çağırma (tool calling) mimarisine geçilmesi.
- [ ] Ajanlar için yerel MCP (Model Context Protocol) GraphQL/gRPC köprüsü inşası.
- [ ] Otonom SkillOpt gece evrim döngüsü (sleep engine) ile Turso'nun doğrudan modifikasyonu.
- [ ] Multi-tenant ajan hafıza indeksleme altyapısı.

---

#### 📝 Sürüm Güncellemeleri
* **v0.2.0 (Mevcut):**
 - Maşa Döngüsü D1-D3 & R2 hijyen denetim kapıları eklendi.
 - 7 dilli melez README mimarisi (`readme-yolbulucu` standardı) entegre edildi.
 - Core modüllere modüler yapı (`codebase`, `docx`, `multilingual`, vb.) kazandırıldı.
* **v0.1.0:** İlk derlenebilir Rust workspace, `ibnunnedim-cli` ve temel Turso veritabanı entegrasyonu (başlangıç yapısı).

---

#### 👥 Katkıda Bulunanlar (Co-Authors)
Projenin mimarisine ve kod tabanına katkı sağlayanlar:
* **Ercan ER** `<ercan.er@medeniyet.edu.tr>` *(Proje Sahibi ve Baş Mimarı)*
* **İbnünnedîm (hermes-agent)** `<291384122+hermes-agent@users.noreply.github.com>` *(Kütüphaneci Ajan)*
* **Mihenk (Claude Opus 5)** `<noreply@anthropic.com>` *(Hijyen, Kod Kalitesi ve Denetim Hakemi)*
* **Cezeri (Demirci)** *(Yazılım & Otonom Sistem Sağlık Takibi - Ajan Arayüzü Mimarisi)*

---

#### 📜 Lisans
Bu proje [MIT Lisansı](LICENSE) ile lisanslanmıştır.