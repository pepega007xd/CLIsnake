use core::fmt;
use crossterm::cursor;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use rand::prelude::SliceRandom;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::io::stdout;
use std::io::Write;
use std::process::exit;
use std::time::Duration;

fn wait_for_unpause() {
    loop {
        match event::read().unwrap() {
            Event::Key(KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => {
                break;
            }
            _ => (),
        }
    }
}

#[derive(Clone, Copy)]
pub enum State {
    Playing,
    Lost,
}
pub struct Game {
    snake: VecDeque<Position>,
    direction: Direction,
    field: Field,
    cycle_time: f64,
}

#[derive(Clone, Copy)]
struct Position {
    x: isize,
    y: isize,
}

impl Position {
    fn step(&self, direction: &Direction) -> Position {
        let mut new = self.clone();
        match direction {
            Direction::Up => new.y -= 1,
            Direction::Right => new.x += 1,
            Direction::Down => new.y += 1,
            Direction::Left => new.x -= 1,
        }
        new
    }
}

#[derive(Clone, Copy)]
enum Block {
    Empty,
    Snake,
    SnakeHead(Direction),
    Wall,
    Food,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let head = match self {
            Direction::Up => "▀▀",
            Direction::Right => " █",
            Direction::Down => "▄▄",
            Direction::Left => "█ ",
        };

        write!(
            f,
            "{}{}{}",
            crossterm::style::SetForegroundColor(crossterm::style::Color::Yellow),
            head,
            crossterm::style::SetForegroundColor(crossterm::style::Color::White)
        )
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Block::Empty => write!(f, "  "),
            Block::Food => write!(f, "▒▒"),
            Block::Snake => write!(f, "██"),
            Block::SnakeHead(d) => d.fmt(f),
            Block::Wall => write!(f, "██"),
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Direction {
    Up,
    #[default]
    Right,
    Down,
    Left,
}

impl Direction {
    fn set(&mut self, direction: &Direction) {
        match (&self, direction) {
            (Direction::Down, Direction::Up) => (),
            (Direction::Up, Direction::Down) => (),
            (Direction::Left, Direction::Right) => (),
            (Direction::Right, Direction::Left) => (),
            _ => *self = direction.clone(),
        }
    }
}

struct Field {
    width: usize,
    height: usize,
    field: Vec<Vec<Block>>,
}

impl Field {
    fn new(width: usize, height: usize) -> Field {
        Field {
            width,
            height,
            field: vec![vec![Block::Empty; width]; height],
        }
    }

    fn set_position(&mut self, position: Position, block: Block) {
        let x = position.x as usize;
        let y = position.y as usize;
        self.field[y][x] = block;
    }

    fn get_position(&self, position: Position) -> Block {
        if position.x < 0 || position.x >= self.width as isize {
            return Block::Wall;
        }

        if position.y < 0 || position.y >= self.height as isize {
            return Block::Wall;
        }

        let x = position.x as usize;
        let y = position.y as usize;

        self.field.get(y).unwrap().get(x).unwrap().to_owned()
    }

    fn place_food(&mut self) {
        let mut allowed: Vec<Position> = vec![];
        for y in 0..self.height as isize {
            for x in 0..self.width as isize {
                let position = Position { x, y };
                if let Block::Empty = self.get_position(position) {
                    allowed.push(position)
                }
            }
        }

        let chosen = allowed.choose(&mut rand::thread_rng()).unwrap();
        self.set_position(*chosen, Block::Food);
    }

    fn draw(&self, length: usize, paused: bool) {
        execute!(
            stdout(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();
        println!("┏{}┓\r", "━━".repeat(self.width));

        self.field.iter().for_each(|row| {
            print!("┃");
            row.iter().for_each(|block| print!("{}", block));
            println!("┃\r");
        });

        let score_str = format!("score: {}", length - 2);
        print!("┗━━");
        print!(" {score_str} ");
        if paused {
            print!(
                "━━ [PAUSED] {}┛",
                "━".repeat(self.width * 2 - score_str.len() - 16)
            );
        } else {
            print!("{}┛", "━".repeat(self.width * 2 - score_str.len() - 4));
        }
        stdout().flush().unwrap();
    }
}

impl Game {
    pub fn new() -> Game {
        enable_raw_mode().unwrap();
        execute!(stdout(), cursor::Hide).unwrap();
        let (term_width, term_height) = crossterm::terminal::size().unwrap();

        let width = term_width / 2 - 2;
        let height = term_height - 2;

        let initial_position = Position {
            x: width as isize / 2,
            y: height as isize / 2,
        };
        let tail_position = Position {
            x: initial_position.x - 1,
            ..initial_position
        };

        let mut field = Field::new(width as usize, height as usize);
        field.set_position(initial_position, Block::SnakeHead(Direction::default()));
        field.set_position(tail_position, Block::Snake);
        field.place_food();

        Game {
            snake: VecDeque::from([initial_position, tail_position]),
            direction: Direction::default(),
            field,
            cycle_time: 300. * 1000. * 1000., // 300 ms
        }
    }

    pub fn play(&mut self) {
        loop {
            self.field.draw(self.snake.len(), false);

            if let Some(direction) = self.poll_key() {
                self.direction.set(&direction);
            }

            if let State::Lost = self.update() {
                disable_raw_mode().unwrap();
                execute!(
                    stdout(),
                    terminal::Clear(terminal::ClearType::All),
                    cursor::MoveTo(0, 0),
                    cursor::Show
                )
                .unwrap();

                println!("Game over!\nScore: {}", self.snake.len() - 3);
                break;
            } else {
                self.cycle_time *= 0.9997;
            }
        }
    }

    fn update(&mut self) -> State {
        let old_head = *self.snake.front().unwrap();
        let new_head = old_head.step(&self.direction);
        self.snake.push_front(new_head);

        let tail = *self.snake.back().unwrap();

        match self.field.get_position(new_head) {
            Block::Empty => {
                self.field
                    .set_position(new_head, Block::SnakeHead(self.direction));
                self.field.set_position(old_head, Block::Snake);
                self.field.set_position(tail, Block::Empty);
                self.snake.pop_back();
                State::Playing
            }

            Block::Food => {
                self.field
                    .set_position(new_head, Block::SnakeHead(self.direction));
                self.field.set_position(old_head, Block::Snake);
                self.field.place_food();
                State::Playing
            }

            _ => State::Lost,
        }
    }

    fn poll_key(&self) -> Option<Direction> {
        if event::poll(Duration::from_nanos(self.cycle_time as u64)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Direction::Up),

                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Direction::Right),

                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('s'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Direction::Down),

                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Direction::Left),

                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => exit(1),

                Event::Key(KeyEvent {
                    code: KeyCode::Char('p'),
                    modifiers: KeyModifiers::NONE,
                }) => {
                    self.field.draw(self.snake.len(), true);
                    wait_for_unpause();
                    None
                }

                _ => None,
            }
        } else {
            None
        }
    }
}
