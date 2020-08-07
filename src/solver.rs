use itertools::{self as iter, Itertools};
use std::ops::{Bound, RangeBounds};

use crate::board::{Cell, CellState, CellCategory, Board};

#[derive(Debug)]
struct Region {
    // Each bound is "or"d with the others.
    mines: Vec<u8>,
    hidden: Vec<(usize, usize)>,
}

fn split_vecs<T: PartialEq>(aa: Vec<T>, bb: Vec<T>) -> (Vec<T>, Vec<T>, Vec<T>) {
    let mut shared_els = vec![];
    let mut aa_only_els = vec![];
    let mut is_only_in_bb = vec![true; bb.len()];
    for a in aa.into_iter() {
        let mut is_in_bb = false;
        for (idx, b) in bb.iter().enumerate() {
            if &a == b {
                is_only_in_bb[idx] = false;
                is_in_bb = true;
                break;
            }
        }
        if is_in_bb {
            shared_els.push(a);
        } else {
            aa_only_els.push(a)
        }
    }
    let bb_only_els = iter::zip(
        bb.into_iter(),
        is_only_in_bb.into_iter(),
    )
        .filter_map(|(b, is_only_in_bb)| if is_only_in_bb {
            Some(b)
        } else {
            None
        })
        .collect();
    (aa_only_els, shared_els, bb_only_els)
}

fn map_bound<T, S, F>(b: Bound<T>, f: F) -> Bound<S>
where
    F: Fn(T) -> S,
{
    match b {
        Bound::Excluded(s) => Bound::Excluded(f(s)),
        Bound::Included(s) => Bound::Included(f(s)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

// TODO Make this entire process more efficient. Cause it should be possible.
pub struct Solver<'a> {
    pub board: &'a Board,
}

// Utility/Helpers
impl<'a> Solver<'a> {
    fn board_locs(&self) -> impl Iterator<Item=(usize, usize)> {
        (0..self.board.h()).cartesian_product(0..self.board.w())
    }
}

// Region extraction methods.
impl<'a> Solver<'a> {
    fn region_around(&self, sentinel_loc @ (_, _): (usize, usize)) -> Option<Region> {
        let (col, row) = sentinel_loc;
        let sentinel = &self.board.cells[row][col];
        // Hidden and empty (with no surrounding mines) means no known mines nearby, and therefore
        // have no region. Marked cells are also useless.
        if sentinel.state != CellState::Visible {
            return None;
        }
        if sentinel.category == CellCategory::Empty(None) {
            return None;
        }

        let mut num_watched_mines = match (sentinel.state, sentinel.category) {
            // A hidden or marked cell contributes no information to its surrounding region.
            // Neither does a revealed cell that is empty.
            (CellState::Hidden, _) => return None,
            (CellState::Marked, _) => return None,
            (CellState::Visible, CellCategory::Empty(None)) => return None,
            // Needs further processing, since the cell contributes information.
            (CellState::Visible, CellCategory::Empty(Some(n))) => n,
            // This is an error state.
            (CellState::Visible, CellCategory::Mine) => panic!("Did not expect to attempt to solve a board with an exploded mine."),
        };
        let mut hidden = vec![];
        for watched_loc in self.board.surroundings_of(sentinel_loc) {
            let watched_cell = self.board.cells[watched_loc.1][watched_loc.0];
            match watched_cell.state {
                // Is known, and therefore not part of the region.
                CellState::Visible => (),
                // Is presumed known, and therefore not part of the region.
                CellState::Marked => {
                    // TODO this assertion can fail if a mistake is made.
                    assert!(num_watched_mines != 0);
                    num_watched_mines -= 1;
                },
                // Is unknown, and therefore required in analysis
                CellState::Hidden => {
                    hidden.push(watched_loc);
                },
            };
        }

        Some(Region {
            mines: vec![num_watched_mines],
            hidden,
        })
    }

    fn board_region(&self) -> Region {
        let mut num_mines = 0;
        let mut hidden = vec![];
        for loc in self.board_locs() {
            let (col, row) = loc;
            let cell = &self.board.cells[row][col];
            match cell {
                // Is known, and therefore not part of the region.
                // OR
                // Is presumed known, and therefore not part of the region.
                Cell { state: CellState::Visible, .. } | Cell { state: CellState::Marked, .. } => (),
                // Is unknown, and therefore required in analysis
                Cell { state: CellState::Hidden, category, .. } => {
                    hidden.push(loc);
                    if *category == CellCategory::Mine {
                        num_mines += 1;
                    }
                },
            };
        }

        Region {
            mines: vec![num_mines],
            hidden,
        }
    }

    fn extract_regions(&self) -> Vec<Region> {
        let mut rr = vec![self.board_region()];
        for (col, row) in self.board_locs() {
            if let Some(r) = self.region_around((col, row)) {
                rr.push(r)
            }
        }
        rr
    }
}

#[derive(Debug)]
struct StrippedRegions {
    zero_locs: Vec<(usize, usize)>,
    regions: Vec<Region>,
}

// Process region lists.
impl<'a> Solver<'a> {
    fn strip_zero_regions_from(&self, rr: Vec<Region>) -> StrippedRegions {
        let (mut zero, mut nonzero) = (vec![], vec![]);
        for r in rr {
            let mut has_nonzero_entry = false;
            for mine_cnt in &r.mines {
                let has_mine = *mine_cnt == 0;
                if !has_mine {
                    has_nonzero_entry = true;
                    break;
                }
            }
            if has_nonzero_entry {
                nonzero.push(r)
            } else {
                zero.push(r)
            }
        }
        let mut zero_locs: Vec<_> = zero.into_iter().flat_map(|r| r.hidden).collect();
        zero_locs.dedup();
        for r in &mut nonzero {
            r.hidden = r.hidden
                .drain(..)
                .filter(|loc| !zero_locs.contains(loc))
                .collect()
        }
        StrippedRegions {
            zero_locs,
            regions: nonzero,
        }
    }

pub struct KnownCells {
    empty: Vec<(usize, usize)>,
    mines: Vec<(usize, usize)>,
}

// The whole point of this struct.
impl<'a> Solver<'a> {
    pub fn calculate_known_cells(&self) -> Option<KnownCells> {
        let regions = self.extract_regions();
        let StrippedRegions { zero_locs, regions } = self.strip_zero_regions_from(regions);
        // TODO somehow recursively breakdown the list of regions into smaller regions and
        // eliminate duplicates until it can't be broken down anymore.
        // TODO Analyze the results after that.
        unimplemented!("Solver not yet fully functional.");
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn solver_test() {
    }
}
