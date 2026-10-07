// Koşu betiklerinin (kurulum.ts, arama-gocu.ts) ortak yardımcıları. Durumsuzdur; rapor
// her betiğin kendisindedir.

import { Database } from "bun:sqlite";
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

export const WIN = process.platform === "win32";

// Dosyanın (ve varsa WAL'ının) sha256'sının ilk 16 hanesi: "değişti mi" sorusu için.
export function sha(yol: string): string {
  const h = createHash("sha256").update(readFileSync(yol));
  if (existsSync(`${yol}-wal`)) h.update(readFileSync(`${yol}-wal`));
  return h.digest("hex").slice(0, 16);
}

// SQLite URI: ters bölü → bölü, ASCII dışı ve boşluk yüzde kodlanır.
export function uri(yol: string): string {
  const p = resolve(yol).replaceAll("\\", "/");
  const k = [...new TextEncoder().encode(p)]
    .map((b) => (/[A-Za-z0-9/._~:-]/.test(String.fromCharCode(b)) && b < 128 ? String.fromCharCode(b) : `%${b.toString(16).toUpperCase().padStart(2, "0")}`))
    .join("");
  return `file:${WIN ? "/" : ""}${k}?mode=ro`;
}

export function kos(komut: string[], env: Record<string, string> = {}): { kod: number; cikti: string } {
  const r = Bun.spawnSync(komut, { env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe" });
  return { kod: r.exitCode ?? -1, cikti: (r.stdout.toString() + r.stderr.toString()).trim() };
}

export function ro_sorgu<T>(yol: string, sql: string): T[] {
  const db = new Database(yol, { readonly: true });
  try {
    return db.query(sql).all() as T[];
  } finally {
    db.close();
  }
}

export function butunluk(yol: string): string {
  return ro_sorgu<{ integrity_check: string }>(yol, "PRAGMA integrity_check").map((r) => r.integrity_check).join(";");
}

// SQL tanımlayıcısı: çift tırnak, içteki tırnak ikilenir.
export function q(ad: string): string {
  return `"${ad.replaceAll('"', '""')}"`;
}
