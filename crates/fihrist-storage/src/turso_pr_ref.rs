//! # Turso (Limbo) B-Tree Count Hızlı Yol Mimarisi ve PR #8982 Referansı
//!
//! Bu modül, `minisqlite` projesinin sıfır-tahsisatlı (zero-allocation) B-tree sayım algoritmasının
//! Turso (Limbo) çekirdeğine port edilmesi sürecini ve mimari izolasyonunu belgeler.
//!
//! ## 1. minisqlite Mimarisi (Kaynak)
//! - Kaynak dosya: `crates/minisqlite-btree/src/count.rs`
//! - minisqlite, sayım işleminde yaprak ve iç sayfalardaki hücre gövdelerini (payload, varint rowid)
//!   hiçbir zaman belleğe açmaz (`read_record` veya `decode_cell` yapmaz).
//! - Yalnızca 2 baytlık `cell_count` başlığını okur ve iç sayfalarda doğrudan 4 baytlık çocuk
//!   sayfa işaretçilerini takip eder.
//!
//! ## 2. Turso (Limbo) Katkısı (PR #8982)
//! - PR URL: `https://github.com/tursodatabase/turso/pull/8982`
//! - Etkilenen modül: `core/storage/btree.rs` (`BTreeCursor::count`)
//! - Öncesi: İç sayfalardan sol alt çocuk sayfaya inerken `contents.cell_get(cell_idx, ...)`
//!   çağrılıyor ve bu işlem dilim transmute, varint çözümleme ve `BTreeCell` enum tahsisatı yapıyordu.
//! - İyileştirme: Turso'nun `core/storage/pager.rs` modülündeki `cell_interior_read_left_child_page`
//!   metodu kullanılarak doğrudan 4 baytlık çocuk işaretçisi sıfır ek maliyetle okunmaya başlandı.
//! - Sonuç: B-Tree sayım (`SELECT count(*)`) traversal döngüsündeki CPU ve bellek tahsisat yükü ortadan kaldırıldı.

/// Turso PR #8982 meta verileri
pub struct TursoBTreePrMetadata {
    pub pr_number: u32,
    pub title: &'static str,
    pub upstream_url: &'static str,
    pub target_component: &'static str,
    pub optimization_type: &'static str,
}

pub const TURSO_BTREE_PR: TursoBTreePrMetadata = TursoBTreePrMetadata {
    pr_number: 8982,
    title: "core/storage: optimize interior left-child page reading in BTreeCursor::count",
    upstream_url: "https://github.com/tursodatabase/turso/pull/8982",
    target_component: "core/storage/btree.rs",
    optimization_type: "Zero-allocation interior pointer traversal ported from minisqlite",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turso_pr_metadata_integrity() {
        assert_eq!(TURSO_BTREE_PR.pr_number, 8982);
        assert!(TURSO_BTREE_PR.upstream_url.contains("8982"));
        assert_eq!(TURSO_BTREE_PR.target_component, "core/storage/btree.rs");
    }
}
