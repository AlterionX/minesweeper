use itertools as iter;
use std::ops::{Bound, RangeBounds};

use crate::board::{Cell, CellState, CellCategory, Board};

struct Region {
    mines: Vec<(Bound<usize>, Bound<usize>)>,
    marked: Vec<(usize, usize)>,
    hidden: Vec<(usize, usize)>,
}

// TODO Make this entire process more efficient. Cause it should be possible.
pub struct Solver<'a> {
    pub board: &'a Board,
}

// Region extraction methods.
impl<'a> Solver<'a> {
    fn region_around(&self, sentinel_loc @ (_, _): (usize, usize)) -> Option<Region> {
        let (col, row) = sentinel_loc;
        let sentinel = &mut self.board.cells[row][col];
        // Hidden and empty (with no surrounding mines) means no known mines nearby, and therefore
        // have no region. Marked cells are also useless.
        if sentinel.state != CellState::Visible {
            return None;
        }
        if sentinel.category == CellCategory::Empty(None) {
            return None;
        }

        let num_watched_mines = match (sentinel.state, sentinel.category) {
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
        let mut marked = vec![];
        for watched_loc in self.board.surroundings_of(sentinel_loc) {
            let watched_cell = self.board.cells[watched_loc.1][watched_loc.0];
            match sentinel.state {
                // Is known, and therefore not part of the region.
                CellState::Visible => (),
                // Is presumed known, and therefore not part of the region.
                CellState::Marked => {
                    marked.push(watched_loc);
                },
                // Is unknown, and therefore required in analysis
                CellState::Hidden => {
                    hidden.push(watched_loc);
                },
            };
        }

        Some(Region {
            mines: vec![(Bound::Included(num_watched_mines as usize), Bound::Included(num_watched_mines as usize))],
            hidden,
            marked,
        })
    }

    fn extract_regions(&self) -> Vec<Region> {
        let mut rr = vec![];
        for row in 0..self.board.h() {
            for col in 0..self.board.w() {
                if let Some(r) = self.region_around((col, row)) {
                    rr.push(r)
                }
            }
        }
        rr
    }
}

// Process region lists.
impl<'a> Solver<'a> {
}

impl<'a> Solver<'a> {
    pub fn calculate_known_cells(&self) -> Option<Vec<(usize, usize)>> {
        let regions = self.extract_regions();
        unimplemented!("Solver not yet fully functional.");
    }
}
