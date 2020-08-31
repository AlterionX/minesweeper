//! This module attempts to perform a variant of CSP on the board's state. This
//! needs to do an exhaustive search over state space, however, so it may be a
//! bit slow. Scratch that, it's definitely going to be slow.
//!
//! Making it faster is on the TODO list.
//!
//! Summary of what's going on:
//!
//! First, we locate connected components of linked sub regions. This means that
//! any of the three regions involved in the linked sub regions contains at
//! least one location in common with another linked sub region.
//!
//! Second, we exhaustively solve the tighter CSP of the connected component,
//! short circuiting on impossible situations. This CSP involves number of mines
//! within each individual component.
//!
//! Lastly, we solve the CSP of the components, shortcircuiting on impossible
//! situations. This CSP involves the target board mine quantity.

mod region;
mod algo;

use indexmap::IndexSet;
use std::collections::VecDeque;
use crate::{
    board::Board,
    solver::region::{Region, StrippedRegions, LinkedSubRegion},
};

// TODO Make this entire process more efficient. Cause it should be possible.
pub struct Solver<'a> {
    pub board: &'a Board,

    board_region: Region,
    valid_regions: Vec<Region>,
    found_empty_locs: IndexSet<(usize, usize)>,
    found_mine_locs: IndexSet<(usize, usize)>,
}

impl<'a> Solver<'a> {
    fn extract_regions(board: &Board) -> Vec<Region> {
        let mut rr = vec![];
        for (col, row) in board.all_locs() {
            if let Some(r) = Region::around(board, (col, row)) {
                rr.push(r)
            }
        }
        rr
    }

    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            board_region: Region::board(board),
            valid_regions: Self::extract_regions(board),
            found_empty_locs: IndexSet::new(),
            found_mine_locs: IndexSet::new(),
        }
    }
}

pub struct KnownCells {
    pub empty: IndexSet<(usize, usize)>,
    pub mines: IndexSet<(usize, usize)>,
}

// The whole point of this struct.
impl<'a> Solver<'a> {
    fn strip_mine_and_empty_regions(&mut self) -> KnownCells {
        let regions = self.valid_regions.drain(..).collect();
        let StrippedRegions { locs: zero_locs, regions } = Region::strip_zero_regions_from(regions);
        let StrippedRegions { locs: mine_locs, regions } = Region::strip_mine_regions_from(regions);
        self.valid_regions = regions;
        self.found_empty_locs.extend(zero_locs.iter());
        self.found_mine_locs.extend(mine_locs.iter());
        KnownCells {
            empty: zero_locs,
            mines: mine_locs,
        }
    }

    // Error when board state contradicts itself. Typically due to error in placing a flagged cell.
    pub fn calculate_known_cells(&mut self) -> Result<Option<KnownCells>, ()> {
        self.strip_mine_and_empty_regions();
        // Find linked
        let mut links = self.valid_regions.iter()
            .enumerate()
            // Unique cartesian product
            .flat_map(|(i, p0)| self.valid_regions[i..].iter().map(move |p1| (p0, p1)))
            .filter_map(|(p0, p1)| LinkedSubRegion::deduce_links(p0, p1))
            .collect::<VecDeque<_>>();
        let mut since_last_change = 0;
        while let Some(link) = links.pop_front() {
            if link.mine_sets.len() != 1 { // Do nothing, as more than one variant exists and we don't do guesses.
                links.push_back(link);
            } else { // Only one variant exists.
                let LinkedSubRegion { r0, rs, r1, mut mine_sets } = link;
                let (m0, ms, m1) = mine_sets.pop()
                    .expect("the element that was just reported to be there.");
                assert!(mine_sets.is_empty(), "mine_sets to have no more elements.");
                let mut link_zero_locs = IndexSet::new();
                let mut link_mine_locs = IndexSet::new();
                if m0 == 0 {
                    link_zero_locs.extend(r0);
                    since_last_change = 0;
                } else if m0 == r0.len() {
                    link_mine_locs.extend(r0);
                    since_last_change = 0;
                }
                if m1 == 0 {
                    link_zero_locs.extend(r1);
                    since_last_change = 0;
                } else if m1 == r1.len() {
                    link_mine_locs.extend(r1);
                    since_last_change = 0;
                }
                if ms == 0 {
                    link_zero_locs.extend(rs);
                    since_last_change = 0;
                } else if ms == rs.len() {
                    link_mine_locs.extend(rs);
                    since_last_change = 0;
                }
                for link in &mut links {
                    link.remove_mines(&link_mine_locs);
                    link.remove_empty(&link_zero_locs);
                    // TODO There are more conclusions available than just this. Figure out what
                    // they are.
                }
                for region in &mut self.valid_regions {
                    region.remove_mine_locs(&link_mine_locs);
                    region.remove_empty_locs(&link_zero_locs);
                }
                self.found_mine_locs.extend(link_mine_locs);
                self.found_empty_locs.extend(link_zero_locs);
            }
            if since_last_change >= links.len() {
                break;
            } else {
                since_last_change += 1;
            }
        }
        // TODO There will be 3 categories of spots: unknown, is_mine, is_empty.
        // unimplemented!("Solver not yet fully functional.");
        if self.found_empty_locs.is_empty() && self.found_mine_locs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(KnownCells {
                empty: self.found_empty_locs.clone(),
                mines: self.found_mine_locs.clone(),
            }))
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn solver_test() {
    }
}
