# el-Fihrist (الفهرست)

> In the name of Allah, the Most Gracious, the Most Merciful. Praise be to the Lord of the worlds, and peace and blessings be upon His Messenger. This project takes its name and spirit from the great bibliographer Abu al-Faraj Muhammad b. Ishaq al-Nadim, who lived in Baghdad in the 10th century, and his immortal work *al-Fihrist*. Inheriting the legacy of our civilization's first librarian, we have built a pure Rust and Turso SQLite-based skill, code, and memory library for AI agents.

---

## 🌍 Dil Seçenekleri / Languages
🇹🇷 [Türkçe](README.md) | 🇬🇧 [English](README.en.md) | 🇸🇦 [العربية](README.ar.md) | 🇯🇵 [日本語](README.ja.md) | 🇨🇳 [中文](README.zh.md) | 🇷🇺 [Русский](README.ru.md) | 🇪🇸 [Español](README.es.md)

---

### 🇬🇧 English

#### 📌 Table of Contents
- [📖 Description and Features](#-description-and-features)
    - [🧰 Available Skills and System Modules](#-available-skills-and-system-modules)
- [⚙️ Requirements and Installation](#️-requirements-and-installation)
- [🏗️ Infrastructure and Working Principle](#️-infrastructure-and-working-principle)
- [🗺️ Roadmap](#️-roadmap)
- [📝 Release Updates](#-release-updates)
- [👥 Contributors (Co-Authors)](#-contributors-co-authors)
- [📜 License](#-license)

---

#### 📖 Description and Features
**el-Fihrist** is a portable skill, code snippet, and memory library engine designed for AI agents (Hermes Agent, Claude Desktop MCP, DeepSeek Harness, OpenCode, Codex).

* **Pure Rust BM25 Search Engine:** High-speed, in-memory relevance search engine with Turkish character folding (e.g., `İ→i`, `I→ı`, `Ş→ş`).
* **Turso / SQLite Core:** A portable SQLite database (`kutup_kutuphane.db`) containing 140+ external skills and built-in Rust tools.
* **Multi-Crate Architecture:** A modular Rust structure: the CLI (`ibnunnedim-cli`) plus nine library crates under `crates/` (listed below).

---

#### ⚙️ Requirements and Installation

##### Dependencies & Crates
* **Rust 1.63+** (Edition 2021)
* Workspace Crates: `ibnunnedim-cli`, `crates/fihrist-core`, `crates/fihrist-storage`, `crates/fihrist-gui`, `crates/fihrist-canli`, `crates/fihrist-nazar`, `crates/fihrist-kaynak`, `crates/fihrist-olcum`, `crates/fihrist-tema`, `crates/terim`
* System Database: `Turso SQLite 0.8.2` (`kutup_kutuphane.db`)

##### Build and Execution
```bash
# Workspace release build
cargo build --release --workspace

# Skill search using BM25
./target/release/ibnunnedim search "rust"

# List skills
./target/release/ibnunnedim list --kategori "software-development"

# Submit a report (Tekmil) score
./target/release/ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Passed outer gauntlet"

# Run as a stdio MCP server (agents/clients connect)
./target/release/ibnunnedim mcp
```

---

#### 🏗️ Infrastructure and Working Principle
* **BM25 Indexing:** All skills in the database are read in a single query and loaded into an in-memory BM25 index. It safely truncates text using the UTF-8 stable `kisalt` function.
* **Database Integration:** Establishes a zero-copy remote & local synced SQLite connection using the `turso::Builder::new_local` driver.
* **Layer Utilities:** `tokio` (async runtime), `clap` (CLI argument parser), `serde_json` (MCP protocol and manifest operations).

---

#### 🗺️ Roadmap
- [x] Pure Rust BM25 search engine with Turkish character folding.
- [x] Turso SQLite `kutup_kutuphane.db` integration and Tekmil scoring mechanism.
- [x] Stdio MCP (JSON-RPC 2.0) server for agents — `ibnunnedim mcp`, 4 tools.
- [ ] Transition to semantic (heuristic, hermeneutic, epistemologic, moral, etc.) dynamic database/skill classification structures for autonomous, point-blank tool calling architecture.
- [ ] Construction of local MCP (Model Context Protocol) GraphQL/gRPC bridge for agents.
- [ ] Direct modification of Turso via autonomous SkillOpt sleep engine.
- [ ] Multi-tenant agent memory indexing infrastructure.
- [ ] Live change notifications and live queries on top of Turso CDC (the equivalent of SurrealDB's `LIVE SELECT`). ([gorev-canli-sorgu-ve-vektor-dizini.md](docs/gorev-canli-sorgu-ve-vektor-dizini.md))
- [ ] Built-in vector index for dense vectors (an HNSW side index if Turso has none) and hybrid similarity search with BM25. ([gorev-canli-sorgu-ve-vektor-dizini.md](docs/gorev-canli-sorgu-ve-vektor-dizini.md))

---

#### 📝 Release Updates
* **v0.2.0 (Current):**
  - Added Maşa Döngüsü D1-D3 & R2 hygiene inspection gates.
  - Integrated 7-language hybrid README architecture (`readme-yolbulucu` standard).
  - Adopted modular structure for core modules (`codebase`, `docx`, `multilingual`, etc.).
* **v0.1.0:** First compilable Rust workspace, `ibnunnedim-cli`, and basic Turso database integration (initial structure).

---

#### 👥 Contributors (Co-Authors)
Contributors to the project's architecture and codebase:
* **Ercan ER** `<ercan.er@medeniyet.edu.tr>` *(Project Owner and Chief Architect)*
* **İbnünnedîm (hermes-agent)** `<291384122+hermes-agent@users.noreply.github.com>` *(Librarian Agent)*
* **Mihenk (Claude Opus 5)** `<noreply@anthropic.com>` *(Hygiene, Code Quality, and Audit Referee)*
* **Cezeri (Demirci)** *(Software & Autonomous System Health Monitoring - Agent Interface Architect)*

---

#### 📜 License
This project is licensed under the [MIT License](LICENSE).
