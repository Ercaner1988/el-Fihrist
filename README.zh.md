# el-Fihrist (الفهرست)

> 奉至仁至慈的安拉之名。赞美归于全世界的主，愿和平与祝福降临于祂的使者。本项目的名称和精神源自10世纪生活在巴格达的伟大书目学家阿布·法拉吉·穆罕默德·本·伊沙克·纳迪姆（Abu al-Faraj Muhammad b. Ishaq al-Nadim）及其不朽著作《书目》（*al-Fihrist*）。传承我们文明中第一位图书馆员的遗产，我们为AI智能体构建了一个纯Rust和基于Turso SQLite的技能、代码与内存库。

---

## 🌍 Dil Seçenekleri / Languages
🇹🇷 [Türkçe](README.md) | 🇬🇧 [English](README.en.md) | 🇸🇦 [العربية](README.ar.md) | 🇯🇵 [日本語](README.ja.md) | 🇨🇳 [中文](README.zh.md) | 🇷🇺 [Русский](README.ru.md) | 🇪🇸 [Español](README.es.md)

---

### 🇨🇳 中文

#### 📌 目录
- [📖 项目简介与特性](#-项目简介与特性)
    - [🧰 可用技能与系统模块](#-可用技能与系统模块)
- [⚙️ 环境要求与安装](#️-环境要求与安装)
- [🏗️ 架构与工作原理](#️-架构与工作原理)
- [🗺️ 路线图](#️-路线图)
- [📝 版本更新](#-版本更新)
- [👥 贡献者 (Co-Authors)](#-贡献者-co-authors)
- [📜 许可证](#-许可证)

---

#### 📖 项目简介与特性
**el-Fihrist** 是一个便携式技能、代码片段及内存库引擎，专为AI智能体（如 Hermes Agent, Claude Desktop MCP, DeepSeek Harness, OpenCode, Codex）设计。

* **纯 Rust BM25 搜索引擎:** 高速的内存相关性搜索引擎，支持土耳其语字符折叠（例如 `İ→i`, `I→ı`, `Ş→ş`）。
* **Turso / SQLite 核心:** 便携式 SQLite 数据库（`kutup_kutuphane.db`），包含 140 多个外部技能及内置 Rust 工具。
* **多 Crate 架构:** 模块化的 Rust 结构，包括 CLI、Core、Turso-DSL、React 和 TLS Server 模块。
* **严格验证机制 (Maşa Döngüsü):** 通过 D1-D3 门和 R2 黄金卫生检查门实现安全的代码和报告审计。

##### 🧰 可用技能与系统模块
核心库（`hermes-tools-core` 及工作区）当前提供 12 个可直接使用的核心技能和模块：
1. **citation:** 基于 Zopay 基础设施的论文和学术文本引用验证与报告（`verify_citations`）。
2. **codebase:** 源代码摘要、文件内容分析和质量检测（`analyze_file_content`）。
3. **docx:** MS Word XML 层解析及轻量级读取（`extract_paragraphs_from_xml`）。
4. **extract:** 智能、无噪音的纯净 HTML 和内容提取引擎（`html_ayikla`）。
5. **masa_dongusu:** 6 门验证机制及自治式门过渡锁定机制（`validate_masa_dongusu`）。
6. **multilingual:** 标准化的并行多语言 README 引擎和质量控制器。
7. **router:** 智能体子网路由、语义图架构以及 Edge/Route 管理（`RouteResult`）。
8. **rules_checker:** 高级 Rust 源代码规则与质量标准审核器（`check_rust_code_rules`）。
9. **session:** 针对通信会话和内存数据的历史结构化 BM25 查询模块（`search_session`）。
10. **skillopt:** 智能体技能自治进化模块；带评分矩阵的软/硬门验证命令基础设施。
11. **turso-dsl:** 针对 Edge SQLite 结构的自定义架构定义命令及抽象层依赖。
12. **hermes-react & hermes-tls:** 允许智能体与外部系统进行安全异步交互的服务器模块。

---

#### ⚙️ 环境要求与安装

##### 依赖与 Crates
* **Rust 1.63+** (Edition 2021)
* 工作区 Crates: `ibnunnedim-cli`, `crates/hermes-tools-core`, `crates/turso-dsl`, `crates/hermes-react`, `crates/hermes-tls-server`
* 系统数据库: `Turso SQLite 0.7.2` (`kutup_kutuphane.db`)

##### 构建与执行
```bash
# 工作区 release 构建
cargo build --release --workspace

# 使用 BM25 进行技能搜索
./target/release/ibnunnedim search "rust"

# 列出技能
./target/release/ibnunnedim list --kategori "software-development"

# 提交报告 (Tekmil) 分数
./target/release/ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Passed outer gauntlet"
```

---

#### 🏗️ 架构与工作原理
* **BM25 索引构建:** 数据库中的所有技能均单次查询读取，并加载到内存 BM25 索引中。使用稳定的 UTF-8 `kisalt` 函数安全地截断文本。
* **数据库集成:** 使用 `turso::Builder::new_local` 驱动程序建立零拷贝的远程及本地同步 SQLite 连接。
* **分层工具:** `tokio` (异步运行时), `clap` (CLI 参数解析器), `axum` & `tower` (HTTP/REST 服务器), `serde_json` (配置清单操作).

---

#### 🗺️ 路线图
- [x] 纯 Rust BM25 搜索引擎，支持土耳其语字符折叠。
- [x] Turso SQLite `kutup_kutuphane.db` 集成与 Tekmil 计分机制。
- [ ] 向语义（启发式、诠释学、认识论、道德等）动态数据库/技能分类结构过渡，实现自治、指哪打哪的工具调用架构。
- [ ] 为智能体构建本地 MCP (Model Context Protocol) GraphQL/gRPC 桥接。
- [ ] 通过自治式 SkillOpt 睡眠引擎直接修改 Turso。
- [ ] 多租户智能体内存索引基础设施。

---

#### 📝 版本更新
* **v0.2.0 (当前):**
  - 新增 Maşa Döngüsü D1-D3 及 R2 卫生检查门。
  - 整合 7 语言混合 README 架构 (`readme-yolbulucu` 标准)。
  - 对核心模块 (`codebase`, `docx`, `multilingual` 等) 采用模块化结构。
* **v0.1.0:** 首次可编译的 Rust 工作区、`ibnunnedim-cli` 及基础 Turso 数据库集成 (初始结构)。

---

#### 👥 贡献者 (Co-Authors)
本项目架构和代码库的贡献者：
* **Ercan ER** `<ercan.er@medeniyet.edu.tr>` *(项目所有者与首席架构师)*
* **İbnünnedîm (hermes-agent)** `<291384122+hermes-agent@users.noreply.github.com>` *(图书管理员智能体)*
* **Mihenk (Claude Opus 5)** `<noreply@anthropic.com>` *(卫生、代码质量与审计裁判)*
* **Cezeri (Demirci)** *(软件与自治系统健康监测 - 智能体接口架构师)*

---

#### 📜 许可证
本项目基于 [MIT 许可证](LICENSE) 开源。