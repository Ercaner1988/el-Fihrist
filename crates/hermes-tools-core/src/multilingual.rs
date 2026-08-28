use serde::{Deserialize, Serialize};
use std::fmt;

pub const ISLAMIC_OPENING_TR: &str = "Bismillahirrahmanirrahim. Rahmân ve Rahîm olan Allah'ın adıyla. Hamd âlemlerin Rabbine, salât ve selâm O'nun Resûlü'ne olsun. Bu proje adını ve ruhunu, 10. yüzyılda Bağdat'ta yaşamış büyük bibliyograf Ebü’l-Ferec Muhammed b. İshâk en-Nedîm ve onun ölümsüz eseri *el-Fihrist*'ten almaktadır. Medeniyetimizin ilk kütüphanecisinin mirasıyla; yapay zekâ ajanları için saf Rust ve Turso SQLite tabanlı yetenek, kod ve hafıza kütüphanesini inşa ettik.";

pub const ISLAMIC_OPENING_EN: &str = "In the name of Allah, the Most Gracious, the Most Merciful. Praise be to the Lord of the worlds, and peace and blessings be upon His Messenger. This project takes its name and spirit from the great bibliographer Abu al-Faraj Muhammad b. Ishaq al-Nadim, who lived in Baghdad in the 10th century, and his immortal work *al-Fihrist*. Inheriting the legacy of our civilization's first librarian, we have built a pure Rust and Turso SQLite-based skill, code, and memory library for AI agents.";

pub const ISLAMIC_OPENING_AR: &str = "بسم الله الرحمن الرحيم. الحمد لله رب العالمين، والصلاة والسلام على رسوله. يستمد هذا المشروع اسمه وروحه من الببليوغرافي العظيم أبو الفرج محمد بن إسحاق النديم الذي عاش في بغداد في القرن العاشر، ومن عمله الخالد *الفهرست*. استلهاماً من تراث أول أمين مكتبة في حضارتنا؛ قمنا ببناء مكتبة للمهارات، والرموز البرمجية، والذاكرة مخصصة للوكلاء الأذكياء (AI agents)، تعتمد على لغة Rust الخالصة وقاعدة بيانات Turso SQLite.";

pub const ISLAMIC_OPENING_JA: &str = "慈悲あまねく慈愛深きアッラーの御名において。万物の主なるアッラーに讃えあれ。その使徒に平安と祝福がありますように。本プロジェクトは、10世紀にバグダードで生きた偉大な書誌学者アブー・アル＝ファラジ・ムハンマド・イブン・イスハーク・アン＝ナディームと、彼の不朽の著作である『アル＝フィフリスト』から名と精神を受け継いでいる。我々の文明の最初の司書の遺産を継承し、AIエージェントのための純粋なRustとTurso SQLiteベースのスキル、コード、およびメモリライブラリを構築した。";

pub const ISLAMIC_OPENING_ZH: &str = "奉至仁至慈的安拉之名。赞美归于全世界的主，愿和平与祝福降临于祂的使者。本项目的名称和精神源自10世纪生活在巴格达的伟大书目学家阿布·法拉吉·穆罕默德·本·伊沙克·纳迪姆（Abu al-Faraj Muhammad b. Ishaq al-Nadim）及其不朽著作《书目》（*al-Fihrist*）。传承我们文明中第一位图书馆员的遗产，我们为AI智能体构建了一个纯Rust和基于Turso SQLite的技能、代码与内存库。";

pub const ISLAMIC_OPENING_RU: &str = "Во имя Аллаха, Милостивого, Милосердного. Хвала Господу миров, мир и благословение Его Посланнику. Этот проект берет свое имя и дух от великого библиографа Абу аль-Фараджа Мухаммада ибн Исхака ан-Надима, жившего в Багдаде в 10 веке, и его бессмертного труда *аль-Фихрист* (al-Fihrist). Наследуя опыт первого библиотекаря нашей цивилизации, мы создали библиотеку навыков, кода и памяти для ИИ-агентов на базе чистого Rust и Turso SQLite.";

pub const ISLAMIC_OPENING_ES: &str = "En nombre de Dios, el Más Clemente, el Más Misericordioso. Alabado sea el Señor de los mundos, y que la paz y las bendiciones sean con Su Mensajero. Este proyecto toma su nombre y espíritu del gran bibliógrafo Abu al-Faraj Muhammad b. Ishaq al-Nadim, quien vivió en Bagdad en el siglo X, y de su obra inmortal *al-Fihrist*. Heredando el legado del primer bibliotecario de nuestra civilización, hemos construido una biblioteca de habilidades, códigos y memoria basada en Rust puro y Turso SQLite para agentes de IA.";

pub const LANGUAGE_BANNER: &str = "🇹🇷 [Türkçe](#-türkçe) | 🇬🇧 [English](#-english) | 🇸🇦 [العربية](#-العربية) | 🇯🇵 [日本語](#-日本語) | 🇨🇳 [中文](#-中文) | 🇷🇺 [Русский](#-русский) | 🇪🇸 [Español](#-español)";

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
    pub co_authors: Vec<String>,
    pub modules: Vec<ModuleInfo>,
    pub roadmap: Vec<RoadmapItem>,
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
            MultilingualError::SectionNotFound(sec) => write!(f, "Section not found: {}", sec),
            MultilingualError::EmptyContent => write!(f, "Content is empty"),
            MultilingualError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
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
        let mut list = vec![format!("* **{}** *(Proje Sahibi)*", author)];
        for ca in co_authors {
            list.push(format!("* **{}**", ca));
        }
        list.join("\n")
    };

    let tr = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇹🇷 Türkçe\n\n#### 📌 İçindekiler\n- [📖 Ne Nedir ve Özellikler](#-ne-nedir-ve-özellikler)\n    - [🧰 Mevcut Yetenekler ve Sistem Modülleri](#-mevcut-yetenekler-ve-sistem-modülleri)\n- [⚙️ Gereksinimler ve Kurulum](#️-gereksinimler-ve-kurulum)\n- [🗺️ Yol Haritası](#️-yol-haritası)\n- [📝 Sürüm Güncellemeleri](#-sürüm-güncellemeleri)\n- [👥 Katkıda Bulunanlar (Co-Authors)](#-katkıda-bulunanlar-co-authors)\n- [📜 Lisans](#-lisans)\n\n---\n\n#### 📖 Ne Nedir ve Özellikler\n{}\n\n##### 🧰 Mevcut Yetenekler ve Sistem Modülleri\n{}\n\n---\n\n#### ⚙️ Gereksinimler ve Kurulum\nSürüm: `{}` | Lisans: `{}` | Yazar: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ Yol Haritası\n{}\n\n---\n\n#### 📝 Sürüm Güncellemeleri\n* **v{} (Mevcut):** İlk kararlı sürüm.\n\n---\n\n#### 👥 Katkıda Bulunanlar (Co-Authors)\n{}\n\n---\n\n#### 📜 Lisans\nBu proje [{}] Lisansı ile lisanslanmıştır.\n",
        meta.name,
        ISLAMIC_OPENING_TR,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let en = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇬🇧 English\n\n#### 📌 Table of Contents\n- [📖 Description and Features](#-description-and-features)\n    - [🧰 Available Skills and System Modules](#-available-skills-and-system-modules)\n- [⚙️ Requirements and Installation](#️-requirements-and-installation)\n- [🗺️ Roadmap](#️-roadmap)\n- [📝 Release Updates](#-release-updates)\n- [👥 Contributors (Co-Authors)](#-contributors-co-authors)\n- [📜 License](#-license)\n\n---\n\n#### 📖 Description and Features\n{}\n\n##### 🧰 Available Skills and System Modules\n{}\n\n---\n\n#### ⚙️ Requirements and Installation\nVersion: `{}` | License: `{}` | Author: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ Roadmap\n{}\n\n---\n\n#### 📝 Release Updates\n* **v{} (Current):** Initial stable release.\n\n---\n\n#### 👥 Contributors (Co-Authors)\n{}\n\n---\n\n#### 📜 License\nThis project is licensed under the [{}] License.\n",
        meta.name,
        ISLAMIC_OPENING_EN,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let ar = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇸🇦 العربية\n\n#### 📌 جدول المحتويات\n- [📖 الوصف والميزات](#-الوصف-والميزات)\n    - [🧰 المهارات المتاحة ووحدات النظام](#-المهارات-المتاحة-ووحدات-النظام)\n- [⚙️ المتطلبات والتثبيت](#️-المتطلبات-والتثبيت)\n- [🗺️ خريطة الطريق](#️-خريطة-الطريق)\n- [📝 تحديثات الإصدار](#-تحديثات-الإصدار)\n- [👥 المساهمون (Co-Authors)](#-المساهمون-co-authors)\n- [📜 الترخيص](#-الترخيص)\n\n---\n\n#### 📖 الوصف والميزات\n{}\n\n##### 🧰 المهارات المتاحة ووحدات النظام\n{}\n\n---\n\n#### ⚙️ المتطلبات والتثبيت\nالإصدار: `{}` | الترخيص: `{}` | المؤلف: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ خريطة الطريق\n{}\n\n---\n\n#### 📝 تحديثات الإصدار\n* **v{} (الحالي):** الإصدار المستقر الأولي.\n\n---\n\n#### 👥 المساهمون (Co-Authors)\n{}\n\n---\n\n#### 📜 الترخيص\nهذا المشروع مرخص بموجب ترخيص [{}].\n",
        meta.name,
        ISLAMIC_OPENING_AR,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let ja = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇯🇵 日本語\n\n#### 📌 目次\n- [📖 概要と特徴](#-概要と特徴)\n    - [🧰 利用可能なスキルとシステムモジュール](#-利用可能なスキルとシステムモジュール)\n- [⚙️ 要件とインストール](#️-要件とインストール)\n- [🗺️ ロードマップ](#️-ロードマップ)\n- [📝 リリース更新](#-リリース更新)\n- [👥 貢献者 (Co-Authors)](#-貢献者-co-authors)\n- [📜 ライセンス](#-ライセンス)\n\n---\n\n#### 📖 概要と特徴\n{}\n\n##### 🧰 利用可能なスキルとシステムモジュール\n{}\n\n---\n\n#### ⚙️ 要件とインストール\nバージョン: `{}` | ライセンス: `{}` | 著者: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ ロードマップ\n{}\n\n---\n\n#### 📝 リリース更新\n* **v{} (現在):** 最初の安定版リリース。\n\n---\n\n#### 👥 貢献者 (Co-Authors)\n{}\n\n---\n\n#### 📜 ライセンス\nこのプロジェクトは [{}] ライセンスのもとで公開されています。\n",
        meta.name,
        ISLAMIC_OPENING_JA,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let zh = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇨🇳 中文\n\n#### 📌 目录\n- [📖 描述与特性](#-描述与特性)\n    - [🧰 可用技能与系统模块](#-可用技能与系统模块)\n- [⚙️ 要求与安装](#️-要求与安装)\n- [🗺️ 路线图](#️-路线图)\n- [📝 发布更新](#-发布更新)\n- [👥 贡献者 (Co-Authors)](#-贡献者-co-authors)\n- [📜 许可证](#-许可证)\n\n---\n\n#### 📖 描述与特性\n{}\n\n##### 🧰 可用技能与系统模块\n{}\n\n---\n\n#### ⚙️ 要求与安装\n版本: `{}` | 许可证: `{}` | 作者: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ 路线图\n{}\n\n---\n\n#### 📝 发布更新\n* **v{} (当前):** 初始稳定版本。\n\n---\n\n#### 👥 贡献者 (Co-Authors)\n{}\n\n---\n\n#### 📜 许可证\n本项目采用 [{}] 许可证。\n",
        meta.name,
        ISLAMIC_OPENING_ZH,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let ru = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇷🇺 Русский\n\n#### 📌 Содержание\n- [📖 Описание и возможности](#-описание-и-возможности)\n    - [🧰 Доступные навыки и системные модули](#-доступные-навыки-и-системные-модули)\n- [⚙️ Требования и установка](#️-требования-и-установка)\n- [🗺️ Дорожная карта](#️-дорожная-карта)\n- [📝 Обновления выпусков](#-обновления-выпусков)\n- [👥 Соавторы (Co-Authors)](#-соавторы-co-authors)\n- [📜 Лицензия](#-лицензия)\n\n---\n\n#### 📖 Описание и возможности\n{}\n\n##### 🧰 Доступные навыки и системные модули\n{}\n\n---\n\n#### ⚙️ Требования и установка\nВерсия: `{}` | Лицензия: `{}` | Автор: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ Дорожная карта\n{}\n\n---\n\n#### 📝 Обновления выпусков\n* **v{} (Текущая):** Первый стабильный релиз.\n\n---\n\n#### 👥 Соавторы (Co-Authors)\n{}\n\n---\n\n#### 📜 Лицензия\nЭтот проект распространяется под лицензией [{}].\n",
        meta.name,
        ISLAMIC_OPENING_RU,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
        format_roadmap(&meta.roadmap),
        meta.version,
        format_co_authors(&meta.author, &meta.co_authors),
        meta.license
    );

    let es = format!(
        "# {}\n\n> {}\n\n---\n\n## 🌍 Dil Seçenekleri / Languages\n{}\n\n---\n\n### 🇪🇸 Español\n\n#### 📌 Tabla de contenidos\n- [📖 Descripción y Características](#-descripción-y-características)\n    - [🧰 Habilidades Disponibles y Módulos del Sistema](#-habilidades-disponibles-y-módulos-del-sistema)\n- [⚙️ Requisitos e Instalación](#️-requisitos-e-instalación)\n- [🗺️ Hoja de ruta](#️-hoja-de-ruta)\n- [📝 Actualizaciones de lanzamientos](#-actualizaciones-de-lanzamientos)\n- [👥 Colaboradores (Co-Authors)](#-colaboradores-co-authors)\n- [📜 Licencia](#-licencia)\n\n---\n\n#### 📖 Descripción y Características\n{}\n\n##### 🧰 Habilidades Disponibles y Módulos del Sistema\n{}\n\n---\n\n#### ⚙️ Requisitos e Instalación\nVersión: `{}` | Licencia: `{}` | Autor: `{}`\n```bash\ncargo add {}\n```\n\n---\n\n#### 🗺️ Hoja de ruta\n{}\n\n---\n\n#### 📝 Actualizaciones de lanzamientos\n* **v{} (Actual):** Primera versión estable.\n\n---\n\n#### 👥 Colaboradores (Co-Authors)\n{}\n\n---\n\n#### 📜 Licencia\nEste proyecto está licenciado bajo la Licencia [{}].\n",
        meta.name,
        ISLAMIC_OPENING_ES,
        LANGUAGE_BANNER,
        meta.description,
        format_modules(&meta.modules),
        meta.version,
        meta.license,
        meta.author,
        meta.name,
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
    for i in (start_line + 1)..lines.len() {
        let line = lines[i].trim();
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
    for i in 0..=start_line {
        result.push_str(lines[i]);
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
    fn islami_ovgu_ve_banner_testi_tum_diller() {
        let meta = create_test_meta();
        let readme = generate_yolbulucu_readme(&meta);

        // Türkçe
        assert!(readme.tr.contains(ISLAMIC_OPENING_TR));
        assert!(readme.tr.contains(LANGUAGE_BANNER));

        // İngilizce
        assert!(readme.en.contains(ISLAMIC_OPENING_EN));
        assert!(readme.en.contains(LANGUAGE_BANNER));

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
