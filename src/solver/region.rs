use indexmap::IndexSet;

use crate::{
    board::{
        Cell,
        CellState,
        CellCategory,
        Board,
    },
    util::split_sets,
};

// Regions have definite number of bombs.
#[derive(Debug, PartialEq, Eq)]
pub struct Region {
    // Each bound is "or"d with the others.
    mines: usize,
    hidden: IndexSet<(usize, usize)>,
}

// Construction.
impl Region {
    pub fn new(mines: usize, hidden: IndexSet<(usize, usize)>) -> Self {
        Self {
            mines,
            hidden,
        }
    }

    pub fn around(board: &Board, sentinel_loc @ (_, _): (usize, usize)) -> Option<Region> {
        let (col, row) = sentinel_loc;
        let sentinel = &board.cells[row][col];
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
        let mut hidden = IndexSet::new();
        for watched_loc in board.surroundings_of(sentinel_loc) {
            let watched_cell = board.cells[watched_loc.1][watched_loc.0];
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
                    hidden.insert(watched_loc);
                },
            };
        }

        Some(Region::new(num_watched_mines as usize, hidden))
    }

    pub fn board(board: &Board) -> Region {
        let mut num_mines: usize = 0;
        let mut num_flagged: usize = 0;
        let mut hidden = IndexSet::new();
        for loc in board.all_locs() {
            let (row, col) = loc;
            let cell = &board.cells[row][col];
            if cell.category == CellCategory::Mine {
                num_mines += 1;
            }
            match cell {
                // Is known. Contributes no information.
                Cell { state: CellState::Visible, .. } => {},
                // Is presumed to be a mine, so detracted from original num_mines.
                Cell { state: CellState::Marked, .. } => {
                    num_flagged += 1;
                },
                // Is unknown, and location is therefore required in analysis.
                Cell { state: CellState::Hidden, .. } => {
                    hidden.insert(loc);
                },
            };
        }

        assert!(num_flagged < num_mines, "flagged cells to be less than mined cells.");
        Self::new(num_mines - num_flagged, hidden)
    }
}

// Stat calculation.
impl Region {
    pub fn is_all_mines(&self) -> bool {
        self.hidden.len() == self.mines
    }

    pub fn is_all_empty(&self) -> bool {
        self.mines == 0
    }
}

// Removing locations from an individual region.
impl Region {
    fn remove_locs_from_hidden<'a, I: IntoIterator<Item = &'a (usize, usize)>>(&mut self, locs: I) -> usize {
        let mut num_removed = 0;
        for remove_loc in locs {
            if self.hidden.remove(remove_loc) {
                num_removed += 1;
            }
        }
        num_removed
    }

    pub fn remove_empty_locs<'a, I: IntoIterator<Item = &'a (usize, usize)>>(&mut self, locs: I) {
        self.remove_locs_from_hidden(locs);
    }

    pub fn remove_mine_locs<'a, I: IntoIterator<Item = &'a (usize, usize)>>(&mut self, locs: I) {
        let num_removed = self.remove_locs_from_hidden(locs);
        self.mines -= num_removed;
    }
}

#[derive(Debug)]
pub struct StrippedRegions {
    pub locs: IndexSet<(usize, usize)>,
    pub regions: Vec<Region>,
}

// Removing regions that are empty/mined.
impl Region {
    pub fn strip_zero_regions_from(rr: Vec<Region>) -> StrippedRegions {
        let (mut zero, mut nonzero) = (vec![], vec![]);
        for r in rr {
            if r.is_all_empty() {
                zero.push(r)
            } else {
                nonzero.push(r)
            }
        }
        let zero_locs = zero.into_iter().flat_map(|r| r.hidden).collect::<IndexSet<_>>();
        for r in &mut nonzero {
            r.remove_empty_locs(&zero_locs);
        }
        StrippedRegions {
            locs: zero_locs,
            regions: nonzero,
        }
    }

    pub fn strip_mine_regions_from(rr: Vec<Region>) -> StrippedRegions {
        let (mut mined, mut partially_unmined) = (vec![], vec![]);
        for r in rr {
            if r.is_all_mines() {
                mined.push(r)
            } else {
                partially_unmined.push(r)
            }
        }
        let mined_locs = mined.into_iter().flat_map(|r| r.hidden).collect::<IndexSet<_>>();
        for r in &mut partially_unmined {
            r.remove_mine_locs(&mined_locs);
        }
        StrippedRegions {
            locs: mined_locs,
            regions: partially_unmined,
        }
    }
}

#[derive(Debug)]
pub struct LinkedSubRegion {
    pub mine_sets: IndexSet<(usize, usize, usize)>,
    pub r0: IndexSet<(usize, usize)>,
    pub rs: IndexSet<(usize, usize)>,
    pub r1: IndexSet<(usize, usize)>,
}

// Create LinkedSubRegions from Regions.
impl LinkedSubRegion {
    pub fn deduce_links(
        parent0: &Region,
        parent1: &Region,
    ) -> Option<LinkedSubRegion> {
        let (r0_hidden, rs_hidden, r1_hidden) =
            split_sets(parent0.hidden.clone(), parent1.hidden.clone());
        // TODO Calculate the constraints on mines, given what is required.
        let rs_num_hidden = rs_hidden.len();
        let r0_num_hidden = r0_hidden.len();
        let r1_num_hidden = r1_hidden.len();
        if rs_num_hidden == 0 {
            // There is no overlap, so do nothing.
            return None;
        }

        let (p0_mines, p1_mines) = (parent0.mines, parent1.mines);

        // The maximum number of shared mines is obviously bounded by three things:
        // - The number of hidden cells
        // - The number of mines present in one parent region
        // - The number of mines present in the other parent region
        let rs_max_mines = (rs_num_hidden).max(p0_mines).max(p1_mines);
        // The minimum number of shared mines is obviously bounded by three things:
        // - 0
        // - The number of mines that don't fit in region 0 of parent 0
        // - The number of mines that don't fit in region 1 of parent 1
        let rs_min_mines = {
            let r0_min_contribution = if p0_mines < r0_num_hidden {
                0
            } else {
                p0_mines - r0_num_hidden
            };
            let r1_min_contribution = if p1_mines < r1_num_hidden {
                0
            } else {
                p1_mines - r1_num_hidden
            };
            r0_min_contribution.max(r1_min_contribution)
        };

        let mut linkages = IndexSet::new();
        for rs_mines in rs_min_mines..=rs_max_mines {
            let r0_mines = p0_mines - rs_mines;
            let r1_mines = p1_mines - rs_mines;
            linkages.insert((r0_mines, rs_mines, r1_mines));
        }

        Some(LinkedSubRegion {
            mine_sets: linkages,
            r0: r0_hidden,
            rs: rs_hidden,
            r1: r1_hidden,
        })
    }
}

// Manipulate locations in the linked regions.
impl LinkedSubRegion {
    fn remove_from_r(&mut self, c: u8, locs: &IndexSet<(usize, usize)>) -> usize {
        let r = match c {
            b'0' => &mut self.r0,
            b's' => &mut self.rs,
            b'1' => &mut self.r1,
            _ => panic!("bad region number provided to linked sub region: {}", c),
        };
        let original_r_size = r.len();
        r.retain(|l| !locs.contains(l));
        let removed = original_r_size - r.len();
        removed
    }

    pub fn remove_mines(&mut self, locs: &IndexSet<(usize, usize)>) {
        // Iterate over locations to remove
        let r0_rem = self.remove_from_r(b'0', locs);
        let rs_rem = self.remove_from_r(b's', locs);
        let r1_rem = self.remove_from_r(b'1', locs);
        self.mine_sets = self.mine_sets.drain(..)
            .filter(|&(m0, ms, m1)| {
                m0 >= r0_rem
                    && ms >= rs_rem
                    && m1 >= r1_rem
            })
            .map(|(m0, ms, m1)| (
                m0 - r0_rem,
                ms - rs_rem,
                m1 - r1_rem,
            ))
            .collect();
    }

    pub fn remove_empty(&mut self, locs: &IndexSet<(usize, usize)>) {
        self.remove_from_r(b'0', locs);
        self.remove_from_r(b's', locs);
        self.remove_from_r(b'1', locs);
        let (r0_len, rs_len, r1_len) = (self.r0.len(), self.rs.len(), self.r1.len());
        self.mine_sets.retain(|&(m0, ms, m1)| {
            m0 <= r0_len
                && ms <= rs_len
                && m1 <= r1_len
        });
        // TODO Actually resolve this error, since it's a valid game state.
        assert!(self.mine_sets.len() != 1, "an empty set should be impossible if marks are correct.");
    }
}

#[cfg(test)]
mod test {
    use indexmap::IndexSet;
    use crate::board::Board;

    use super::Region;

    const MINES: usize = 5;
    const LOCS: [(usize, usize); MINES] = [
        (0, 0),
        (0, 4),
        (0, 9),
        (1, 2),
        (1, 8),
    ];
    fn test_board() -> Board {
        Board::from_save(include_bytes!("../../testing/boards/basic.txt"))
            .expect("board to parse correctly from file.")
    }

    #[test]
    fn new_test() { // Really?
        let locs: IndexSet<_> = LOCS.iter().cloned().collect();
        let r = Region::new(MINES, locs);
    }

    #[test]
    fn board_test() { // Really?
        let locs: IndexSet<_> = LOCS.iter().cloned().collect();
        let r = Region::new(MINES, locs);
        let test = Region::board(&test_board());

        assert!(r == test);
    }

    #[test]
    fn surroundings_test() { // Really?
        let r = Region::new(0, IndexSet::new());

        let b = test_board();
        assert!(true);
    }
}

