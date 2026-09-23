# el-Fihrist

Ercan'ın araç ailesinde (zopay, zopay-word, agent-reach-rs, pasli-beyin, kervan) ortak yeteneklerin tek kaynaktan dağıtıldığı merkez depo.

## Dil

### Ödünç alma biçimleri

**Merkez ödünç alma**:
Bir hedef aracın bir kaynak kütüphaneye doğrudan değil, el-Fihrist'in crate'i üzerinden path bağımlılığıyla bağlanması.
_Kaçın_: hub, köprü, "el-Fihrist'ten çekmek"

**Parça ödünç alma**:
Bir kod parçasının `kutup_kutuphane.db`'den alınıp hedef depoya kopyalanması.
_Kaçın_: vendor, yapıştırma

**Yöntem ödünç alma**:
Kod alınmadan yalnız yaklaşımın alınması; iki depo ayrı kalır.
_Kaçın_: esinlenme, port

### Aktörler

**Kaynak kütüphane**:
Hedef araçlara yetenek sağlayan dış ya da iç depo (turso, agentfs, graphify-rs, archify-graft-rs, laya, regex, rank_bm25, bge-embed-rs).

**Hedef araç**:
Ercan'a ait, kaynak kütüphaneleri tüketen araç (zopay, zopay-word, agent-reach-rs, el-Fihrist, pasli-beyin, kervan).

**YZ arayüzü**:
Hedef araçları MCP ya da CLI üzerinden çağıran çıkarım ortamı (Claude Code, Claude Desktop, Antigravity).

## İlişkiler

- Bir **Merkez ödünç alma** tam olarak bir **Kaynak kütüphane**yi el-Fihrist'in bir crate'i ardından bir **Hedef araç**a taşır.
- Bir **Hedef araç**, **YZ arayüzü**ne kendi MCP sunucusuyla görünür; ödünç aldığı kütüphaneyi ayrıca göstermez.

## Örnek konuşma

> **Geliştirici:** "Pasli-beyin bge-m3 gömmesini doğrudan bge-embed-rs'den mi alıyor?"
> **Ercan:** "Hayır — **Merkez ödünç alma**: pasli-beyin `ibnunnedim-cli`'ye bağlı, bge-embed-rs'yi yalnız el-Fihrist tanır."

## İşaretlenen belirsizlikler

- "el-Fihrist üzerinden ödünç alma" üç anlamda kullanılıyordu — çözüldü: varsayılan anlam **Merkez ödünç alma**; öteki ikisi ayrı adlarla anılır.
- Merkez bugün `ibnunnedim-cli` (lib+bin) crate'idir, `fihrist-core` değil — ince bir kütüphaneye taşınıp taşınmayacağı açık.
- pasli-beyin'deki "Ollama bge-m3" ifadeleri eskimiş: gömme arka ucu bge-embed-rs'dir (127.0.0.1:11435).
