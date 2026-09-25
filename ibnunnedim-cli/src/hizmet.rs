//! Tek el-Fihrist hizmeti (F2b).
//!
//! Her YZ oturumu kendi stdio MCP sürecini açıyor; turso dosyayı süreç başına
//! kilitlediği için oturumlar açıkken bakım yazıları (`gomme`, vektör onarımı)
//! yapılamıyordu (2026-09-24: 10 süreç, os error 33). Hizmet TEK süreçtir:
//! oturumların `ibnunnedim mcp` süreçleri ona aktarıcı olur.
//!
//! **Protokol yeni değil:** hizmet `127.0.0.1` üzerinde, MCP'nin stdio'da
//! konuştuğu satır bazlı JSON-RPC'yi konuşur (`mcp::konus`, aynı döngü). HTTP
//! kütüphanesi yok. Her TCP bağlantısı bir oturumdur; aktarıcının ilk satırı
//! `el-fihrist/baglam` (oturum kimliği + proje). Bağlantı çift yönlü — F3'te
//! hizmet `tools/list_changed`'ı oturuma bu yoldan iter.
//!
//! Yalnız loopback'e bağlanır. Port `FIHRIST_HIZMET_PORT` (varsayılan 11437).

use crate::olay::{simdi_ms, Baglam};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

pub const VARSAYILAN_PORT: u16 = 11437;

pub fn port() -> u16 {
    std::env::var("FIHRIST_HIZMET_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(VARSAYILAN_PORT)
}

/// `ibnunnedim hizmet`: bağlantıları kabul eder, her birini ayrı görevde konuşur.
/// Port doluysa (başka bir hizmet ayakta) hata verip çıkar — iki hizmet olmaz.
pub async fn calistir() -> crate::Result<()> {
    let dinleyici = TcpListener::bind(("127.0.0.1", port())).await?;
    eprintln!("el-fihrist hizmeti dinliyor: 127.0.0.1:{}", port());
    loop {
        let (akis, _) = dinleyici.accept().await?;
        tokio::spawn(async move {
            let (okur, yazar) = akis.into_split();
            // Aktarıcı bağlamını göndermezse (doğrudan bağlanan istemci) kimlik bağlantıya ait.
            let b = Baglam {
                arayuz: None,
                oturum: format!("uzak-{}", simdi_ms()),
                proje: None,
            };
            if let Err(e) = crate::mcp::konus(BufReader::new(okur), yazar, b).await {
                eprintln!("! hizmet bağlantısı: {e}");
            }
        });
    }
}

/// Hizmete bağlanır; yoksa başlatıp ~5 sn bekler. Bağlamı gönderir, yanıtını
/// TÜKETİR (istemciye gitmez). Ulaşılamazsa `None` — çağıran süreç içine düşer.
pub async fn baglan_ya_da_baslat(b: &Baglam) -> Option<TcpStream> {
    let adres = ("127.0.0.1", port());
    let mut akis = match TcpStream::connect(adres).await {
        Ok(a) => a,
        Err(_) => {
            baslat()?;
            let mut bulunan = None;
            for _ in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if let Ok(a) = TcpStream::connect(adres).await {
                    bulunan = Some(a);
                    break;
                }
            }
            bulunan?
        }
    };
    let istek = json!({
        "jsonrpc": "2.0", "id": "el-fihrist-baglam", "method": "el-fihrist/baglam",
        "params": {"oturum": b.oturum, "proje": b.proje}
    });
    akis.write_all(format!("{istek}\n").as_bytes()).await.ok()?;
    // Yanıtı BAYT BAYT oku: BufReader fazlasını tamponlayıp sonraki satırları yutardı.
    let oku = async {
        let mut bayt = [0u8; 1];
        loop {
            akis.read_exact(&mut bayt).await.ok()?;
            if bayt[0] == b'\n' {
                return Some(());
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(3), oku)
        .await
        .ok()??;
    Some(akis)
}

/// Hizmeti bağımsız süreç olarak başlatır: aktarıcının stdio'sunu MİRAS ALMAZ
/// (stdout protokole aittir), oturum kapanınca ölmez. Tanı çıktısı günlük
/// dizinindeki `hizmet.log`'a.
fn baslat() -> Option<()> {
    let exe = std::env::current_exe().ok()?;
    let gunluk = crate::kutuphane_yolu()
        .ok()
        .map(|y| y.with_file_name("kutup_olaylar"))
        .and_then(|d| {
            std::fs::create_dir_all(&d)
                .ok()
                .map(|_| d.join("hizmet.log"))
        })
        .and_then(|y| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(y)
                .ok()
        });
    let mut k = std::process::Command::new(exe);
    k.arg("hizmet")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(gunluk.map_or_else(std::process::Stdio::null, std::process::Stdio::from));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP: konsolsuz, oturumun Ctrl+C'si ulaşmaz.
        k.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    k.spawn().ok().map(|_| ())
}

#[cfg(test)]
mod testler {
    use super::*;
    use tokio::io::AsyncBufReadExt;

    /// Aktarıcının bağlam isteği ve ardından bir `ping`, hizmetin konuşma
    /// döngüsünde doğru sırayla yanıtlanmalı; bağlam yanıtı `{}` olmalı.
    #[tokio::test]
    async fn baglam_sonra_ping_ayni_baglantida() {
        let (istemci, sunucu) = tokio::io::duplex(4096);
        let (s_okur, s_yazar) = tokio::io::split(sunucu);
        let b = Baglam {
            arayuz: None,
            oturum: "t".into(),
            proje: None,
        };
        let hizmet = tokio::spawn(crate::mcp::konus(BufReader::new(s_okur), s_yazar, b));

        let (i_okur, mut i_yazar) = tokio::io::split(istemci);
        i_yazar
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":\"el-fihrist-baglam\",\"method\":\"el-fihrist/baglam\",\"params\":{\"oturum\":\"123-9\"}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n")
            .await
            .unwrap();
        let mut satirlar = BufReader::new(i_okur).lines();
        let bir: serde_json::Value =
            serde_json::from_str(&satirlar.next_line().await.unwrap().unwrap()).unwrap();
        let iki: serde_json::Value =
            serde_json::from_str(&satirlar.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(bir["id"], "el-fihrist-baglam");
        assert_eq!(bir["result"], json!({}));
        assert_eq!(iki["id"], 2);
        i_yazar.shutdown().await.unwrap();
        hizmet.await.unwrap().unwrap();
    }
}
