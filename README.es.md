# el-Fihrist (الفهرست)

> En nombre de Dios, el Más Clemente, el Más Misericordioso. Alabado sea el Señor de los mundos, y que la paz y las bendiciones sean con Su Mensajero. Este proyecto toma su nombre y espíritu del gran bibliógrafo Abu al-Faraj Muhammad b. Ishaq al-Nadim, quien vivió en Bagdad en el siglo X, y de su obra inmortal *al-Fihrist*. Heredando el legado del primer bibliotecario de nuestra civilización, hemos construido una biblioteca de habilidades, códigos y memoria basada en Rust puro y Turso SQLite para agentes de IA.

---

## 🌍 Dil Seçenekleri / Languages
🇹🇷 [Türkçe](#-türkçe) | 🇬🇧 [English](#-english) | 🇸🇦 [العربية](#-العربية) | 🇯🇵 [日本語](#-日本語) | 🇨🇳 [中文](#-中文) | 🇷🇺 [Русский](#-русский) | 🇪🇸 [Español](#-español)

---

### 🇪🇸 Español

#### 📌 Índice
- [📖 Descripción y Características](#-descripción-y-características)
    - [🧰 Habilidades Disponibles y Módulos del Sistema](#-habilidades-disponibles-y-módulos-del-sistema)
- [⚙️ Requisitos e Instalación](#️-requisitos-e-instalación)
- [🏗️ Infraestructura y Principio de Funcionamiento](#️-infraestructura-y-principio-de-funcionamiento)
- [🗺️ Hoja de Ruta](#️-hoja-de-ruta)
- [📝 Actualizaciones de Versión](#-actualizaciones-de-versión)
- [👥 Colaboradores (Co-Authors)](#-colaboradores-co-authors)
- [📜 Licencia](#-licencia)

---

#### 📖 Descripción y Características
**el-Fihrist** es un motor de biblioteca de habilidades, fragmentos de código y memoria altamente portátil, diseñado para agentes de IA (Hermes Agent, Claude Desktop MCP, DeepSeek Harness, OpenCode, Codex).

* **Motor de Búsqueda BM25 en Rust Puro:** Motor de búsqueda de relevancia en memoria de alta velocidad con plegado de caracteres turcos (por ejemplo, `İ→i`, `I→ı`, `Ş→ş`).
* **Núcleo Turso / SQLite:** Una base de datos SQLite portátil (`kutup_kutuphane.db`) que contiene más de 140 habilidades externas y herramientas integradas de Rust.
* **Arquitectura de Múltiples Crates:** Una estructura modular de Rust que comprende los módulos CLI, Core, Turso-DSL, React y Servidor TLS.
* **Verificación Estricta (Maşa Döngüsü):** Auditoría segura de códigos e informes a través de las puertas de inspección higiénica D1-D3 y la puerta dorada R2.

##### 🧰 Habilidades Disponibles y Módulos del Sistema
La biblioteca principal (`hermes-tools-core` y el espacio de trabajo) proporciona actualmente 12 habilidades y módulos principales listos para usar:
1. **citation:** Verificación y generación de informes de citas para tesis y textos académicos a través de la infraestructura Zopay (`verify_citations`).
2. **codebase:** Resumen de código fuente, análisis del contenido de archivos y detección de calidad (`analyze_file_content`).
3. **docx:** Análisis de la capa XML de MS Word y lectura de bajo factor de forma (`extract_paragraphs_from_xml`).
4. **extract:** Motor de extracción inteligente y sin ruido para obtener contenido y HTML limpio (`html_ayikla`).
5. **masa_dongusu:** Mecanismos de validación de 6 puertas y de bloqueo autónomo para la transición entre puertas (`validate_masa_dongusu`).
6. **multilingual:** Motor estandarizado para la creación en paralelo de README en varios idiomas y controlador de calidad.
7. **router:** Enrutamiento de subredes de agentes, arquitectura de gráficos semánticos y gestión de rutas/Edge (`RouteResult`).
8. **rules_checker:** Auditor avanzado de estándares de calidad y reglas para el código fuente en Rust (`check_rust_code_rules`).
9. **session:** Módulo de consulta BM25 estructurado histórico para sesiones de comunicación y datos de memoria (`search_session`).
10. **skillopt:** Evolución autónoma de las habilidades del agente; infraestructura de comandos de validación para puertas suaves/estrictas (Soft/Hard gates) con matriz de puntuación.
11. **turso-dsl:** Comandos de definición de esquemas personalizados y dependencias de la capa de abstracción para estructuras Edge SQLite.
12. **hermes-react & hermes-tls:** Módulos de servidor que permiten la interacción asíncrona y segura del agente con sistemas externos.

---

#### ⚙️ Requisitos e Instalación

##### Dependencias y Crates
* **Rust 1.63+** (Edición 2021)
* Crates del Espacio de Trabajo (Workspace): `ibnunnedim-cli`, `crates/hermes-tools-core`, `crates/turso-dsl`, `crates/hermes-react`, `crates/hermes-tls-server`
* Base de Datos del Sistema: `Turso SQLite 0.7.2` (`kutup_kutuphane.db`)

##### Compilación y Ejecución
```bash
# Compilación para el entorno de producción (release)
cargo build --release --workspace

# Búsqueda de habilidades utilizando BM25
./target/release/ibnunnedim search "rust"

# Lista de habilidades
./target/release/ibnunnedim list --kategori "software-development"

# Envío de puntuación de informe (Tekmil)
./target/release/ibnunnedim tekmil --ajan "Kassam" --yetenek "zopay-rust-porting" --puan 100 --gerekce "Passed outer gauntlet"
```

---

#### 🏗️ Infraestructura y Principio de Funcionamiento
* **Indexación BM25:** Todas las habilidades en la base de datos se leen en una sola consulta y se cargan en un índice BM25 en memoria. El texto se trunca de forma segura utilizando la función `kisalt`, que es estable con UTF-8.
* **Integración de Base de Datos:** Establece una conexión SQLite sincronizada a nivel local y remoto, sin copias (zero-copy), utilizando el controlador `turso::Builder::new_local`.
* **Utilidades de Capa:** `tokio` (entorno asíncrono), `clap` (analizador de argumentos de la línea de comandos), `axum` & `tower` (servidores HTTP/REST), `serde_json` (operaciones de manifiesto).

---

#### 🗺️ Hoja de Ruta
- [x] Motor de búsqueda BM25 en Rust puro con plegado de caracteres turcos.
- [x] Integración de Turso SQLite (`kutup_kutuphane.db`) y mecanismo de puntuación Tekmil.
- [ ] Transición a estructuras dinámicas de clasificación de bases de datos/habilidades de índole semántica (heurística, hermenéutica, epistemológica, moral, etc.) para una arquitectura autónoma de ejecución de herramientas ("disparar y olvidar").
- [ ] Construcción de un puente MCP (Model Context Protocol) local de tipo GraphQL/gRPC para agentes.
- [ ] Modificación directa de Turso a través de un motor de suspensión (sleep engine) autónomo del sistema SkillOpt.
- [ ] Infraestructura de indexación de memoria para agentes multi-inquilino (multi-tenant).

---

#### 📝 Actualizaciones de Versión
* **v0.2.0 (Actual):**
  - Añadidas las puertas de inspección higiénica Maşa Döngüsü D1-D3 & R2.
  - Integración de arquitectura híbrida README en 7 idiomas (estándar `readme-yolbulucu`).
  - Adopción de una estructura modular para los componentes principales (p. ej., `codebase`, `docx`, `multilingual`).
* **v0.1.0:** Primer espacio de trabajo en Rust compilable, `ibnunnedim-cli` y una integración básica con la base de datos Turso (estructura inicial).

---

#### 👥 Colaboradores (Co-Authors)
Colaboradores que han participado en la arquitectura y el código del proyecto:
* **Ercan ER** `<ercan.er@medeniyet.edu.tr>` *(Propietario del Proyecto y Arquitecto Principal)*
* **İbnünnedîm (hermes-agent)** `<291384122+hermes-agent@users.noreply.github.com>` *(Agente Bibliotecario)*
* **Mihenk (Claude Opus 5)** `<noreply@anthropic.com>` *(Árbitro de Higiene, Calidad del Código y Auditoría)*
* **Cezeri (Demirci)** *(Monitoreo del Estado de Salud del Software y de Sistemas Autónomos - Arquitecto de Interfaces de Agente)*

---

#### 📜 Licencia
Este proyecto está bajo la [Licencia MIT](LICENSE).
