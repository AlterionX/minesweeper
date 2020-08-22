use rand::{Rng, RngCore, SeedableRng, distributions::Uniform, rngs::OsRng};
use rand_xoshiro::Xoshiro256PlusPlus as BaseRng;

use crate::solver::Solver;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Error {
    OOB,
    Dead,
    Marked,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CellCategory {
    Mine,
    Empty(Option<u8>),
}

impl Default for CellCategory {
    fn default() -> Self {
        Self::Empty(None)
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CellState {
    Hidden,
    Marked,
    Visible,
}

impl Default for CellState {
    fn default() -> Self {
        Self::Hidden
    }
}

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct Cell {
    // TODO convert hidden/marked into enum Hidden/Marked/Visible
    pub state: CellState,
    pub category: CellCategory,
    pub scratch: bool,
}

impl Cell {
    fn to_char(&self) -> char {
        match self.state {
            CellState::Hidden => '\u{25A1}',
            CellState::Marked => 'F',
            CellState::Visible => match self.category {
                CellCategory::Mine => 'M',
                CellCategory::Empty(None) => '\u{25A0}',
                CellCategory::Empty(Some(n)) => (b'0' + n) as char,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Dim {
    Square(usize),
    Rect(usize, usize),
}

impl Dim {
    fn w(&self) -> usize {
        match self {
            Dim::Square(n) => *n,
            Dim::Rect(n, _) => *n,
        }
    }
    fn h(&self) -> usize {
        match self {
            Dim::Square(n) => *n,
            Dim::Rect(_, n) => *n,
        }
    }
}

pub struct Board {
    pub cells: Box<[Box<[Cell]>]>,
    dims: (usize, usize),
}

// Helpers
impl Board {
    pub fn is_loc(&self, (x, y): (usize, usize)) -> bool {
        (0..self.dims.0).contains(&x) && (0..self.dims.1).contains(&y)
    }

    pub fn surroundings_of(&self, loc: (usize, usize)) -> impl Iterator<Item = (usize, usize)> {
        let dims = self.dims;
        (0..9)
            .map(|i| (i % 3, i / 3))
            // Remove out of bounds and loc.
            .filter(move |offset| {
                if *offset == (1, 1) {
                    return false;
                }

                // check x
                if offset.0 == 0 && loc.0 == 0 {
                    // If decrement and at minimum
                    return false;
                }
                if offset.0 == 2 && loc.0 == (dims.0 - 1) {
                    // If increment and at maximum
                    return false;
                }

                // check y
                if offset.1 == 0 && loc.1 == 0 {
                    // If decrement and at minimum
                    return false;
                }
                if offset.1 == 2 && loc.1 == (dims.1 - 1) {
                    // If increment and at maximum
                    return false;
                }

                true
            })
            // Map offsets to actual locations.
            .map(move |offset| {
                // offset
                // 0 means decrement
                // 1 means ignore
                // 2 means increment
                let x = match offset.0 {
                    0 => loc.0 - 1, // decrement
                    2 => loc.0 + 1, // increment
                    _ => loc.0, // Ignore 1 and everything else
                };
                let y = match offset.1 {
                    0 => loc.1 - 1, // decrement
                    2 => loc.1 + 1, // increment
                    _ => loc.1, // Ignore 1 and everything else
                };
                (x, y)
            })
    }

    pub fn w(&self) -> usize {
        self.dims.0
    }

    pub fn h(&self) -> usize {
        self.dims.1
    }
}

// Constructors
impl Board {
    pub fn beginner() -> Result<Self, ()> {
        Self::new(Dim::Square(9), 10)
    }

    pub fn intermediate() -> Result<Self, ()> {
        Self::new(Dim::Square(16), 40)
    }

    pub fn advanced() -> Result<Self, ()> {
        Self::new(Dim::Rect(30, 16), 99)
    }

    pub fn new(dim: Dim, num_mines: u64) -> Result<Self, ()> {
        let mut seed = [0; 32];
        OsRng.fill_bytes(&mut seed);
        Self::new_seeded(dim, num_mines, seed)
    }

    pub fn new_seeded(dim: Dim, num_mines: u64, seed: <BaseRng as SeedableRng>::Seed) -> Result<Self, ()> {
        let mut randos = BaseRng::from_seed(seed);

        let mut seed = [[0; 32]; 2];
        randos.fill_bytes(&mut seed[0]);
        randos.fill_bytes(&mut seed[1]);
        let x_rng = BaseRng::from_seed(seed[0]);
        let y_rng = BaseRng::from_seed(seed[1]);

        let x_range = Uniform::from(0..dim.w());
        let y_range = Uniform::from(0..dim.h());

        let (x_randos, y_randos) = (x_rng.sample_iter(x_range), y_rng.sample_iter(y_range));

        Self::new_fixed(dim, x_randos.zip(y_randos).take(num_mines as usize))
    }

    pub fn new_fixed<I>(dim: Dim, locs: I) -> Result<Self, ()> where I: IntoIterator<Item = (usize, usize)> {
        let (w, h) = (dim.w(), dim.h());
        let mut cells = vec![vec![Cell::default(); w as usize]; h as usize]
            .into_iter()
            .map(|v| v.into_boxed_slice())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        for loc in locs.into_iter() {
            cells[loc.1][loc.0].category = CellCategory::Mine;
        }

        Self::from_cells(cells)
    }

    #[cfg(test)]
    pub fn from_save(cells: &[u8]) -> Result<Self, ()> {
        let mut board = {
            let mut board = vec![];
            let mut row = vec![];
            for cell in cells {
                match cell {
                    b'\n' => {
                        board.push(row.into_boxed_slice());
                        row = vec![];
                    }
                    b' ' => {
                        row.push(Cell {
                            state: CellState::Visible,
                            category: CellCategory::Empty(None),
                            scratch: false,
                        })
                    }
                    b'x' => {
                        row.push(Cell {
                            state: CellState::Hidden,
                            category: CellCategory::Mine,
                            scratch: false,
                        })
                    }
                    b'H' => {
                        row.push(Cell {
                            state: CellState::Hidden,
                            category: CellCategory::Empty(None),
                            scratch: false,
                        })
                    }
                    _ => return Err(()),
                }
            }
            if !row.is_empty() {
                board.push(row.into_boxed_slice())
            }
            board.into_boxed_slice()
        };

        // Validate board size.
        let h = board.len();
        let w = board.first().map_or(0, |v| v.len());
        for row in board.iter() {
            if row.len() != w {
                return Err(());
            }
        }

        Self::from_cells(board)
    }

    pub fn from_cells(cells: Box<[Box<[Cell]>]>) -> Result<Self, ()> {
        let h = cells.len();
        let w = cells.first().map_or(0, |v| v.len());
        let mut board = Self {
            cells,
            dims: (w, h),
        };

        for row in 0..h {
            for col in 0..w {
                let category = board.cells[row][col].category;
                if category == CellCategory::Mine {
                    continue
                }
                let surroundings = board.surroundings_of((col, row));
                let nearby_bombs = surroundings
                    .filter(|(x, y)| board.cells[*y][*x].category == CellCategory::Mine)
                    .count() as u8;
                if nearby_bombs == 0 {
                    continue
                }
                board.cells[row][col].category = CellCategory::Empty(Some(nearby_bombs));
            }
        }

        Ok(board)
    }
}

// Marking and digging.
impl Board {
    pub fn mark(&mut self, point: (usize, usize)) -> Result<(), Error> {
        let (x, y) = point;
        if !self.is_loc(point) {
            // TODO Consider replacing this error with an assert.
            return Err(Error::OOB);
        }

        let cell = &mut self.cells[y][x];
        cell.state = match cell.state {
            CellState::Hidden => CellState::Marked,
            CellState::Marked => CellState::Hidden,
            CellState::Visible => CellState::Visible,
        };
        Ok(())
    }

    fn chord(&mut self, point: (usize, usize), target_num_mines: u8) -> Result<(), Error> {
        let surroundings: Vec<_> = self.surroundings_of(point)
            .collect();
        let marked_mines = surroundings.iter()
            .filter(|(x, y)| self.cells[*y][*x].state == CellState::Marked)
            .count() as u8;
        if marked_mines != target_num_mines {
            return Ok(());
        }
        let unmarked_mines = surroundings.iter()
            .filter(|(x, y)| {
                let cell = &mut self.cells[*y][*x];
                (cell.state != CellState::Marked) && (cell.category == CellCategory::Mine)
            })
            .count() as u8;
        for (x, y) in surroundings.into_iter() {
            let cell = &mut self.cells[y][x];
            if cell.state != CellState::Marked {
                if cell.category == CellCategory::Empty(None) {
                    self.dig_region((x, y))?;
                } else {
                    cell.state = CellState::Visible;
                }
            }
        }
        if unmarked_mines != 0 {
            Err(Error::Dead)
        } else {
            Ok(())
        }
    }

    fn dig_region(&mut self, start: (usize, usize)) -> Result<(), Error> {
        let mut scanning_locs = vec![start];
        for y in 0..self.dims.1 {
            for x in 0..self.dims.0 {
                self.cells[y][x].scratch = false;
            }
        }
        self.cells[start.1][start.0].state = CellState::Visible;
        while let Some(loc) = scanning_locs.pop() {
            self.surroundings_of(loc).for_each(|to_scan_loc @ (_, _)| {
                let (x, y) = to_scan_loc;
                let cell = &mut self.cells[y][x];
                if let CellCategory::Empty(num_mines) = cell.category {
                    // Only reveal if no mines in surroundings.
                    if num_mines.is_none() && cell.state != CellState::Marked && !cell.scratch {
                        cell.scratch = true;
                        scanning_locs.push(to_scan_loc);
                    }
                    if cell.state != CellState::Marked {
                        cell.state = CellState::Visible;
                    }
                } else {
                    unimplemented!("Found a mine while flood filling an empty region. This should be impossible.");
                }
            });
        }
        Ok(())
    }

    pub fn dig(&mut self, point: (usize, usize)) -> Result<(), Error> {
        let (x, y) = point;
        if !self.is_loc(point) {
            // TODO Consider replacing this error with an assert.
            return Err(Error::OOB);
        }
        let cell = &mut self.cells[y][x];
        if cell.state == CellState::Marked {
            return Err(Error::Marked);
        }

        match cell.category {
            CellCategory::Mine => Err(Error::Dead),
            CellCategory::Empty(None) => if cell.state == CellState::Hidden {
                self.dig_region(point)
            } else {
                Ok(())
            },
            CellCategory::Empty(Some(num_mines)) => if cell.state == CellState::Hidden {
                cell.state = CellState::Visible;
                Ok(())
            } else {
                self.chord(point, num_mines)
            },
        }
    }
}

// Probing and stat checking.
impl Board {
    pub fn is_all_but_mines_revealed(&self) -> bool {
        let (w, h) = self.dims;
        for row in 0..h {
            for col in 0..w {
                let cell = self.cells[row][col];
                if cell.category != CellCategory::Mine && cell.state != CellState::Visible {
                    return false;
                }
            }
        }
        true
    }

    pub fn launch_probe(&self) -> Result<(), Error> {
        // Check for any 100% valid moves.
        let valid_moves = Solver { board: self }.calculate_known_cells()
            .expect("player did not make a mistake. Which needs to be dealt with eventually, since humans always make mistakes. Except that one person. Yeah, that one.");
        if valid_moves.is_some() {
            Err(Error::Dead)
        } else {
            Ok(())
        }
    }
}

impl Board {
    pub fn display(&self, max_dims: (usize, usize), top_left: (usize, usize)) -> Result<Box<[Box<[char]>]>, ()> {
        let rem_dims = (self.dims.0 - top_left.0, self.dims.1 - top_left.1);
        let true_dims = (max_dims.0.min(rem_dims.0), max_dims.1.min(rem_dims.1));
        let mut snippet = vec![vec!['\u{25A1}'; true_dims.0]; true_dims.1]
            .into_iter()
            .map(|row| row.into_boxed_slice())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        for row in 0..true_dims.1 {
            for col in 0..true_dims.0 {
                let cell = &self.cells[row][col];
                snippet[row][col] = cell.to_char();
            }
        }
        Ok(snippet)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn board_surroundings_iter() {
        let board = Board::new(20, 20, rand::rngs::SmallRngs::from_entropy());
    }
}
