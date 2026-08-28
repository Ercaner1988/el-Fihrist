# el-Fihrist (الفهرست)

> 慈悲あまねく慈愛深きアッラーの御名において。万物の主なるアッラーに讃えあれ。その使徒に平安と祝福がありますように。本プロジェクトは、10世紀にバグダードで生きた偉大な書誌学者アブー・アル＝ファラジ・ムハンマド・イブン・イスハーク・アン＝ナディームと、彼の不朽の著作である『アル＝フィフリスト』から名と精神を受け継いでいる。我々の文明の最初の司書の遺産を継承し、AIエージェントのための純粋なRustとTurso SQLiteベースのスキル、コード、およびメモリライブラリを構築した。

---

## 🌍 Dil Seçenekleri / Languages
🇹🇷 [Türkçe](README.md) | 🇬🇧 [English](README.en.md) | 🇸🇦 [العربية](README.ar.md) | 🇯🇵 [日本語](README.ja.md) | 🇨🇳 [中文](README.zh.md) | 🇷🇺 [Русский](README.ru.md) | 🇪🇸 [Español](README.es.md)

---

### 🇯🇵 日本語

#### 📌 目次
- [📖 概要と機能](#-概要と機能)
    - [🧰 利用可能なスキルとシステムモジュール](#-利用可能なスキルとシステムモジュール)
- [⚙️ 要件とインストール](#️-要件とインストール)
- [🏗️ インフラストラクチャと動作原理](#️-インフラストラクチャと動作原理)
- [🗺️ ロードマップ](#️-ロードマップ)
- [📝 リリースごとの更新](#-リリースごとの更新)
- [👥 貢献者 (Co-Authors)](#-貢献者-co-authors)
- [📜 ライセンス](#-ライセンス)

---

#### 📖 概要と機能
**el-Fihrist** は、AIエージェント（Hermes Agent, Claude Desktop MCP, DeepSeek Harness, OpenCode, Codex）向けに設計された、ポータブルなスキル、コードスニペット、およびメモリライブラリエンジンである。

* **純粋なRust BM25検索エンジン:** トルコ語の文字折りたたみ（例: `İ→i`, `I→ı`, `Ş→ş`）を備えた、メモリ内高速関連性検索エンジン。
* **Turso / SQLiteコア:** 140以上の外部スキルと組み込みのRustツールを含むポータブルSQLiteデータベース（`kutup_kutuphane.db`）。
* **マルチクレートアーキテクチャ:** CLI、Core、Turso-DSL、React、およびTLS Serverモジュールで構成されるモジュール式Rust構造。
* **厳格な検証 (Maşa Döngüsü):** D1-D3およびR2のGolden衛生検査ゲートによる、安全なコードおよびレポート監査。

##### 🧰 利用可能なスキルとシステムモジュール
コアライブラリ（`hermes-tools-core` およびワークスペース）には、現在、直接使用できる12の主要なスキルとモジュールが用意されている:
1. **citation:** Zopayインフラストラクチャを介した、論文や学術テキストの引用の検証と報告（`verify_citations`）。
2. **codebase:** ソースコードの要約、ファイルコンテンツ分析、および品質検出（`analyze_file_content`）。
3. **docx:** MS Word XMLレイヤーの解析と低フォームファクタ読み取り（`extract_paragraphs_from_xml`）。
4. **extract:** ノイズのないクリーンなHTMLおよびコンテンツ抽出用のスマートエンジン（`html_ayikla`）。
5. **masa_dongusu:** 6つのゲートによる検証と自律的なゲート移行ロックメカニズム（`validate_masa_dongusu`）。
6. **multilingual:** 標準化された並列多言語READMEエンジンと品質コントローラ。
7. **router:** エージェントサブネットのルーティング、セマンティックグラフアーキテクチャ、およびEdge/Route管理（`RouteResult`）。
8. **rules_checker:** 高度なRustソースコードルールおよび品質基準の監査（`check_rust_code_rules`）。
9. **session:** 通信セッションおよびメモリデータ向けの、過去の構造化BM25クエリモジュール（`search_session`）。
10. **skillopt:** エージェントスキルの自律的な進化。スコアリングマトリクスを利用したソフト/ハードゲートの検証コマンドインフラストラクチャ。
11. **turso-dsl:** Edge SQLite構造向けの、カスタマイズされたスキーマ定義コマンドと抽象化レイヤーの依存関係。
12. **hermes-react & hermes-tls:** 外部システムとのエージェントの安全な非同期通信を可能にするサーバーモジュール。

---

#### ⚙️ 要件とインストール

##### 依存関係とクレート
* **Rust 1.63+** (Edition 2021)
* ワークスペースクレート: `ibnunnedim-cli`, `crates/hermes-tools-core`, `crates/turso-dsl`, `crates/hermes-react`, `crates/hermes-tls-server`
* システムデータベース: `Turso SQLite 0.7.2` (`kutup_kutuphane.db`)

##### ビルドと実行
```bash
# ワークスペースのリリースビルド
cargo build --release --workspace

# BM25を使用したスキル検索
./target/release/ibnunnedim search "rust"

# スキルのリスト表示
./target/release/ibnunnedim list --kategori "software-development"

# レポート（Tekmil）スコアの送信
./target/release/ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Passed outer gauntlet"
```

---

#### 🏗️ インフラストラクチャと動作原理
* **BM25インデックス作成:** データベース内のすべてのスキルは単一のクエリで読み取られ、メモリ内のBM25インデックスに読み込まれる。UTF-8で安定した `kisalt` 関数を使用して、安全にテキストを切り捨てる。
* **データベース統合:** `turso::Builder::new_local` ドライバを使用して、ゼロコピーのリモートおよびローカルの同期SQLite接続を確立する。
* **レイヤーユーティリティ:** `tokio` (非同期ランタイム)、`clap` (CLI引数パーサー)、`axum` & `tower` (HTTP/RESTサーバー)、`serde_json` (マニフェスト操作)。

---

#### 🗺️ ロードマップ
- [x] トルコ語の文字折りたたみが可能な純粋なRust BM25検索エンジン。
- [x] Turso SQLite `kutup_kutuphane.db` 統合およびTekmilスコアリングメカニズム。
- [ ] 自律的かつ的確なツール呼び出しアーキテクチャのための、セマンティック（ヒューリスティック、解釈学的、認識論的、道徳的など）な動的データベース/スキル分類構造への移行。
- [ ] エージェント用のローカルMCP (Model Context Protocol) GraphQL/gRPCブリッジの構築。
- [ ] 自律的なSkillOptスリープエンジンによるTursoの直接的な変更。
- [ ] マルチテナントのエージェントメモリインデックスインフラストラクチャ。

---

#### 📝 リリースごとの更新
* **v0.2.0 (現在):**
  - Maşa Döngüsü D1-D3およびR2の衛生検査ゲートを追加。
  - 7言語のハイブリッドREADMEアーキテクチャ（`readme-yolbulucu`標準）を統合。
  - コアモジュール（`codebase`, `docx`, `multilingual` など）にモジュール構造を採用。
* **v0.1.0:** 初期のコンパイル可能なRustワークスペース、`ibnunnedim-cli`、および基本的なTursoデータベースの統合（初期構造）。

---

#### 👥 貢献者 (Co-Authors)
プロジェクトのアーキテクチャとコードベースへの貢献者:
* **Ercan ER** `<ercan.er@medeniyet.edu.tr>` *(プロジェクトオーナー兼チーフアーキテクト)*
* **İbnünnedîm (hermes-agent)** `<291384122+hermes-agent@users.noreply.github.com>` *(司書エージェント)*
* **Mihenk (Claude Opus 5)** `<noreply@anthropic.com>` *(衛生、コード品質、および監査のレフェリー)*
* **Cezeri (Demirci)** *(ソフトウェアおよび自律システムのヘルスモニタリング - エージェントインターフェースアーキテクト)*

---

#### 📜 ライセンス
このプロジェクトは[MITライセンス](LICENSE)の下でライセンスされている。
