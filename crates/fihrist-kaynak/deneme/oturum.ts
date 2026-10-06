// Dışarıdan oturum kanıtı (G-3): derlenmiş `ibnunnedim mcp`yi stdio'dan sürer.
//
// Kullanım:
//   bun oturum.ts <ibnunnedim ikilisi> <fihrist-izle ikilisi> [--hizmetsiz]
//
// --hizmetsiz: FIHRIST_HIZMETSIZ=1 (süreç içi). Verilmezse varsayılan yol:
// `mcp` tek hizmete aktarır (yoksa başlatır). Hizmet, bu betiğin seçtiği boş
// bir portta (FIHRIST_HIZMET_PORT) başlar ve sonda öldürülür.
//
// Geçici dizinde boş ama geçerli bir `kutup_kutuphane.db` (TURSO_DB_PATH) ve
// sentetik `kutup_kayitlar.db` kurulur; tetikleyiciler `fihrist-izle --kur`
// ile. Yazışlar ayrı süreçten, `python3 -I` ile. Her adım denetlenir; biri
// tutmazsa çıkış kodu 1.

import { mkdtempSync, readFileSync, existsSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [ikili, izle] = [process.argv[2], process.argv[3]];
const hizmetsiz = process.argv.includes("--hizmetsiz");
if (!ikili || !izle) {
  console.error("kullanım: bun oturum.ts <ibnunnedim> <fihrist-izle> [--hizmetsiz]");
  process.exit(2);
}

const basla = performance.now();
const an = () => (performance.now() - basla).toFixed(0).padStart(6, " ");
let hatali = 0;
function denetle(kosul: boolean, adim: string, ayrinti = "") {
  console.log(`${an()} ms ${kosul ? "✓" : "✗"} ${adim}${ayrinti ? " — " + ayrinti : ""}`);
  if (!kosul) hatali++;
}
function bitir(): never {
  console.log(`\nSONUÇ: ${hatali === 0 ? "GEÇTİ" : `DÜŞTÜ (${hatali} adım)`}`);
  process.exit(hatali === 0 ? 0 : 1);
}

function python(kod: string, ...arg: string[]) {
  const s = Bun.spawnSync(["python3", "-I", "-c", kod, ...arg]);
  if (s.exitCode !== 0) {
    throw new Error(`python3 (${arg.join(" ")}): ${s.stderr.toString()}`);
  }
}

// ── Kurulum ────────────────────────────────────────────────────────────────
const dizin = mkdtempSync(join(tmpdir(), "g3-oturum-"));
const katalog = join(dizin, "kutup_kutuphane.db");
const kayitlar = join(dizin, "kutup_kayitlar.db");
python(
  "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('CREATE TABLE yetenekler (id TEXT PRIMARY KEY, ad TEXT NOT NULL, aciklama TEXT, tam_metin_md TEXT, basari_puani_ort REAL, kategori TEXT)'); c.commit(); c.close()",
  katalog,
);
// arac_cagrilari: ibnunnedim-cli `kayitlari_al`daki şemanın aynısı.
python(
  "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('CREATE TABLE arac_cagrilari (kimlik TEXT PRIMARY KEY, oturum TEXT NOT NULL, proje TEXT NOT NULL, arac TEXT NOT NULL, ozet TEXT NOT NULL, hata INTEGER NOT NULL, baslangic TEXT NOT NULL, sure_ms INTEGER)'); c.commit(); c.close()",
  kayitlar,
);
console.log(`dizin: ${dizin}`);

// fihrist-izle --kur kurar, sonra izlemeye geçer: "kuruldu" görülünce durdurulur.
{
  const k = Bun.spawn([izle, kayitlar, "--kur"], { stdout: "pipe", stderr: "pipe" });
  const okur = k.stderr.getReader();
  let hata = "";
  const son = Date.now() + 15000;
  // Bekleyen okuma tek tutulur: uyku kazanınca yeni read() açılırsa eskisinin
  // getirdiği parça kaybolur (ikili 500 ms'den geç yazınca betik düşerdi).
  let bekleyen = okur.read();
  while (!hata.includes("izleniyor:") && Date.now() < son) {
    const r = await Promise.race([bekleyen, Bun.sleep(500).then(() => null)]);
    if (r === null) continue;
    if (r.done) break;
    if (r.value) hata += new TextDecoder().decode(r.value);
    bekleyen = okur.read();
  }
  k.kill();
  await k.exited;
  denetle(hata.includes("kuruldu: 1 tablo izleniyor (arac_cagrilari)"), "fihrist-izle --kur", hata.trim().replaceAll("\n", " | "));
}

// ── Sunucu ─────────────────────────────────────────────────────────────────
function bosPort(): number {
  // /proc/net/tcp{,6}: yerel adresin portu onaltılık, durum 0A = LISTEN.
  const dinlenen = new Set<number>();
  for (const f of ["/proc/net/tcp", "/proc/net/tcp6"]) {
    if (!existsSync(f)) continue;
    for (const satir of readFileSync(f, "utf8").split("\n").slice(1)) {
      const s = satir.trim().split(/\s+/);
      if (s[3] === "0A") dinlenen.add(parseInt(s[1].split(":")[1], 16));
    }
  }
  for (;;) {
    const p = 20000 + Math.floor(Math.random() * 9000);
    if (!dinlenen.has(p)) return p;
  }
}
const port = bosPort();
const ortam: Record<string, string> = {
  ...(process.env as Record<string, string>),
  TURSO_DB_PATH: katalog,
  FIHRIST_HIZMET_PORT: String(port),
};
delete ortam.FIHRIST_HIZMETSIZ;
if (hizmetsiz) ortam.FIHRIST_HIZMETSIZ = "1";
console.log(`yol: ${hizmetsiz ? "FIHRIST_HIZMETSIZ=1 (süreç içi)" : `hizmete aktarım (port ${port})`}`);

const sunucu = Bun.spawn([ikili, "mcp"], {
  env: ortam,
  stdin: "pipe",
  stdout: "pipe",
  stderr: "pipe",
});
let sunucuHata = "";
(async () => {
  for await (const p of sunucu.stderr) sunucuHata += new TextDecoder().decode(p);
})();

type Ileti = { an: number; ham: string; v: any };
const gelen: Ileti[] = [];
const dokum: string[] = [];
(async () => {
  let tampon = "";
  for await (const p of sunucu.stdout) {
    tampon += new TextDecoder().decode(p);
    let i;
    while ((i = tampon.indexOf("\n")) >= 0) {
      const ham = tampon.slice(0, i);
      tampon = tampon.slice(i + 1);
      if (!ham.trim()) continue;
      dokum.push(`${an()} ← ${ham}`);
      let v: any = null;
      try {
        v = JSON.parse(ham);
      } catch {
        denetle(false, "stdout yalnız JSON-RPC", ham);
      }
      gelen.push({ an: performance.now(), ham, v });
    }
  }
})();

function gonder(nesne: object) {
  const ham = JSON.stringify(nesne);
  dokum.push(`${an()} → ${ham}`);
  sunucu.stdin.write(ham + "\n");
  sunucu.stdin.flush();
}
let sira = 0;
async function istek(method: string, params: object = {}): Promise<any> {
  const id = ++sira;
  gonder({ jsonrpc: "2.0", id, method, params });
  const son = performance.now() + 10000;
  while (performance.now() < son) {
    const y = gelen.find((m) => m.v?.id === id);
    if (y) return y.v;
    await Bun.sleep(5);
  }
  throw new Error(`${method}: 10 sn içinde yanıt yok`);
}
const bildirimler = (uri: string, sonra: number) =>
  gelen.filter(
    (m) => m.an >= sonra && m.v?.method === "notifications/resources/updated" && m.v?.params?.uri === uri,
  );
async function bildirimBekle(uri: string, sonra: number, ms: number): Promise<Ileti | undefined> {
  const son = sonra + ms;
  while (performance.now() < son) {
    const b = bildirimler(uri, sonra)[0];
    if (b) return b;
    await Bun.sleep(5);
  }
  return bildirimler(uri, sonra)[0];
}
function yaz(kimlik: string) {
  python(
    "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('INSERT INTO arac_cagrilari VALUES (?, ?, ?, ?, ?, 0, ?, 12)', (sys.argv[2], 'g3', 'wt-g3', 'search_skills', 'özet: çağrı', '2026-10-06T00:00:00Z')); c.commit(); c.close()",
    kayitlar,
    kimlik,
  );
}

const URI = "fihrist://kutup_kayitlar/arac_cagrilari";
try {
  const ilk = await istek("initialize", {
    protocolVersion: "2024-11-05",
    capabilities: {},
    clientInfo: { name: "g3-oturum", version: "1" },
  });
  const kaynak = ilk.result?.capabilities?.resources;
  denetle(kaynak?.subscribe === true && kaynak?.listChanged === false, "initialize: capabilities.resources", JSON.stringify(kaynak));
  gonder({ jsonrpc: "2.0", method: "notifications/initialized" });
  const yol = hizmetsiz ? "hazır (protokol" : "hizmete aktarılıyor";
  denetle(sunucuHata.includes(yol), `sunucu yolu: "${yol}"`, sunucuHata.trim().replaceAll("\n", " | "));

  const liste = await istek("resources/list");
  const uriler = (liste.result?.resources ?? []).map((r: any) => r.uri);
  denetle(uriler.includes(URI), "resources/list", JSON.stringify(uriler));

  const abone = await istek("resources/subscribe", { uri: URI });
  denetle(abone.result !== undefined && !abone.error, "resources/subscribe", JSON.stringify(abone));

  const kimlik = `g3-${Date.now()}-İğne`;
  yaz(kimlik);
  const yazildi = performance.now();
  const b = await bildirimBekle(URI, yazildi, 1000);
  denetle(!!b, "yazıştan sonra 1 sn içinde notifications/resources/updated", b ? `${(b.an - yazildi).toFixed(0)} ms, ${b.ham}` : "gelmedi");
  await Bun.sleep(600);
  denetle(bildirimler(URI, yazildi).length === 1, "tek yazışa tek bildirim", `${bildirimler(URI, yazildi).length} bildirim`);

  const oku = await istek("resources/read", { uri: URI });
  const ic = oku.result?.contents?.[0];
  let govde: any = {};
  try {
    govde = JSON.parse(ic?.text ?? "");
  } catch {}
  const kayit = (govde.kayitlar ?? []).find(
    (k: any) => k.islem === "e" && JSON.stringify(k.anahtar) === JSON.stringify([kimlik]),
  );
  denetle(ic?.mimeType === "application/json" && ic?.uri === URI && !!kayit, "resources/read yeni kaydı içerir", JSON.stringify(kayit ?? ic));

  const cik = await istek("resources/unsubscribe", { uri: URI });
  denetle(cik.result !== undefined && !cik.error, "resources/unsubscribe", JSON.stringify(cik));
  yaz(`${kimlik}-sonra`);
  const yazildi2 = performance.now();
  await Bun.sleep(1500);
  denetle(bildirimler(URI, yazildi2).length === 0, "abonelikten çıkınca 1,5 sn içinde bildirim yok", `${bildirimler(URI, yazildi2).length} bildirim`);

  const ana = await istek("resources/subscribe", { uri: "fihrist://kutup_kutuphane/yetenekler" });
  denetle(!!ana.error && String(ana.error.message).includes("ana katalog"), "ana katalog aboneliği hata döner", JSON.stringify(ana));
} catch (e) {
  denetle(false, "oturum", String(e));
}

// ── Kapanış ve döküm ───────────────────────────────────────────────────────
sunucu.stdin.end();
const kod = await Promise.race([sunucu.exited, Bun.sleep(8000).then(() => "zaman aşımı")]);
denetle(kod === 0, "stdin kapanınca sunucu temiz çıkar", `çıkış ${kod}`);
if (kod !== 0) sunucu.kill();

if (!hizmetsiz) {
  // Hizmet ayrı süreçtir; yalnız bu porta bağlı olanı öldür.
  for (const pid of readdirSync("/proc").filter((d) => /^\d+$/.test(d))) {
    try {
      const komut = readFileSync(`/proc/${pid}/cmdline`, "utf8").split("\0");
      const env = readFileSync(`/proc/${pid}/environ`, "utf8").split("\0");
      if (komut[1] === "hizmet" && env.includes(`FIHRIST_HIZMET_PORT=${port}`)) {
        process.kill(Number(pid));
        console.log(`hizmet durduruldu: pid ${pid} (port ${port})`);
      }
    } catch {}
  }
}

console.log("\n── ham ileti dökümü (→ istemci→sunucu, ← sunucu→istemci)");
console.log(dokum.join("\n"));
console.log("\n── sunucu stderr");
console.log(sunucuHata.trimEnd());
const hizmetGunlugu = join(dizin, "kutup_olaylar", "hizmet.log");
if (existsSync(hizmetGunlugu)) {
  console.log("\n── hizmet.log");
  console.log(readFileSync(hizmetGunlugu, "utf8").trimEnd());
}
bitir();
