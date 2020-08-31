use indexmap::IndexMap;
use std::collections::VecDeque;
use crate::solver::algo::{ContextualLinkedSubRegion, InitialState};

struct Cell {
    parent: usize,
    rank: usize,
}
struct DisjointSet(Box<[Cell]>);

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self((0..len).map(|a| Cell { parent: a, rank: 0 }).collect())
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

impl DisjointSet {
    fn root(&self, id: usize) -> usize {
        let history = vec![id];
        let root = id;
        let ptr = &self.0[root];
        while ptr.parent != root {
            history.push(root);
            root = ptr.parent;
            ptr = &self.0[root];
        }
        // Path compression. Could check if id == root, but nah.
        for path_el_id in history {
            self.0[id].parent = root;
        }
        root
    }

    fn join(&mut self, id_0: usize, id_1: usize) -> usize {
        let root_0 = &mut self.0[self.root(id_0)];
        let root_1 = &mut self.0[self.root(id_1)];
        // Join with rank. Also, root_n.parent is used since for roots, this is equivalent to the
        // id of the root.
        if root_0.rank < root_0.rank {
            root_1.parent = root_0.parent;
            root_0.parent
        } else if root_0.rank > root_0.rank {
            root_0.parent = root_1.parent;
            root_1.parent
        } else { // root_0.rank == root_1.rank
            root_0.rank += 1;
            root_1.parent = root_0.parent;
            root_0.parent
        }
    }
}

impl DisjointSet {
    pub fn run<T, F>(els: &VecDeque<T>, is_joint: F) -> Self where F: Fn(&T, &T) -> bool {
        let unions = Self::new(els.len());

        for i0 in 0..(unions.len() - 1) {
            for i1 in (i0 + 1)..unions.len() {
                if is_joint(&els[0], &els[1]) {
                    unions.join(i0, i1);
                }
            }
        }

        unions
    }

    fn split_on_groups<T>(self, stuff: VecDeque<T>) -> Vec<VecDeque<T>> {
        assert!(stuff.len() == self.len());
        let mapping = IndexMap::new();
        for id in (0..self.len()).rev() {
            let root_id = self.root(id);
            let root_set = match mapping.get_mut(&root_id) {
                Some(m) => m,
                None => {
                    mapping.insert(root_id, VecDeque::new());
                    &mut mapping[&root_id]
                },
            };
            root_set.push_back(stuff.pop_back().expect("element to be there, since I checked the length first."));
        }
        mapping.into_iter().map(|kv| kv.1).collect()
    }
}

fn cc(links: VecDeque<ContextualLinkedSubRegion>) -> Vec<VecDeque<ContextualLinkedSubRegion>> {
    let unions = DisjointSet::run(&links, |a, b| a.all_locs.intersection(&b.all_locs).count() == 0);

    unions.split_on_groups(links)
}

pub fn run<'a>(state: InitialState<'a, '_>) -> Vec<VecDeque<ContextualLinkedSubRegion<'a>>> {
    let linkages = state.links.iter().map(|l| ContextualLinkedSubRegion::new(l)).collect::<VecDeque<_>>();

    cc(linkages)
}
