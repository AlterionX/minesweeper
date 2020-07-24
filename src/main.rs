use std::{io::{stdin, stdout, Read, Write}, collections::VecDeque};
use structopt::StructOpt;

use termion::{
    raw::{IntoRawMode, RawTerminal},
    input::{TermRead, MouseTerminal, Events},
    event::{Key, MouseButton, Event, MouseEvent},
};

mod board;
use board::{Board, Dim, Error};

mod opts;
use opts::{Opts, Def, Preset};

enum Direction {
    Up,
    Left,
    Down,
    Right,
}

enum Action {
    Mark,
    Dig,
    LaunchProbe,
    ExitGame,
    Move(Direction),
    JumpTo((usize, usize)),
}
struct Input {
    action: Action,
    point: (usize, usize),
}

fn read_input<T: Read + TermRead>(stream: &mut Events<T>) -> Result<Option<(Action, Option<Action>)>, ()> {
    let next_event = stream.next().expect("Terminal read to be fine."); // TODO Convert `expect` to `Error`.
    match next_event {
        Err(_) => {
            // Unexpected error. TODO Consider panicking.
            Err(())
        },
        Ok(ev) => match ev {
            Event::Unsupported(_) => Ok(None),
            Event::Key(k) => {
                let action = match k {
                    Key::Up | Key::Char('w') | Key::Char('k') =>
                        Some(Action::Move(Direction::Up)),
                    Key::Left | Key::Char('a') | Key::Char('h') =>
                        Some(Action::Move(Direction::Left)),
                    Key::Down | Key::Char('s') | Key::Char('j') =>
                        Some(Action::Move(Direction::Down)),
                    Key::Right | Key::Char('d') | Key::Char('l') =>
                        Some(Action::Move(Direction::Right)),
                    Key::Char('m') => Some(Action::Mark),
                    Key::Char('u') => Some(Action::Dig),
                    Key::Char('q') => Some(Action::ExitGame),
                    Key::Char('!') => Some(Action::LaunchProbe),
                    _ => None,
                };
                Ok(action.map(|a| (a, None)))
            },
            Event::Mouse(m) => match m {
                MouseEvent::Release(_, _) => Ok(None),
                MouseEvent::Hold(_, _) => Ok(None),
                MouseEvent::Press(button, x, y) => {
                    // TODO Figure out how to handle two buttons down at the same time.
                    let action = match button {
                        MouseButton::Left => Action::Dig,
                        MouseButton::Right => Action::Mark,
                        _ => return Ok(None),
                    };

                    for ev in stream {
                        match ev {
                            Err(_) => return Ok(None),
                            // Ignore until we hit a release.
                            Ok(e) => {
                                if let Event::Mouse(MouseEvent::Release(x_up, y_up)) = e {
                                    if (x_up, y_up) == (x, y) {
                                        // Input events are 1 indexed, so we're converting it to
                                        // being 0 indexed.
                                        let coords = (x_up as usize - 1, y_up as usize - 1);
                                        return Ok(Some(
                                            (Action::JumpTo(coords), Some(action))
                                        ));
                                    } else {
                                        return Ok(None);
                                    }
                                }
                            }
                        }
                    }

                    // Unexpected end.
                    Err(())
                },
            }
        },
    }
}

fn print_board<W: Write>(
    output: &mut RawTerminal<W>,
    board: &Board,
    top_left: (usize, usize),
    current_point: (usize, usize),
) -> Option<(usize, usize)> {
    let mut new_top_left = top_left;
    let size = termion::terminal_size().expect("no problem getting the terminal size.");
    let size = (size.0 as usize, size.1 as usize);
    if size.0 == 0 || size.1 == 0 {
        return None;
    }
    let bot_right = (top_left.0 + size.0, top_left.1 + size.1);
    if current_point.0 >= bot_right.0 {
        new_top_left.0 = current_point.0 - size.0;
    }
    if current_point.0 < top_left.0 {
        new_top_left.0 = current_point.0;
    }
    if current_point.1 >= bot_right.1 {
        new_top_left.1 = current_point.1 - size.1;
    }
    if current_point.1 < top_left.1 {
        new_top_left.1 = current_point.1;
    }

    let snippet = board.display(size, new_top_left).expect("no problem with updating the screen.");

    write!(output, "{}{}", termion::clear::All, termion::cursor::Goto(1, 1))
       .expect("write to be fine.");
    for row in &snippet[..] {
        for cell in &row[..] {
            write!(output, "{}", cell).expect("output to standard out without an issue.");
        }
        write!(output, "\n\r").expect("write to be fine.");
    }
    write!(
        output,
        "{}",
        termion::cursor::Goto(
            (current_point.0 + 1) as u16,
            (current_point.1 + 1) as u16,
        ),
    ).expect("write to be fine.");
    output.flush().expect("flush to be fine.");

    Some(new_top_left)
}

fn main() {
    let cfg = Opts::from_args();

    println!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
    // TODO ASCII art for the welcome message.
    println!("\
Hello, and welcome to Minesweeper. (The ASCII art is in the works. I swear.)
We're working on the control scheme, but for now press:
\tup/w/k to move up
\tleft/a/h to move left
\tdown/s/j to move down
\tright/d/l to move right
\tm/right click to mark
\tu/left click on a hidden tile to reveal
\tu/left click on an exposed tile to chord

Press any key to continue.");

    let mut stdout = MouseTerminal::from(stdout().into_raw_mode().unwrap());
    let mut events = stdin().events();
    while let None = events.next() {}

    let mut board = match cfg.def {
        Def::Preset(Preset::Beginner) => Board::beginner(),
        Def::Preset(Preset::Intermediate) => Board::intermediate(),
        Def::Preset(Preset::Advanced) => Board::advanced(),
        Def::Descrip { width, height: Some(height), mines } => Board::new(Dim::Rect(width, height), mines),
        Def::Descrip { width, height: None, mines } => Board::new(Dim::Square(width), mines),
    }.expect("board to be created without a hitch.");

    let mut current_point = (0, 0);
    let mut queued_actions = VecDeque::new();
    let mut top_left = (0, 0);
    print_board(&mut stdout, &board, top_left, current_point);

    loop {
        let input = if queued_actions.is_empty() {
            match read_input(&mut events) {
                Ok(Some((action, secondary))) => {
                    if let Some(to_queue) = secondary {
                        queued_actions.push_back(to_queue)
                    }
                    Input {
                        action,
                        point: current_point,
                    }
                },
                Ok(None) => continue,
                // Unexpected error, but carry on instead of terminating.
                Err(_) => continue,
            }
        } else {
            Input {
                action: queued_actions.pop_front()
                    .expect("just checked action to be present."),
                point: current_point,
            }
        };
        // TODO Get input from terminal.
        let res = match input.action {
            Action::ExitGame => break,
            Action::LaunchProbe => board.launch_probe(),
            Action::Mark => board.mark(input.point),
            Action::Dig => board.dig(input.point),
            Action::JumpTo(p) => {
                if board.is_loc(p) {
                    current_point = p;
                }
                Ok(())
            },
            Action::Move(d) => {
                match d {
                    Direction::Up => {
                        if current_point.1 != 0 {
                            current_point.1 -= 1;
                        }
                    },
                    Direction::Left => {
                        if current_point.0 != 0 {
                            current_point.0 -= 1;
                        }
                    },
                    Direction::Down => {
                        current_point.1 += 1;
                        if !board.is_loc(current_point) {
                            current_point.1 -= 1
                        }
                    },
                    Direction::Right => {
                        current_point.0 += 1;
                        if !board.is_loc(current_point) {
                            current_point.0 -= 1
                        }
                    },
                };
                Ok(())
            },
        };

        if let Some(new_top_left) = print_board(&mut stdout, &board, top_left, current_point) {
            top_left = new_top_left;
        }

        match res {
            Ok(_) => (),
            // Somehow print here.
            Err(Error::OOB) => continue,
            Err(Error::Marked) => continue,
            Err(Error::Dead) => {
                let size = termion::terminal_size()
                    .expect("no problem getting the terminal size.");
                write!(stdout, "{}", termion::cursor::Goto(0, size.1 - 1))
                    .expect("write to be fine.");
                write!(stdout, "You have died!")
                    .expect("write to be fine.");
                break
            },
        }

        if board.is_completed() {
            let size = termion::terminal_size()
                .expect("no problem getting the terminal size.");
            write!(stdout, "{}", termion::cursor::Goto(0, size.1 - 1))
                .expect("write to be fine.");
            write!(stdout, "Congratulations!")
                .expect("write to be fine.");
            break;
        }
    }

    write!(stdout, "\n\rThanks for playing! Farewell.\n\r")
        .expect("write to be fine.");
}
