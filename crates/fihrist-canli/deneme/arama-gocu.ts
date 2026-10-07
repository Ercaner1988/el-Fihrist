// ADR 0006 göçü: ana katalogdaki fts5 tabloları türetilmiş arama dosyasına (kutup_arama.db) taşınır.
//
// Kullanım (depo kökünden):
//   bun crates/fihrist-canli/deneme/arama-gocu.ts --dizin "<kutuphane dizini>" \
//       [--ikili target/release] [--gercek] [--cikti <klasör>]
//   bun crates/fihrist-canli/deneme/arama-gocu.ts --dizin "<kutuphane dizini>" --tazele
//
// İki evre (kurulum.ts ile aynı düzen). PROVA (varsayılan): ana katalog salt-okunur `.backup`
// ile kopyalanır, göç kopyada koşar. GERÇEK (`--gercek`): prova yeşilse aynı göç gerçek dosyada.
// İlk kırmızıda durur. Rapor: <cikti>/arama-gocu-raporu.md. `--ikili` verilirse Turso'nun
// (`yaz` örneği) göçten sonra bütün tabloları gördüğü de ölçülür.
//
// Şema dosyadan okunur, tablo adları varsayılmaz:
// - fts5 dışında sanal tablo, içeriksiz fts5 (`content=''`), fts5'e dayanan görünüm: durur.
// - Dış içerikli fts5 (`content=T`): arama dosyasında T'den kurulur; rowid korunur, T'nin birincil
//   anahtarı sona `UNINDEXED` sütun olarak eklenir. Kaynağı T olduğu için `--tazele` onu yeniler.
// - İçerikli fts5: satırları rowid'leriyle aynen taşınır. Kaynağı yoktur; onu yazan araç ATTACH
//   ile yazmayı sürdürür, `--tazele` dokunmaz.
// - fts5'e dokunan bütün tetikleyiciler düşer (biri kalırsa ana tabloya her yazış düşer); tam SQL'leri
//   rapora yazılır.
// Kurulum ve temizlik tek bağlantıda, tek işlemdedir (ATTACH). WAL kipinde çok dosyalı commit
// çökmeye karşı atomik değildir; yarım kalan göç yeniden koşulunca algılanır, yedek her zaman var.
//
// `--tazele`: dış içerikli tabloların damgası kaynakla tutmuyorsa o tablo yeniden kurulur. Ana
// kataloğu yalnız okur, yalnız arama dosyasına yazar. Damga: kaynak satırları rowid sırasıyla, her
// satır boşluksuz JSON dizisi + "\n", UTF-8, sha256 onaltılık. Satır `[rowid, sütunlar…, anahtar]`.
//
// Python okuru: `ATTACH '<dizin>/kutup_arama.db' AS arama`; sorgular nitelemesiz adla değişmeden
// çalışır (ölçüldü). Tazelik için aramadan önce `--tazele` çağrılır.

import { Database } from "bun:sqlite";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { WIN, butunluk, kos, q, ro_sorgu, sha, uri } from "./ortak";

const KATALOG = "kutup_kutuphane";
const ARAMA = "kutup_arama";
const META = "arama_meta";
const GOLGE = ["data", "idx", "content", "docsize", "config"];
const EXE = WIN ? ".exe" : "";

function arg(ad: string): string | undefined {
  const i = process.argv.indexOf(ad);
  return i > 0 ? process.argv[i + 1] : undefined;
}
const dizin = arg("--dizin");
if (!dizin) {
  console.error('kullanım: bun arama-gocu.ts --dizin "<kutuphane dizini>" [--ikili target/release] [--gercek] [--cikti <klasör>] | --tazele');
  process.exit(2);
}
const katalog_yolu = join(dizin, `${KATALOG}.db`);
const arama_yolu = join(dizin, `${ARAMA}.db`);
const tazele_kipi = process.argv.includes("--tazele");
const gercek = process.argv.includes("--gercek");
const ikili = arg("--ikili");
const YAZ = ikili ? join(resolve(ikili), "examples", `yaz${EXE}`) : undefined;

// ── Rapor (yalnız göçte; --tazele konsola yazar) ─────────────────────────────
const rapor: string[] = [];
let cikti = "";
let kirmizi = 0;
let uyari = 0;
function yaz_rapor() {
  if (cikti) writeFileSync(join(cikti, "arama-gocu-raporu.md"), rapor.join("\n") + "\n", "utf8");
}
function adim(ad: string, gecti: boolean, ayrinti = ""): boolean {
  const s = `${gecti ? "✓" : "✗"} ${ad}${ayrinti ? ` — ${ayrinti}` : ""}`;
  console.log(s);
  rapor.push(`- ${s}`);
  if (!gecti) kirmizi++;
  yaz_rapor();
  return gecti;
}
function uyar(ad: string, ayrinti = "") {
  const s = `⚠ ${ad}${ayrinti ? ` — ${ayrinti}` : ""}`;
  console.log(s);
  rapor.push(`- ${s}`);
  uyari++;
  yaz_rapor();
}
function dur(neden: string): never {
  rapor.push(``, `**DURDU:** ${neden}`);
  yaz_rapor();
  console.error(`DURDU: ${neden}${cikti ? `\nrapor: ${join(cikti, "arama-gocu-raporu.md")}` : ""}`);
  process.exit(1);
}
function bolum(baslik: string) {
  console.log(`\n── ${baslik}`);
  rapor.push(``, `## ${baslik}`, ``);
}

// ── fts5 tanımının çözümü ───────────────────────────────────────────────────
// `USING fts5(...)` içindeki argümanlar: üst düzey virgülde bölünür; tırnak ve parantez korunur.
function argumanlar(sql: string): string[] {
  const m = /USING\s+fts5\s*\(/i.exec(sql);
  if (!m) throw new Error(`fts5 tanımı okunamadı: ${sql}`);
  const sonuc: string[] = [];
  let parca = "";
  let derinlik = 0;
  let tirnak = "";
  for (let i = m.index + m[0].length; i < sql.length; i++) {
    const c = sql[i];
    if (tirnak) {
      parca += c;
      if (c === tirnak) {
        if (tirnak !== "]" && sql[i + 1] === tirnak) parca += sql[++i];
        else tirnak = "";
      }
      continue;
    }
    if (c === "'" || c === '"' || c === "`" || c === "[") {
      tirnak = c === "[" ? "]" : c;
      parca += c;
      continue;
    }
    if (c === "(") derinlik++;
    if (c === ")") {
      if (derinlik === 0) {
        sonuc.push(parca.trim());
        return sonuc.filter((x) => x !== "");
      }
      derinlik--;
    }
    if (c === "," && derinlik === 0) {
      sonuc.push(parca.trim());
      parca = "";
      continue;
    }
    parca += c;
  }
  throw new Error(`fts5 tanımında kapanmayan parantez: ${sql}`);
}
function tirnaksiz(v: string): string {
  const t = v.trim();
  const a = t[0];
  if ((a === "'" || a === '"' || a === "`") && t.endsWith(a)) return t.slice(1, -1).replaceAll(a + a, a);
  if (a === "[" && t.endsWith("]")) return t.slice(1, -1);
  return t;
}
const SECENEK = /^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([\s\S]+)$/;

interface Plan {
  ad: string;
  kip: "dis" | "icerikli";
  sql: string;
  sutun_tanimlari: string[]; // ham, UNINDEXED ve tırnaklar korunur
  sutunlar: string[]; // adlar (PRAGMA table_info)
  secenekler: string[]; // ham, content/content_rowid hariç
  kaynak: string | null;
  kaynak_rowid: string;
  anahtar: string | null; // eklenecek ya da zaten var olan anahtar sütunu
  anahtar_ekle: boolean;
}
interface Tetikleyici {
  name: string;
  tbl_name: string;
  sql: string;
}

function ad_deseni(adlar: string[]): RegExp {
  const kac = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const govde = adlar.map((a) => `${kac(a)}(?:_(?:${GOLGE.join("|")}))?`).join("|");
  return new RegExp(`(^|[^A-Za-z0-9_])(${govde})($|[^A-Za-z0-9_])`, "i");
}

// Dosyanın sanal tablolarını, fts5 planlarını ve fts5'e dokunan nesneleri çıkarır. Yazmaz.
function kesif(db: Database, etiket: string): { planlar: Plan[]; tetikleyiciler: Tetikleyici[] } {
  const sanal = db.query("SELECT name, sql FROM main.sqlite_master WHERE type = 'table' AND sql LIKE 'CREATE VIRTUAL TABLE%'").all() as { name: string; sql: string }[];
  const planlar: Plan[] = [];
  for (const { name, sql } of sanal) {
    const modul = /USING\s+([A-Za-z0-9_]+)/i.exec(sql)?.[1]?.toLowerCase();
    if (modul !== "fts5") dur(`${etiket}: ${name} fts5 değil (${modul}); ADR 0006 yalnız fts5'i taşır`);
    const parcalar = argumanlar(sql);
    const tanimlar = parcalar.filter((p) => !SECENEK.test(p));
    const secenek = new Map<string, string>();
    const korunan: string[] = [];
    for (const p of parcalar.filter((p) => SECENEK.test(p))) {
      const [, k, v] = SECENEK.exec(p)!;
      secenek.set(k.toLowerCase(), v);
      if (!["content", "content_rowid"].includes(k.toLowerCase())) korunan.push(p);
    }
    const sutunlar = (db.query(`PRAGMA main.table_info(${q(name)})`).all() as { name: string }[]).map((s) => s.name);
    if (sutunlar.length !== tanimlar.length) dur(`${etiket}: ${name} sütun sayısı tutmuyor (tanım ${tanimlar.length}, table_info ${sutunlar.length})`);
    const icerik = secenek.has("content") ? tirnaksiz(secenek.get("content")!) : null;
    if (icerik === "") dur(`${etiket}: ${name} içeriksiz fts5 (content=''); kaynak metin yok, taşınamaz`);
    const plan: Plan = { ad: name, kip: icerik ? "dis" : "icerikli", sql, sutun_tanimlari: tanimlar, sutunlar, secenekler: korunan, kaynak: icerik, kaynak_rowid: "rowid", anahtar: null, anahtar_ekle: false };
    if (icerik) {
      plan.kaynak_rowid = secenek.has("content_rowid") ? tirnaksiz(secenek.get("content_rowid")!) : "rowid";
      const bilgi = db.query(`PRAGMA main.table_info(${q(icerik)})`).all() as { name: string; pk: number }[];
      if (bilgi.length === 0) dur(`${etiket}: ${name} dış içerik tablosu ${icerik} yok`);
      const var_mi = (s: string) => bilgi.some((b) => b.name.toLowerCase() === s.toLowerCase());
      for (const s of sutunlar) if (!var_mi(s)) dur(`${etiket}: ${name}.${s} sütunu ${icerik} tablosunda yok`);
      if (plan.kaynak_rowid.toLowerCase() !== "rowid" && !var_mi(plan.kaynak_rowid)) dur(`${etiket}: ${name} content_rowid ${plan.kaynak_rowid} ${icerik} tablosunda yok`);
      const pk = bilgi.filter((b) => b.pk > 0);
      if (pk.length > 1) dur(`${etiket}: ${icerik} bileşik anahtarlı; ADR 0006 tek sütunlu anahtar varsayar`);
      plan.anahtar = pk[0]?.name ?? null;
      plan.anahtar_ekle = plan.anahtar !== null && !sutunlar.some((s) => s.toLowerCase() === plan.anahtar!.toLowerCase());
    }
    planlar.push(plan);
  }
  if (planlar.length === 0) return { planlar, tetikleyiciler: [] };
  const desen = ad_deseni(planlar.map((p) => p.ad));
  const nesneler = db.query("SELECT type, name, tbl_name, sql FROM main.sqlite_master WHERE type IN ('trigger', 'view') AND sql IS NOT NULL").all() as (Tetikleyici & { type: string })[];
  const gorunum = nesneler.filter((n) => n.type === "view" && desen.test(n.sql));
  if (gorunum.length) dur(`${etiket}: fts5'e dayanan görünüm var (${gorunum.map((g) => g.name).join(", ")}); görünüm başka dosyadaki tabloya bakamaz, önce elle karar verilmeli`);
  const tetikleyiciler = nesneler.filter((n) => n.type === "trigger" && (desen.test(n.sql) || desen.test(n.tbl_name)));
  return { planlar, tetikleyiciler };
}

// ── Satır seçimleri ve damga ─────────────────────────────────────────────────
function rowid_ifadesi(s: string): string {
  return s.toLowerCase() === "rowid" ? "rowid" : q(s);
}
function damga_sutunlari(p: Plan): string[] {
  return p.anahtar_ekle ? [...p.sutunlar, p.anahtar!] : p.sutunlar;
}
// Kaynak satırları: dış içerikte T'den, içerikli tabloda fts5 tablosunun kendisinden.
function kaynak_secimi(p: Plan, sema: string): string {
  const s = damga_sutunlari(p).map(q).join(", ");
  if (p.kip === "dis") return `SELECT ${rowid_ifadesi(p.kaynak_rowid)}, ${s} FROM ${sema}.${q(p.kaynak!)} ORDER BY ${rowid_ifadesi(p.kaynak_rowid)}`;
  return `SELECT rowid, ${s} FROM ${sema}.${q(p.ad)} ORDER BY rowid`;
}
function arama_secimi(p: Plan, sema: string): string {
  return `SELECT rowid, ${damga_sutunlari(p).map(q).join(", ")} FROM ${sema}.${q(p.ad)} ORDER BY rowid`;
}
function damga(db: Database, sql: string): { n: number; ozet: string } {
  const h = createHash("sha256");
  let n = 0;
  for (const r of db.query(sql).values()) {
    h.update(JSON.stringify(r) + "\n", "utf8");
    n++;
  }
  return { n, ozet: h.digest("hex") };
}
function arama_tanimi(p: Plan, sema: string): string {
  const parcalar = [...p.sutun_tanimlari, ...(p.anahtar_ekle ? [`${q(p.anahtar!)} UNINDEXED`] : []), ...p.secenekler];
  return `CREATE VIRTUAL TABLE ${sema}.${q(p.ad)} USING fts5(${parcalar.join(", ")})`;
}
function doldur(db: Database, p: Plan, sema: string) {
  const hedef = ["rowid", ...damga_sutunlari(p)].map((s) => (s === "rowid" ? s : q(s))).join(", ");
  db.run(`INSERT INTO ${sema}.${q(p.ad)}(${hedef}) ${kaynak_secimi(p, "main")}`);
}
// Dizinlenen sütunlar: tanımda UNINDEXED olmayanlar (eklenen anahtar dahil değil).
function dizinli_sutunlar(db: Database, sema: string, tablo: string): string[] {
  const sql = (db.query(`SELECT sql FROM ${sema}.sqlite_master WHERE name = ?`).get(tablo) as { sql: string }).sql;
  const tanimlar = argumanlar(sql).filter((p) => !SECENEK.test(p));
  const adlar = (db.query(`PRAGMA ${sema}.table_info(${q(tablo)})`).all() as { name: string }[]).map((s) => s.name);
  return adlar.filter((_, i) => !/\bUNINDEXED\b/i.test(tanimlar[i] ?? ""));
}
// Dizinlenen sütunlardaki ilk sözcük (3+ harf; birleşik işaretler sözcüğün parçası, unicode61'deki
// gibi). Metin yoksa null.
function ornek_soz(db: Database, tablo_ifadesi: string, sutunlar: string[]): string | null {
  if (sutunlar.length === 0) return null;
  for (const r of db.query(`SELECT ${sutunlar.map(q).join(", ")} FROM ${tablo_ifadesi} ORDER BY rowid LIMIT 20`).values()) {
    for (const v of r) {
      const soz = typeof v === "string" ? /[\p{L}\p{N}\p{M}]{3,}/u.exec(v)?.[0] : undefined;
      if (soz) return soz;
    }
  }
  return null;
}
function match_sayisi(db: Database, tablo_ifadesi: string, ad: string, soz: string): number {
  return (db.query(`SELECT count(*) AS n FROM ${tablo_ifadesi} WHERE ${q(ad)} MATCH ?`).get(`"${soz}"`) as { n: number }).n;
}
// Örnek sözcük kendi satırını bulmalı. Döner: rapor metni ve geçip geçmediği.
function match_denetimi(db: Database, tablo_ifadesi: string, ad: string, sutunlar: string[]): { gecti: boolean; metin: string } {
  const soz = ornek_soz(db, tablo_ifadesi, sutunlar);
  if (!soz) return { gecti: true, metin: "metin yok, MATCH denenmedi" };
  const n = match_sayisi(db, tablo_ifadesi, ad, soz);
  return { gecti: n > 0, metin: `"${soz}": ${n} satır` };
}

// ── Göç ─────────────────────────────────────────────────────────────────────
// Bir katalog dosyasını göç ettirir; arama dosyası yoksa yaratılır. Hata olursa işlem geri alınır
// ve bu çağrının yarattığı arama dosyası silinir; katalog değişmez.
function goc(katalog: string, arama: string, etiket: string) {
  if (existsSync(arama)) dur(`${etiket}: ${arama} zaten var`);
  const db = new Database(katalog);
  let islemde = false;
  try {
    db.run("PRAGMA busy_timeout = 5000");
    const { planlar, tetikleyiciler } = kesif(db, etiket);
    if (planlar.length === 0) dur(`${etiket}: sanal tablo yok, göç edilecek bir şey yok`);
    for (const p of planlar) {
      const kaynak = p.kip === "dis" ? `kaynak ${p.kaynak}, rowid ${p.kaynak_rowid}, anahtar ${p.anahtar ?? "YOK"}${p.anahtar_ekle ? " (UNINDEXED eklenecek)" : ""}` : "kaynaksız: satırlar aynen taşınır";
      adim(`${etiket}: ${p.ad} (${p.kip === "dis" ? "dış içerikli" : "içerikli"})`, true, `${kaynak}; sütunlar ${p.sutunlar.join(", ")}${p.secenekler.length ? `; seçenekler ${p.secenekler.join(", ")}` : ""}`);
      if (p.kip === "dis" && !p.anahtar) uyar(`${etiket}: ${p.kaynak} açık birincil anahtarsız`, "birleştirme yalnız rowid ile yapılabilir");
    }
    rapor.push(``, `Düşecek tetikleyiciler (${tetikleyiciler.length}):`, ``, "```sql", ...tetikleyiciler.map((t) => `${t.sql};`), "```", ``);
    console.log(`düşecek tetikleyiciler: ${tetikleyiciler.map((t) => t.name).join(", ") || "yok"}`);

    db.run(`ATTACH '${resolve(arama).replaceAll("'", "''")}' AS ara`);
    db.run("BEGIN IMMEDIATE");
    islemde = true;
    db.run(`CREATE TABLE ara.${META} (tablo TEXT PRIMARY KEY, kip TEXT NOT NULL, kaynak TEXT, kaynak_rowid TEXT NOT NULL, anahtar TEXT, sutunlar TEXT NOT NULL, damga TEXT NOT NULL, kuruldu TEXT NOT NULL)`);
    for (const p of planlar) {
      db.run(arama_tanimi(p, "ara"));
      const yeni_sql = (db.query("SELECT sql FROM ara.sqlite_master WHERE name = ?").get(p.ad) as { sql: string }).sql;
      const yeni_secenek = argumanlar(yeni_sql).filter((x) => SECENEK.test(x)).sort().join(", ");
      if (!adim(`${etiket}: ${p.ad} seçenekleri korundu`, yeni_secenek === [...p.secenekler].sort().join(", "), yeni_secenek || "seçenek yok")) throw new Error(`${p.ad} seçenekleri tutmadı`);
      doldur(db, p, "ara");
      const k = damga(db, kaynak_secimi(p, "main"));
      const a = damga(db, arama_secimi(p, "ara"));
      if (!adim(`${etiket}: ${p.ad} dolduruldu, içerik özeti aynı`, k.n === a.n && k.ozet === a.ozet, `${k.n} → ${a.n} satır; ${k.ozet.slice(0, 12)} / ${a.ozet.slice(0, 12)}`)) throw new Error(`${p.ad} içerik özeti tutmadı`);
      const t = q(p.ad);
      db.run(`INSERT INTO ara.${t}(${t}) VALUES('integrity-check')`);
      const m = match_denetimi(db, `ara.${t}`, p.ad, dizinli_sutunlar(db, "ara", p.ad));
      if (!adim(`${etiket}: ${p.ad} fts5 integrity-check ve MATCH`, m.gecti, m.metin)) throw new Error(`${p.ad}: ${m.metin}`);
      // Aynı sözcük eski dizinde: fark tokenizer seçeneğinin kaybını ya da eski dizinin
      // bayatlığını gösterir (Turso yazışları fts5 tetikleyicilerini atlamış olabilir). Durdurmaz.
      const soz = ornek_soz(db, `ara.${t}`, dizinli_sutunlar(db, "ara", p.ad));
      if (soz) {
        const eski = match_sayisi(db, `main.${t}`, p.ad, soz);
        const yeni = match_sayisi(db, `ara.${t}`, p.ad, soz);
        if (eski !== yeni) uyar(`${etiket}: ${p.ad} "${soz}" eski dizinde ${eski}, yenide ${yeni} satır`, "eski dizin bayat olabilir; yeni dizin kaynaktan kuruldu");
      }
      db.query(`INSERT INTO ara.${META} VALUES (?, ?, ?, ?, ?, ?, ?, ?)`).run(p.ad, p.kip, p.kaynak, p.kaynak_rowid, p.anahtar, JSON.stringify(damga_sutunlari(p)), k.ozet, new Date().toISOString());
    }

    // Tetikleyiciler ve tablolar aynı işlemde düşer; fts5'e dokunan bir tetikleyici kalırsa ana
    // tabloya her yazış düşer (ADR 0006). Aşağıdaki deneme yazışı bunu sınar.
    for (const t of tetikleyiciler) db.run(`DROP TRIGGER main.${q(t.name)}`);
    for (const p of planlar) db.run(`DROP TABLE main.${q(p.ad)}`);
    const kalan = db.query("SELECT name FROM main.sqlite_master WHERE sql LIKE 'CREATE VIRTUAL TABLE%'").all() as { name: string }[];
    const golge = (db.query("SELECT name FROM main.sqlite_master WHERE type = 'table'").all() as { name: string }[]).filter((t) => planlar.some((p) => GOLGE.some((g) => t.name.toLowerCase() === `${p.ad}_${g}`.toLowerCase())));
    if (!adim(`${etiket}: ana katalogda sanal ve gölge tablo kalmadı`, kalan.length === 0 && golge.length === 0, [...kalan, ...golge].map((x) => x.name).join(", ") || "yok")) throw new Error("ana katalogda fts5 kaldı");

    // Tetikleyicisi düşen her tabloya deneme yazışı: hata vermemeli, sonra geri alınır.
    for (const tablo of new Set(tetikleyiciler.map((t) => t.tbl_name))) {
      const s = (db.query(`PRAGMA main.table_info(${q(tablo)})`).all() as { name: string }[])[0]?.name;
      if (!s) continue; // tablo fts5'in kendisiydi
      db.run("SAVEPOINT yazis_denemesi");
      let hata = "";
      try {
        db.run(`UPDATE main.${q(tablo)} SET ${q(s)} = ${q(s)}`);
      } catch (e) {
        hata = (e as Error).message;
      }
      db.run("ROLLBACK TO yazis_denemesi");
      db.run("RELEASE yazis_denemesi");
      if (!adim(`${etiket}: ${tablo} tablosuna yazış düşmüyor`, hata === "", hata || "UPDATE geçti, geri alındı")) throw new Error(`${tablo}: ${hata}`);
    }
    db.run("COMMIT");
    islemde = false;
    adim(`${etiket}: işlem tamamlandı (COMMIT)`, true);
  } catch (e) {
    if (islemde) {
      try {
        db.run("ROLLBACK");
      } catch {
        // SQLite bazı hatalarda işlemi kendisi geri alır; ikinci ROLLBACK hata verir.
      }
    }
    db.close();
    rmSync(arama, { force: true });
    rmSync(`${arama}-journal`, { force: true });
    dur(`${etiket}: ${(e as Error).message} — işlem geri alındı, katalog değişmedi`);
  }
  db.close();
  const kb = butunluk(katalog);
  if (!adim(`${etiket}: ana katalog integrity_check`, kb === "ok", kb.slice(0, 300))) dur(`${etiket}: göç sonrası bütünlük ok değil`);
  const ab = butunluk(arama);
  if (!adim(`${etiket}: arama dosyası integrity_check`, ab === "ok", ab.slice(0, 300))) dur(`${etiket}: arama dosyası bütünlüğü ok değil`);
}

// Python okurunun yapacağı gibi: kataloğu aç, arama dosyasını ATTACH et, nitelemesiz MATCH.
function okur_denetimi(katalog: string, arama: string, etiket: string) {
  const db = new Database(katalog, { readonly: true });
  try {
    db.run(`ATTACH '${resolve(arama).replaceAll("'", "''")}' AS arama`);
    for (const { tablo } of db.query(`SELECT tablo FROM arama.${META} ORDER BY tablo`).all() as { tablo: string }[]) {
      // Tablo adı nitelemesiz: Python okurunun sorgusu gibi.
      const m = match_denetimi(db, q(tablo), tablo, dizinli_sutunlar(db, "arama", tablo));
      adim(`${etiket}: ${tablo} ATTACH ile nitelemesiz MATCH`, m.gecti, m.metin);
    }
  } finally {
    db.close();
  }
}

// Turso (`yaz`) her tabloyu görüyor mu? Satır döndürmeyen SELECT: salt-okur.
function turso_gorunurlugu(katalog: string, etiket: string, beklenen: boolean) {
  if (!YAZ) return rapor.push(`- ölçülmedi: Turso görünürlüğü (${etiket}; --ikili verilmedi)`);
  const tablolar = ro_sorgu<{ name: string }>(katalog, "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND sql NOT LIKE 'CREATE VIRTUAL TABLE%'").map((t) => t.name);
  const gormeyen = tablolar.filter((t) => kos([YAZ, katalog, `SELECT 1 FROM ${q(t)} WHERE 0`]).kod !== 0);
  const ayrinti = `${tablolar.length - gormeyen.length}/${tablolar.length} tablo${gormeyen.length ? `; görmediği: ${gormeyen.join(", ")}` : ""}`;
  if (beklenen) adim(`${etiket}: Turso bütün tabloları görüyor`, gormeyen.length === 0, ayrinti);
  else rapor.push(`- bilgi: ${etiket}: Turso görünürlüğü göçten önce — ${ayrinti}`);
}

// ── Tazele ──────────────────────────────────────────────────────────────────
interface Meta {
  tablo: string;
  kip: string;
  kaynak: string | null;
  kaynak_rowid: string;
  anahtar: string | null;
  sutunlar: string;
  damga: string;
}
// Arama dosyasının meta satırları; dosya yoksa, SQLite değilse ya da meta yoksa null.
function meta_oku(arama: string): Meta[] | null {
  if (!existsSync(arama)) return null;
  try {
    const var_mi = ro_sorgu<{ n: number }>(arama, `SELECT count(*) AS n FROM sqlite_master WHERE name = '${META}'`)[0].n;
    return var_mi ? ro_sorgu<Meta>(arama, `SELECT * FROM ${META} ORDER BY tablo`) : null;
  } catch {
    return null; // yarım kalmış göçün artığı olabilir; çağıran "tanınmıyor" der
  }
}
function tazele(katalog: string, arama: string): number {
  const kat = new Database(katalog, { readonly: true });
  const ara = new Database(arama);
  let yenilenen = 0;
  try {
    ara.run("PRAGMA busy_timeout = 5000");
    for (const m of ara.query(`SELECT * FROM ${META} ORDER BY tablo`).all() as Meta[]) {
      if (m.kip !== "dis") {
        console.log(`· ${m.tablo}: içerikli (kaynaksız), dokunulmadı`);
        continue;
      }
      const sutunlar = JSON.parse(m.sutunlar) as string[];
      const p = { ad: m.tablo, kip: "dis", kaynak: m.kaynak, kaynak_rowid: m.kaynak_rowid, sutunlar, anahtar_ekle: false } as Plan;
      const k = damga(kat, kaynak_secimi(p, "main"));
      if (k.ozet === m.damga) {
        console.log(`✓ ${m.tablo}: taze (${k.n} satır)`);
        continue;
      }
      const satirlar = kat.query(kaynak_secimi(p, "main")).values();
      const ekle = ara.query(`INSERT INTO ${q(m.tablo)}(rowid, ${sutunlar.map(q).join(", ")}) VALUES (${["?", ...sutunlar.map(() => "?")].join(", ")})`);
      ara.transaction(() => {
        ara.run(`DELETE FROM ${q(m.tablo)}`);
        for (const r of satirlar) ekle.run(...(r as never[]));
        const a = damga(ara, arama_secimi(p, "main"));
        if (a.ozet !== k.ozet) throw new Error(`${m.tablo}: yeniden kurulan içerik kaynakla tutmuyor`);
        ara.query(`UPDATE ${META} SET damga = ?, kuruldu = ? WHERE tablo = ?`).run(k.ozet, new Date().toISOString(), m.tablo);
      })();
      ara.run(`INSERT INTO ${q(m.tablo)}(${q(m.tablo)}) VALUES('integrity-check')`);
      console.log(`↻ ${m.tablo}: yeniden kuruldu (${k.n} satır)`);
      yenilenen++;
    }
  } finally {
    kat.close();
    ara.close();
  }
  return yenilenen;
}

// ── Akış ────────────────────────────────────────────────────────────────────
if (!existsSync(katalog_yolu)) {
  console.error(`${katalog_yolu} yok`);
  process.exit(1);
}
const sanal_sayisi = ro_sorgu<{ n: number }>(katalog_yolu, "SELECT count(*) AS n FROM sqlite_master WHERE sql LIKE 'CREATE VIRTUAL TABLE%'")[0].n;
const gercek_meta = meta_oku(arama_yolu);

if (tazele_kipi) {
  if (sanal_sayisi > 0 || !gercek_meta) {
    console.error(`tazele: göç yapılmamış (${KATALOG}.db'de ${sanal_sayisi} sanal tablo, ${ARAMA}.db ${gercek_meta ? "var" : "yok ya da tanınmıyor"})`);
    process.exit(1);
  }
  try {
    const n = tazele(katalog_yolu, arama_yolu);
    console.log(n ? `${n} tablo yenilendi` : "arama dosyası taze");
    process.exit(0);
  } catch (e) {
    console.error(`tazele: ${(e as Error).message}`);
    process.exit(1);
  }
}

if (sanal_sayisi === 0) {
  if (gercek_meta) {
    console.log(`göç zaten yapılmış: ${KATALOG}.db'de sanal tablo yok, ${ARAMA}.db'de ${gercek_meta.length} tablo. Tazelik için --tazele.`);
    process.exit(0);
  }
  console.error(`${KATALOG}.db'de sanal tablo yok ve ${ARAMA}.db ${existsSync(arama_yolu) ? "tanınmıyor" : "yok"}. Göçe gerek yok ya da önceki göç yarıda kaldı: önceki raporun yedeğine bakın.`);
  process.exit(1);
}

const damga_zaman = new Date().toISOString().replace(/[:.]/g, "-");
cikti = resolve(arg("--cikti") ?? `arama-gocu-${damga_zaman}`);
mkdirSync(cikti, { recursive: true });
rapor.push(`# ADR 0006 arama dosyası göçü`, ``, `- zaman: ${new Date().toISOString()}`, `- dizin: ${dizin}`, `- kip: ${gercek ? "GERÇEK" : "PROVA"}`, ``);

bolum("Ön koşullar");
try {
  const d = new Database(":memory:");
  d.run("CREATE VIRTUAL TABLE t USING fts5(a)");
  d.close();
  adim("bun:sqlite fts5", true);
} catch (e) {
  dur(`bun:sqlite fts5 desteklemiyor: ${(e as Error).message}`);
}
if (YAZ && !adim("yaz (örnek) ikilisi", existsSync(YAZ), YAZ)) dur(`${YAZ} yok; önce: cargo build --release -p fihrist-canli --examples`);
const s3 = kos(["sqlite3", "-version"]);
if (!adim("sqlite3 CLI", s3.kod === 0, s3.cikti.split(" ")[0])) dur("sqlite3 bulunamadı (.backup için gerekli)");
if (WIN) {
  const t = kos(["tasklist", "/FO", "CSV", "/NH"]);
  const acik = ["ibnunnedim.exe", "fihrist-izle.exe", "python.exe", "pythonw.exe"].filter((a) => t.cikti.toLowerCase().includes(`"${a}"`));
  if (!adim("kataloğu tutan süreç yok", acik.length === 0, acik.join(", ") || "yok")) dur(`önce durdurun (mevcut başlatma betikleriyle): ${acik.join(", ")}`);
} else rapor.push("- ölçülmedi: açık süreç denetimi (yalnız Windows'ta)");
if (existsSync(arama_yolu)) uyar(`${ARAMA}.db zaten var ama katalogda hâlâ fts5 var`, "yarım kalmış bir göçün artığı; GERÇEK evrede kenara alınır (silinmez)");

bolum("Yedek (salt-okunur .backup)");
const yedek = join(cikti, "yedek");
mkdirSync(yedek, { recursive: true });
const ilk_sha = sha(katalog_yolu);
const yedek_yolu = join(yedek, `${KATALOG}.db`);
const b = kos(["sqlite3", uri(katalog_yolu), `.backup '${yedek_yolu.replaceAll("\\", "/")}'`]);
if (!adim(`${KATALOG}: yedek alındı`, b.kod === 0 && existsSync(yedek_yolu), b.cikti)) dur("yedek alınamadı");
const yb = butunluk(yedek_yolu);
if (!adim(`${KATALOG}: integrity_check (yedekte)`, yb === "ok", yb.slice(0, 300))) dur("yedeğin bütünlüğü ok değil; göçten önce onarılmalı (ADR 0006, ölçülmedi bölümü)");
if (!adim(`${KATALOG}: yedek alırken gerçek dosya değişmedi`, sha(katalog_yolu) === ilk_sha, ilk_sha)) dur("katalog yedek sırasında değişti (açık yazan var mı?)");

bolum("PROVA (kopyada)");
const prova = join(cikti, "prova");
mkdirSync(prova, { recursive: true });
const prova_katalog = join(prova, `${KATALOG}.db`);
const prova_arama = join(prova, `${ARAMA}.db`);
copyFileSync(yedek_yolu, prova_katalog);
turso_gorunurlugu(prova_katalog, "prova", false);
goc(prova_katalog, prova_arama, "prova");
okur_denetimi(prova_katalog, prova_arama, "prova");
turso_gorunurlugu(prova_katalog, "prova", true);
// Tazelik: önce taze olmalı; kaynak değişince (yalnız kopyada) yeniden kurulmalı.
function tazele_say(): number | string {
  try {
    return tazele(prova_katalog, prova_arama);
  } catch (e) {
    return (e as Error).message;
  }
}
const dis = (meta_oku(prova_arama) ?? []).filter((m) => m.kip === "dis");
const ilk_tazele = tazele_say();
adim("prova: --tazele göçten hemen sonra iş yapmıyor", ilk_tazele === 0, String(ilk_tazele));
if (dis.length) {
  const m = dis[0];
  const s = (JSON.parse(m.sutunlar) as string[])[0];
  const d = new Database(prova_katalog);
  d.run(`UPDATE ${q(m.kaynak!)} SET ${q(s)} = coalesce(${q(s)}, '') || ' tazelik' WHERE ${rowid_ifadesi(m.kaynak_rowid)} = (SELECT min(${rowid_ifadesi(m.kaynak_rowid)}) FROM ${q(m.kaynak!)})`);
  d.close();
  const ikinci = tazele_say();
  adim(`prova: kaynak değişince --tazele yeniden kuruyor (${m.tablo})`, ikinci === 1, String(ikinci));
  const ucuncu = tazele_say();
  adim("prova: ikinci --tazele iş yapmıyor", ucuncu === 0, String(ucuncu));
} else rapor.push("- ölçülmedi: --tazele yeniden kurulumu (dış içerikli tablo yok)");
if (kirmizi > 0) dur(`provada ${kirmizi} kırmızı`);

if (gercek) {
  bolum("GERÇEK");
  if (!adim(`${KATALOG}: provadan beri değişmedi`, sha(katalog_yolu) === ilk_sha)) dur("katalog yedekten sonra değişti; yeniden koşun");
  if (existsSync(arama_yolu)) {
    const eski = `${arama_yolu}.eski-${damga_zaman}`;
    renameSync(arama_yolu, eski);
    for (const ek of ["-wal", "-shm", "-journal"]) if (existsSync(arama_yolu + ek)) renameSync(arama_yolu + ek, eski + ek);
    uyar(`önceki ${ARAMA}.db kenara alındı`, eski);
  }
  goc(katalog_yolu, arama_yolu, "gerçek");
  okur_denetimi(katalog_yolu, arama_yolu, "gerçek");
  turso_gorunurlugu(katalog_yolu, "gerçek", true);
  rapor.push("", `Geri dönüş: ${KATALOG}.db'yi \`${yedek_yolu}\` ile değiştirin, ${ARAMA}.db'yi silin.`);
} else rapor.push("", "GERÇEK evre koşulmadı (`--gercek` verilmedi).");

rapor.push("", `**SONUÇ:** ${kirmizi === 0 ? "YEŞİL" : `${kirmizi} kırmızı`}${uyari ? ` · ${uyari} UYARI (yukarıda ⚠)` : ""}`, "", `Yedek: ${yedek_yolu}`);
yaz_rapor();
console.log(`\nSONUÇ: ${kirmizi === 0 ? "YEŞİL" : `${kirmizi} kırmızı`}${uyari ? ` · ${uyari} UYARI` : ""} — rapor: ${join(cikti, "arama-gocu-raporu.md")}`);
process.exit(kirmizi === 0 ? 0 : 1);
