//! Stdio MCP sunucusu (JSON-RPC 2.0, satır bazlı).
//!
//! ÖNCESİ: `plugin.json` `ibnunnedim`'i `args: ["search","rust"]` ile kaydediyordu.
//! O bir MCP sunucusu değil; tek sabit sorgu koşup çıkan bir komut. İstemci
//! `initialize` gönderiyor, karşılığında arama çıktısı görüyor ve el sıkışma
//! düşüyordu — yani el-Fihrist hiçbir ajanın aracı olarak kullanılamıyordu.
//!
//! KURAL: stdout YALNIZ protokole aittir. Tanı çıktısı stderr'e gider; tek bir
//! kaçak `println!` el sıkışmayı bozar.

use crate::{fts_indeksleri, kutuphane_yolu, list_all_skills, say, search_skills, tekmil_ver};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use turso::Connection;

/// Konuştuğumuz MCP sürümü. İstemci başkasını isterse kendi sürümümüzü
/// bildiririz; uyumu istemci karara bağlar (spec'in öngördüğü davranış).
const PROTOKOL: &str = "2024-11-05";

/// Gelen bir satırın protokol karşılığı.
///
/// `Bildirim`in ayrı bir varyant olmasının sebebi: JSON-RPC'de `id` taşımayan
/// istek bir *bildirimdir* ve ona yanıt yazmak protokol ihlalidir. Bunu
/// `Cagri` ile aynı yolda ele alırsak sessizce fazladan yanıt üretiriz.
#[derive(Debug, PartialEq)]
pub enum Istek {
    Cagri {
        id: Value,
        yontem: String,
        parametre: Value,
    },
    Bildirim,
    Bozuk(String),
}

/// Satırı ayrıştırır. Saf: ne stdio ne DB gerekir, sınanabilir.
pub fn ayristir(satir: &str) -> Istek {
    let v: Value = match serde_json::from_str(satir) {
        Ok(v) => v,
        Err(e) => return Istek::Bozuk(e.to_string()),
    };
    let yontem = match v.get("method").and_then(Value::as_str) {
        Some(m) => m.to_string(),
        None => return Istek::Bozuk("method alanı yok".into()),
    };
    match v.get("id") {
        // null id de bildirim sayılır; spec id'yi "yok ya da anlamlı" diye ayırır.
        None | Some(Value::Null) => Istek::Bildirim,
        Some(id) => Istek::Cagri {
            id: id.clone(),
            yontem,
            parametre: v.get("params").cloned().unwrap_or_else(|| json!({})),
        },
    }
}

pub fn yanit(id: &Value, sonuc: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": sonuc})
}

pub fn hata(id: &Value, kod: i64, mesaj: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": kod, "message": mesaj}})
}

/// Açılan araçlar. `plugin.json`'daki `hermes.tools` listesiyle aynı adlar.
pub fn araclar() -> Value {
    json!([
        {
            "name": "search_skills",
            "description": "Yetenek kütüphanesinde BM25 ile arama yapar. Türkçe katlamalı (ı ş ğ ü ö ç). Sonuç: id, ad, kategori, başarı puanı, açıklama.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Aranacak metin"},
                    "limit": {"type": "integer", "description": "Azami sonuç sayısı (varsayılan 10)"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "list_skills",
            "description": "Yetenekleri listeler; istenirse tek kategoriyle sınırlar.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kategori": {"type": "string", "description": "Süzülecek kategori; boşsa tümü"}
                }
            }
        },
        {
            "name": "info",
            "description": "Kütüphane istatistiği: çözülen veritabanı yolu ve tablo sayıları.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "tekmil_ver",
            "description": "Bir yeteneğe 0-100 arası tekmil puanı yazar ve ortalamayı günceller. YAZMA işlemidir.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ajan": {"type": "string"},
                    "yetenek": {"type": "string", "description": "Yetenek id (slug)"},
                    "puan": {"type": "number", "description": "0-100"},
                    "gerekce": {"type": "string"},
                    "hafta": {"type": "integer", "description": "YYYYWW (varsayılan 202634)"}
                },
                "required": ["ajan", "yetenek", "puan", "gerekce"]
            }
        }
    ])
}

fn metin(p: &Value, ad: &str) -> std::result::Result<String, String> {
    p.get(ad)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("'{ad}' alanı gerekli (metin)"))
}

/// Aracı koşturur. Hata metni çağırana `isError` ile döner — süreç düşmez;
/// bir aracın patlaması oturumu bitirmemeli.
async fn arac_calistir(
    conn: &Connection,
    ad: &str,
    p: &Value,
) -> std::result::Result<String, String> {
    match ad {
        "search_skills" => {
            let q = metin(p, "query")?;
            let limit = p.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
            let (skills, toplam, sure) = search_skills(conn, &q, limit)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "{} / {toplam} kayıt · {:.1} ms\n{}",
                skills.len(),
                sure.as_secs_f64() * 1000.0,
                serde_json::to_string_pretty(&skills).map_err(|e| e.to_string())?
            ))
        }
        "list_skills" => {
            let kat = p.get("kategori").and_then(Value::as_str);
            let skills = list_all_skills(conn, kat)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&skills).map_err(|e| e.to_string())
        }
        "info" => {
            let yol = kutuphane_yolu().unwrap_or_else(std::path::PathBuf::from);
            let yetenek = say(conn, "yetenekler").await.map_err(|e| e.to_string())?;
            let tekmil = say(conn, "ajan_tekmilleri")
                .await
                .map_err(|e| e.to_string())?;
            let kod = say(conn, "kod_hazinesi").await.map_err(|e| e.to_string())?;
            let fts = fts_indeksleri(conn).await.map_err(|e| e.to_string())?;
            Ok(format!(
                "veritabanı: {}\nyetenek: {yetenek}\ntekmil: {tekmil}\nkod hazinesi: {kod}\n\
                 fts5 tabloları (tanımlı, kullanılmıyor): {}",
                yol.display(),
                if fts.is_empty() {
                    "yok".to_string()
                } else {
                    fts.join(", ")
                }
            ))
        }
        "tekmil_ver" => {
            let ajan = metin(p, "ajan")?;
            let yetenek = metin(p, "yetenek")?;
            let gerekce = metin(p, "gerekce")?;
            let puan = p
                .get("puan")
                .and_then(Value::as_f64)
                .ok_or("'puan' alanı gerekli (sayı)")?;
            let hafta = p.get("hafta").and_then(Value::as_i64).unwrap_or(202634);
            tekmil_ver(conn, hafta, &ajan, &yetenek, puan, &gerekce)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "tekmil kaydedildi: {ajan} → {yetenek} ({puan}, hafta {hafta})"
            ))
        }
        _ => Err(format!("bilinmeyen araç: {ad}")),
    }
}

/// Bir isteği yanıta çevirir. `None` dönerse hiçbir şey yazılmaz.
async fn ele_al(conn: &Connection, istek: Istek) -> Option<Value> {
    match istek {
        Istek::Bildirim => None,
        Istek::Bozuk(e) => Some(hata(&Value::Null, -32700, &format!("ayrıştırılamadı: {e}"))),
        Istek::Cagri {
            id,
            yontem,
            parametre,
        } => Some(match yontem.as_str() {
            "initialize" => yanit(
                &id,
                json!({
                    "protocolVersion": PROTOKOL,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "el-fihrist", "version": env!("CARGO_PKG_VERSION")}
                }),
            ),
            "ping" => yanit(&id, json!({})),
            "tools/list" => yanit(&id, json!({"tools": araclar()})),
            "tools/call" => {
                let ad = parametre
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let arg = parametre
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match arac_calistir(conn, &ad, &arg).await {
                    Ok(t) => yanit(&id, json!({"content": [{"type": "text", "text": t}]})),
                    Err(e) => yanit(
                        &id,
                        json!({"content": [{"type": "text", "text": e}], "isError": true}),
                    ),
                }
            }
            _ => hata(&id, -32601, &format!("bilinmeyen yöntem: {yontem}")),
        }),
    }
}

/// stdin'den satır okur, stdout'a yanıt yazar. EOF'ta biter.
pub async fn calistir(conn: &Connection) -> crate::Result<()> {
    let mut girdi = BufReader::new(tokio::io::stdin()).lines();
    let mut cikti = tokio::io::stdout();
    eprintln!("el-fihrist MCP: hazır (protokol {PROTOKOL})");

    while let Some(satir) = girdi.next_line().await? {
        if satir.trim().is_empty() {
            continue;
        }
        if let Some(y) = ele_al(conn, ayristir(&satir)).await {
            cikti.write_all(y.to_string().as_bytes()).await?;
            cikti.write_all(b"\n").await?;
            cikti.flush().await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    /// SESSİZCE BOZULAN SÖZLEŞME: bildirime yanıt yazmak protokol ihlalidir.
    /// `notifications/initialized` her oturumda gelir; ona yanıt yazan sunucu
    /// istemcide "beklenmeyen yanıt" hatası doğurur ve el sıkışma düşer.
    #[test]
    fn bildirim_yanit_almaz() {
        assert_eq!(
            ayristir(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
            Istek::Bildirim
        );
        // null id de bildirimdir.
        assert_eq!(
            ayristir(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#),
            Istek::Bildirim
        );
    }

    #[test]
    fn idli_istek_cagridir() {
        match ayristir(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#) {
            Istek::Cagri { id, yontem, .. } => {
                assert_eq!(id, json!(1));
                assert_eq!(yontem, "tools/list");
            }
            baska => panic!("çağrı bekleniyordu: {baska:?}"),
        }
    }

    #[test]
    fn bozuk_json_bozuk_doner() {
        assert!(matches!(ayristir("{bu json degil"), Istek::Bozuk(_)));
        // method'suz geçerli JSON da işlenemez.
        assert!(matches!(ayristir(r#"{"id":1}"#), Istek::Bozuk(_)));
    }

    /// Açılan araç adları `plugin.json`'daki listeyle aynı kalmalı; kayarsa
    /// istemci var olmayan aracı çağırır.
    #[test]
    fn arac_adlari_beklenen_dortlu() {
        let a = araclar();
        let adlar: Vec<&str> = a
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            adlar,
            vec!["search_skills", "list_skills", "info", "tekmil_ver"]
        );
    }

    /// Her aracın şeması olmalı — şemasız araç istemcide sessizce çağrılamaz olur.
    #[test]
    fn her_aracin_girdi_semasi_var() {
        for t in araclar().as_array().unwrap() {
            assert_eq!(
                t["inputSchema"]["type"], "object",
                "şemasız araç: {}",
                t["name"]
            );
        }
    }
}
