use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use termutils::{
    terminal::Terminal, CellStyle, Mouse, MouseEvent, Rgb, WindowManager, KEY_DOWN, KEY_LEFT,
    KEY_RIGHT, KEY_UP, MOUSE_EVENT,
};

const CELL_WIDTH: i32 = 2;
const CELL_HEIGHT: i32 = 2;
const HEADER_HEIGHT: i32 = 3;
const MIN_WIDTH: u16 = 30;
const MIN_HEIGHT: u16 = 11;
const INPUT_FRAME_MS: u64 = 33;
const GENERATION_MS: u64 = 180;
const DEFAULT_PATH: &str = "life.world";
const LIVE_COLOR: Rgb = Rgb::new(34, 197, 94);
const CURSOR_COLOR: Rgb = Rgb::new(250, 204, 21);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
struct World {
    cells: BTreeSet<Point>,
    generation: u64,
}

impl World {
    fn is_alive(&self, point: Point) -> bool {
        self.cells.contains(&point)
    }

    fn set_alive(&mut self, point: Point, alive: bool) {
        if alive {
            self.cells.insert(point);
        } else {
            self.cells.remove(&point);
        }
    }

    fn toggle(&mut self, point: Point) {
        let alive = !self.is_alive(point);
        self.set_alive(point, alive);
    }

    fn clear(&mut self) {
        self.cells.clear();
        self.generation = 0;
    }

    fn step(&mut self) {
        let mut neighbours = BTreeMap::<Point, u8>::new();
        for cell in &self.cells {
            neighbours.entry(*cell).or_default();
            for delta_y in -1..=1 {
                for delta_x in -1..=1 {
                    if delta_x == 0 && delta_y == 0 {
                        continue;
                    }
                    let neighbour = Point::new(cell.x + delta_x, cell.y + delta_y);
                    *neighbours.entry(neighbour).or_default() += 1;
                }
            }
        }

        self.cells = neighbours
            .into_iter()
            .filter_map(|(point, count)| {
                (count == 3 || (count == 2 && self.is_alive(point))).then_some(point)
            })
            .collect();
        self.generation = self.generation.saturating_add(1);
    }

    fn serialize(&self) -> String {
        let mut output = format!("TERMUTILS_LIFE 1\n{}\n", self.generation);
        for point in &self.cells {
            output.push_str(&format!("{} {}\n", point.x, point.y));
        }
        output
    }

    fn deserialize(input: &str) -> Result<Self, String> {
        let mut lines = input.lines();
        if lines.next() != Some("TERMUTILS_LIFE 1") {
            return Err("not a termutils life v1 file".into());
        }
        let generation = lines
            .next()
            .ok_or("missing generation")?
            .parse::<u64>()
            .map_err(|_| "invalid generation")?;
        let mut cells = BTreeSet::new();
        for (index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let mut fields = line.split_whitespace();
            let x = fields
                .next()
                .and_then(|value| value.parse::<i32>().ok())
                .ok_or_else(|| format!("invalid x on line {}", index + 3))?;
            let y = fields
                .next()
                .and_then(|value| value.parse::<i32>().ok())
                .ok_or_else(|| format!("invalid y on line {}", index + 3))?;
            if fields.next().is_some() {
                return Err(format!("extra data on line {}", index + 3));
            }
            cells.insert(Point::new(x, y));
        }
        Ok(Self { cells, generation })
    }

    fn save(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.serialize())
    }

    fn load(path: &Path) -> io::Result<Self> {
        let input = fs::read_to_string(path)?;
        Self::deserialize(&input).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Viewport {
    columns: i32,
    rows: i32,
}

impl Viewport {
    fn from_terminal(height: u16, width: u16) -> Option<Self> {
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            return None;
        }
        Some(Self {
            columns: i32::from(width) / CELL_WIDTH,
            rows: (i32::from(height) - HEADER_HEIGHT) / CELL_HEIGHT,
        })
    }

    fn contains_mouse(self, mouse: Mouse) -> bool {
        let x = i32::from(mouse.x);
        let y = i32::from(mouse.y);
        x >= 0
            && x < self.columns * CELL_WIDTH
            && y >= HEADER_HEIGHT
            && y < HEADER_HEIGHT + self.rows * CELL_HEIGHT
    }

    fn mouse_point(self, camera: Point, mouse: Mouse) -> Option<Point> {
        self.contains_mouse(mouse).then(|| {
            Point::new(
                camera.x + i32::from(mouse.x) / CELL_WIDTH,
                camera.y + (i32::from(mouse.y) - HEADER_HEIGHT) / CELL_HEIGHT,
            )
        })
    }

    fn screen_point(self, camera: Point, world: Point) -> Option<Point> {
        let column = world.x - camera.x;
        let row = world.y - camera.y;
        (column >= 0 && column < self.columns && row >= 0 && row < self.rows)
            .then(|| Point::new(column * CELL_WIDTH, HEADER_HEIGHT + row * CELL_HEIGHT))
    }

    fn center(self, camera: Point) -> Point {
        Point::new(camera.x + self.columns / 2, camera.y + self.rows / 2)
    }
}

struct App {
    world: World,
    camera: Point,
    paused: bool,
    path: PathBuf,
    status: String,
    last_generation: Instant,
}

impl App {
    fn new(path: PathBuf) -> Self {
        Self {
            world: World::default(),
            camera: Point::new(0, 0),
            paused: true,
            path,
            status: "Paused — add cells, then press Space".into(),
            last_generation: Instant::now(),
        }
    }

    fn move_camera(&mut self, delta_x: i32, delta_y: i32) {
        self.camera.x = self.camera.x.saturating_add(delta_x);
        self.camera.y = self.camera.y.saturating_add(delta_y);
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.last_generation = Instant::now();
        self.status = if self.paused {
            "Simulation paused".into()
        } else {
            "Simulation running".into()
        };
    }

    fn save(&mut self) {
        self.status = match self.world.save(&self.path) {
            Ok(()) => format!("Saved {}", self.path.display()),
            Err(error) => format!("Save failed: {error}"),
        };
    }

    fn load(&mut self) {
        match World::load(&self.path) {
            Ok(world) => {
                self.world = world;
                self.paused = true;
                self.status = format!("Loaded {}", self.path.display());
            }
            Err(error) => self.status = format!("Load failed: {error}"),
        }
        self.last_generation = Instant::now();
    }

    fn advance_if_due(&mut self) {
        if !self.paused && self.last_generation.elapsed() >= Duration::from_millis(GENERATION_MS) {
            self.world.step();
            self.last_generation = Instant::now();
        }
    }
}

fn add_cell_window(
    windows: &mut WindowManager,
    screen: Point,
    layer: i32,
    color: Rgb,
) -> termutils::WindowId {
    let id = windows.new_window(layer);
    let window = windows.get_mut(id).expect("new window exists");
    window.x = screen.x;
    window.y = screen.y;
    window.width = CELL_WIDTH;
    window.height = CELL_HEIGHT;
    window.always_on_screen = false;
    window.border = 0;
    window.border_style = CellStyle::foreground(color);
    window.filling = false;
    id
}

fn render(
    terminal: &mut Terminal,
    windows: &mut WindowManager,
    app: &App,
    height: u16,
    width: u16,
) -> io::Result<Option<Viewport>> {
    windows.del_all_windows();

    let background_id = windows.new_window(0);
    let background = windows.get_mut(background_id).expect("background exists");
    background.toggle_fullscreen(height, width);
    background.border = -1;
    background.filling = true;

    let Some(viewport) = Viewport::from_terminal(height, width) else {
        let warning_id = windows.new_window(10);
        let warning = windows.get_mut(warning_id).expect("warning exists");
        warning.x = 0;
        warning.y = 0;
        warning.width = i32::from(width);
        warning.height = 3;
        warning.border = -1;
        warning.posprint(
            0,
            0,
            format!("Life needs at least {MIN_WIDTH}x{MIN_HEIGHT}; Q quits"),
        );
        windows.render_windows(terminal)?;
        return Ok(None);
    };

    for cell in &app.world.cells {
        if let Some(screen) = viewport.screen_point(app.camera, *cell) {
            add_cell_window(windows, screen, 2, LIVE_COLOR);
        }
    }

    let cursor = viewport.center(app.camera);
    let cursor_screen = viewport
        .screen_point(app.camera, cursor)
        .expect("viewport center is visible");
    add_cell_window(windows, cursor_screen, 3, CURSOR_COLOR);

    let header_id = windows.new_window(10);
    let header = windows.get_mut(header_id).expect("header exists");
    header.x = 0;
    header.y = 0;
    header.width = i32::from(width);
    header.height = HEADER_HEIGHT;
    header.border = 0;
    header.filling = true;
    header.name = Some(" LIFE ".into());
    header.posprint(
        0,
        1,
        format!(
            "{}  gen:{} cells:{}  cam:({}, {}) | Arrows Space N Enter S/L C Q | {}",
            if app.paused { "PAUSED" } else { "RUN" },
            app.world.generation,
            app.world.cells.len(),
            cursor.x,
            cursor.y,
            app.status
        ),
    );

    windows.render_windows(terminal)?;
    Ok(Some(viewport))
}

fn handle_mouse(app: &mut App, viewport: Option<Viewport>, mouse: Mouse) {
    if mouse.event == MouseEvent::WHEELUP {
        app.move_camera(0, -1);
        return;
    }
    if mouse.event == MouseEvent::WHEELDOWN {
        app.move_camera(0, 1);
        return;
    }
    let Some(point) = viewport.and_then(|viewport| viewport.mouse_point(app.camera, mouse)) else {
        return;
    };
    match mouse.event {
        MouseEvent::LEFT => {
            app.world.toggle(point);
            app.status = format!("Toggled cell ({}, {})", point.x, point.y);
        }
        MouseEvent::SPAMLEFT => {
            app.world.set_alive(point, true);
            app.status = format!("Drew cell ({}, {})", point.x, point.y);
        }
        MouseEvent::RIGHT | MouseEvent::SPAMRIGHT => {
            app.world.set_alive(point, false);
            app.status = format!("Erased cell ({}, {})", point.x, point.y);
        }
        _ => {}
    }
}

fn handle_key(app: &mut App, viewport: Option<Viewport>, key: i32) -> bool {
    match key {
        113 | 81 => return false,
        KEY_LEFT => app.move_camera(-1, 0),
        KEY_RIGHT => app.move_camera(1, 0),
        KEY_UP => app.move_camera(0, -1),
        KEY_DOWN => app.move_camera(0, 1),
        32 => app.toggle_pause(),
        10 | 13 => {
            if let Some(viewport) = viewport {
                let point = viewport.center(app.camera);
                app.world.toggle(point);
                app.status = format!("Toggled center cell ({}, {})", point.x, point.y);
            }
        }
        110 | 78 if app.paused => {
            app.world.step();
            app.status = "Advanced one generation".into();
        }
        115 | 83 => app.save(),
        108 | 76 => app.load(),
        99 | 67 => {
            app.world.clear();
            app.status = "Field cleared".into();
        }
        _ => {}
    }
    true
}

fn main() -> io::Result<()> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PATH));
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let mut app = App::new(path);

    loop {
        app.advance_if_due();
        let (height, width) = terminal.getsize();
        let viewport = render(&mut terminal, &mut windows, &app, height, width)?;
        let key = terminal.get_key(INPUT_FRAME_MS, true)?;
        if key == MOUSE_EVENT {
            handle_mouse(&mut app, viewport, terminal.getmouse());
        } else if !handle_key(&mut app, viewport, key) {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blinker_oscillates() {
        let mut world = World::default();
        world
            .cells
            .extend([Point::new(-1, 0), Point::new(0, 0), Point::new(1, 0)]);

        world.step();
        assert_eq!(
            world.cells,
            BTreeSet::from([Point::new(0, -1), Point::new(0, 0), Point::new(0, 1)])
        );
        world.step();
        assert_eq!(
            world.cells,
            BTreeSet::from([Point::new(-1, 0), Point::new(0, 0), Point::new(1, 0)])
        );
    }

    #[test]
    fn world_format_round_trips_negative_coordinates() {
        let world = World {
            cells: BTreeSet::from([Point::new(-12, 7), Point::new(4, -9)]),
            generation: 42,
        };
        assert_eq!(World::deserialize(&world.serialize()).unwrap(), world);
    }

    #[test]
    fn world_saves_and_loads_from_disk() {
        let world = World {
            cells: BTreeSet::from([Point::new(-3, 8), Point::new(12, 1)]),
            generation: 7,
        };
        let path = env::temp_dir().join(format!(
            "termutils-life-{}-{}.world",
            std::process::id(),
            std::thread::current().name().unwrap_or("world")
        ));

        world.save(&path).unwrap();
        let loaded = World::load(&path).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(loaded, world);
    }

    #[test]
    fn camera_maps_mouse_to_the_unbounded_world() {
        let viewport = Viewport {
            columns: 10,
            rows: 5,
        };
        let point = viewport.mouse_point(
            Point::new(-20, 30),
            Mouse {
                x: 7,
                y: 8,
                event: MouseEvent::LEFT,
            },
        );
        assert_eq!(point, Some(Point::new(-17, 32)));
    }

    #[test]
    fn mouse_places_and_removes_cells() {
        let viewport = Viewport {
            columns: 10,
            rows: 5,
        };
        let mut app = App::new(PathBuf::from(DEFAULT_PATH));
        let mut mouse = Mouse {
            x: 3,
            y: 6,
            event: MouseEvent::LEFT,
        };
        let point = Point::new(1, 1);

        handle_mouse(&mut app, Some(viewport), mouse);
        assert!(app.world.is_alive(point));

        mouse.event = MouseEvent::RIGHT;
        handle_mouse(&mut app, Some(viewport), mouse);
        assert!(!app.world.is_alive(point));
    }

    #[test]
    fn live_cells_are_two_by_two_windows() {
        let mut windows = WindowManager::new();
        let id = add_cell_window(&mut windows, Point::new(6, 9), 2, LIVE_COLOR);
        let window = windows.get(id).unwrap();
        assert_eq!((window.x, window.y), (6, 9));
        assert_eq!((window.width, window.height), (2, 2));
        assert_eq!(window.border_style, CellStyle::foreground(LIVE_COLOR));
    }
}
