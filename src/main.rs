use rand::prelude::*;
use std::io::{stdout, Write};
use termion::async_stdin;
use termion::cursor::Goto;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;

const UP_DOWN_TICK: u64 = 150;
const LEFT_RIGHT_TICK: u64 = 50;
const HEADER_SPACE: u16 = 2;

const BORDER: char = '=';
const EMPTY: char = ' ';
const SNAKE: char = '#';
const APPLE: char = '@';

fn main() -> std::io::Result<()> {
    let num;
    {
        let mut stdout = stdout().into_raw_mode()?;
        let (x, y) = termion::terminal_size()?;
        let mut reader = async_stdin().events().into_iter();
        write!(stdout, "{}", termion::clear::All)?;
        stdout.flush()?;
        let mut game = Game::new(x, y);
        game.setup(&mut stdout)?;
        num = game.play(&mut stdout, &mut reader)?;
        write!(stdout, "{}", Goto(1, 1))?;
    }
    println!("You caught {} apples", num);
    Ok(())
}

struct Game {
    apple: Pos,
    snake: Snake,
    direction: Direction,
    apples: u32,
}

impl Game {
    fn new(x: u16, y: u16) -> Self {
        Self {
            apple: Pos::rand(x, y),
            snake: Snake::new(x / 2, y / 2),
            direction: Direction::Right,
            apples: 0,
        }
    }

    fn setup(&self, stdout: &mut dyn Write) -> std::io::Result<()> {
        let mut s = String::new();
        let (x, _) = termion::terminal_size()?;
        for _ in 1..x {
            s.push(BORDER);
        }
        write!(
            stdout,
            "{}{}{}{}{}",
            Goto(1, 1),
            "Apples: ",
            self.apples,
            Goto(1, HEADER_SPACE),
            s
        )
    }

    fn play<R: std::io::Read>(
        &mut self,
        mut stdout: &mut dyn Write,
        input: &mut termion::input::Events<R>,
    ) -> std::io::Result<u32> {
        loop {
            self.setup(stdout)?;
            let (x, y) = termion::terminal_size()?;

            let mut next_event = input.next();
            while let Some(n) = input.next() {
                next_event = Some(n);
            }
            let next = self.compute_step(x, y, next_event.map(|r| r.unwrap()))?;
            next.display(&mut stdout, x, y)?;
            stdout.flush()?;
            std::thread::sleep(if self.direction.is_updown() {
                std::time::Duration::from_millis(UP_DOWN_TICK)
            } else {
                std::time::Duration::from_millis(LEFT_RIGHT_TICK)
            });
            if !next.is_continuing() {
                break;
            }
        }
        Ok(self.apples)
    }

    fn compute_step(&mut self, x: u16, y: u16, event: Option<Event>) -> std::io::Result<Step> {
        if let Some(new_dir) = event {
            match new_dir {
                Event::Key(Key::Char('l')) | Event::Key(Key::Right) => {
                    if self.direction != Direction::Left {
                        self.direction = Direction::Right
                    }
                }
                Event::Key(Key::Char('h')) | Event::Key(Key::Left) => {
                    if self.direction != Direction::Right {
                        self.direction = Direction::Left
                    }
                }
                Event::Key(Key::Char('k')) | Event::Key(Key::Up) => {
                    if self.direction != Direction::Down {
                        self.direction = Direction::Up
                    }
                }
                Event::Key(Key::Char('j')) | Event::Key(Key::Down) => {
                    if self.direction != Direction::Up {
                        self.direction = Direction::Down
                    }
                }
                Event::Key(Key::Char('q')) => return Ok(Step::Quite),
                _ => {}
            }
        }
        let prev_snake = self.snake.clone();
        let ate = self.snake.move_in(self.direction, self.apple);
        if ate {
            self.apple = Pos::rand(x, y);
            self.apples += 1;
        }
        if prev_snake.contains(&self.snake.head()) {
            Ok(Step::Done {
                message: format!("Self intercept at {:?}", self.snake.head()),
            })
        } else if self.snake.head().x < HEADER_SPACE || self.snake.head().x > x {
            Ok(Step::Done {
                message: format!(
                    "Broke out the top({}) or bottom at {:?}",
                    x,
                    self.snake.head()
                ),
            })
        } else if self.snake.head().y < HEADER_SPACE || self.snake.head().y > y {
            Ok(Step::Done {
                message: String::from("Broke out the sides"),
            })
        } else {
            let diff_add = self.snake.dif(&prev_snake);
            let diff_neg = prev_snake.dif(&self.snake);
            Ok(Step::Continuing {
                diff_add,
                diff_neg,
                apple: Some(self.apple),
            })
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
struct Pos {
    x: u16,
    y: u16,
}

impl Pos {
    fn rand(x: u16, y: u16) -> Self {
        let mut rng = thread_rng();
        Pos {
            x: rng.gen::<u16>() % x,
            y: HEADER_SPACE + rng.gen::<u16>() % (y - HEADER_SPACE),
        }
    }
}

impl std::fmt::Debug for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", Goto(self.x, self.y))
    }
}

#[derive(Clone)]
struct Snake {
    internal: Vec<Pos>,
}

impl Snake {
    fn new(x: u16, y: u16) -> Self {
        Self {
            internal: vec![Pos { x, y }],
        }
    }

    fn contains(&self, p: &Pos) -> bool {
        self.internal.contains(&p)
    }

    fn head(&self) -> Pos {
        self.internal[0]
    }

    fn move_to(&mut self, p: Pos, del_tail: bool) {
        self.internal.insert(0, p);
        if del_tail {
            self.internal.pop();
        }
    }

    fn move_in(&mut self, d: Direction, apple: Pos) -> bool {
        let mut h = self.head();
        match d {
            Direction::Down => h.y += 1,
            Direction::Up => h.y -= 1,
            Direction::Left => h.x -= 1,
            Direction::Right => h.x += 1,
        }
        let ate = self.head() == apple;
        self.move_to(h, !ate);
        ate
    }
    /// Returns the points in self and not in other.
    fn dif(&self, other: &Snake) -> Vec<Pos> {
        self.internal
            .iter()
            .filter(|p| !other.internal.contains(p))
            .map(|p| p.clone())
            .collect()
    }
}

#[derive(PartialEq)]
enum Step {
    Done {
        message: String,
    },
    Continuing {
        diff_add: Vec<Pos>,
        diff_neg: Vec<Pos>,
        apple: Option<Pos>, // wheither to redraw the apple
    },
    Quite,
}

impl Step {
    fn is_continuing(&self) -> bool {
        match self {
            Step::Continuing {
                diff_add: _,
                diff_neg: _,
                apple: _,
            } => true,
            _ => false,
        }
    }

    fn display(&self, io: &mut dyn Write, x: u16, y: u16) -> std::io::Result<()> {
        match self {
            Step::Done { message } => println!(
                "{}{}",
                Goto((x / 2) - (message.len() / 2) as u16, y / 2),
                message
            ),
            Step::Continuing {
                diff_add,
                diff_neg,
                apple,
            } => {
                diff_add
                    .iter()
                    .for_each(|p| write!(io, "{}{}", p, SNAKE).unwrap());
                diff_neg
                    .iter()
                    .for_each(|p| write!(io, "{}{}", p, EMPTY).unwrap());
                if let Some(apple) = apple {
                    write!(io, "{}{}", apple, APPLE)?;
                }
            }
            Step::Quite => {}
        }
        Ok(())
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn is_updown(&self) -> bool {
        match self {
            Direction::Left | Direction::Right => false,
            Direction::Up | Direction::Down => true,
        }
    }
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn snake_diff() {
        let s1 = Snake::new(10, 10);
        let s2 = Snake::new(11, 27);
        assert_eq!(s1.dif(&s2), vec![Pos { x: 10, y: 10 }]);
        assert_eq!(s2.dif(&s2), vec![]);
        assert_eq!(s2.dif(&s1), vec![Pos { x: 11, y: 27 }]);
    }
}
