use core::fmt;
use std::fmt::Formatter;
use std::io::Write;
use std::io::stdout;
use std::time::Duration;
use std::collections::VecDeque;
use crossterm::execute;
use crossterm::terminal;
use rand::prelude::SliceRandom;
use crossterm::cursor;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

pub enum State {
    Playing,
    Lost,
}
pub struct Game {
    snake: VecDeque<Position>,
    direction: Direction,
    field: Field,
}

#[derive(Clone)]
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
    Wall,
    Food,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let block = match self {
            Block::Empty => "  ",
            Block::Food => "▒▒",
            Block::Snake => "██",
            Block::Wall => "██",
        };

        write!(f, "{}", block)
    }
}

#[derive(Clone)]
enum Direction {
    Up,
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
    content: Vec<Vec<Block>>,
}

impl Field {
    fn new(width: usize, height: usize) -> Field {
        Field {
            width,
            height,
            content: vec![vec![Block::Empty; width]; height],
        }
    }

    fn set_position(&mut self, position: &Position, block: Block) {
        let x = position.x as usize;
        let y = position.y as usize;
        self.content[y][x] = block;
    }

    fn get_position(&self, position: &Position) -> Block {
        if position.x < 0 || position.x >= self.width as isize {
            return Block::Wall;
        }

        if position.y < 0 || position.y >= self.height as isize {
            return Block::Wall;
        }

        let x = position.x as usize;
        let y = position.y as usize;

        self.content
            .get(y).unwrap()
            .get(x).unwrap()
            .to_owned()
    }

    fn place_food(&mut self) {
        let mut allowed: Vec<Position> = vec![];
        for y in 0..self.height as isize {
            for x in 0..self.width as isize {
                let position = Position { x, y };
                if let Block::Empty = self.get_position(&position) {
                    allowed.push(position)
                }
            }
        }
        
        let chosen = allowed.choose(&mut rand::thread_rng()).unwrap();
        self.set_position(chosen, Block::Food);
    }
    
    fn draw(&self) {
        execute!(stdout(), 
            terminal::Clear(terminal::ClearType::All), 
            cursor::MoveTo(0,0)).unwrap();
        println!("┏{}┓\r", "━━".repeat(self.width));

        self.content.iter().for_each(|row| {
            print!("┃");
            row.iter().for_each(|block| print!("{}", block));
            println!("┃\r");           
        });

        print!("┗{}┛", "━━".repeat(self.width));
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
            y: height as isize / 2 };
        let mut field = Field::new(width as usize, height as usize);
        field.set_position(&initial_position, Block::Snake);
        field.place_food();

        Game {
            snake: VecDeque::from([initial_position]),
            direction: Direction::Right,
            field,
        }
    }

    pub fn play(&mut self) {
        loop {
            self.field.draw();
            
            if let Some(direction) = self.poll_key() {
                self.direction.set(&direction);
            }

            if let State::Lost = self.update() {
                disable_raw_mode().unwrap();
                execute!(stdout(), 
                    terminal::Clear(terminal::ClearType::All),
                    cursor::MoveTo(0,0),
                    cursor::Show).unwrap();

                println!("Game over!");
                break;
            }

        }
    }

    fn update(&mut self) -> State {
        let head = self.snake.front().unwrap().step(&self.direction);
        self.snake.push_front(head);

        let head = self.snake.front().unwrap();
        let tail = self.snake.back().unwrap();

        match self.field.get_position(head) {
            Block::Empty => {
                self.field.set_position(head, Block::Snake);
                self.field.set_position(tail, Block::Empty);
                self.snake.pop_back();
                State::Playing
            },

            Block::Food => {
                self.field.set_position(head, Block::Snake);
                self.field.place_food();
                State::Playing
            },

            Block::Snake => State::Lost,
            
            Block::Wall => State::Lost,
        }
    }

    fn poll_key(&self) -> Option<Direction> {
        if event::poll(Duration::from_millis(300)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE
                }) => Some(Direction::Up),

                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    modifiers: KeyModifiers::NONE
                }) => Some(Direction::Right),

                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE
                }) => Some(Direction::Down),

                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    modifiers: KeyModifiers::NONE
                }) => Some(Direction::Left),

                _ => None,
            }
        } else {
            None
        }
    }
        
}
