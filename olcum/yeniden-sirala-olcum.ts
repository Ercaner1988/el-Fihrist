// yeniden-sirala-olcum.ts — bge-reranker-v2-m3, gommenin ilk 20 adayini yeniden
// siralayinca ne kazandirir? Ayrica: "uygun arac yok" diyebilir mi (olumsuz sorgular)?
//
// Kullanim: bun olcum/yeniden-sirala-olcum.ts [rerank_url]
//   rerank_url varsayilan http://127.0.0.1:11439/v1/rerank (BGE_RERANK=1 ile acilmis ornek)
//
// SONUC (2026-09-25, bge-reranker-v2-m3, candle, CPU; canli sunucu da yuklu) — KULLANILMADI:
//   aday ust siniri (dogru kayit gommenin ilk 20'sinde) 18/18
//   gomme  : ilk 5 16/18 · MRR 0.83
//   sirali : ilk 5 15/18 · MRR 0.69 · 30,6 sn/sorgu (20 aday)
//   Ingilizce/ad sorgularinda iyi (0.86-0.99), Turkce sorgu → Ingilizce belgede
//   cokuyor: 'kok neden bulma yontemi' gommede 1., siralayicida 20.; olumlu Turkce
//   sorgularin en iyi olasiligi 0.002-0.07, olumsuzlarinki <=0.002 → tek esikle
//   "uygun arac yok" DENEMEZ. Yeniden siralayici kodu bge-embed-rs'e girmedi.
//   Bu betik Laya ince ayari (ya da baska bir siralayici) ayni govdeyle olculurken
//   yeniden kullanilir: rerank_url'yi degistirmek yeter.
//
// Adaylar el-Fihrist'in KENDI aramasindan gelir (ibnunnedim search --limit 20, gomme ana
// kanal) — yani olculen sey uretimdeki zincirin kendisi. Belge metinleri DB'den salt-okunur.
import { $ } from "bun";

const RERANK = Bun.argv[2] ?? "http://127.0.0.1:11439/v1/rerank";
const EXE = "C:/Users/buzbe/Desktop/mcp-tools/el-fihrist/ibnunnedim.exe";
const KUTUP = "C:/Users/buzbe/Desktop/hermes yazılım/kutuphane";
const DB = `${KUTUP}/kutup_kutuphane.db`, DEPO_DB = `${KUTUP}/kutup_depolar.db`; // depolar ayrı dosyada (main.rs DEPO_DB_ADI)
const K = 20, LIMIT = 5;

// Altin set: [[sorgu]] metin + beklenen (kimlik alt dizeleri, alternatif).
const toml = await Bun.file(`${import.meta.dir}/altin-sorgular.toml`).text();
const altin = [...toml.matchAll(/metin = "([^"]+)"\s*\nbeklenen = \[([^\]]*)\]/g)].map((m) => ({
  metin: m[1], beklenen: [...m[2].matchAll(/"([^"]+)"/g)].map((x) => x[1]),
}));
// Olumsuzlar: katalogda (yazilim/arastirma yetenekleri) karsiligi olmamasi beklenen sorular.
// Elle secildi; aday listeleri asagida basilir ki gercekten karsiliksiz olduklari gorulsun.
const OLUMSUZ = ["pizza hamuru tarifi", "İstanbul'dan Ankara'ya uçak bileti al", "gitarı akort et",
  "kedi tüyü dökülmesi neden olur", "dünkü futbol maçının skoru", "balkondaki çiçekleri ne sıklıkla sulamalıyım",
  "araba lastiği nasıl değiştirilir", "en iyi kahve demleme yöntemi"];

async function adaylar(sorgu: string): Promise<string[]> {
  const cikti = await $`${EXE} search ${sorgu} --limit ${K}`.quiet().nothrow().text();
  if (/düşme: BM25/.test(cikti)) console.error(`! '${sorgu}': gomme dustu, adaylar BM25'ten`);
  return [...cikti.matchAll(/^\[([^\]]+)\] /gm)].map((m) => m[1]);
}

async function metinler(idler: string[]): Promise<Map<string, string>> {
  const liste = idler.map((i) => `'${i.replaceAll("'", "''")}'`).join(",");
  const oku = async (db: string, tablo: string) => {
    const sql = `SELECT id, ad, aciklama, substr(tam_metin_md,1,1500) AS govde FROM ${tablo} WHERE id IN (${liste});`;
    return JSON.parse((await $`sqlite3 -readonly -json ${db} ${sql}`.quiet().text()) || "[]");
  };
  const satirlar = [...(await oku(DB, "yetenekler")), ...(await oku(DEPO_DB, "depolar"))];
  return new Map(satirlar.map((r: any) => [r.id, `${r.ad}: ${r.aciklama}\n${r.govde ?? ""}`]));
}

async function puanla(sorgu: string, belgeler: string[]): Promise<number[]> {
  const r = await fetch(RERANK, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ query: sorgu, documents: belgeler }) });
  if (!r.ok) throw new Error(`rerank http ${r.status}: ${await r.text()}`);
  return (await r.json()).results.map((x: any) => x.score);
}

const sig = (x: number) => 1 / (1 + Math.exp(-x));
const sira = (ids: string[], beklenen: string[]) => ids.findIndex((id) => beklenen.some((b) => id.includes(b)));
const ozet = { gomme: { isabet: 0, rr: 0 }, sirali: { isabet: 0, rr: 0 }, ust: 0 };
const olumluEnIyi: number[] = [];
let ms = 0;

for (const q of altin) {
  const ids = await adaylar(q.metin);
  const m = await metinler(ids);
  const t = performance.now();
  const p = await puanla(q.metin, ids.map((i) => m.get(i) ?? i));
  ms += performance.now() - t;
  const yeni = ids.map((id, i) => ({ id, p: p[i] })).sort((a, b) => b.p - a.p).map((x) => x.id);
  const [g, s] = [sira(ids, q.beklenen), sira(yeni, q.beklenen)];
  if (g >= 0) ozet.ust++;
  if (g >= 0 && g < LIMIT) { ozet.gomme.isabet++; ozet.gomme.rr += 1 / (g + 1); }
  if (s >= 0 && s < LIMIT) { ozet.sirali.isabet++; ozet.sirali.rr += 1 / (s + 1); }
  olumluEnIyi.push(sig(Math.max(...p)));
  console.log(`${s >= 0 && s < LIMIT ? "✓" : "✗"} '${q.metin}' gomme ${g < 0 ? "yok" : g + 1}. → sirali ${s < 0 ? "yok" : s + 1}. · en iyi ${sig(Math.max(...p)).toFixed(3)} (${yeni[0]})`);
}
const n = altin.length;
console.log(`\nADAY ÜST SINIRI (doğru kayıt ilk ${K}'de): ${ozet.ust}/${n}`);
console.log(`gomme    : ilk ${LIMIT} ${ozet.gomme.isabet}/${n} · MRR ${(ozet.gomme.rr / n).toFixed(2)}`);
console.log(`sirali   : ilk ${LIMIT} ${ozet.sirali.isabet}/${n} · MRR ${(ozet.sirali.rr / n).toFixed(2)} · yeniden sıralama ${(ms / n / 1000).toFixed(2)} sn/sorgu (${K} aday)`);

const olumsuzEnIyi: number[] = [];
console.log(`\nOLUMSUZLAR (en yüksek olasılık ve adayı):`);
for (const q of OLUMSUZ) {
  const ids = await adaylar(q);
  const m = await metinler(ids);
  const p = await puanla(q, ids.map((i) => m.get(i) ?? i));
  const en = p.indexOf(Math.max(...p));
  olumsuzEnIyi.push(sig(p[en]));
  console.log(`  '${q}' → ${sig(p[en]).toFixed(3)} (${ids[en]})`);
}
const [a, b] = [[...olumluEnIyi].sort((x, y) => x - y), [...olumsuzEnIyi].sort((x, y) => x - y)];
console.log(`\nolumlu sorgularda en iyi aday olasılığı: en düşük ${a[0].toFixed(3)} · medyan ${a[a.length >> 1].toFixed(3)}`);
console.log(`olumsuz sorgularda en iyi aday olasılığı: medyan ${b[b.length >> 1].toFixed(3)} · en yüksek ${b.at(-1)!.toFixed(3)}`);
console.log(a[0] > b.at(-1)! ? `AYRIŞIYOR: ${b.at(-1)!.toFixed(3)} < eşik < ${a[0].toFixed(3)}` : "AYRIŞMIYOR: tek eşikle 'uygun araç yok' denemez");
