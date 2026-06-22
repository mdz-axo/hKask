//! CRDT contract tests — property-based verification of CRDT convergence.
//!
//! Tests commutativity, associativity, and idempotence of CRDT merge operations.
//! Uses proptest for randomized state-space exploration.

use hkask_federation::crdt::{GSet, LWWMap, ORSet, VersionVector};
use proptest::prelude::*;
use std::collections::HashSet;

// ── Version Vector ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn version_vector_merge_commutative(
        a_pairs in prop::collection::vec(("[a-z]{1,4}", 0u64..100), 0..10),
        b_pairs in prop::collection::vec(("[a-z]{1,4}", 0u64..100), 0..10),
    ) {
        let a = vv_from_pairs(&a_pairs);
        let b = vv_from_pairs(&b_pairs);
        assert_eq!(a.merge(&b), b.merge(&a));
    }

    #[test]
    fn version_vector_merge_associative(
        a_pairs in prop::collection::vec(("[a-z]{1,3}", 0u64..50), 0..8),
        b_pairs in prop::collection::vec(("[a-z]{1,3}", 0u64..50), 0..8),
        c_pairs in prop::collection::vec(("[a-z]{1,3}", 0u64..50), 0..8),
    ) {
        let a = vv_from_pairs(&a_pairs);
        let b = vv_from_pairs(&b_pairs);
        let c = vv_from_pairs(&c_pairs);
        assert_eq!(a.merge(&b).merge(&c), a.merge(&b.merge(&c)));
    }

    #[test]
    fn version_vector_merge_idempotent(
        pairs in prop::collection::vec(("[a-z]{1,4}", 0u64..100), 0..10),
    ) {
        let a = vv_from_pairs(&pairs);
        assert_eq!(a.merge(&a), a);
    }
}

fn vv_from_pairs(pairs: &[(String, u64)]) -> VersionVector {
    let mut vv = VersionVector::new();
    for (r, c) in pairs {
        let current = vv.get(r);
        vv.increment(r.clone());
    }
    vv
}

// ── GSet ────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn gset_merge_commutative(
        a_items in prop::collection::vec("[a-z]{1,8}", 0..20),
        b_items in prop::collection::vec("[a-z]{1,8}", 0..20),
    ) {
        let mut a: GSet<String> = GSet::new();
        let mut b: GSet<String> = GSet::new();
        for item in &a_items { a.insert(item.clone()); }
        for item in &b_items { b.insert(item.clone()); }

        let mut a1 = GSet::new();
        a1.merge(&a);
        a1.merge(&b);

        let mut b1 = GSet::new();
        b1.merge(&b);
        b1.merge(&a);

        let a_set: HashSet<_> = a1.elements().cloned().collect();
        let b_set: HashSet<_> = b1.elements().cloned().collect();
        assert_eq!(a_set, b_set);
    }

    #[test]
    fn gset_insert_idempotent(
        items in prop::collection::vec("[a-z]{1,8}", 1..30),
    ) {
        let mut s: GSet<String> = GSet::new();
        for item in &items {
            s.insert(item.clone());
            s.insert(item.clone()); // duplicate
        }
        let unique: HashSet<_> = items.iter().cloned().collect();
        assert_eq!(s.len(), unique.len());
    }
}

// ── ORSet ───────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn orset_merge_commutative_add_only(
        a_items in prop::collection::vec("[a-z]{1,6}", 0..20),
        b_items in prop::collection::vec("[a-z]{1,6}", 0..20),
    ) {
        let mut a = ORSet::new("alpha".into());
        let mut b = ORSet::new("beta".into());
        for item in &a_items { a.add(item.clone()); }
        for item in &b_items { b.add(item.clone()); }
        let a_copy = copy_orset(&a);
        let b_copy = copy_orset(&b);
        let mut a1 = copy_orset(&a);
        let mut b1 = copy_orset(&b);
        a1.merge(&b);
        b1.merge(&a);
        assert_eq!(a1.elements(), b1.elements());
    }

    #[test]
    fn orset_merge_idempotent(
        items in prop::collection::vec("[a-z]{1,6}", 0..20),
    ) {
        let mut a = ORSet::new("alpha".into());
        for item in &items { a.add(item.clone()); }
        let mut a2 = copy_orset(&a);
        a2.merge(&a);
        assert_eq!(a.elements(), a2.elements());
    }
}

fn copy_orset(s: &ORSet<String>) -> ORSet<String> {
    let mut copy = ORSet::new("alpha".into());
    for elem in s.elements() {
        copy.add(elem);
    }
    copy
}
