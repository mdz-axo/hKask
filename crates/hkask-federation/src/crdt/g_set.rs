//! Grow-Only Set — elements can be added but never removed.
//!
//! Merge is set union. No conflicts possible — the set only grows.

use std::collections::HashSet;
use std::hash::Hash;

/// Grow-Only Set — additive, no removal, no conflicts.
pub struct GSet<T: Hash + Eq> {
    elements: HashSet<T>,
}

impl<T: Hash + Eq + Clone> GSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashSet::new(),
        }
    }

    pub fn insert(&mut self, element: T) {
        self.elements.insert(element);
    }

    pub fn contains(&self, element: &T) -> bool {
        self.elements.contains(element)
    }

    pub fn elements(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Merge — set union. Both sets' elements are preserved.
    pub fn merge(&mut self, other: &Self) {
        for elem in &other.elements {
            self.elements.insert(elem.clone());
        }
    }
}

impl<T: Hash + Eq + Clone> Default for GSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_contains() {
        let mut s: GSet<String> = GSet::new();
        s.insert("hello".into());
        assert!(s.contains(&"hello".into()));
    }

    #[test]
    fn merge_is_commutative() {
        let mut a: GSet<String> = GSet::new();
        let mut b: GSet<String> = GSet::new();
        a.insert("x".into());
        b.insert("y".into());

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
    fn insert_is_idempotent() {
        let mut s: GSet<String> = GSet::new();
        s.insert("x".into());
        s.insert("x".into());
        assert_eq!(s.len(), 1);
    }
}
