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
    pub de: String,
    pub fr: String,
}

pub fn generate_multilingual_readme(meta: &ProjectMeta) -> MultilingualReadme {
    let tr = format!(
        "# {}\n\n> {}\n\nSürüm: `{}` | Lisans: `{}` | Yazar: `{}`\n\n## Kurulum\n```bash\ncargo add {}\n```\n\n## Kullanım\nProjenize ekleyin ve çalıştırmaya başlayın.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let en = format!(
        "# {}\n\n> {}\n\nVersion: `{}` | License: `{}` | Author: `{}`\n\n## Installation\n```bash\ncargo add {}\n```\n\n## Usage\nAdd to your project and start executing.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let de = format!(
        "# {}\n\n> {}\n\nVersion: `{}` | Lizenz: `{}` | Autor: `{}`\n\n## Installation\n```bash\ncargo add {}\n```\n\n## Verwendung\nFügen Sie es Ihrem Projekt hinzu und starten Sie.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    let fr = format!(
        "# {}\n\n> {}\n\nVersion: `{}` | Licence: `{}` | Auteur: `{}`\n\n## Installation\n```bash\ncargo add {}\n```\n\n## Utilisation\nAjoutez à votre projet et commencez l'exécution.\n",
        meta.name, meta.description, meta.version, meta.license, meta.author, meta.name
    );

    MultilingualReadme { tr, en, de, fr }
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
        assert!(readme.de.contains("Lizenz: `MIT`"));
        assert!(readme.fr.contains("Licence: `MIT`"));
    }
}
