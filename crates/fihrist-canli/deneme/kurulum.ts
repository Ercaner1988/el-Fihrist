// G-1 kurulum betiği: bildirim günlüğü ve tetikleyicileri araç veritabanlarına kurar.
//
// Kullanım (depo kökünden, önce: cargo build --release -p fihrist-canli --bins --examples):
//   bun crates/fihrist-canli/deneme/kurulum.ts --dizin "<kutuphane dizini>" \
//       [--ikili target/release] [--zopay <kesfuzzunun ikilisi> --docx <bir .docx>] [--gercek]
// (--zopay: KESFUZZUNUN_HAFIZA_DB'yi tanıyan kesfuzzunun.exe; eski zopay.exe onu tanımaz, gerçek hafızaya yazar.)
//
// İki evre. PROVA (varsayılan): gerçek dosyalar salt-okunur `.backup` ile kopyalanır,
// her şey kopyada koşar; gerçek dosyalara yalnız okumak için dokunulur (sha256,
// integrity_check). GERÇEK (`--gercek`): prova yeşilse aynı adımlar gerçek dosyalarda.
// İlk kırmızıda durur. Rapor: <cikti>/kurulum-raporu.md (her adım, komut, ham çıktı).
//
// Gerçek dosyada e/g/s denemesi gerçek satırlara dokunmaz: kurulumdan önce
// `kurulum_denemesi` tablosu yaratılır, ölçümler ona yazılır, sonunda DROP edilir;
// deneme tüketicisinin imleç dosyası silinir (yoksa budamayı sonsuza dek tutar).
// Turso yazarı `yaz` örneği, SQLite yazarı bun:sqlite (ve varsa Python).
//
// Ana katalogda sanal tablo varsa (ADR 0006 göçünden önce) kurulum orada reddedilmeli ve fts5
// sağlığı uyarı olarak raporlanır. Göçten sonra sanal tablo kalmaz; katalog öteki dört dosya
// gibi kurulur ve ölçülür (fts5 artık kutup_arama.db'de, bkz. arama-gocu.ts).

import { Database } from "bun:sqlite";
import { existsSync, mkdirSync, rmSync, writeFileSync, copyFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { WIN, butunluk, kos, ro_sorgu, sha, uri } from "./ortak";

const IZLENEN = ["kutup_kurallar", "kutup_ortak", "kutup_depolar", "kutup_kayitlar"];
const KATALOG = "kutup_kutuphane"; // fts5'li ise kurulum reddedilmeli (ADR 0003, 0005); göçten sonra izlenir (ADR 0006)
const DENEME = "kurulum_denemesi";
const TUKETICI = "kurulum-denetimi";
const EXE = WIN ? ".exe" : "";

function arg(ad: string): string | undefined {
  const i = process.argv.indexOf(ad);
  return i > 0 ? process.argv[i + 1] : undefined;
}
const dizin = arg("--dizin");
if (!dizin) {
  console.error('kullanım: bun kurulum.ts --dizin "<kutuphane dizini>" [--ikili target/release] [--zopay <ikili> --docx <dosya>] [--gercek]');
  process.exit(2);
}
const ikili = resolve(arg("--ikili") ?? "target/release");
const IZLE = join(ikili, `fihrist-izle${EXE}`);
const YAZ = join(ikili, "examples", `yaz${EXE}`);
const gercek = process.argv.includes("--gercek");
const zopay = arg("--zopay");
const docx = arg("--docx");
const damga = new Date().toISOString().replace(/[:.]/g, "-");
const cikti = resolve(arg("--cikti") ?? `kurulum-${damga}`);
mkdirSync(cikti, { recursive: true });

// ── Rapor ────────────────────────────────────────────────────────────────────
const rapor: string[] = [`# G-1 kurulum raporu`, ``, `- zaman: ${new Date().toISOString()}`, `- dizin: ${dizin}`, `- kip: ${gercek ? "GERÇEK" : "PROVA"}`, ``];
let kirmizi = 0;
let uyari = 0;
function yaz_rapor() {
  writeFileSync(join(cikti, "kurulum-raporu.md"), rapor.join("\n") + "\n", "utf8");
}
function adim(ad: string, gecti: boolean, ayrinti = "") {
  const s = `${gecti ? "✓" : "✗"} ${ad}${ayrinti ? ` — ${ayrinti}` : ""}`;
  console.log(s);
  rapor.push(`- ${s}`);
  if (!gecti) kirmizi++;
  yaz_rapor();
  return gecti;
}
// Kurulumu durdurmayan bulgu (ana katalog sağlığı): ayrı sayılır, raporda öne çıkar.
function uyar(ad: string, gecti: boolean, ayrinti = "") {
  const s = `${gecti ? "✓" : "⚠"} ${ad}${ayrinti ? ` — ${ayrinti}` : ""}`;
  console.log(s);
  rapor.push(`- ${s}`);
  if (!gecti) uyari++;
  yaz_rapor();
}
function dur(neden: string): never {
  rapor.push(``, `**DURDU:** ${neden}`);
  yaz_rapor();
  console.error(`DURDU: ${neden}\nrapor: ${join(cikti, "kurulum-raporu.md")}`);
  process.exit(1);
}
function bolum(baslik: string) {
  console.log(`\n── ${baslik}`);
  rapor.push(``, `## ${baslik}`, ``);
}

// ── Yardımcılar (ortakları ortak.ts'te) ─────────────────────────────────────
function sqlite_yaz(yol: string, sql: string) {
  const db = new Database(yol);
  try {
    db.run(sql);
  } finally {
    db.close();
  }
}
// `kos`un eşzamansızı: çocuk koşarken olay döngüsü (izleyici okuyucusu) çalışmayı sürdürür.
async function kos_async(komut: string[]): Promise<{ kod: number; cikti: string }> {
  const p = Bun.spawn(komut, { stdout: "pipe", stderr: "pipe" });
  const [o, e] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
  return { kod: p.exitCode ?? -1, cikti: (o + e).trim() };
}
function gunluk(yol: string, tablo: string): { no: number; islem: string; anahtar: string }[] {
  return ro_sorgu(yol, `SELECT no, islem, anahtar FROM fihrist_degisiklik WHERE tablo = '${tablo}' ORDER BY no`);
}

// fihrist-izle'yi başlatır; stderr'de `bekle` görülene ya da süreç bitene dek okur.
// Bekleyen read() tek tutulur (oturum.ts'deki yarışın dersi).
async function izle_baslat(argv: string[], bekle: string, sure = 15000) {
  const p = Bun.spawn([IZLE, ...argv], { stdout: "pipe", stderr: "pipe" });
  const satirlar: { t: number; s: string }[] = [];
  (async () => {
    const okur = p.stdout.getReader();
    let tampon = "";
    for (;;) {
      const { value, done } = await okur.read();
      if (done) break;
      tampon += new TextDecoder().decode(value);
      let i;
      while ((i = tampon.indexOf("\n")) >= 0) {
        satirlar.push({ t: Date.now(), s: tampon.slice(0, i) });
        tampon = tampon.slice(i + 1);
      }
    }
  })();
  const err = p.stderr.getReader();
  let hata = "";
  const son = Date.now() + sure;
  let bekleyen = err.read();
  while (!hata.includes(bekle) && Date.now() < son) {
    const r = await Promise.race([bekleyen, Bun.sleep(300).then(() => null)]);
    if (r === null) {
      if (p.exitCode !== null) break;
      continue;
    }
    if (r.done) break;
    hata += new TextDecoder().decode(r.value);
    bekleyen = err.read();
  }
  return { p, satirlar, hata: () => hata };
}

// ── Bir dosyada kurulum ve kabul ölçümleri ──────────────────────────────────
async function kur_ve_olc(db: string, etiket: string, kayitlar: boolean) {
  const once_butunluk = butunluk(db);
  if (!adim(`${etiket}: integrity_check (önce)`, once_butunluk === "ok", once_butunluk)) dur(`${etiket} bütünlük denetimi ok değil`);

  sqlite_yaz(db, `CREATE TABLE ${DENEME} (id INTEGER PRIMARY KEY, ad TEXT)`);
  const k = await izle_baslat([db, "--kur", "--tuketici", TUKETICI, "--aralik", "250"], "izleniyor:");
  const kuruldu = k.hata().match(/kuruldu: (\d+) tablo/);
  if (!adim(`${etiket}: fihrist-izle --kur`, !!kuruldu, k.hata().trim().split("\n")[0])) {
    k.p.kill();
    dur(`${etiket} kurulamadı`);
  }

  // e/g/s: Turso (yaz), SQLite (bun:sqlite) ve varsa Python; gecikme fihrist-izle'den.
  const yazislar: [string, string, string][] = [
    ["e", "turso", `INSERT INTO ${DENEME} VALUES (1, 'İğne')`],
    ["g", "sqlite", `UPDATE ${DENEME} SET ad = 'Şimşek' WHERE id = 1`],
    ["s", "turso", `DELETE FROM ${DENEME} WHERE id = 1`],
  ];
  for (const [islem, motor, sql] of yazislar) {
    // Gecikme yazışın BİTİŞİNDEN ölçülür: `yaz` ayrı süreçtir, açılışı ve şema okuması bellek
    // darken 1-8 sn sürdü (ölçüldü, 2026-10-10, %98 bellek); başlangıçtan ölçmek o süreyi
    // izleyiciye yüklüyor, 2 sn'yi aşınca hiç beklemeden "-1 ms" veriyordu. Yazış süresi ayrıca yazılır.
    // `yaz` eşzamansız koşar (kos_async): spawnSync olay döngüsünü durdurur, izleyicinin satırları
    // yazış bitene dek damgalanmaz ve gecikme hep ~0 görünürdü (yargıç yeniden üretti).
    let t0 = Date.now();
    if (motor === "turso") {
      // Turso dosyayı bloklamayan fcntl kilidiyle açar; fihrist-izle o anda yokluyorsa yazış
      // "Locking error" alır (ölçüldü: 250 ms yoklamada 200 yazışın 5'i). Sınırlı yeniden deneme,
      // sayısı rapora yazılır.
      let r = await kos_async([YAZ, db, sql]);
      let tekrar = 0;
      while (r.kod !== 0 && /Locking error/.test(r.cikti) && tekrar < 5) {
        await Bun.sleep(50);
        t0 = Date.now();
        r = await kos_async([YAZ, db, sql]);
        tekrar++;
      }
      const ayrinti = tekrar ? `${tekrar} kilit çakışmasından sonra${r.cikti ? `; ${r.cikti}` : ""}` : r.cikti;
      if (!adim(`${etiket}: Turso yazışı (${islem})`, r.kod === 0, ayrinti)) dur("Turso yazışı başarısız");
    } else sqlite_yaz(db, sql);
    const bitti = Date.now();
    let gecikme = -1;
    while (Date.now() - bitti < 2000) {
      const s = k.satirlar.find((x) => x.s.split("\t")[2] === DENEME && x.s.split("\t")[3] === islem);
      if (s) {
        // Bildirim yazıcı süreci kapanmadan gelmiş olabilir (commit önce, çıkış sonra): 0 sayılır.
        gecikme = Math.max(0, s.t - bitti);
        break;
      }
      await Bun.sleep(20);
    }
    adim(`${etiket}: ${islem} (${motor}) yazıştan sonra 1 sn içinde bildirildi`, gecikme >= 0 && gecikme < 1000, `${gecikme < 0 ? "2 sn'de gelmedi" : `${gecikme} ms`}; yazış ${bitti - t0} ms`);
  }
  const py = kos([WIN ? "python" : "python3", "-I", "-c", `import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute("INSERT INTO ${DENEME} VALUES (2,'py')"); c.commit()`, db]);
  if (py.kod === 0) {
    await Bun.sleep(700);
    adim(`${etiket}: Python sqlite3 yazışı günlükte`, gunluk(db, DENEME).some((g) => g.islem === "e" && g.anahtar === "[2]"));
  } else rapor.push(`- ölçülmedi: Python yazışı (python bulunamadı: ${py.cikti.split("\n")[0]})`);

  // Öldür, yaz, başlat: kayıp ve tekrar 0.
  k.p.kill();
  await k.p.exited;
  const son_once = Math.max(0, ...k.satirlar.map((x) => Number(x.s.split("\t")[0]) || 0));
  for (let i = 10; i < 15; i++) sqlite_yaz(db, `INSERT INTO ${DENEME} VALUES (${i}, 'ö${i}')`);
  const k2 = await izle_baslat([db, "--tuketici", TUKETICI, "--aralik", "250"], "izleniyor:");
  await Bun.sleep(1200);
  k2.p.kill();
  await k2.p.exited;
  const gelen = k2.satirlar.map((x) => Number(x.s.split("\t")[0]));
  const yeni = gelen.filter((n) => n > son_once);
  const tekil = new Set(gelen).size === gelen.length;
  const deneme_e = k2.satirlar.filter((x) => x.s.split("\t")[2] === DENEME && x.s.split("\t")[3] === "e").length;
  adim(`${etiket}: öldür, yaz (5), başlat`, deneme_e >= 5 && tekil && yeni.length >= 5, `gelen ${gelen.length}, yeni ${yeni.length}, deneme eklemesi ${deneme_e}, tekrar ${tekil ? 0 : "VAR"}`);

  // zopay (yalnız kayitlar, yalnız kopyada): kayıt yazan komut günlüğe düşmeli.
  // Keşfü'z-Zunûn adıyla (2026-10-07) hafıza `kesfuzzunun_*` tablolarına yazar; eski `zopay_*`
  // tablolarına artık yazılmaz. Başarılı koşu da "hafıza: <yol>" basar: yalnız hata satırı kırmızıdır.
  if (kayitlar && !gercek_evre) {
    if (zopay && docx) {
      const once = gunluk(db, "kesfuzzunun_kosular").length;
      const r = kos([zopay, "sayfa-dogrula", docx], { KESFUZZUNUN_HAFIZA_DB: db });
      const sonra = gunluk(db, "kesfuzzunun_kosular").length;
      rapor.push("", "```text", r.cikti.split("\n").slice(-15).join("\n"), "```", "");
      adim(`${etiket}: zopay sayfa-dogrula (KESFUZZUNUN_HAFIZA_DB=kopya) bozulmadı`, r.kod === 0 && !/hafıza( açılamadı|: .*(yazılamadı|okunamadı))/.test(r.cikti) && sonra > once, `çıkış ${r.kod}, kesfuzzunun_kosular günlüğü ${once} → ${sonra}`);
    } else rapor.push(`- ölçülmedi: zopay denemesi (--zopay ve --docx verilmedi)`);
  }

  // Temizlik: deneme tablosu ve deneme tüketicisinin imleci.
  sqlite_yaz(db, `DROP TABLE ${DENEME}`);
  rmSync(join(`${db}.imlec`, TUKETICI), { force: true });
  adim(`${etiket}: deneme tablosu ve imleç kaldırıldı`, !existsSync(join(`${db}.imlec`, TUKETICI)) && ro_sorgu(db, `SELECT 1 FROM sqlite_master WHERE name = '${DENEME}'`).length === 0);
  const sonra_butunluk = butunluk(db);
  if (!adim(`${etiket}: integrity_check (sonra)`, sonra_butunluk === "ok", sonra_butunluk)) dur(`${etiket} kurulum sonrası bütünlük ok değil`);
}

// fts5'in kendi denetimi: dizin içerikle tutarlı mı (Turso yazışları fts5
// tetikleyicilerini atlamış olabilir; turso#9282). Komut bir INSERT'tir: YALNIZ kopyada.
function katalog_fts_denetimi(kopya: string) {
  const fts = ro_sorgu<{ name: string }>(kopya, "SELECT name FROM sqlite_master WHERE sql LIKE 'CREATE VIRTUAL TABLE%USING fts5%'");
  if (fts.length === 0) return uyar("ana katalog: fts5 tablosu yok", true);
  const db = new Database(kopya);
  try {
    for (const { name } of fts) {
      const q = `"${name.replaceAll('"', '""')}"`;
      try {
        // rank = 1: dış içerik tablosuyla da karşılaştır (yalın biçim bayat dizini görmez; ölçüldü).
        db.run(`INSERT INTO ${q}(${q}, rank) VALUES('integrity-check', 1)`);
        uyar(`ana katalog: ${name} fts5 integrity-check`, true, "tutarlı");
      } catch (e) {
        uyar(`ana katalog: ${name} fts5 integrity-check`, false, `${(e as Error).message} — onarım (SQLite ile, yedekten sonra): INSERT INTO ${q}(${q}) VALUES('rebuild')`);
      }
    }
  } finally {
    db.close();
  }
}

async function katalog_reddi(db: string, etiket: string) {
  const once = sha(db);
  const r = kos([IZLE, db, "--kur"]);
  const sonra = sha(db);
  adim(`${etiket}: ana katalogda kurulum reddedildi`, r.kod !== 0 && /sanal tablo/.test(r.cikti), r.cikti.split("\n")[0]);
  if (!adim(`${etiket}: ana katalog sha256 değişmedi`, once === sonra, `${once} → ${sonra}`)) dur("ana katalog değişti");
}

// ── Akış ─────────────────────────────────────────────────────────────────────
let gercek_evre = false;
bolum("Ön koşullar");
for (const [ad, yol] of [["fihrist-izle", IZLE], ["yaz (örnek)", YAZ]] as const) {
  if (!adim(`${ad} ikilisi`, existsSync(yol), yol)) dur(`${yol} yok; önce: cargo build --release -p fihrist-canli --bins --examples`);
}
const s3 = kos(["sqlite3", "-version"]);
if (!adim("sqlite3 CLI", s3.kod === 0, s3.cikti.split(" ")[0])) dur("sqlite3 bulunamadı (.backup için gerekli)");
for (const ad of [...IZLENEN, KATALOG]) {
  if (!adim(`${ad}.db var`, existsSync(join(dizin, `${ad}.db`)))) dur(`${ad}.db yok`);
}
if (WIN) {
  const t = kos(["tasklist", "/FO", "CSV", "/NH"]);
  const acik = ["ibnunnedim.exe", "zopay.exe", "kesfuzzunun.exe", "fihrist-izle.exe", "python.exe"].filter((a) => t.cikti.toLowerCase().includes(`"${a}"`));
  if (!adim("dosyaları tutan süreç yok", acik.length === 0, acik.join(", ") || "yok")) dur(`önce durdurun (mevcut başlatma betikleriyle): ${acik.join(", ")}`);
} else rapor.push("- ölçülmedi: açık süreç denetimi (yalnız Windows'ta)");

bolum("Yedek (salt-okunur .backup)");
const yedek = join(cikti, "yedek");
mkdirSync(yedek, { recursive: true });
const ilk_sha: Record<string, string> = {};
for (const ad of [...IZLENEN, KATALOG]) {
  const kaynak = join(dizin, `${ad}.db`);
  ilk_sha[ad] = sha(kaynak);
  const hedef = join(yedek, `${ad}.db`);
  const r = kos(["sqlite3", uri(kaynak), `.backup '${hedef.replaceAll("\\", "/")}'`]);
  if (!adim(`${ad}: yedek alındı`, r.kod === 0 && existsSync(hedef), r.cikti)) dur(`${ad} yedeklenemedi`);
  const b = butunluk(hedef);
  if (ad === KATALOG) uyar(`${ad}: integrity_check (yedekte)`, b === "ok", b.slice(0, 300));
  else if (!adim(`${ad}: integrity_check (yedekte)`, b === "ok", b)) dur(`${ad} bütünlük ok değil`);
  if (!adim(`${ad}: yedek alırken gerçek dosya değişmedi`, sha(kaynak) === ilk_sha[ad], ilk_sha[ad])) dur(`${ad} yedek sırasında değişti (açık yazan var mı?)`);
}

// ADR 0006 göçünden sonra katalogda sanal tablo kalmaz: o zaman o da izlenir.
const katalog_sanal = ro_sorgu<{ n: number }>(join(yedek, `${KATALOG}.db`), "SELECT count(*) AS n FROM sqlite_master WHERE sql LIKE 'CREATE VIRTUAL TABLE%'")[0].n > 0;
const kurulacak = katalog_sanal ? IZLENEN : [...IZLENEN, KATALOG];
if (!katalog_sanal) rapor.push("", `- ${KATALOG}: sanal tablo yok (ADR 0006 göçü yapılmış); öteki dosyalar gibi kurulur`);

bolum("PROVA (kopyalarda)");
const prova = join(cikti, "prova");
mkdirSync(prova, { recursive: true });
for (const ad of [...IZLENEN, KATALOG]) copyFileSync(join(yedek, `${ad}.db`), join(prova, `${ad}.db`));
for (const ad of kurulacak) await kur_ve_olc(join(prova, `${ad}.db`), `prova/${ad}`, ad === "kutup_kayitlar");
if (katalog_sanal) {
  await katalog_reddi(join(prova, `${KATALOG}.db`), `prova/${KATALOG}`);
  bolum("Ana katalog sağlığı (kopyada; kurulumu durdurmaz)");
  const fts_kopya = join(cikti, `${KATALOG}-fts-denetimi.db`);
  copyFileSync(join(yedek, `${KATALOG}.db`), fts_kopya);
  katalog_fts_denetimi(fts_kopya);
}
if (kirmizi > 0) dur(`provada ${kirmizi} kırmızı`);

if (gercek) {
  gercek_evre = true;
  bolum("GERÇEK");
  for (const ad of [...IZLENEN, KATALOG]) {
    if (!adim(`${ad}: provadan beri değişmedi`, sha(join(dizin, `${ad}.db`)) === ilk_sha[ad])) dur(`${ad} yedekten sonra değişti; yeniden koşun`);
  }
  for (const ad of kurulacak) await kur_ve_olc(join(dizin, `${ad}.db`), ad, ad === "kutup_kayitlar");
  if (katalog_sanal) await katalog_reddi(join(dizin, `${KATALOG}.db`), KATALOG);
} else rapor.push("", "GERÇEK evre koşulmadı (`--gercek` verilmedi).");

rapor.push("", `**SONUÇ:** ${kirmizi === 0 ? "YEŞİL" : `${kirmizi} kırmızı`}${uyari ? ` · ana katalogda ${uyari} UYARI (yukarıda ⚠)` : ""}`, "", `Yedekler: ${yedek}`);
yaz_rapor();
console.log(`\nSONUÇ: ${kirmizi === 0 ? "YEŞİL" : `${kirmizi} kırmızı`}${uyari ? ` · ${uyari} UYARI` : ""} — rapor: ${join(cikti, "kurulum-raporu.md")}`);
process.exit(kirmizi === 0 ? 0 : 1);
