use std::collections::VecDeque;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use termutils::{KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP, WindowManager, terminal::Terminal};

const SEGMENT_WIDTH: i32 = 5;
const SEGMENT_HEIGHT: i32 = 3;
const MIN_COLUMNS: i32 = 8;
const MIN_ROWS: i32 = 5;
const FRAME_MS: u64 = 90;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Grid {
    columns: i32,
    rows: i32,
    origin_x: i32,
    origin_y: i32,
}

impl Grid {
    fn from_terminal(height: u16, width: u16) -> Option<Self> {
        let columns = i32::from(width) / SEGMENT_WIDTH;
        let rows = i32::from(height) / SEGMENT_HEIGHT;
        if columns < MIN_COLUMNS || rows < MIN_ROWS {
            return None;
        }

        Some(Self {
            columns,
            rows,
            origin_x: (i32::from(width) - columns * SEGMENT_WIDTH) / 2,
            origin_y: (i32::from(height) - rows * SEGMENT_HEIGHT) / 2,
        })
    }

    fn terminal_position(self, point: Point) -> Point {
        Point {
            x: self.origin_x + point.x * SEGMENT_WIDTH,
            y: self.origin_y + point.y * SEGMENT_HEIGHT,
        }
    }
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
    grid: Grid,
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
    fn new(seed: u64, grid: Grid) -> Self {
        let center = Point {
            x: grid.columns / 2,
            y: grid.rows / 2,
        };
        let mut game = Self {
            grid,
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
        *self = Self::new(seed, self.grid);
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
            x: (head.x + delta.x).rem_euclid(self.grid.columns),
            y: (head.y + delta.y).rem_euclid(self.grid.rows),
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
        let area = (self.grid.columns * self.grid.rows) as usize;
        if self.snake.len() >= area {
            return None;
        }

        for _ in 0..area {
            let candidate = Point {
                x: self.rng.range(self.grid.columns),
                y: self.rng.range(self.grid.rows),
            };
            if !self.snake.contains(&candidate) {
                return Some(candidate);
            }
        }

        (0..self.grid.rows)
            .flat_map(|y| (0..self.grid.columns).map(move |x| Point { x, y }))
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

fn add_square_window(
    windows: &mut WindowManager,
    grid: Grid,
    point: Point,
    layer: i32,
    border: i32,
) -> termutils::WindowId {
    let position = grid.terminal_position(point);
    let id = windows.new_window(layer);
    let window = windows.get_mut(id).expect("new window exists");
    window.x = position.x;
    window.y = position.y;
    window.width = SEGMENT_WIDTH;
    window.height = SEGMENT_HEIGHT;
    window.always_on_screen = false;
    window.border = border;
    window.filling = true;
    id
}

fn add_overlay(windows: &mut WindowManager, terminal_height: u16, terminal_width: u16, text: &str) {
    let width = text.chars().count() as i32 + 4;
    let id = windows.new_window(100);
    let window = windows.get_mut(id).expect("new window exists");
    window.x = (i32::from(terminal_width) - width) / 2;
    window.y = (i32::from(terminal_height) - 3) / 2;
    window.width = width;
    window.height = 3;
    window.border = 1;
    window.filling = true;
    window.posprint(0, 1, text);
}

fn add_status(windows: &mut WindowManager, score: usize) {
    let text = format!(" score: {score} | q: quit ");
    let id = windows.new_window(200);
    let window = windows.get_mut(id).expect("new window exists");
    window.x = -1;
    window.y = -1;
    window.width = text.chars().count() as i32 + 2;
    window.height = 3;
    window.always_on_screen = false;
    window.border = -1;
    window.filling = false;
    window.posprint(0, 0, text);
}

fn render(
    terminal: &mut Terminal,
    windows: &mut WindowManager,
    terminal_height: u16,
    terminal_width: u16,
    game: &Game,
) -> io::Result<()> {
    windows.del_all_windows();

    let background_id = windows.new_window(0);
    let background = windows.get_mut(background_id).expect("new window exists");
    background.toggle_fullscreen(terminal_height, terminal_width);
    background.border = -1;
    background.filling = true;

    if let Some(food) = game.food {
        let food_id = add_square_window(windows, game.grid, food, 1, 0);
        windows
            .get_mut(food_id)
            .expect("food window exists")
            .poswchar(0, 1, '◆');
    }

    for (index, point) in game.snake.iter().enumerate().rev() {
        let segment_id = add_square_window(windows, game.grid, *point, 2, 1);
        if index == 0 {
            windows
                .get_mut(segment_id)
                .expect("head window exists")
                .poswchar(0, 1, '●');
        }
    }

    if game.paused {
        add_overlay(windows, terminal_height, terminal_width, "PAUSED");
    } else if game.game_over {
        add_overlay(
            windows,
            terminal_height,
            terminal_width,
            "GAME OVER — r: retry",
        );
    }
    add_status(windows, game.score);

    windows.render_windows(terminal)
}

fn render_too_small(terminal: &mut Terminal) -> io::Result<()> {
    terminal.clear()?;
    terminal.posprint(
        0,
        0,
        format!(
            "Terminal is too small: need at least {}x{} (press q to quit)",
            MIN_COLUMNS * SEGMENT_WIDTH,
            MIN_ROWS * SEGMENT_HEIGHT,
        ),
    );
    terminal.refresh()
}

fn main() -> io::Result<()> {
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let mut game: Option<Game> = None;

    loop {
        let (terminal_height, terminal_width) = terminal.getsize();
        let grid = Grid::from_terminal(terminal_height, terminal_width);

        if let Some(grid) = grid {
            if game.as_ref().is_none_or(|game| game.grid != grid) {
                game = Some(Game::new(seed(), grid));
            }
            render(
                &mut terminal,
                &mut windows,
                terminal_height,
                terminal_width,
                game.as_ref().expect("game was initialized"),
            )?;
        } else {
            game = None;
            render_too_small(&mut terminal)?;
        }

        let key = terminal.get_key(FRAME_MS, true)?;
        if matches!(key, 113 | 81) {
            break;
        }

        let Some(game) = game.as_mut() else {
            continue;
        };
        match key {
            114 | 82 if game.game_over => game.restart(),
            32 if !game.game_over => game.paused = !game.paused,
            KEY_UP | 119 | 87 => game.turn(Direction::Up),
            KEY_DOWN | 115 | 83 => game.turn(Direction::Down),
            KEY_LEFT | 97 | 65 => game.turn(Direction::Left),
            KEY_RIGHT | 100 | 68 => game.turn(Direction::Right),
            _ => {}
        }
        game.tick();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::from_terminal(24, 80).unwrap()
    }

    #[test]
    fn grid_uses_the_full_terminal_in_square_windows() {
        let grid = grid();
        assert_eq!((grid.columns, grid.rows), (16, 8));
        assert_eq!(
            grid.terminal_position(Point { x: 15, y: 7 }),
            Point { x: 75, y: 21 }
        );
    }

    #[test]
    fn rejects_an_immediate_reverse() {
        let mut game = Game::new(1, grid());
        game.turn(Direction::Left);
        assert_eq!(game.queued_direction, Direction::Right);
    }

    #[test]
    fn eating_grows_the_snake_and_increases_score() {
        let mut game = Game::new(1, grid());
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
    fn movement_wraps_around_the_fullscreen_grid() {
        let mut game = Game::new(1, grid());
        game.snake = VecDeque::from([Point { x: 0, y: 3 }]);
        game.direction = Direction::Left;
        game.queued_direction = Direction::Left;
        game.food = Some(Point { x: 10, y: 5 });
        game.tick();
        assert_eq!(game.snake.front(), Some(&Point { x: 15, y: 3 }));
    }
}
