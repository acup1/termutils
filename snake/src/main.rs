use std::collections::VecDeque;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use termutils::{KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP, WindowManager, terminal::Terminal};

const BOARD_WIDTH: i32 = 40;
const BOARD_HEIGHT: i32 = 20;
const WINDOW_WIDTH: i32 = BOARD_WIDTH + 2;
const WINDOW_HEIGHT: i32 = BOARD_HEIGHT + 2;
const FRAME_MS: u64 = 90;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn delta(self) -> Point {
        match self {
            Self::Up => Point { x: 0, y: -1 },
            Self::Down => Point { x: 0, y: 1 },
            Self::Left => Point { x: -1, y: 0 },
            Self::Right => Point { x: 1, y: 0 },
        }
    }

    fn is_opposite(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Left, Self::Right)
                | (Self::Right, Self::Left)
        )
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, upper: i32) -> i32 {
        (self.next() % upper as u64) as i32
    }
}

struct Game {
    snake: VecDeque<Point>,
    direction: Direction,
    queued_direction: Direction,
    food: Option<Point>,
    score: usize,
    paused: bool,
    game_over: bool,
    rng: Rng,
}

impl Game {
    fn new(seed: u64) -> Self {
        let center = Point {
            x: BOARD_WIDTH / 2,
            y: BOARD_HEIGHT / 2,
        };
        let mut game = Self {
            snake: VecDeque::from([
                center,
                Point {
                    x: center.x - 1,
                    y: center.y,
                },
                Point {
                    x: center.x - 2,
                    y: center.y,
                },
            ]),
            direction: Direction::Right,
            queued_direction: Direction::Right,
            food: None,
            score: 0,
            paused: false,
            game_over: false,
            rng: Rng::new(seed),
        };
        game.food = game.next_food();
        game
    }

    fn restart(&mut self) {
        let seed = self.rng.next();
        *self = Self::new(seed);
    }

    fn turn(&mut self, direction: Direction) {
        if !direction.is_opposite(self.direction) {
            self.queued_direction = direction;
        }
    }

    fn tick(&mut self) {
        if self.paused || self.game_over {
            return;
        }

        self.direction = self.queued_direction;
        let delta = self.direction.delta();
        let head = self.snake.front().copied().expect("snake is never empty");
        let next = Point {
            x: (head.x + delta.x).rem_euclid(BOARD_WIDTH),
            y: (head.y + delta.y).rem_euclid(BOARD_HEIGHT),
        };
        let ate_food = self.food == Some(next);
        let occupied_length = self.snake.len() - usize::from(!ate_food);

        if self
            .snake
            .iter()
            .take(occupied_length)
            .any(|point| *point == next)
        {
            self.game_over = true;
            return;
        }

        self.snake.push_front(next);
        if ate_food {
            self.score += 1;
            self.food = self.next_food();
            if self.food.is_none() {
                self.game_over = true;
            }
        } else {
            self.snake.pop_back();
        }
    }

    fn next_food(&mut self) -> Option<Point> {
        let area = (BOARD_WIDTH * BOARD_HEIGHT) as usize;
        if self.snake.len() >= area {
            return None;
        }

        for _ in 0..area {
            let candidate = Point {
                x: self.rng.range(BOARD_WIDTH),
                y: self.rng.range(BOARD_HEIGHT),
            };
            if !self.snake.contains(&candidate) {
                return Some(candidate);
            }
        }

        (0..BOARD_HEIGHT)
            .flat_map(|y| (0..BOARD_WIDTH).map(move |x| Point { x, y }))
            .find(|point| !self.snake.contains(point))
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(1)
        ^ u64::from(std::process::id())
}

fn render(terminal: &mut Terminal, windows: &mut WindowManager, game: &Game) -> io::Result<bool> {
    let (terminal_height, terminal_width) = terminal.getsize();
    terminal.clear()?;

    if i32::from(terminal_width) < WINDOW_WIDTH || i32::from(terminal_height) < WINDOW_HEIGHT {
        terminal.posprint(
            0,
            0,
            format!(
                "Terminal is too small: need at least {WINDOW_WIDTH}x{WINDOW_HEIGHT} (press q to quit)"
            ),
        );
        terminal.refresh()?;
        return Ok(false);
    }

    windows.del_all_windows();
    let id = windows.new_window(0);
    let window = windows.get_mut(id).expect("new window exists");
    window.x = (i32::from(terminal_width) - WINDOW_WIDTH) / 2;
    window.y = (i32::from(terminal_height) - WINDOW_HEIGHT) / 2;
    window.width = WINDOW_WIDTH;
    window.height = WINDOW_HEIGHT;
    window.always_on_screen = true;
    window.border = 1;
    window.filling = true;
    window.name = Some(format!(" Snake | score: {} ", game.score));

    if let Some(food) = game.food {
        window.poswchar(food.y, food.x, '◆');
    }
    for (index, point) in game.snake.iter().enumerate().rev() {
        window.poswchar(point.y, point.x, if index == 0 { '●' } else { 'o' });
    }

    if game.paused {
        window.posprint(BOARD_HEIGHT / 2, BOARD_WIDTH / 2 - 4, " PAUSED ");
    }
    if game.game_over {
        window.posprint(
            BOARD_HEIGHT / 2,
            BOARD_WIDTH / 2 - 11,
            " GAME OVER — r: retry ",
        );
    }

    windows.render_windows(terminal)?;
    Ok(true)
}

fn main() -> io::Result<()> {
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let mut game = Game::new(seed());

    loop {
        let board_visible = render(&mut terminal, &mut windows, &game)?;
        let key = terminal.get_key(FRAME_MS, true)?;

        match key {
            113 | 81 => break,
            114 | 82 if game.game_over => game.restart(),
            32 if !game.game_over => game.paused = !game.paused,
            KEY_UP | 119 | 87 => game.turn(Direction::Up),
            KEY_DOWN | 115 | 83 => game.turn(Direction::Down),
            KEY_LEFT | 97 | 65 => game.turn(Direction::Left),
            KEY_RIGHT | 100 | 68 => game.turn(Direction::Right),
            _ => {}
        }

        if board_visible {
            game.tick();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_immediate_reverse() {
        let mut game = Game::new(1);
        game.turn(Direction::Left);
        assert_eq!(game.queued_direction, Direction::Right);
    }

    #[test]
    fn eating_grows_the_snake_and_increases_score() {
        let mut game = Game::new(1);
        let head = game.snake.front().copied().unwrap();
        game.food = Some(Point {
            x: head.x + 1,
            y: head.y,
        });
        let old_length = game.snake.len();
        game.tick();
        assert_eq!(game.snake.len(), old_length + 1);
        assert_eq!(game.score, 1);
    }

    #[test]
    fn movement_wraps_around_the_board() {
        let mut game = Game::new(1);
        game.snake = VecDeque::from([Point { x: 0, y: 3 }]);
        game.direction = Direction::Left;
        game.queued_direction = Direction::Left;
        game.food = Some(Point { x: 10, y: 10 });
        game.tick();
        assert_eq!(game.snake.front(), Some(&Point { x: 39, y: 3 }));
    }
}
