use super::*;
use serde_json::json;
use std::io::Write;

fn satir(tip: &str, uuid: &str, ts: &str, icerik: serde_json::Value) -> String {
    json!({"type": tip, "uuid": uuid, "timestamp": ts, "sessionId": "o1", "message": {"content": icerik}}).to_string()
}

fn yaz(yol: &std::path::Path, satirlar: &[String], son_satir_yarim: Option<&str>) {
    let mut f = std::fs::File::create(yol).unwrap();
    for s in satirlar {
        writeln!(f, "{s}").unwrap();
    }
    if let Some(y) = son_satir_yarim {
        write!(f, "{y}").unwrap();
    }
}

fn temel() -> Vec<String> {
    vec![
        satir(
            "user",
            "u1",
            "2026-10-14T10:00:00.000Z",
            json!("Asabiyye kavramını İbn Haldun'dan ara"),
        ),
        satir(
            "assistant",
            "a1",
            "2026-10-14T10:00:05.000Z",
            json!([{"type": "text", "text": "Mukaddime'de geçiyor."},
                   {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "C:/tez/mukaddime.pdf"}}]),
        ),
        satir(
            "user",
            "u2",
            "2026-10-14T10:00:06.000Z",
            json!([{"type": "tool_result", "tool_use_id": "t1", "content": "GİZLİ-ÇIKTI-GÖVDESİ"}]),
        ),
        r#"{"type":"last-prompt","sessionId":"o1"}"#.to_string(),
    ]
}

#[tokio::test]
async fn artimli_iceri_aktarma_yinelemez_ve_yarim_satiri_bekler() {
    let d = tempfile::tempdir().unwrap();
    let dokum = d.path().join("o1.jsonl");
    yaz(
        &dokum,
        &temel(),
        Some(r#"{"type":"user","uuid":"u3","timesta"#),
    );
    let depo = Depo::ac(&d.path().join("arsiv.db")).await.unwrap();

    let o = depo.ice_aktar(&dokum).await.unwrap();
    assert_eq!(
        (o.yeni, o.satir, o.bozuk),
        (4, 4, 0),
        "ercan + model + arac + sonuç izi; yarım satır sayılmaz: {o:?}"
    );
    // Aynı dökümü yeniden vermek hiçbir şey eklemez (konum ilerlemiş).
    assert_eq!(depo.ice_aktar(&dokum).await.unwrap().yeni, 0);

    // Döküm sürdü: yarım satır tamamlandı + yeni bir mesaj. Yalnız yeniler girer.
    let mut tam = temel();
    tam.push(satir(
        "user",
        "u3",
        "2026-10-14T10:01:00.000Z",
        json!("ikinci soru"),
    ));
    yaz(&dokum, &tam, None);
    let o = depo.ice_aktar(&dokum).await.unwrap();
    assert_eq!((o.yeni, o.satir), (1, 1), "{o:?}");

    // Devam ettirilen oturumun yeni dosyası eski satırları kopyalar: tekillik ikinci kez almaz.
    let kopya = d.path().join("o2.jsonl");
    yaz(&kopya, &tam, None);
    let o = depo.ice_aktar(&kopya).await.unwrap();
    assert_eq!(o.yeni, 0, "(uuid, blok) tekilliği: {o:?}");

    // Küçülen döküm baştan okunur ve yine yinelemez.
    yaz(&dokum, &temel()[..2], None);
    let o = depo.ice_aktar(&dokum).await.unwrap();
    assert!(o.bastan && o.yeni == 0, "{o:?}");

    let s = depo.sayim().await.unwrap();
    assert_eq!(
        s,
        [
            ("arac".to_string(), 1),
            ("ercan".to_string(), 2),
            ("model".to_string(), 1),
            ("sonuc".to_string(), 1)
        ]
    );
}

#[tokio::test]
async fn bozuk_satir_sayilir_ve_atlanir() {
    let d = tempfile::tempdir().unwrap();
    let dokum = d.path().join("o.jsonl");
    let mut s = temel();
    s.insert(1, "çöp satır, JSON değil".into());
    yaz(&dokum, &s, None);
    let depo = Depo::ac(&d.path().join("arsiv.db")).await.unwrap();
    let o = depo.ice_aktar(&dokum).await.unwrap();
    assert_eq!((o.bozuk, o.yeni), (1, 4), "{o:?}");
}

#[tokio::test]
async fn arama_turkce_katlar_ve_arac_ciktisini_aramaz() {
    let d = tempfile::tempdir().unwrap();
    let dokum = d.path().join("o1.jsonl");
    yaz(&dokum, &temel(), None);
    let depo = Depo::ac(&d.path().join("arsiv.db")).await.unwrap();
    depo.ice_aktar(&dokum).await.unwrap();

    // "ASABİYYE" ve "asabiyye" katlamada eşit; sonuç Ercan'ın mesajı ve asıl dökümdeki yeri.
    let r = depo.ara("ASABİYYE", 5, None).await.unwrap();
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!((r[0].tur.as_str(), r[0].oturum.as_str()), ("ercan", "o1"));
    assert!(r[0].yer.ends_with("o1.jsonl:1"), "{}", r[0].yer);

    // Araç çağrısının girdisi aranır (yol), çıktısı arşivde yok.
    let r = depo.ara("mukaddime", 5, Some("arac")).await.unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].arac.as_deref(), Some("Read"));
    assert!(
        depo.ara("GİZLİ", 5, None).await.unwrap().is_empty(),
        "araç çıktısı gövdesi aranamaz"
    );
    assert!(depo.ara("", 5, None).await.unwrap().is_empty());
}

#[tokio::test]
async fn yeni_sema_surumu_reddedilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("arsiv.db");
    drop(Depo::ac(&yol).await.unwrap());
    {
        let db = turso::Builder::new_local(yol.to_str().unwrap())
            .build()
            .await
            .unwrap();
        db.connect()
            .unwrap()
            .execute("UPDATE meta SET deger = '99' WHERE anahtar = 'surum'", ())
            .await
            .unwrap();
    }
    let e = Depo::ac(&yol).await.err().expect("reddedilmeli");
    assert!(e.contains("99"), "{e}");
}

// ---- MCP Seçimi eşlemesi ----

/// Nazar Seçim günlüğünün (nazar-gunluk) şemasının yeterli kısmı; gerçek dosya o crate'in testlerinde doğrulanır.
/// (arama kimliği, an, oturum, kanal, sonuç yolları)
type Arama<'a> = (i64, i64, Option<&'a str>, &'a str, &'a [&'a str]);

async fn nazar_gunlugu(yol: &std::path::Path, aramalar: &[Arama<'_>]) {
    let db = turso::Builder::new_local(yol.to_str().unwrap())
        .build()
        .await
        .unwrap();
    let b = db.connect().unwrap();
    b.execute_batch(
        "CREATE TABLE aramalar (id INTEGER PRIMARY KEY, zaman_ms INTEGER NOT NULL, kanal TEXT NOT NULL, oturum TEXT);
         CREATE TABLE sonuclar (arama_id INTEGER NOT NULL, sira INTEGER NOT NULL, yol TEXT NOT NULL, yaprak INTEGER, satir INTEGER);",
    )
    .await
    .unwrap();
    for (id, zaman, oturum, kanal, yollar) in aramalar {
        b.execute(
            "INSERT INTO aramalar (id, zaman_ms, kanal, oturum) VALUES (?1, ?2, ?3, ?4)",
            turso::params_from_iter([
                turso::Value::from(*id),
                (*zaman).into(),
                (*kanal).into(),
                oturum.map(str::to_string).into(),
            ]),
        )
        .await
        .unwrap();
        for (sira, y) in yollar.iter().enumerate() {
            b.execute(
                "INSERT INTO sonuclar (arama_id, sira, yol, yaprak, satir) VALUES (?1, ?2, ?3, 7, NULL)",
                turso::params_from_iter([turso::Value::from(*id), (sira as i64).into(), (*y).into()]),
            )
            .await
            .unwrap();
        }
    }
}

fn arac(uuid: &str, ts: &str, ad: &str, girdi: serde_json::Value, yan: bool) -> String {
    let mut d = json!({"type": "assistant", "uuid": uuid, "timestamp": ts, "sessionId": "o1",
        "message": {"content": [{"type": "tool_use", "id": uuid, "name": ad, "input": girdi}]}});
    if yan {
        d["isSidechain"] = json!(true);
    }
    d.to_string()
}

#[tokio::test]
async fn mcp_secimi_sonraki_cagrinin_girdisinden_cikar() {
    use crate::esle::esle_kopyadan;
    let d = tempfile::tempdir().unwrap();
    let dokum = d.path().join("o1.jsonl");
    // nazar_ara t=10:00:00.500'de döndü (günlükteki an). Sonraki çağrılar:
    yaz(
        &dokum,
        &[
            // Öncesi: aynı yolu daha önce okumuş; arama sonrası değil, sayılmaz.
            arac(
                "c0",
                "2026-10-14T09:59:00.000Z",
                "Read",
                json!({"file_path": r"C:\tez\a.pdf"}),
                false,
            ),
            arac(
                "c1",
                "2026-10-14T10:00:00.000Z",
                "mcp__nazar__nazar_ara",
                json!({"sorgu": r"C:\tez\b.pdf"}),
                false,
            ),
            // arama SONRASI bir nazar_ara sorgusunda yol geçmesi kullanım değildir (ikinci arama).
            arac(
                "c1b",
                "2026-10-14T10:00:05.000Z",
                "mcp__nazar__nazar_ara",
                json!({"sorgu": r"C:/tez/b.pdf"}),
                false,
            ),
            // Kaçışlı Windows yolu (JSON'da `\`), sonuçtaki düz yolla aynı dosya.
            arac(
                "c2",
                "2026-10-14T10:00:10.000Z",
                "Read",
                json!({"file_path": r"C:\tez\A.pdf"}),
                false,
            ),
            // Git Bash biçimi.
            arac(
                "c3",
                "2026-10-14T10:00:20.000Z",
                "Bash",
                json!({"command": "cat /c/tez/b.pdf"}),
                false,
            ),
            // Alt ajanın çağrısı da aynı oturumda sayılır.
            arac(
                "c4",
                "2026-10-14T10:00:30.000Z",
                "Edit",
                json!({"file_path": r"C:/tez/c.pdf"}),
                true,
            ),
        ],
        None,
    );
    let depo = Depo::ac(&d.path().join("arsiv.db")).await.unwrap();
    depo.ice_aktar(&dokum).await.unwrap();

    let nazar = d.path().join("nazar.db");
    let t0 = 1_791_972_000_500; // 2026-10-14T10:00:00.500Z
    nazar_gunlugu(
        &nazar,
        &[
            (
                1,
                t0,
                Some("o1"),
                "mcp",
                &[
                    r"C:\tez\a.pdf",
                    r"C:\tez\b.pdf",
                    r"C:\tez\c.pdf",
                    r"C:\tez\d.pdf",
                ],
            ),
            (2, t0, None, "mcp", &[r"C:\tez\a.pdf"]), // oturumsuz: eşlenemez
            (3, t0, Some("yok"), "mcp", &[r"C:\tez\a.pdf"]), // arşivde yok
            (4, t0, Some("o1"), "masaustu", &[r"C:\tez\a.pdf"]), // masaüstü: bu işin konusu değil
        ],
    )
    .await;

    let (s, r) = esle_kopyadan(&depo, &nazar).await.unwrap();
    let ozet: Vec<_> = s
        .iter()
        .map(|x| (x.arama_id, x.yol.as_str(), x.arac.as_str()))
        .collect();
    assert_eq!(
        ozet,
        [
            (1, r"C:\tez\a.pdf", "Read"),
            (1, r"C:\tez\b.pdf", "Bash"), // nazar_ara'nın kendi sorgusu sayılmaz; Bash çağrısı sayılır
            (1, r"C:\tez\c.pdf", "Edit"),
        ],
        "d.pdf hiç kullanılmadı: Seçim değil"
    );
    assert_eq!(s[0].yaprak, Some(7));
    assert!(s[0].zaman_ms > t0, "Seçim arama sonrasında");
    assert_eq!((r.arama, r.oturumsuz, r.arsivsiz), (3, 1, 1), "{r:?}");
}

#[test]
fn yol_normallestirme_kacisli_ve_duz_yolu_bir_yapar() {
    use crate::esle::normallestir;
    let duz = normallestir(r"C:\Tez\Ünite\A.pdf");
    let json = normallestir(r"C:\Tez\Ünite\A.pdf");
    assert_eq!(duz, json);
    assert_eq!(duz, "c:/tez/ünite/a.pdf");
}

#[test]
#[ignore = "sqlite3 CLI gerekir (PATH)"]
fn sqlite3_backup_kopyasi_okunur() {
    let d = tempfile::tempdir().unwrap();
    let kaynak = d.path().join("boşluklu klasör").join("nazar.db");
    std::fs::create_dir_all(kaynak.parent().unwrap()).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(nazar_gunlugu(&kaynak, &[(1, 5, Some("o"), "mcp", &["a"])]));
    let hedef = d.path().join("kopya.db");
    crate::esle::kopyala(&kaynak, &hedef).unwrap();
    assert!(hedef.metadata().unwrap().len() > 0);
}

#[tokio::test]
async fn degismeyen_dokum_atlanir_degisen_alinir() {
    let d = tempfile::tempdir().unwrap();
    let (a, b) = (d.path().join("a.jsonl"), d.path().join("b.jsonl"));
    yaz(&a, &temel(), None);
    yaz(&b, &temel()[..1], None);
    let depo = Depo::ac(&d.path().join("arsiv.db")).await.unwrap();
    let hepsi = [a.clone(), b.clone(), d.path().join("yok.jsonl")];
    // Hiçbiri işlenmedi: ikisi yeni, olmayan dosya da işlenir (hata `ice_aktar`ta açıkça çıksın).
    assert_eq!(depo.degisenler(&hepsi).await.unwrap().len(), 3);
    depo.ice_aktar(&a).await.unwrap();
    depo.ice_aktar(&b).await.unwrap();
    let kalan = depo.degisenler(&hepsi).await.unwrap();
    assert_eq!(
        kalan,
        [d.path().join("yok.jsonl")],
        "işlenmiş ve değişmemiş dökümler atlanır"
    );
    // Döküm büyüdü: yalnız o geri gelir.
    yaz(&b, &temel(), None);
    let kalan = depo.degisenler(&[a, b.clone()]).await.unwrap();
    assert_eq!(kalan, [b]);
}

#[test]
fn yol_yalniz_tam_yol_olarak_gecince_eslesir() {
    use crate::esle::yol_geciyor;
    assert!(yol_geciyor(r#"{"file_path":"/r/a.rs"}"#, "/r/a.rs"));
    assert!(yol_geciyor("cat /r/a.rs | head", "/r/a.rs"));
    assert!(
        !yol_geciyor(r#"{"file_path":"/r/a.rs.bak"}"#, "/r/a.rs"),
        "sonek"
    );
    assert!(
        !yol_geciyor(r#"{"file_path":"/eski/r/a.rs"}"#, "/r/a.rs"),
        "önek"
    );
    assert!(
        yol_geciyor("/r/a.rs.bak /r/a.rs", "/r/a.rs"),
        "ikinci geçiş tam"
    );
}

#[test]
fn kanca_mi_yalniz_arsivle_kancasini_tanir() {
    use crate::komut::{kanca_mi, AraArgs, ArsivleArgs, Komutlar};
    let arsivle = |kanca| {
        Komutlar::Arsivle(ArsivleArgs {
            dokum: None,
            tumu: false,
            kok: None,
            kanca,
        })
    };
    assert!(kanca_mi(&arsivle(true)));
    assert!(!kanca_mi(&arsivle(false)));
    let ara = Komutlar::ArsivAra(AraArgs {
        sorgu: vec![],
        limit: 1,
        tur: None,
    });
    assert!(!kanca_mi(&ara));
}
