use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub description: String,
    pub license: String,
    pub version: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultilingualReadme {
    pub tr: String,
    pub en: String,
    pub ar: String,
    pub ja: String,
    pub zh: String,
    pub ru: String,
    pub es: String,
}

pub fn generate_multilingual_readme(meta: &ProjectMeta) -> MultilingualReadme {
    let tr = format!(
        "# {}\n\n> {}\n\nSürüm: `{}` | Lisans: `{}` | Yazar: `{}`\n\n## Kurulum\n```bash\ncargo add {}\n```\n\n## Kullanım\nProjenize ekleyin ve çalıştırın.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let en = format!(
        "# {}\n\n> {}\n\nVersion: `{}` | License: `{}` | Author: `{}`\n\n## Installation\n```bash\ncargo add {}\n```\n\n## Usage\nAdd to your project and start executing.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let ar = format!(
        "# {}\n\n> {}\n\nالإصدار: `{}` | الترخيص: `{}` | المؤلف: `{}`\n\n## التثبيت\n```bash\ncargo add {}\n```\n\n## الاستخدام\nأضفه إلى مشروعك وابدأ في التشغيل.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let ja = format!(
        "# {}\n\n> {}\n\nバージョン: `{}` | ライセンス: `{}` | 著者: `{}`\n\n## インストール\n```bash\ncargo add {}\n```\n\n## 使い方\nプロジェクトに追加して実行を開始します。\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let zh = format!(
        "# {}\n\n> {}\n\n版本: `{}` | 许可证: `{}` | 作者: `{}`\n\n## 安装\n```bash\ncargo add {}\n```\n\n## 使用方法\n添加到您的项目中并开始运行。\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let ru = format!(
        "# {}\n\n> {}\n\nВерсия: `{}` | Лицензия: `{}` | Автор: `{}`\n\n## Установка\n```bash\ncargo add {}\n```\n\n## Использование\nДобавьте в свой проект и запустите.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let es = format!(
        "# {}\n\n> {}\n\nVersión: `{}` | Licencia: `{}` | Autor: `{}`\n\n## Instalación\n```bash\ncargo add {}\n```\n\n## Uso\nAñádelo a tu proyecto y comienza a ejecutar.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    MultilingualReadme { tr, en, ar, ja, zh, ru, es }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn cok_dilli_readme_uretimi() {
        let meta = ProjectMeta {
            name: "hermes-core".into(),
            description: "Hermes Rust araç kütüphanesi".into(),
            license: "MIT".into(),
            version: "0.1.0".into(),
            author: "Ercan ER".into(),
        };
        let readme = generate_multilingual_readme(&meta);
        assert!(readme.tr.contains("Sürüm: `0.1.0`"));
        assert!(readme.en.contains("Version: `0.1.0`"));
        assert!(readme.ar.contains("الإصدار: `0.1.0`"));
        assert!(readme.ja.contains("バージョン: `0.1.0`"));
        assert!(readme.zh.contains("版本: `0.1.0`"));
        assert!(readme.ru.contains("Версия: `0.1.0`"));
        assert!(readme.es.contains("Versión: `0.1.0`"));
    }
}
