use serde::{Deserialize, Serialize};
use std::fmt;

pub const ISLAMIC_OPENING_TR: &str = "Bismillahirrahmanirrahim. Rahmân ve Rahîm olan Allah'ın adıyla. Hamd âlemlerin Rabbine, salât ve selâm O'nun Resûlü'ne olsun. Bu proje adını ve ruhunu, 10. yüzyılda Bağdat'ta yaşamış büyük bibliyograf Ebü’l-Ferec Muhammed b. İshâk en-Nedîm ve onun ölümsüz eseri *el-Fihrist*'ten almaktadır. Medeniyetimizin ilk kütüphanecisinin mirasıyla; yapay zekâ ajanları için saf Rust ve Turso SQLite tabanlı yetenek, kod ve hafıza kütüphanesini inşa ettik.";

pub const ISLAMIC_OPENING_EN: &str = "In the name of Allah, the Most Gracious, the Most Merciful. Praise be to the Lord of the worlds, and peace and blessings be upon His Messenger. This project takes its name and spirit from the great bibliographer Abu al-Faraj Muhammad b. Ishaq al-Nadim, who lived in Baghdad in the 10th century, and his immortal work *al-Fihrist*. Inheriting the legacy of our civilization's first librarian, we have built a pure Rust and Turso SQLite-based skill, code, and memory library for AI agents.";

pub const ISLAMIC_OPENING_AR: &str = "بسم الله الرحمن الرحيم. الحمد لله رب العالمين، والصلاة والسلام على رسوله. يستمد هذا المشروع اسمه وروحه من الببليوغرافي العظيم أبو الفرج محمد بن إسحاق النديم الذي عاش في بغداد في القرن العاشر، ومن عمله الخالد *الفهرست*. استلهاماً من تراث أول أمين مكتبة في حضارتنا؛ قمنا ببناء مكتبة للمهارات، والرموز البرمجية، والذاكرة مخصصة للوكلاء الأذكياء (AI agents)، تعتمد على لغة Rust الخالصة وقاعدة بيانات Turso SQLite.";

pub const ISLAMIC_OPENING_JA: &str = "慈悲あまねく慈愛深きアッラーの御名において。万物の主なるアッラーに讃えあれ。その使徒に平安と祝福がありますように。本プロジェクトは、10世紀にバグダードで生きた偉大な書誌学者アブー・アル＝ファラジ・ムハンマド・イブン・イスハーク・アン＝ナディームと、彼の不朽の著作である『アル＝フィフリスト』から名と精神を受け継いでいる。我々の文明の最初の司書の遺産を継承し、AIエージェントのための純粋なRustとTurso SQLiteベースのスキル、コード、およびメモリライブラリを構築した。";

pub const ISLAMIC_OPENING_ZH: &str = "奉至仁至慈的安拉之名。赞美归于全世界的主，愿和平与祝福降临于祂的使者。本项目的名称和精神源自10世纪生活在巴格达的伟大书目学家阿布·法拉吉·穆罕默德·本·伊沙克·纳迪姆（Abu al-Faraj Muhammad b. Ishaq al-Nadim）及其不朽著作《书目》（*al-Fihrist*）。传承我们文明中第一位图书馆员的遗产，我们为AI智能体构建了一个纯Rust和基于Turso SQLite的技能、代码与内存库。";

pub const ISLAMIC_OPENING_RU: &str = "Во имя Аллаха, Милостивого, Милосердного. Хвала Господу миров, мир и благословение Его Посланнику. Этот проект берет свое имя и дух от великого библиографа Абу аль-Фараджа Мухаммада ибн Исхака ан-Надима, жившего в Багдаде в 10 веке, и его бессмертного труда *аль-Фихрист* (al-Fihrist). Наследуя опыт первого библиотекаря нашей цивилизации, мы создали библиотеку навыков, кода и памяти для ИИ-агентов на базе чистого Rust и Turso SQLite.";

pub const ISLAMIC_OPENING_ES: &str = "En nombre de Dios, el Más Clemente, el Más Misericordioso. Alabado sea el Señor de los mundos, y que la paz y las bendiciones sean con Su Mensajero. Este proyecto toma su nombre y espíritu del gran bibliógrafo Abu al-Faraj Muhammad b. Ishaq al-Nadim, quien vivió en Bagdad en el siglo X, y de su obra inmortal *al-Fihrist*. Heredando el legado del primer bibliotecario de nuestra civilización, hemos construido una biblioteca de habilidades, códigos y memoria basada en Rust puro y Turso SQLite para agentes de IA.";

pub const LANGUAGE_BANNER: &str = "🇹🇷 [Türkçe](README.md) | 🇬🇧 [English](README.en.md) | 🇸🇦 [العربية](README.ar.md) | 🇯🇵 [日本語](README.ja.md) | 🇨🇳 [中文](README.zh.md) | 🇷🇺 [Русский](README.ru.md) | 🇪🇸 [Español](README.es.md)";

/// Shields.io rozetleri için özel kaçış kuralları (- -> --, _ -> __)
pub fn escape_badge_text(text: &str) -> String {
    text.replace('-', "--").replace('_', "__")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BadgeInfo {
    pub label: String,
    pub message: String,
    pub color: String,
    pub link: Option<String>,
}

impl BadgeInfo {
    pub fn new(
        label: impl Into<String>,
        message: impl Into<String>,
        color: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            message: message.into(),
            color: color.into(),
            link: None,
        }
    }

    pub fn with_link(mut self, link: impl Into<String>) -> Self {
        self.link = Some(link.into());
        self
    }

    pub fn to_shields_url(&self) -> String {
        let clean_label = escape_badge_text(&self.label);
        let clean_msg = escape_badge_text(&self.message);
        let clean_color = escape_badge_text(&self.color);
        format!(
            "https://img.shields.io/badge/{}-{}-{}.svg",
            clean_label, clean_msg, clean_color
        )
    }

    pub fn to_markdown(&self) -> String {
        let url = self.to_shields_url();
        let alt = format!("{}: {}", self.label, self.message);
        if let Some(ref link) = self.link {
            format!("[![{alt}]({url})]({link})")
        } else {
            format!("![{alt}]({url})")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadmapItem {
    pub completed: bool,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub description: String,
    pub license: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub author_github: Option<String>,
    #[serde(default)]
    pub author_website: Option<String>,
    #[serde(default)]
    pub author_twitter: Option<String>,
    #[serde(default)]
    pub author_linkedin: Option<String>,
    #[serde(default)]
    pub repository_url: Option<String>,
    #[serde(default)]
    pub documentation_url: Option<String>,
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub contributing_url: Option<String>,
    #[serde(default)]
    pub install_command: Option<String>,
    #[serde(default)]
    pub usage_command: Option<String>,
    #[serde(default)]
    pub test_command: Option<String>,
    #[serde(default)]
    pub badges: Vec<BadgeInfo>,
    #[serde(default)]
    pub co_authors: Vec<String>,
    #[serde(default)]
    pub modules: Vec<ModuleInfo>,
    #[serde(default)]
    pub roadmap: Vec<RoadmapItem>,
}

impl Default for ProjectMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            license: "MIT".to_string(),
            version: "0.1.0".to_string(),
            author: String::new(),
            author_github: None,
            author_website: None,
            author_twitter: None,
            author_linkedin: None,
            repository_url: None,
            documentation_url: None,
            homepage_url: None,
            issues_url: None,
            contributing_url: None,
            install_command: None,
            usage_command: None,
            test_command: None,
            badges: Vec::new(),
            co_authors: Vec::new(),
            modules: Vec::new(),
            roadmap: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReadmeUpdates {
    pub features: Option<String>,
    pub roadmap: Option<Vec<RoadmapItem>>,
    pub release_updates: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultilingualReadme {
    pub tr: String,
    pub en: String,
    pub ar: String,
    pub ja: String,
    pub zh: String,
    pub ru: String,
    pub es: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultilingualError {
    SectionNotFound(String),
    EmptyContent,
    InvalidFormat(String),
}

impl std::error::Error for MultilingualError {}

impl fmt::Display for MultilingualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultilingualError::SectionNotFound(sec) => write!(f, "Bölüm bulunamadı: {}", sec),
            MultilingualError::EmptyContent => write!(f, "İçerik boş"),
            MultilingualError::InvalidFormat(msg) => write!(f, "Geçersiz biçim: {}", msg),
        }
    }
}

/// Cargo.toml içeriğinden proje meta verilerini ayrıştırır
pub fn parse_cargo_toml_content(content: &str) -> ProjectMeta {
    let mut meta = ProjectMeta::default();
    let mut in_package_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]" || trimmed == "[workspace.package]";
            continue;
        }

        if in_package_section {
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'').trim();

                match key {
                    "name" if meta.name.is_empty() => meta.name = val.to_string(),
                    "version" if meta.version == "0.1.0" => meta.version = val.to_string(),
                    "description" if meta.description.is_empty() => {
                        meta.description = val.to_string()
                    }
                    "license" => meta.license = val.to_string(),
                    "authors" => {
                        // authors = ["Adı Soyadı <eposta>"]
                        let clean = val.trim_matches('[').trim_matches(']');
                        for part in clean.split(',') {
                            let author_part = part.trim().trim_matches('"').trim_matches('\'');
                            if !author_part.is_empty() && meta.author.is_empty() {
                                meta.author = author_part.to_string();
                            }
                        }
                    }
                    "repository" => meta.repository_url = Some(val.to_string()),
                    "homepage" => meta.homepage_url = Some(val.to_string()),
                    "documentation" => meta.documentation_url = Some(val.to_string()),
                    _ => {}
                }
            }
        }
    }

    if !meta.name.is_empty() {
        meta.install_command = Some(format!("cargo add {}", meta.name));
        meta.usage_command = Some("cargo run".to_string());
        meta.test_command = Some("cargo test".to_string());
    }

    meta
}

/// package.json içeriğinden proje meta verilerini ayrıştırır
pub fn parse_package_json_content(content: &str) -> Result<ProjectMeta, String> {
    let json: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("package.json ayrıştırma hatası: {e}"))?;

    let mut meta = ProjectMeta::default();

    if let Some(n) = json.get("name").and_then(|v| v.as_str()) {
        meta.name = n.to_string();
    }
    if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
        meta.version = v.to_string();
    }
    if let Some(d) = json.get("description").and_then(|v| v.as_str()) {
        meta.description = d.to_string();
    }
    if let Some(l) = json.get("license").and_then(|v| v.as_str()) {
        meta.license = l.to_string();
    }
    if let Some(a) = json.get("author") {
        if let Some(author_str) = a.as_str() {
            meta.author = author_str.to_string();
        } else if let Some(author_obj) = a.as_object() {
            if let Some(name) = author_obj.get("name").and_then(|v| v.as_str()) {
                meta.author = name.to_string();
            }
            if let Some(url) = author_obj.get("url").and_then(|v| v.as_str()) {
                meta.author_website = Some(url.to_string());
            }
        }
    }
    if let Some(repo) = json.get("repository") {
        if let Some(repo_str) = repo.as_str() {
            meta.repository_url = Some(repo_str.to_string());
        } else if let Some(repo_obj) = repo.as_object() {
            if let Some(url) = repo_obj.get("url").and_then(|v| v.as_str()) {
                meta.repository_url = Some(url.to_string());
            }
        }
    }
    if let Some(hp) = json.get("homepage").and_then(|v| v.as_str()) {
        meta.homepage_url = Some(hp.to_string());
    }
    if let Some(bugs) = json.get("bugs") {
        if let Some(bugs_str) = bugs.as_str() {
            meta.issues_url = Some(bugs_str.to_string());
        } else if let Some(bugs_obj) = bugs.as_object() {
            if let Some(url) = bugs_obj.get("url").and_then(|v| v.as_str()) {
                meta.issues_url = Some(url.to_string());
            }
        }
    }

    if let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) {
        if scripts.contains_key("test") {
            meta.test_command = Some("npm test".to_string());
        }
        if scripts.contains_key("start") {
            meta.usage_command = Some("npm start".to_string());
        }
    }
    meta.install_command = Some("npm install".to_string());

    Ok(meta)
}

/// Git config içeriğinden remote URL ve repo yolunu ayrıştırır
pub fn parse_git_config_content(content: &str) -> Option<(String, String)> {
    let mut in_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[remote \"origin\"]" {
            in_origin = true;
            continue;
        }
        if in_origin {
            if trimmed.starts_with('[') {
                break;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim() == "url" {
                    let mut url = v.trim().to_string();
                    if url.ends_with(".git") {
                        url.truncate(url.len() - 4);
                    }
                    // SSH URL: git@github.com:owner/repo -> https://github.com/owner/repo
                    if url.starts_with("git@") {
                        if let Some((_, path)) = url.split_once(':') {
                            let clean_url = format!("https://github.com/{}", path);
                            return Some((clean_url, path.to_string()));
                        }
                    }
                    if let Some(idx) = url.find("github.com/") {
                        let path = url[idx + 11..].to_string();
                        return Some((url, path));
                    }
                    return Some((url.clone(), url));
                }
            }
        }
    }
    None
}

/// Standart otomatik rozetleri oluşturur
pub fn build_standard_badges(meta: &ProjectMeta) -> Vec<BadgeInfo> {
    let mut badges = Vec::new();

    // Sürüm rozeti
    if !meta.version.is_empty() {
        badges.push(BadgeInfo::new("version", &meta.version, "blue"));
    }

    // Lisans rozeti
    if !meta.license.is_empty() {
        badges.push(BadgeInfo::new("license", &meta.license, "green"));
    }

    // Bakım durumu
    badges.push(BadgeInfo::new("Maintained%3F", "yes", "brightgreen"));

    // Kullanıcının tanımladığı ek rozetler
    for b in &meta.badges {
        badges.push(b.clone());
    }

    badges
}

pub fn generate_yolbulucu_readme(meta: &ProjectMeta) -> MultilingualReadme {
    let format_modules = |modules: &[ModuleInfo]| {
        if modules.is_empty() {
            return "Modül bilgisi bulunmamaktadır.".to_string();
        }
        modules
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. **{}:** {}", i + 1, m.name, m.description))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let format_modules_en = |modules: &[ModuleInfo]| {
        if modules.is_empty() {
            return "No module details provided.".to_string();
        }
        modules
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. **{}:** {}", i + 1, m.name, m.description))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let format_roadmap = |roadmap: &[RoadmapItem]| {
        if roadmap.is_empty() {
            return "- [ ] Yol haritası maddesi eklenmedi.".to_string();
        }
        roadmap
            .iter()
            .map(|r| {
                let mark = if r.completed { "x" } else { " " };
                format!("- [{}] {}", mark, r.title)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let format_co_authors = |author: &str, co_authors: &[String]| {
        let mut list = Vec::new();
        if !author.is_empty() {
            list.push(format!("* **{}** *(Proje Sahibi)*", author));
        }
        for ca in co_authors {
            list.push(format!("* **{}**", ca));
        }
        if list.is_empty() {
            list.push("* Proje geliştiricileri".to_string());
        }
        list.join("\n")
    };

    let format_badges = |badges: &[BadgeInfo]| {
        if badges.is_empty() {
            return String::new();
        }
        let rendered: Vec<String> = badges.iter().map(|b| b.to_markdown()).collect();
        format!("<p align=\"center\">\n  {}\n</p>\n\n", rendered.join(" "))
    };

    let all_badges = build_standard_badges(meta);
    let badge_str = format_badges(&all_badges);

    let install_cmd = meta
        .install_command
        .as_deref()
        .unwrap_or("cargo add <paket-adi>");
    let usage_cmd = meta.usage_command.as_deref().unwrap_or("cargo run");
    let test_cmd = meta.test_command.as_deref().unwrap_or("cargo test");

    let author_contact_tr = {
        let mut parts = Vec::new();
        if !meta.author.is_empty() {
            parts.push(format!("👤 **{}**", meta.author));
        }
        if let Some(ref gh) = meta.author_github {
            parts.push(format!("* GitHub: [@{}](https://github.com/{})", gh, gh));
        }
        if let Some(ref tw) = meta.author_twitter {
            parts.push(format!("* Twitter: [@{}](https://twitter.com/{})", tw, tw));
        }
        if let Some(ref li) = meta.author_linkedin {
            parts.push(format!(
                "* LinkedIn: [@{}](https://linkedin.com/in/{})",
                li, li
            ));
        }
        if let Some(ref ws) = meta.author_website {
            parts.push(format!("* Web Sitesi: {}", ws));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("\n\n---\n\n#### 👤 Yazar ve İletişim\n{}", parts.join("\n"))
        }
    };

    let author_contact_en = {
        let mut parts = Vec::new();
        if !meta.author.is_empty() {
            parts.push(format!("👤 **{}**", meta.author));
        }
        if let Some(ref gh) = meta.author_github {
            parts.push(format!("* GitHub: [@{}](https://github.com/{})", gh, gh));
        }
        if let Some(ref tw) = meta.author_twitter {
            parts.push(format!("* Twitter: [@{}](https://twitter.com/{})", tw, tw));
        }
        if let Some(ref li) = meta.author_linkedin {
            parts.push(format!(
                "* LinkedIn: [@{}](https://linkedin.com/in/{})",
                li, li
            ));
        }
        if let Some(ref ws) = meta.author_website {
            parts.push(format!("* Website: {}", ws));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("\n\n---\n\n#### 👤 Author & Contact\n{}", parts.join("\n"))
        }
    };

    let tr = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇹🇷 Türkçe\n\n#### 📌 İçindekiler\n- [📖 Ne Nedir ve Özellikler](#-ne-nedir-ve-özellikler)\n    - [🧰 Mevcut Yetenekler ve Sistem Modülleri](#-mevcut-yetenekler-ve-sistem-modülleri)\n- [⚙️ Gereksinimler ve Kurulum](#️-gereksinimler-ve-kurulum)\n- [🚀 Kullanım](#-kullanım)\n- [✅ Test ve Doğrulama](#-test-ve-doğrulama)\n- [🗺️ Yol Haritası](#️-yol-haritası)\n- [📝 Sürüm Güncellemeleri](#-sürüm-güncellemeleri)\n- [👥 Katkıda Bulunanlar (Co-Authors)](#-katkıda-bulunanlar-co-authors){}\n- [📜 Lisans](#-lisans)\n\n---\n\n#### 📖 Ne Nedir ve Özellikler\n{}\n\n##### 🧰 Mevcut Yetenekler ve Sistem Modülleri\n{}\n\n---\n\n#### ⚙️ Gereksinimler ve Kurulum\nSürüm: `{}` | Lisans: `{}` | Yazar: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 Kullanım\n```bash\n{}\n```\n\n---\n\n#### ✅ Test ve Doğrulama\n```bash\n{}\n```\n\n---\n\n#### 🗺️ Yol Haritası\n{}\n\n---\n\n#### 📝 Sürüm Güncellemeleri\n* **v{} (Mevcut):** İlk kararlı sürüm.\n\n---\n\n#### 👥 Katkıda Bulunanlar (Co-Authors)\n{}{}\n\n---\n\n#### 📜 Lisans\nBu proje [{}] Lisansı ile lisanslanmıştır.\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_TR,
        LANGUAGE_BANNER,
        if author_contact_tr.is_empty() { "" } else { "\n- [👤 Yazar ve İletişim](#-yazar-ve-iletişim)" },
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        author_contact_tr,
        meta.license
    );

    let en = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇬🇧 English\n\n#### 📌 Table of Contents\n- [📖 Description and Features](#-description-and-features)\n    - [🧰 Available Skills and System Modules](#-available-skills-and-system-modules)\n- [⚙️ Requirements and Installation](#️-requirements-and-installation)\n- [🚀 Usage](#-usage)\n- [✅ Running Tests](#-running-tests)\n- [🗺️ Roadmap](#️-roadmap)\n- [📝 Release Updates](#-release-updates)\n- [👥 Contributors (Co-Authors)](#-contributors-co-authors){}\n- [📜 License](#-license)\n\n---\n\n#### 📖 Description and Features\n{}\n\n##### 🧰 Available Skills and System Modules\n{}\n\n---\n\n#### ⚙️ Requirements and Installation\nVersion: `{}` | License: `{}` | Author: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 Usage\n```bash\n{}\n```\n\n---\n\n#### ✅ Running Tests\n```bash\n{}\n```\n\n---\n\n#### 🗺️ Roadmap\n{}\n\n---\n\n#### 📝 Release Updates\n* **v{} (Current):** Initial stable release.\n\n---\n\n#### 👥 Contributors (Co-Authors)\n{}{}\n\n---\n\n#### 📜 License\nThis project is licensed under the [{}] License.\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_EN,
        LANGUAGE_BANNER,
        if author_contact_en.is_empty() { "" } else { "\n- [👤 Author & Contact](#-author--contact)" },
        meta.description,
        format_modules_en(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        author_contact_en,
        meta.license
    );

    let ar = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇸🇦 العربية\n\n#### 📌 جدول المحتويات\n- [📖 الوصف والميزات](#-الوصف-والميزات)\n    - [🧰 المهارات المتاحة ووحدات النظام](#-المهارات-المتاحة-ووحدات-النظام)\n- [⚙️ المتطلبات والتثبيت](#️-المتطلبات-والتثبيت)\n- [🚀 الاستخدام](#-الاستخدام)\n- [✅ الاختبارات والتحقق](#-الاختبارات-والتحقق)\n- [🗺️ خريطة الطريق](#️-خريطة-الطريق)\n- [📝 تحديثات الإصدار](#-تحديثات-الإصدار)\n- [👥 المساهمون (Co-Authors)](#-المساهمون-co-authors)\n- [📜 الترخيص](#-الترخيص)\n\n---\n\n#### 📖 الوصف والميزات\n{}\n\n##### 🧰 المهارات المتاحة ووحدات النظام\n{}\n\n---\n\n#### ⚙️ المتطلبات والتثبيت\nالإصدار: `{}` | الترخيص: `{}` | المؤلف: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 الاستخدام\n```bash\n{}\n```\n\n---\n\n#### ✅ الاختبارات والتحقق\n```bash\n{}\n```\n\n---\n\n#### 🗺️ خريطة الطريق\n{}\n\n---\n\n#### 📝 تحديثات الإصدار\n* **v{} (الحالي):** الإصدار المستقر الأولي.\n\n---\n\n#### 👥 المساهمون (Co-Authors)\n{}\n\n---\n\n#### 📜 الترخيص\nهذا المشروع مرخص بموجب ترخيص [{}].\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_AR,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let ja = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇯🇵 日本語\n\n#### 📌 目次\n- [📖 概要と特徴](#-概要と特徴)\n    - [🧰 利用可能なスキルとシステムモジュール](#-利用可能なスキルとシステムモジュール)\n- [⚙️ 要件とインストール](#️-要件とインストール)\n- [🚀 使い方](#-使い方)\n- [✅ テストの実行](#-テストの実行)\n- [🗺️ ロードマップ](#️-ロードマップ)\n- [📝 リリース更新](#-リリース更新)\n- [👥 貢献者 (Co-Authors)](#-貢献者-co-authors)\n- [📜 ライセンス](#-ライセンス)\n\n---\n\n#### 📖 概要と特徴\n{}\n\n##### 🧰 利用可能なスキルとシステムモジュール\n{}\n\n---\n\n#### ⚙️ 要件とインストール\nバージョン: `{}` | ライセンス: `{}` | 著者: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 使い方\n```bash\n{}\n```\n\n---\n\n#### ✅ テストの実行\n```bash\n{}\n```\n\n---\n\n#### 🗺️ ロードマップ\n{}\n\n---\n\n#### 📝 リリース更新\n* **v{} (現在):** 最初の安定版リリース。\n\n---\n\n#### 👥 貢献者 (Co-Authors)\n{}\n\n---\n\n#### 📜 ライセンス\nこのプロジェクトは [{}] ライセンスのもとで公開されています。\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_JA,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let zh = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇨🇳 中文\n\n#### 📌 目录\n- [📖 描述与特性](#-描述与特性)\n    - [🧰 可用技能与系统模块](#-可用技能与系统模块)\n- [⚙️ 要求与安装](#️-要求与安装)\n- [🚀 使用说明](#-使用说明)\n- [✅ 运行测试](#-运行测试)\n- [🗺️ 路线图](#️-路线图)\n- [📝 发布更新](#-发布更新)\n- [👥 贡献者 (Co-Authors)](#-贡献者-co-authors)\n- [📜 许可证](#-许可证)\n\n---\n\n#### 📖 描述与特性\n{}\n\n##### 🧰 可用技能与系统模块\n{}\n\n---\n\n#### ⚙️ 要求与安装\n版本: `{}` | 许可证: `{}` | 作者: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 使用说明\n```bash\n{}\n```\n\n---\n\n#### ✅ 运行测试\n```bash\n{}\n```\n\n---\n\n#### 🗺️ 路线图\n{}\n\n---\n\n#### 📝 发布更新\n* **v{} (当前):** 初始稳定版本。\n\n---\n\n#### 👥 贡献者 (Co-Authors)\n{}\n\n---\n\n#### 📜 许可证\n本项目采用 [{}] 许可证。\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_ZH,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let ru = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇷🇺 Русский\n\n#### 📌 Содержание\n- [📖 Описание и возможности](#-описание-и-возможности)\n    - [🧰 Доступные навыки и системные модули](#-доступные-навыки-и-системные-модули)\n- [⚙️ Требования и установка](#️-требования-и-установка)\n- [🚀 Использование](#-использование)\n- [✅ Запуск тестов](#-запуск-тестов)\n- [🗺️ Дорожная карта](#️-дорожная-карта)\n- [📝 Обновления выпусков](#-обновления-выпусков)\n- [👥 Соавторы (Co-Authors)](#-соавторы-co-authors)\n- [📜 Лицензия](#-лицензия)\n\n---\n\n#### 📖 Описание и возможности\n{}\n\n##### 🧰 Доступные навыки и системные модули\n{}\n\n---\n\n#### ⚙️ Требования и установка\nВерсия: `{}` | Лицензия: `{}` | Автор: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 Использование\n```bash\n{}\n```\n\n---\n\n#### ✅ Запуск тестов\n```bash\n{}\n```\n\n---\n\n#### 🗺️ Дорожная карта\n{}\n\n---\n\n#### 📝 Обновления выпусков\n* **v{} (Текущая):** Первый стабильный релиз.\n\n---\n\n#### 👥 Соавторы (Co-Authors)\n{}\n\n---\n\n#### 📜 Лицензия\nЭтот проект распространяется под лицензией [{}].\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_RU,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let es = format!(
        "# {}\n\n{}\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇪🇸 Español\n\n#### 📌 Tabla de contenidos\n- [📖 Descripción y Características](#-descripción-y-características)\n    - [🧰 Habilidades Disponibles y Módulos del Sistema](#-habilidades-disponibles-y-módulos-del-sistema)\n- [⚙️ Requisitos e Instalación](#️-requisitos-e-instalación)\n- [🚀 Uso](#-uso)\n- [✅ Ejecución de pruebas](#-ejecución-de-pruebas)\n- [🗺️ Hoja de ruta](#️-hoja-de-ruta)\n- [📝 Actualizaciones de lanzamientos](#-actualizaciones-de-lanzamientos)\n- [👥 Colaboradores (Co-Authors)](#-colaboradores-co-authors)\n- [📜 Licencia](#-licencia)\n\n---\n\n#### 📖 Descripción y Características\n{}\n\n##### 🧰 Habilidades Disponibles y Módulos del Sistema\n{}\n\n---\n\n#### ⚙️ Requisitos e Instalación\nVersión: `{}` | Licencia: `{}` | Autor: `{}`\n```bash\n{}\n```\n\n---\n\n#### 🚀 Uso\n```bash\n{}\n```\n\n---\n\n#### ✅ Ejecución de pruebas\n```bash\n{}\n```\n\n---\n\n#### 🗺️ Hoja de ruta\n{}\n\n---\n\n#### 📝 Actualizaciones de lanzamientos\n* **v{} (Actual):** Primera versión estable.\n\n---\n\n#### 👥 Colaboradores (Co-Authors)\n{}\n\n---\n\n#### 📜 Licencia\nEste proyecto está licenciado bajo la Licencia [{}].\n",
        meta.name,
        badge_str,
        ISLAMIC_OPENING_ES,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        install_cmd,
        usage_cmd,
        test_cmd,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    MultilingualReadme {
        tr,
        en,
        ar,
        ja,
        zh,
        ru,
        es,
    }
}

pub fn generate_multilingual_readme(meta: &ProjectMeta) -> MultilingualReadme {
    generate_yolbulucu_readme(meta)
}

fn replace_section_content(
    readme: &str,
    header_keywords: &[&str],
    new_content: &str,
) -> Result<String, MultilingualError> {
    let lines: Vec<&str> = readme.lines().collect();
    let mut header_index = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            for kw in header_keywords {
                if trimmed.contains(kw) {
                    header_index = Some(i);
                    break;
                }
            }
        }
        if header_index.is_some() {
            break;
        }
    }

    let start_line = match header_index {
        Some(idx) => idx,
        None => {
            return Err(MultilingualError::SectionNotFound(
                header_keywords.first().unwrap_or(&"unknown").to_string(),
            ))
        }
    };

    let header_line = lines[start_line].trim();
    let header_level = header_line.chars().take_while(|c| *c == '#').count();

    let mut end_line = lines.len();
    for (i, ham) in lines.iter().enumerate().skip(start_line + 1) {
        let line = ham.trim();
        if line.starts_with("---") {
            end_line = i;
            break;
        }
        if line.starts_with('#') {
            let level = line.chars().take_while(|c| *c == '#').count();
            if level <= header_level {
                end_line = i;
                break;
            }
        }
    }

    let mut result = String::new();
    for line in lines.iter().take(start_line + 1) {
        result.push_str(line);
        result.push('\n');
    }

    result.push_str(new_content.trim());
    result.push('\n');

    if end_line < lines.len() {
        for i in end_line..lines.len() {
            result.push_str(lines[i]);
            if i + 1 < lines.len() || readme.ends_with('\n') {
                result.push('\n');
            }
        }
    }

    Ok(result)
}

pub fn update_dynamic_sections(
    current_readme: &str,
    updates: &ReadmeUpdates,
) -> Result<String, MultilingualError> {
    if current_readme.trim().is_empty() {
        return Err(MultilingualError::EmptyContent);
    }

    let mut updated = current_readme.to_string();

    if let Some(ref features_content) = updates.features {
        let keywords = [
            "Ne Nedir ve Özellikler",
            "Description and Features",
            "الوصف والميزات",
            "概要と特徴",
            "描述与特性",
            "Описание и возможности",
            "Descripción y Características",
            "Özellikler",
            "Features",
        ];
        updated = replace_section_content(&updated, &keywords, features_content)?;
    }

    if let Some(ref roadmap_items) = updates.roadmap {
        let keywords = [
            "Yol Haritası",
            "Roadmap",
            "خريطة الطريق",
            "ロードマップ",
            "路线图",
            "Дорожная карта",
            "Hoja de ruta",
        ];
        let formatted_roadmap = roadmap_items
            .iter()
            .map(|r| {
                let mark = if r.completed { "x" } else { " " };
                format!("- [{}] {}", mark, r.title)
            })
            .collect::<Vec<_>>()
            .join("\n");
        updated = replace_section_content(&updated, &keywords, &formatted_roadmap)?;
    }

    if let Some(ref release_content) = updates.release_updates {
        let keywords = [
            "Sürüm Güncellemeleri",
            "Release Updates",
            "تحديثات الإصدار",
            "リリース更新",
            "发布更新",
            "Обновления выпусков",
            "Actualizaciones de lanzamientos",
        ];
        updated = replace_section_content(&updated, &keywords, release_content)?;
    }

    Ok(updated)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn create_test_meta() -> ProjectMeta {
        ProjectMeta {
            name: "el-Fihrist".into(),
            description: "Yapay zekâ ajanları için yetenek kütüphanesi.".into(),
            license: "MIT".into(),
            version: "0.2.0".into(),
            author: "Ercan ER".into(),
            author_github: Some("Ercaner1988".into()),
            author_twitter: Some("ercan_er".into()),
            author_linkedin: Some("ercaner".into()),
            author_website: Some("https://fihrist.yerel".into()),
            repository_url: Some("https://github.com/Ercaner1988/el-fihrist".into()),
            documentation_url: Some("https://docs.yerel/fihrist".into()),
            homepage_url: Some("https://fihrist.yerel".into()),
            issues_url: Some("https://github.com/Ercaner1988/el-fihrist/issues".into()),
            contributing_url: Some(
                "https://github.com/Ercaner1988/el-fihrist/blob/main/CONTRIBUTING.md".into(),
            ),
            install_command: Some("cargo add el-fihrist".into()),
            usage_command: Some("cargo run -p ibnunnedim-cli".into()),
            test_command: Some("cargo test -p hermes-tools-core".into()),
            badges: vec![BadgeInfo::new("Rust", "100%", "orange")],
            co_authors: vec![
                "İbnünnedîm (hermes-agent)".into(),
                "Mihenk (Claude Opus 5)".into(),
                "Cezeri (Demirci)".into(),
            ],
            modules: vec![
                ModuleInfo {
                    name: "citation".into(),
                    description: "Atıf doğrulama.".into(),
                },
                ModuleInfo {
                    name: "codebase".into(),
                    description: "Kod tabanı analizi.".into(),
                },
            ],
            roadmap: vec![
                RoadmapItem {
                    completed: true,
                    title: "Saf Rust BM25 Arama".into(),
                },
                RoadmapItem {
                    completed: false,
                    title: "Semantik Sınıflandırma".into(),
                },
            ],
        }
    }

    #[test]
    fn badge_kacis_ve_url_testi() {
        let badge = BadgeInfo::new("Maintained?", "yes", "brightgreen");
        let url = badge.to_shields_url();
        assert_eq!(
            url,
            "https://img.shields.io/badge/Maintained?-yes-brightgreen.svg"
        );

        let badge_with_dash = BadgeInfo::new("my-crate", "v1.0.0-beta_1", "blue");
        let clean_label = escape_badge_text(&badge_with_dash.label);
        let clean_msg = escape_badge_text(&badge_with_dash.message);
        assert_eq!(clean_label, "my--crate");
        assert_eq!(clean_msg, "v1.0.0--beta__1");
    }

    #[test]
    fn cargo_toml_ayristirma_testi() {
        let toml = r#"
[package]
name = "ornek-paket"
version = "1.2.3"
description = "Harika bir Rust kütüphanesi"
license = "Apache-2.0"
authors = ["Test Geliştirici <test@ornek.com>"]
repository = "https://github.com/ornek/ornek-paket"
"#;
        let meta = parse_cargo_toml_content(toml);
        assert_eq!(meta.name, "ornek-paket");
        assert_eq!(meta.version, "1.2.3");
        assert_eq!(meta.description, "Harika bir Rust kütüphanesi");
        assert_eq!(meta.license, "Apache-2.0");
        assert_eq!(meta.author, "Test Geliştirici <test@ornek.com>");
        assert_eq!(
            meta.install_command,
            Some("cargo add ornek-paket".to_string())
        );
    }

    #[test]
    fn package_json_ayristirma_testi() {
        let json = r#"{
  "name": "ornek-js",
  "version": "2.0.0",
  "description": "JS Paketi",
  "license": "MIT",
  "author": {
    "name": "JS Uzmanı",
    "url": "https://js.ornek.com"
  },
  "scripts": {
    "test": "jest",
    "start": "node index.js"
  }
}"#;
        let meta = parse_package_json_content(json).unwrap();
        assert_eq!(meta.name, "ornek-js");
        assert_eq!(meta.version, "2.0.0");
        assert_eq!(meta.author, "JS Uzmanı");
        assert_eq!(
            meta.author_website,
            Some("https://js.ornek.com".to_string())
        );
        assert_eq!(meta.install_command, Some("npm install".to_string()));
        assert_eq!(meta.test_command, Some("npm test".to_string()));
        assert_eq!(meta.usage_command, Some("npm start".to_string()));
    }

    #[test]
    fn git_config_ayristirma_testi() {
        let config = r#"
[core]
	repositoryformatversion = 0
[remote "origin"]
	url = git@github.com:Ercaner1988/el-Fihrist.git
	fetch = +refs/heads/*:refs/remotes/origin/*
"#;
        let (url, repo) = parse_git_config_content(config).unwrap();
        assert_eq!(url, "https://github.com/Ercaner1988/el-Fihrist");
        assert_eq!(repo, "Ercaner1988/el-Fihrist");
    }

    #[test]
    fn islami_ovgu_ve_banner_testi_tum_diller() {
        let meta = create_test_meta();
        let readme = generate_yolbulucu_readme(&meta);

        // Türkçe
        assert!(readme.tr.contains(ISLAMIC_OPENING_TR));
        assert!(readme.tr.contains(LANGUAGE_BANNER));
        assert!(readme.tr.contains("cargo add el-fihrist"));
        assert!(readme.tr.contains("👤 **Ercan ER**"));

        // İngilizce
        assert!(readme.en.contains(ISLAMIC_OPENING_EN));
        assert!(readme.en.contains(LANGUAGE_BANNER));
        assert!(readme.en.contains("cargo add el-fihrist"));

        // Arapça
        assert!(readme.ar.contains(ISLAMIC_OPENING_AR));
        assert!(readme.ar.contains(LANGUAGE_BANNER));

        // Japonca
        assert!(readme.ja.contains(ISLAMIC_OPENING_JA));
        assert!(readme.ja.contains(LANGUAGE_BANNER));

        // Çince
        assert!(readme.zh.contains(ISLAMIC_OPENING_ZH));
        assert!(readme.zh.contains(LANGUAGE_BANNER));

        // Rusça
        assert!(readme.ru.contains(ISLAMIC_OPENING_RU));
        assert!(readme.ru.contains(LANGUAGE_BANNER));

        // İspanyolca
        assert!(readme.es.contains(ISLAMIC_OPENING_ES));
        assert!(readme.es.contains(LANGUAGE_BANNER));
    }

    #[test]
    fn statik_alan_koruma_testi() {
        let meta = create_test_meta();
        let original_readme = generate_yolbulucu_readme(&meta).tr;

        let updates = ReadmeUpdates {
            features: Some("Yeni güncellenmiş özellikler dizisi.".into()),
            roadmap: Some(vec![RoadmapItem {
                completed: true,
                title: "Yeni Yol Haritası Maddesi".into(),
            }]),
            release_updates: Some("* **v0.3.0:** Yenilikler.".into()),
        };

        let updated_readme = update_dynamic_sections(&original_readme, &updates).unwrap();

        // Statik alanlar değişmemeli
        assert!(updated_readme.contains(ISLAMIC_OPENING_TR));
        assert!(updated_readme.contains(LANGUAGE_BANNER));
        assert!(updated_readme.contains("#### 📜 Lisans"));
        assert!(updated_readme.contains("Bu proje [MIT] Lisansı ile lisanslanmıştır."));
        assert!(updated_readme.contains("#### ⚙️ Gereksinimler ve Kurulum"));
        assert!(updated_readme.contains("Ercan ER"));

        // Dinamik alanlar güncellenmiş olmalı
        assert!(updated_readme.contains("Yeni güncellenmiş özellikler dizisi."));
        assert!(updated_readme.contains("- [x] Yeni Yol Haritası Maddesi"));
        assert!(updated_readme.contains("* **v0.3.0:** Yenilikler."));
    }

    #[test]
    fn bos_icerik_hatasi() {
        let updates = ReadmeUpdates::default();
        let res = update_dynamic_sections("", &updates);
        assert_eq!(res, Err(MultilingualError::EmptyContent));
    }
}
