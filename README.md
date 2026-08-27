# el-Fihrist (الفهرست)

> **"Ey iman edenler! Kendinizin veya anne babanızın ve akrabanızın aleyhine bile olsa adaleti ayakta tutun..."** *(Nisâ 135)*

## 📚 Giriş ve Tarihsel Atıf

Bu proje adını ve ruhunu, 10. yüzyılda Bağdat'ta yaşamış büyük Arap bibliyografya âlimi **Ebü’l-Ferec Muhammed b. Ebî Ya‘kūb İshâk b. Muhammed b. İshâk en-Nedîm** (ö. 385/995 [?]) ve onun ölümsüz eseri ***el-Fihrist***'ten (s. 330) almaktadır [TDV İslâm Ansiklopedisi](https://islamansiklopedisi.org.tr/ibnun-nedim).

İbnü’n-Nedîm, medeniyetimizin ilk büyük kütüphanecisi ve ilim tahlilcisidir. *el-Fihrist*, İslâm’ın altın çağında kaleme alınan, tüm ilimleri, mezhepleri, telif ve tercüme eserleri tek bir çatı altında toplayan ilk bibliyografik başyapıttır. 

Biz de bu mirastan ilhamla; yapay zekâ ajan ekosistemi (Hermes, Claude Desktop, DeepSeek Harness, OpenCode, Codex) için yetenekleri (skills), kod parçalarını, prompt şablonlarını ve hafıza kısayollarını birleştiren **saf Rust ve Turso SQLite tabanlı bilgi deposunu** inşa ettik ve adını **el-Fihrist** koyduk.

---

## 🌍 README / Dil Seçenekleri

[Türkçe](#-türkçe) | [English](#-english) | [العربية](#-العربية) | [日本語](#-日本語)

---

### 🇹🇷 Türkçe

**el-Fihrist**, yapay zekâ ajanları için taşınabilir yetenek, kod ve hafıza kütüphanesidir.

#### Özellikler
- **Saf Rust BM25 Arama Motoru:** Bellek içi, Türkçe katlamalı (`İ→i`, `I→ı`) yüksek hızlı arama.
- **Turso / SQLite Çekirdeği:** 140+ yetenek ve 9+ Rust kod parçasıyla taşınabilir SQLite veritabanı (`kutup_kutuphane.db`).
- **Eklenti / Plugin Uyumlu:** Hermes Agent, Claude Desktop MCP, DeepSeek Harness ve OpenCode için hazır universal plugin manifesti (`plugin.json`).
- **Katı Doğrulama (Maşa Döngüsü):** D1-D3 ve R2 Altın Hygiene denetim kapılarıyla güvenli kod çalıştırma.

#### Kurulum & Çalıştırma
```bash
cargo build --release --workspace
./target/release/ibnunnedim search "rust"
```

---

### 🇬🇧 English

**el-Fihrist** is a portable skill, code snippet, and memory library engine for AI agents.

#### Key Features
- **Pure Rust BM25 Search Engine:** In-memory, Turkish-folding fast relevance search engine.
- **Turso / SQLite Core:** Portable database housing 140+ skills and 9+ Rust core modules (`kutup_kutuphane.db`).
- **Universal Plugin & MCP Ready:** Universal manifest (`plugin.json`) supporting Hermes Agent, Claude Desktop, DeepSeek Harness, OpenCode, and Codex.
- **Strict Verification (Maşa Döngüsü):** Built-in D1-D3 & R2 hygiene gate verifiers.

#### Build & Run
```bash
cargo build --release --workspace
./target/release/ibnunnedim search "rust"
```

---

### 🇸🇦 العربية

**الفهرست (el-Fihrist)** هو محرك مكتبة محمول للمهارات وأكواد البرمجة والذاكرة للوكلاء الأذكياء (AI Agents).

#### الميزات الرئيسية
- **محرك بحث BM25 بلغة Rust الخالصة:** بحث سريع في الذاكرة مع دعم طي الحروف.
- **نواة Turso / SQLite:** قاعدة بيانات محمولة تضم أكثر من 140 مهارة و9 وحدات برمجية بلغة Rust.
- **توافق شامل مع الإضافات (Plugins & MCP):** يدعم Hermes Agent وClaude Desktop وDeepSeek Harness وOpenCode.
- **تحقق صارم (Maşa Döngüsü):** بوابات فحص الأمان لضمان سلامة الكود.

#### التثبيت والتشغيل
```bash
cargo build --release --workspace
./target/release/ibnunnedim search "rust"
```

---

### 🇯🇵 日本語

**el-Fihrist** は、AIエージェントのためのポータブルなスキル、コードスニペット、メモリーライブラリエンジです。

#### 主な機能
- **純粋なRust BM25検索エンジン:** メモリ内高速関連性検索エンジン。
- **Turso / SQLite コア:** 140以上のスキルと9つのRustモジュールを収容するポータブルデータベース (`kutup_kutuphane.db`)。
- **マルチプラットフォーム対応 (Plugin / MCP):** Hermes Agent、Claude Desktop、DeepSeek Harness、OpenCodeに対応したマニフェスト (`plugin.json`)。
- **厳格な検証機能 (Maşa Döngüsü):** コードの安全性と衛生基準を保証するゲート検証機能。

#### ビルドと実行
```bash
cargo build --release --workspace
./target/release/ibnunnedim search "rust"
```

---

## 👥 Katkıda Bułunanlar & Emeği Geçenler

Bu sürümün ortaya çıkmasında emeği geçen tüm mimarlara teşekkür ederiz:
- **Ercan ER** *(Proje Sahibi ve Baş Mimarı)*
- **İbnünnedîm (el-Kütübî)** *(Kütüphaneci Ajan / Librarian Agent)*
- **Mihenk (Claude Opus 5)** *(Hijyen, Mimari ve Denetim Hakemi)*

---

## 👏 Teşekkür & Atıflar

Projemizin geliştirilmesinde aşağıdaki açık kaynak projelerden ve topluluk çalışmalarından ilham alınmış ve yararlanılmıştır:

1. **[Nous Research - Hermes Agent](https://github.com/NousResearch/hermes-agent):** Harika ajan altyapısı ve yetenek ekosistemi için.
2. **[Turso / libSQL](https://github.com/tursodatabase/libsql):** Yüksek hızlı SQLite ve Edge veritabanı sürücüleri için.
3. **[rank_bm25](https://github.com/dorianbrown/rank_bm25):** Saf Rust BM25 arama motorumuzun algoritmik referans modeli için (Dorian Brown ve katkıda bulunanlar).
4. **[TDV İslâm Ansiklopedisi](https://islamansiklopedisi.org.tr/ibnun-nedim):** İbnü'n-Nedîm ve el-Fihrist hakkındaki eşsiz tarihsel kaynak için.

---

## 📜 Lisans

[MIT License](LICENSE)
