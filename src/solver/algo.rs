mod cc;
mod csp0;
mod csp1;

use indexmap::IndexSet;
use std::collections::VecDeque;
use crate::{
    board::Board,
    solver::region::LinkedSubRegion,
};

pub struct InitialState<'solver, 'board: 'solver> {
    solver: &'solver mut super::Solver<'board>,
    empties: &'solver mut IndexSet<(usize, usize)>,
    mines: &'solver mut IndexSet<(usize, usize)>,
    links: VecDeque<&'solver LinkedSubRegion>,
}

// This is essentially going to be a linked list.
pub struct Node<'solver, 'board: 'solver> {
    board: &'board Board,
    empties: &'solver mut IndexSet<(usize, usize)>,
    mines: &'solver mut IndexSet<(usize, usize)>,
    decision: Option<(usize, usize, usize)>,
    backing_links: VecDeque<&'solver LinkedSubRegion>,
    // TODO Unsafe, but here for convenience. If it is unecessary, remove it.
    parent: Option<*const Node<'solver, 'board>>,
}

pub struct ContextualLinkedSubRegion<'link> {
    link: &'link LinkedSubRegion,
    all_locs: IndexSet<(usize, usize)>,
}

impl <'l> ContextualLinkedSubRegion<'l> {
    fn new(link: &'l LinkedSubRegion) -> Self {
        Self {
            all_locs: link.r0.iter()
                .chain(link.r1.iter())
                .chain(link.rs.iter())
                .cloned()
                .collect(),
            link,
        }
    }
}

impl<'s, 'b> From<InitialState<'s, 'b>> for Node<'s, 'b> {
    fn from(s: InitialState<'s, 'b>) -> Self {
        Self {
            board: s.solver.board,
            empties: s.empties,
            mines: s.mines,
            decision: None,
            backing_links: s.links,
            parent: None,
        }
    }
}

fn intercc_csp() {
}

fn intracc_csp() {
}

pub fn run(mut state: InitialState<'_, '_>) -> () {
    let stage0 = cc::run(state);

    // TODO Convert stage0 to they input to `csp0::run`.

    let stage1 = stage0.into_iter().map(csp0::run).collect::<Vec<_>>();

    // TODO Convert stage1 to they input to `csp0::run`.

    let stage2 = csp1::run(stage1);

    // TODO Convert stage2 to expected output.
}
