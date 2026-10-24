use std::collections::VecDeque;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use termutils::{
    terminal::Terminal, CellStyle, Mouse, MouseEvent, Rgb, WindowManager, KEY_DOWN, KEY_LEFT,
    KEY_RIGHT, KEY_UP, MOUSE_EVENT,
};

const SEGMENT_WIDTH: i32 = 5;
const SEGMENT_HEIGHT: i32 = 3;
const MIN_COLUMNS: i32 = 8;
const MIN_ROWS: i32 = 5;
const MENU_FRAME_MS: u64 = 50;
const MENU_WIDTH: i32 = 38;
const MENU_HEIGHT: i32 = 11;
const APPLE_GLYPH: char = '\u{e29e}';
const SNAKE_GREEN: Rgb = Rgb::new(34, 197, 94);
const APPLE_RED: Rgb = Rgb::new(239, 68, 68);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];

    fn number(self) -> usize {
        match self {
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Easy => "EASY",
            Self::Normal => "NORMAL",
            Self::Hard => "HARD",
        }
    }

    fn frame_ms(self) -> u64 {
        match self {
            Self::Easy => 210,
            Self::Normal => 140,
            Self::Hard => 90,
        }
    }

    fn from_number(key: i32) -> Option<Self> {
        match key {
            49 => Some(Self::Easy),
            50 => Some(Self::Normal),
            51 => Some(Self::Hard),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HitBox {
    x: i32,
    y: i32,
    width: i32,
}

impl HitBox {
    fn contains(self, mouse: Mouse) -> bool {
        i32::from(mouse.y) == self.y
            && i32::from(mouse.x) >= self.x
            && i32::from(mouse.x) < self.x + self.width
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MenuLayout {
    options: [(Difficulty, HitBox); 3],
}

impl MenuLayout {
    fn clicked(self, mouse: Mouse) -> Option<Difficulty> {
        if mouse.event != MouseEvent::LEFT {
            return None;
        }
        self.options
            .into_iter()
            .find_map(|(difficulty, hitbox)| hitbox.contains(mouse).then_some(difficulty))
    }
}

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

fn add_status(windows: &mut WindowManager, score: usize, difficulty: Difficulty) {
    let text = format!(
        " score: {score} | {} | q: quit ",
        difficulty.name().to_lowercase()
    );
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
    difficulty: Difficulty,
) -> io::Result<()> {
    windows.del_all_windows();

    let background_id = windows.new_window(0);
    let background = windows.get_mut(background_id).expect("new window exists");
    background.toggle_fullscreen(terminal_height, terminal_width);
    background.border = -1;
    background.filling = true;

    if let Some(food) = game.food {
        let food_id = add_square_window(windows, game.grid, food, 1, -1);
        windows
            .get_mut(food_id)
            .expect("food window exists")
            .posstyled(0, 1, APPLE_GLYPH, CellStyle::foreground(APPLE_RED));
    }

    for (index, point) in game.snake.iter().enumerate().rev() {
        let segment_id = add_square_window(windows, game.grid, *point, 2, i32::from(index != 0));
        let segment = windows.get_mut(segment_id).expect("segment window exists");
        segment.border_style = CellStyle::foreground(SNAKE_GREEN);
        if index == 0 {
            segment.posprint_styled(0, 0, "^~^", CellStyle::foreground(SNAKE_GREEN));
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
    add_status(windows, game.score, difficulty);

    windows.render_windows(terminal)
}

fn add_background(windows: &mut WindowManager, terminal_height: u16, terminal_width: u16) {
    let background_id = windows.new_window(0);
    let background = windows.get_mut(background_id).expect("new window exists");
    background.toggle_fullscreen(terminal_height, terminal_width);
    background.border = -1;
    background.filling = true;
}

fn render_too_small(
    terminal: &mut Terminal,
    windows: &mut WindowManager,
    terminal_height: u16,
    terminal_width: u16,
) -> io::Result<()> {
    windows.del_all_windows();
    add_background(windows, terminal_height, terminal_width);
    let text = format!(
        "Terminal is too small: need at least {}x{} (press q to quit)",
        MIN_COLUMNS * SEGMENT_WIDTH,
        MIN_ROWS * SEGMENT_HEIGHT,
    );
    let id = windows.new_window(1);
    let window = windows.get_mut(id).expect("new window exists");
    window.x = -1;
    window.y = -1;
    window.width = text.chars().count() as i32 + 2;
    window.height = 3;
    window.always_on_screen = false;
    window.border = -1;
    window.filling = false;
    window.posprint(0, 0, text);
    windows.render_windows(terminal)
}

fn render_difficulty_menu(
    terminal: &mut Terminal,
    windows: &mut WindowManager,
    terminal_height: u16,
    terminal_width: u16,
    selected: usize,
) -> io::Result<MenuLayout> {
    windows.del_all_windows();
    add_background(windows, terminal_height, terminal_width);

    let x = (i32::from(terminal_width) - MENU_WIDTH) / 2;
    let y = (i32::from(terminal_height) - MENU_HEIGHT) / 2;
    let menu_id = windows.new_window(10);
    let menu = windows.get_mut(menu_id).expect("new window exists");
    menu.x = x;
    menu.y = y;
    menu.width = MENU_WIDTH;
    menu.height = MENU_HEIGHT;
    menu.always_on_screen = false;
    menu.border = 0;
    menu.filling = true;
    menu.name = Some(" SNAKE — SELECT DIFFICULTY ".into());
    menu.posprint(0, 7, "speed / move");

    let mut options = [(
        Difficulty::Easy,
        HitBox {
            x: 0,
            y: 0,
            width: 0,
        },
    ); 3];
    for (index, difficulty) in Difficulty::ALL.into_iter().enumerate() {
        let content_y = 2 + index as i32 * 2;
        let marker = if index == selected { '>' } else { ' ' };
        menu.posprint(
            content_y,
            3,
            format!(
                "{marker} {}. {:<8} {:>3} ms",
                difficulty.number(),
                difficulty.name(),
                difficulty.frame_ms()
            ),
        );
        options[index] = (
            difficulty,
            HitBox {
                x: x + 1,
                y: y + content_y + 1,
                width: MENU_WIDTH - 2,
            },
        );
    }
    menu.posprint(8, 4, "↑/↓ + Enter, 1–3 or click");
    windows.render_windows(terminal)?;
    Ok(MenuLayout { options })
}

fn choose_difficulty(
    terminal: &mut Terminal,
    windows: &mut WindowManager,
) -> io::Result<Option<Difficulty>> {
    let mut selected = 1;
    loop {
        let (terminal_height, terminal_width) = terminal.getsize();
        let layout = if Grid::from_terminal(terminal_height, terminal_width).is_some() {
            Some(render_difficulty_menu(
                terminal,
                windows,
                terminal_height,
                terminal_width,
                selected,
            )?)
        } else {
            render_too_small(terminal, windows, terminal_height, terminal_width)?;
            None
        };

        let key = terminal.get_key(MENU_FRAME_MS, true)?;
        if matches!(key, 113 | 81) {
            return Ok(None);
        }
        if let Some(difficulty) = Difficulty::from_number(key) {
            return Ok(Some(difficulty));
        }
        match key {
            KEY_UP => selected = (selected + Difficulty::ALL.len() - 1) % Difficulty::ALL.len(),
            KEY_DOWN => selected = (selected + 1) % Difficulty::ALL.len(),
            10 | 13 => return Ok(Some(Difficulty::ALL[selected])),
            MOUSE_EVENT => {
                if let Some(difficulty) =
                    layout.and_then(|layout| layout.clicked(terminal.getmouse()))
                {
                    return Ok(Some(difficulty));
                }
            }
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let Some(difficulty) = choose_difficulty(&mut terminal, &mut windows)? else {
        return Ok(());
    };
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
                difficulty,
            )?;
        } else {
            game = None;
            render_too_small(&mut terminal, &mut windows, terminal_height, terminal_width)?;
        }

        let key = terminal.get_key(difficulty.frame_ms(), true)?;
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

    #[test]
    fn hard_keeps_the_original_speed_and_levels_get_faster() {
        assert_eq!(Difficulty::Hard.frame_ms(), 90);
        assert!(Difficulty::Easy.frame_ms() > Difficulty::Normal.frame_ms());
        assert!(Difficulty::Normal.frame_ms() > Difficulty::Hard.frame_ms());
    }

    #[test]
    fn number_keys_select_difficulty() {
        assert_eq!(Difficulty::from_number('1' as i32), Some(Difficulty::Easy));
        assert_eq!(
            Difficulty::from_number('2' as i32),
            Some(Difficulty::Normal)
        );
        assert_eq!(Difficulty::from_number('3' as i32), Some(Difficulty::Hard));
        assert_eq!(Difficulty::from_number('4' as i32), None);
    }

    #[test]
    fn mouse_click_selects_the_matching_menu_row() {
        let layout = MenuLayout {
            options: [
                (
                    Difficulty::Easy,
                    HitBox {
                        x: 10,
                        y: 4,
                        width: 20,
                    },
                ),
                (
                    Difficulty::Normal,
                    HitBox {
                        x: 10,
                        y: 6,
                        width: 20,
                    },
                ),
                (
                    Difficulty::Hard,
                    HitBox {
                        x: 10,
                        y: 8,
                        width: 20,
                    },
                ),
            ],
        };
        assert_eq!(
            layout.clicked(Mouse {
                x: 15,
                y: 6,
                event: MouseEvent::LEFT,
            }),
            Some(Difficulty::Normal)
        );
        assert_eq!(
            layout.clicked(Mouse {
                x: 15,
                y: 6,
                event: MouseEvent::IDLE,
            }),
            None
        );
    }
}
