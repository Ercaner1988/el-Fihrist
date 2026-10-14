# Oturum arşivi: Claude Code dökümlerinden, ayrı Turso dosyasında; MCP Seçimi buradan çıkarılır

**Durum: uygulandı, kancalar kurulmadı** (2026-10-14; kanca kurulumu Ercan'ın onayını bekliyor). Kod: `crates/fihrist-arsiv`. Komutlar: `ibnunnedim arsivle | arsiv-ara | secim-esle`. ADR 0007'nin ikinci adımı.

## Karar

- **Kaynak: Claude Code dökümleri** (`~/.claude/projects/**/*.jsonl`). Döküm özetlemeden önceki konuşmanın tamamını saklıyor; ayrı bir bağlam sıkıştırma aracı gerekmiyor.
- **Kayıt türleri** (CONTEXT.md "Oturum arşivi"): `ercan` (Ercan'ın mesajı), `model` (modelin cevabı), `arac` (araç adı + girdi), `ozet` (sıkıştırma özeti), `sinir` (`compact_boundary`), ve yalnız iz olarak `sonuc` (araç çıktısının **gövdesi girmez**: kimlik, hata bayrağı, uzunluk; asıl yeri `döküm:satır` gösterir).
  - Girmeyenler: düşünme blokları, ekler, görseller, `isMeta` satırları, alt ajanın düz metni ve sonuç izi. **Alt ajanın araç çağrıları girer** (`yan=1`): MCP Seçimi eşlemesi "aynı oturumda herhangi bir araç çağrısına" bakıyor ve alt ajanlar oturumun çağrılarının çoğunu yapıyor (ölçülen 1224 dosyanın 1135'i alt ajan dökümü).
  - Araç girdisi alan alan kesilir (2000 karakter, toplam 16 KB), kesilen metne yazılır. Toplam üzerinden kesmek yetmezdi: `serde_json` alanları harf sırasına dizer, `content` `file_path`'ten önce gelir; uzun bir Write gövdesi yolu atardı (sınamayla yakalandı).
- **Depo: ayrı Turso dosyası `kutup_arsiv.db`**, katalogun yanında (`kutup_kayitlar.db`, `kutup_depolar.db` kalıbı). Ana kataloğa girmez (ADR 0006). **fts5 yok**: Turso fts5'li dosyayı kesiyor ya da açmıyor (ADR 0005/0006), SQLite'ın fts5'i de `ışık`/`IŞIK`'ı eşlemiyor. Arama `katla` ile Türkçe katlamalı BM25, bellekte, her aramada kayıtlardan kurulur.
- **Artımlı ve yinelemez:** iki katman. (1) Her dökümün işlenen bayt konumu (`dokumler`), kayıtla **aynı işlemde** ilerler; yarım yazılmış son satır sonraki koşuya kalır. (2) `(uuid, blok)` tekilliği: bir oturum devam ettirilince Claude Code eski satırları yeni dosyaya kopyalıyor (ölçüldü: 216 bin satırın 39,8 bini, ~%18); onlar ikinci kez girmez. Küçülen döküm baştan okunur (tekillik yinelemeyi önler).
- **Mahremiyet: Kişisel.** Arşiv Köprülü'nün süzgeci yokken **MCP'ye bağlanmaz**: bağlansaydı arşiv aramasının her sonucu Bulut modele giderdi. Yalnız CLI.
- **Tetik (kurulmadı):** `PreCompact` → `ibnunnedim arsivle --kanca`; `SessionStart` → `ibnunnedim arsivle --kanca` (kalanları tamamlar; sıkıştırma özeti de buradan girer, çünkü özet PreCompact'ta henüz yazılmamıştır). `--kanca` stdin'deki kanca JSON'unu okur ve hata olsa da **çıkış 0** verir: PreCompact'ta sıfır olmayan çıkış sıkıştırmayı engeller. Turso dosyayı kilitler; üst üste binen kancalar için açılış kısa süre yeniden dener. Önerilen kanca bloğu (`~/.claude/settings.json`) aşağıda.

## MCP Seçimi eşlemesi (`secim-esle`)

Kural (Nazar CONTEXT.md "Seçim"): bir `nazar_ara` sonucundaki yollardan biri, aynı oturumda **sonraki** herhangi bir araç çağrısının girdisinde geçiyorsa o sonuç Seçimdir. Okunup işe yaramayan yol da Seçim sayılır (bilinen gürültü, kabul edildi).

- Toplu iş. Nazar günlüğü Turso'da ve nazard tek yazıcı; bu yüzden günlüğün **kopyasını** `sqlite3 "file:<yol>?mode=ro" ".backup '<hedef>'"` ile alır, kopyadan okur. Seçimleri nazard'a boru üzerinden `Istek::Secim` (`kanal=mcp`, geçmiş `zaman_ms`, `eylem`=kullanan aracın adı) ile yazdırır. nazard aynı (arama, yol) MCP Seçimini ikinci kez yazmaz: koşu yinelenebilir.
- Sonraki çağrı = aynı oturumda, aramanın günlükteki anından **sonraki** (`>`), Nazar araçları hariç (bir sonraki `nazar_ara` sorgusunda yol geçebilir; sınama: bozunca kırılıyor). Yol karşılaştırması: küçük harf, `\`→`/`, ardışık `/` tek; ayrıca Git Bash biçimi (`c:/x` → `/c/x`). Aynı yolun birden çok yaprağı tek Seçim.
- Eşlenemeyenler **sayılır ve söylenir**: oturum kimliği olmayan MCP araması (`CLAUDE_CODE_SESSION_ID` yok), arşivde kaydı olmayan oturum (döküm henüz arşivlenmemiş olabilir).
- `--kuru` yalnız gösterir, nazard'a yazmaz.

## Ölçülen (2026-10-14, gerçek dökümler, arşiv geçici dizinde)

| | |
|---|---|
| Dökümler | 1229 dosya (2,1 GB) |
| İlk koşu (debug derleme) | 305 sn; 526 396 satır okundu, 141 009 kayıt eklendi, 0 bozuk satır |
| Arşiv dosyası | 125 MB |
| Artımlı yeniden koşu (1229 dosya taranır; değişmeyenler veritabanına sorulmadan atlanır) | 0,8 sn (release); ön süzgeçsiz 29 sn (debug) |
| Arama (release, 141 bin kayıt) | 2,8 sn tüm türlerde; 3,2 sn yalnız `ercan` (debug: 29 sn) |

Metin hacmi (dedupe sonrası): Ercan 3 202 mesaj / 2,7 MB, model 13 763 / 5,8 MB, araç çağrısı 36 438 / 29,6 MB (+ alt ajan 21 117 / 16,8 MB), özet 130 / 2,6 MB, `compact_boundary` 165.

## Sınırlar ve borç

- **Arama her çağrıda kayıtları baştan indeksliyor.** Release'te ~3 sn, debug'da 29 sn. Hızlanması gerekirse kalıcı indeks ya da `Hizmet` sürecinde önbellek; şimdilik sıfır kalıcı durum, bayatlama yok.
- BM25 `ibnunnedim_cli::arama::Indeks`'in kopyası: `ibnunnedim-cli` bu crate'e bağımlı olduğu için döngü var. Ortak crate'e çıkarılınca ikisi birleşir. (`iso_ms`, `dokumleri_bul`, `varsayilan_kok` zaten buraya taşındı; `kayit.rs` yeniden dışa veriyor, `ibnunnedim-cli` 3341 → 3296 satır.)
- Eşleme günlükteki **kanal=mcp** aramalarıyla sınırlı; nazard yeni sürümle çalışmadıkça günlük boş. Köprülü araya girince araç adı `mcp__kopru__nazar_ara` olur; eşleme araç adına değil `nazar_` alt dizesine bakıyor, değişmez.
- Çalışan eski nazard `Istek::Secim`'i bilmez: `secim-esle` ilk reddedişte nedenle durur.
- Alt ajan dökümlerindeki `sessionId` ana oturumunki; oturum kimliği `CLAUDE_CODE_SESSION_ID` ile aynı olduğu sürece eşleme çalışır. Alt ajanın kendi MCP sürecinin oturum kimliğinin ne olduğu ölçülmedi.

## Önerilen kanca bloğu (kurulmadı)

```json
{
  "hooks": {
    "PreCompact":   [{ "hooks": [{ "type": "command", "command": "ibnunnedim arsivle --kanca" }] }],
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "ibnunnedim arsivle --kanca" }] }]
  }
}
```

İlk koşu elle yapılmalı (`ibnunnedim arsivle --tumu`, birkaç dakika); kancalar yalnız artımı alır.
