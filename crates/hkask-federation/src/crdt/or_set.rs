//! Observed-Remove Set — elements can be added and removed.
//!
//! Removals observe specific adds via dots. Concurrent add+remove → add wins (add-bias).
//! Causal remove wins: if a remove observes the most recent add, the element is gone.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ReplicaId;
use crate::crdt::dot::Dot;
use crate::crdt::version_vector::VersionVector;

/// Observed-Remove Set with causal context.
///
/// - `add(e)` — adds element `e` with a unique dot.
/// - `remove(e)` — tombstones all currently-visible dots for `e`.
/// - `merge(other)` — merges another replica's state. Elements in `other`'s
///   add_set that aren't causally removed in `self` are added.
pub struct ORSet<T: Hash + Eq + Clone> {
    add_set: HashMap<T, Vec<Dot>>,
    remove_set: HashMap<T, Vec<Dot>>,
    replica: ReplicaId,
    counter: AtomicU64,
}

impl<T: Hash + Eq + Clone> ORSet<T> {
    /// Create a new OR-Set for the given replica.
    pub fn new(replica: ReplicaId) -> Self {
        Self {
            add_set: HashMap::new(),
            remove_set: HashMap::new(),
            replica,
            counter: AtomicU64::new(0),
        }
    }

    /// Add an element. Returns the dot for this add.
    pub fn add(&mut self, element: T) -> Dot {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let dot = Dot::new(self.replica.clone(), counter);
        self.add_set.entry(element).or_default().push(dot.clone());
        dot
    }

    /// Remove an element — tombstones all currently-visible dots.
    pub fn remove(&mut self, element: &T) {
        if let Some(dots) = self.add_set.get(element) {
            self.remove_set
                .entry(element.clone())
                .or_default()
                .extend(dots.clone());
        }
    }

    /// Check if the element is present (at least one dot not causally removed).
    pub fn contains(&self, element: &T) -> bool {
        let added = self.add_set.get(element);
        let removed = self.remove_set.get(element);
        match (added, removed) {
            (Some(added_dots), None) => !added_dots.is_empty(),
            (Some(added_dots), Some(removed_dots)) => {
                added_dots.iter().any(|d| !removed_dots.contains(d))
            }
            _ => false,
        }
    }

    /// All elements currently present in the set.
    pub fn elements(&self) -> HashSet<T> {
        self.add_set
            .keys()
            .filter(|k| self.contains(k))
            .cloned()
            .collect()
    }

    /// Compute a version vector from all dots in the add_set.
    pub fn version_vector(&self) -> VersionVector {
        let mut vv = VersionVector::new();
        for dots in self.add_set.values() {
            for dot in dots {
                let current = vv.get(&dot.replica);
                if dot.counter > current {
                    vv.increment(dot.replica.clone()); // will set to dot.counter + 1 from current
                }
            }
        }
        vv
    }

    /// Merge another OR-Set's state into this one.
    ///
    /// For each element in `other`'s add_set: if it's not causally removed
    /// in `self` (i.e., self's remove_set doesn't contain any dot that
    /// dominates or equals the other's dot), add it to self.
    pub fn merge(&mut self, other: &Self) {
        for (element, other_dots) in &other.add_set {
            let my_removed = self.remove_set.get(element);
            let surviving: Vec<Dot> = other_dots
                .iter()
                .filter(|dot| {
                    !my_removed.is_some_and(|removed_dots| {
                        removed_dots
                            .iter()
                            .any(|rd| rd.replica == dot.replica && rd.counter >= dot.counter)
                    })
                })
                .cloned()
                .collect();
            if !surviving.is_empty() {
                self.add_set
                    .entry(element.clone())
                    .or_default()
                    .extend(surviving);
            }
        }
        // Union remove sets (tombstones propagate)
        for (element, dots) in &other.remove_set {
            self.remove_set
                .entry(element.clone())
                .or_default()
                .extend(dots.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_a() -> ORSet<String> {
        ORSet::new("alpha".into())
    }

    fn set_b() -> ORSet<String> {
        ORSet::new("beta".into())
    }

    #[test]
    fn add_then_contains() {
        let mut s = set_a();
        s.add("hello".into());
        assert!(s.contains(&"hello".into()));
    }

    #[test]
    fn remove_then_not_contains() {
        let mut s = set_a();
        s.add("hello".into());
        s.remove(&"hello".into());
        assert!(!s.contains(&"hello".into()));
    }

    #[test]
    fn concurrent_add_remove_add_wins() {
        let mut a = set_a();
        let mut b = set_b();

        a.add("x".into());
        b.remove(&"x".into());
        // Concurrent: A added, B removed (but B hadn't seen A's add)
        // OR-Set add-bias: add wins
        a.merge(&b);
        assert!(a.contains(&"x".into()));
    }

    #[test]
    fn causal_remove_wins() {
        let mut a = set_a();
        let mut b = set_b();

        a.add("x".into());
        // Sync: B sees A's add
        b.merge(&a);
        assert!(b.contains(&"x".into()));

        // Now A removes x
        a.remove(&"x".into());
        // Sync: B sees A's removal (causal — B had seen the add before the remove)
        b.merge(&a);
        assert!(!b.contains(&"x".into()));
    }

    #[test]
    fn merge_commutative() {
        let mut a = set_a();
        let mut b = set_b();
        a.add("x".into());
        b.add("y".into());
        b.remove(&"x".into());

        let mut a1 = make_copy(&a);
        let mut b1 = make_copy(&b);
        a1.merge(&b);
        b1.merge(&a);

        let a_elems: HashSet<String> = a1.elements();
        let b_elems: HashSet<String> = b1.elements();
        assert_eq!(a_elems, b_elems);
    }

    #[test]
    fn merge_idempotent() {
        let mut a = set_a();
        a.add("x".into());
        a.add("y".into());
        a.remove(&"x".into());

        let mut a2 = make_copy(&a);
        a2.merge(&a);
        let orig: HashSet<String> = a.elements();
        let merged: HashSet<String> = a2.elements();
        assert_eq!(orig, merged);
    }

    fn make_copy(s: &ORSet<String>) -> ORSet<String> {
        let mut copy = ORSet::new("alpha".into());
        for elem in s.add_set.keys() {
            for dot in s.add_set.get(elem).into_iter().flatten() {
                copy.add_set
                    .entry(elem.clone())
                    .or_default()
                    .push(dot.clone());
            }
        }
        for (elem, dots) in &s.remove_set {
            copy.remove_set.insert(elem.clone(), dots.clone());
        }
        copy
    }

    #[test]
    fn elements_consistent() {
        let mut a = set_a();
        a.add("x".into());
        a.add("y".into());
        a.remove(&"x".into());
        let elems: HashSet<String> = a.elements();
        assert!(elems.contains(&"y".to_string()));
        assert!(!elems.contains(&"x".to_string()));
    }

    #[test]
    fn empty_set_no_elements() {
        let a = set_a();
        assert!(a.elements().is_empty());
    }
}
