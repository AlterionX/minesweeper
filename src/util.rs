use std::ops::Bound;
use indexmap::IndexSet;


pub fn split_sets<T: Eq + std::hash::Hash + Copy>(aa: IndexSet<T>, bb: IndexSet<T>) -> (IndexSet<T>, IndexSet<T>, IndexSet<T>) {
    (
        aa.difference(&bb).cloned().collect(),
        aa.intersection(&bb).cloned().collect(),
        bb.difference(&aa).cloned().collect(),
    )
}

pub fn map_bound<T, S, F>(b: Bound<T>, f: F) -> Bound<S>
where
    F: Fn(T) -> S,
{
    match b {
        Bound::Excluded(s) => Bound::Excluded(f(s)),
        Bound::Included(s) => Bound::Included(f(s)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

