use itertools::{self as iter, Itertools};
use std::{ops::{Bound, RangeBounds}, collections::VecDeque};

use indexmap::IndexSet;

use crate::board::{Cell, CellState, CellCategory, Board};

#[derive(Debug)]
struct LinkedSubRegion {
    mine_sets: IndexSet<(usize, usize, usize)>,
    r0: IndexSet<(usize, usize)>,
    rs: IndexSet<(usize, usize)>,
    r1: IndexSet<(usize, usize)>,
}

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
    fn remove_mines(&mut self, locs: &IndexSet<(usize, usize)>) {
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
    fn remove_empty(&mut self, locs: &IndexSet<(usize, usize)>) {
        self.remove_from_r(b'0', locs);
        self.remove_from_r(b's', locs);
        self.remove_from_r(b'1', locs);
        self.mine_sets.retain(|&(m0, ms, m1)| {
            m0 <= self.r0.len()
                && ms <= self.rs.len()
                && m1 <= self.r1.len()
        });
        // TODO Actually resolve this error, since it's a valid game state.
        assert!(self.mine_sets.len() != 1, "an empty set should be impossible if marks are correct.");
    }
}

// Regions have definite number of bombs.
#[derive(Debug)]
struct Region {
    // Each bound is "or"d with the others.
    mines: usize,
    hidden: IndexSet<(usize, usize)>,
}

impl Region {
    fn all_mines(&self) -> bool {
        self.hidden.len() == self.mines
    }

    fn remove_locs_from_hidden<'a, I: IntoIterator<Item = &'a (usize, usize)>>(&mut self, locs: I) -> usize {
        let mut num_removed = 0;
        for remove_loc in locs {
            if self.hidden.remove(remove_loc) {
                num_removed += 1;
            }
        }
        num_removed
    }

    fn remove_empty_locs<'a, I: IntoIterator<Item = &'a (usize, usize)>>(&mut self, locs: I) {
        self.remove_locs_from_hidden(locs);
    }

    fn remove_mine_locs<'a, I: IntoIterator<Item = &'a (usize, usize)>>(&mut self, locs: I) {
        let num_removed = self.remove_locs_from_hidden(locs);
        self.mines -= num_removed;
    }
}

fn split_sets<T: Eq + std::hash::Hash + Copy>(aa: IndexSet<T>, bb: IndexSet<T>) -> (IndexSet<T>, IndexSet<T>, IndexSet<T>) {
    (
        aa.difference(&bb).cloned().collect(),
        aa.intersection(&bb).cloned().collect(),
        bb.difference(&aa).cloned().collect(),
    )
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
        let mut hidden = IndexSet::new();
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
                    hidden.insert(watched_loc);
                },
            };
        }

        Some(Region {
            mines: num_watched_mines as usize,
            hidden,
        })
    }

    fn board_region(&self) -> Region {
        let mut num_mines = 0;
        let mut hidden = IndexSet::new();
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
                    hidden.insert(loc);
                    if *category == CellCategory::Mine {
                        num_mines += 1;
                    }
                },
            };
        }

        Region {
            mines: num_mines,
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
    zero_locs: IndexSet<(usize, usize)>,
    regions: Vec<Region>,
}

#[derive(Debug)]
struct SegmentedRegions {
    r0: Option<Region>,
    rs: Region, // shared
    r1: Option<Region>,
}

// Process region lists.
impl<'a> Solver<'a> {
    fn strip_zero_regions_from(&self, rr: Vec<Region>) -> StrippedRegions {
        let (mut zero, mut nonzero) = (vec![], vec![]);
        for r in rr {
            if r.mines == 0 {
                nonzero.push(r)
            } else {
                zero.push(r)
            }
        }
        let zero_locs: IndexSet<_> = zero.into_iter().flat_map(|r| r.hidden).collect();
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

    fn establish_regional_links(
        &self,
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

        let linkages = IndexSet::new();
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

pub struct KnownCells {
    pub empty: IndexSet<(usize, usize)>,
    pub mines: IndexSet<(usize, usize)>,
}

// The whole point of this struct.
impl<'a> Solver<'a> {
    // Error when board state contradicts itself. Typically due to error in placing a flagged cell.
    pub fn calculate_known_cells(&self) -> Result<Option<KnownCells>, ()> {
        let regions = self.extract_regions();
        let StrippedRegions { mut zero_locs, regions } = self.strip_zero_regions_from(regions);
        let mine_locs = IndexSet::new();
        // Find linked
        let mut links = regions.iter()
            .enumerate()
            .flat_map(|(i, p0)| regions[i..].iter().map(|p1| (p0, p1)))
            .filter_map(|(p0, p1)| self.establish_regional_links(p0, p1))
            .collect::<VecDeque<_>>();
        while let Some(link) = links.pop_front() {
            if link.mine_sets.len() == 1 { // Only one variant exists.
                let LinkedSubRegion { r0, rs, r1, mine_sets } = link;
                let (m0, ms, m1) = mine_sets.pop()
                    .expect("the element that was just reported to be there.");
                let mut link_zero_locs = IndexSet::new();
                let mut link_mine_locs = IndexSet::new();
                if m0 == 0 {
                    link_zero_locs.extend(r0);
                } else if m0 == r0.len() {
                    link_mine_locs.extend(r0)
                }
                if m1 == 0 {
                    link_zero_locs.extend(r1);
                } else if m1 == r1.len() {
                    link_mine_locs.extend(r1)
                }
                if ms == 0 {
                    link_zero_locs.extend(rs);
                } else if ms == rs.len() {
                    link_mine_locs.extend(r1)
                }
                for link in links {
                    link.remove_mines(&link_mine_locs);
                    link.remove_empty(&link_zero_locs);
                    // TODO There are more conclusions available than just this. Figure out what
                    // they are.
                }
                for region in regions {
                    region.remove_mine_locs(&link_mine_locs);
                    region.remove_empty_locs(&link_zero_locs);
                }
                mine_locs.extend(link_mine_locs);
                zero_locs.extend(link_zero_locs);
            } else { // Do nothing, as more than one variant exists.
                links.push_back(link);
            }
        }
        // TODO There will be 3 categories of spots: unknown, is_mine, is_empty.
        // unimplemented!("Solver not yet fully functional.");
        if zero_locs.is_empty() && mine_locs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(KnownCells {
                empty: zero_locs,
                mines: mine_locs,
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
