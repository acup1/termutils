use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use terminalib::{
    terminal::Terminal, CellStyle, Layout, LayoutError, Mouse, MouseEvent, Rect, Rgb, Window,
    WindowManager, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP, MOUSE_EVENT,
};

const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 24;
const FRAME_MS: u64 = 33;
const DEFAULT_PATH: &str = "drawing.tpaint";
const FILE_MENU_WIDTH: i32 = 18;
const CONTEXT_MENU_WIDTH: i32 = 18;
const SYMBOL_MENU_WIDTH: i32 = 30;
const PICKER_WIDTH: i32 = 34;
const PICKER_HEIGHT: i32 = 17;
const PICKER_COLUMNS: usize = 13;
const PICKER_ROWS: usize = 12;
const HUE_STEPS: usize = 12;
const SYMBOLS: [char; 12] = ['█', '▓', '▒', '░', '●', '◆', '★', '@', '#', '+', '*', '.'];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
}

impl Tool {
    const ALL: [Self; 3] = [Self::Pencil, Self::Eraser, Self::Fill];

    fn name(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
        }
    }

    fn shortcut(self) -> char {
        match self {
            Self::Pencil => 'P',
            Self::Eraser => 'E',
            Self::Fill => 'F',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ColorTarget {
    Foreground,
    Background,
}

impl ColorTarget {
    fn name(self) -> &'static str {
        match self {
            Self::Foreground => "Foreground",
            Self::Background => "Background",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StrokeButton {
    Draw,
    Erase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PaintCell {
    symbol: char,
    foreground: Rgb,
    background: Rgb,
}

#[derive(Debug, Default)]
struct Canvas {
    width: i32,
    height: i32,
    cells: BTreeMap<(i32, i32), PaintCell>,
}

impl Canvas {
    fn resize(&mut self, width: i32, height: i32) {
        self.width = width.max(0);
        self.height = height.max(0);
    }

    fn paint(&mut self, x: i32, y: i32, tool: Tool, cell: PaintCell) {
        if !self.contains(x, y) {
            return;
        }
        match tool {
            Tool::Pencil => {
                self.cells.insert((x, y), cell);
            }
            Tool::Eraser => {
                self.cells.remove(&(x, y));
            }
            Tool::Fill => self.fill(x, y, cell),
        }
    }

    fn paint_line(&mut self, from: (i32, i32), to: (i32, i32), tool: Tool, cell: PaintCell) {
        let (mut x, mut y) = from;
        let (to_x, to_y) = to;
        let dx = (to_x - x).abs();
        let step_x = if x < to_x { 1 } else { -1 };
        let dy = -(to_y - y).abs();
        let step_y = if y < to_y { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.paint(x, y, tool, cell);
            if (x, y) == (to_x, to_y) {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                error += dy;
                x += step_x;
            }
            if doubled <= dx {
                error += dx;
                y += step_y;
            }
        }
    }

    fn fill(&mut self, x: i32, y: i32, replacement: PaintCell) {
        let target = self.cells.get(&(x, y)).copied();
        if target == Some(replacement) {
            return;
        }
        let mut pending = vec![(x, y)];
        let mut visited = BTreeMap::new();
        while let Some((x, y)) = pending.pop() {
            if !self.contains(x, y) || visited.insert((x, y), ()).is_some() {
                continue;
            }
            if self.cells.get(&(x, y)).copied() != target {
                continue;
            }
            self.cells.insert((x, y), replacement);
            pending.extend([(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
        }
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    fn clear(&mut self) {
        self.cells.clear();
    }

    fn serialize(&self) -> String {
        let mut output = format!("TERMINALIB_PAINT 1\n{} {}\n", self.width, self.height);
        for ((x, y), cell) in &self.cells {
            output.push_str(&format!(
                "{x} {y} {:X} {} {}\n",
                u32::from(cell.symbol),
                rgb_hex(cell.foreground),
                rgb_hex(cell.background)
            ));
        }
        output
    }

    fn deserialize(input: &str) -> Result<Self, String> {
        let mut lines = input.lines();
        if lines.next() != Some("TERMINALIB_PAINT 1") {
            return Err("not a terminalib paint v1 file".into());
        }
        let dimensions = lines.next().ok_or("missing canvas dimensions")?;
        let mut dimensions = dimensions.split_whitespace();
        let width = parse_i32(dimensions.next(), "width")?;
        let height = parse_i32(dimensions.next(), "height")?;
        if width < 0 || height < 0 || dimensions.next().is_some() {
            return Err("invalid canvas dimensions".into());
        }

        let mut canvas = Self {
            width,
            height,
            cells: BTreeMap::new(),
        };
        for (index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let mut fields = line.split_whitespace();
            let x = parse_i32(fields.next(), "x")?;
            let y = parse_i32(fields.next(), "y")?;
            let symbol = fields
                .next()
                .and_then(|value| u32::from_str_radix(value, 16).ok())
                .and_then(char::from_u32)
                .ok_or_else(|| format!("invalid symbol on line {}", index + 3))?;
            let foreground = parse_rgb(fields.next(), "foreground")?;
            let background = parse_rgb(fields.next(), "background")?;
            if fields.next().is_some() || x < 0 || y < 0 {
                return Err(format!("invalid cell on line {}", index + 3));
            }
            canvas.cells.insert(
                (x, y),
                PaintCell {
                    symbol,
                    foreground,
                    background,
                },
            );
        }
        Ok(canvas)
    }

    fn save(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.serialize())
    }

    fn open(path: &Path) -> io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        Self::deserialize(&contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

fn parse_i32(value: Option<&str>, name: &str) -> Result<i32, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse()
        .map_err(|_| format!("invalid {name}"))
}

fn rgb_hex(color: Rgb) -> String {
    format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

fn parse_rgb(value: Option<&str>, name: &str) -> Result<Rgb, String> {
    let value = value.ok_or_else(|| format!("missing {name}"))?;
    if value.len() != 6 {
        return Err(format!("invalid {name}"));
    }
    let number = u32::from_str_radix(value, 16).map_err(|_| format!("invalid {name}"))?;
    Ok(Rgb::new(
        ((number >> 16) & 0xff) as u8,
        ((number >> 8) & 0xff) as u8,
        (number & 0xff) as u8,
    ))
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> Rgb {
    let chroma = value * saturation;
    let sector = (hue / 60.0).rem_euclid(6.0);
    let secondary = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = value - chroma;
    Rgb::new(
        ((red + offset) * 255.0).round() as u8,
        ((green + offset) * 255.0).round() as u8,
        ((blue + offset) * 255.0).round() as u8,
    )
}

fn rgb_to_hsv(color: Rgb) -> (f32, f32, f32) {
    let red = f32::from(color.r) / 255.0;
    let green = f32::from(color.g) / 255.0;
    let blue = f32::from(color.b) / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let hue = if delta == 0.0 {
        0.0
    } else if maximum == red {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if maximum == green {
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    let saturation = if maximum == 0.0 { 0.0 } else { delta / maximum };
    (hue, saturation, maximum)
}

fn picker_color(hue: usize, saturation: usize, value: usize) -> Rgb {
    hsv_to_rgb(
        hue as f32 * 360.0 / HUE_STEPS as f32,
        saturation as f32 / (PICKER_COLUMNS - 1) as f32,
        1.0 - value as f32 / (PICKER_ROWS - 1) as f32,
    )
}

fn picker_indices(color: Rgb) -> (usize, usize, usize) {
    let (hue, saturation, value) = rgb_to_hsv(color);
    (
        ((hue / 360.0 * HUE_STEPS as f32).round() as usize) % HUE_STEPS,
        (saturation * (PICKER_COLUMNS - 1) as f32).round() as usize,
        ((1.0 - value) * (PICKER_ROWS - 1) as f32).round() as usize,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileAction {
    Open,
    Save,
}

impl FileAction {
    const ALL: [Self; 2] = [Self::Open, Self::Save];

    fn name(self) -> &'static str {
        match self {
            Self::Open => "Open…  O",
            Self::Save => "Save…  S",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextAction {
    Pencil,
    Eraser,
    Fill,
    Clear,
}

impl ContextAction {
    const ALL: [Self; 4] = [Self::Pencil, Self::Eraser, Self::Fill, Self::Clear];

    fn name(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill here",
            Self::Clear => "Clear canvas",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Overlay {
    None,
    FileMenu {
        selected: usize,
    },
    ContextMenu {
        x: i32,
        y: i32,
        canvas_x: i32,
        canvas_y: i32,
        selected: usize,
    },
    SymbolMenu {
        selected: usize,
    },
    ColorPicker {
        target: ColorTarget,
        hue: usize,
        saturation: usize,
        value: usize,
        original: Rgb,
    },
    PathDialog {
        action: FileAction,
        input: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OverlayChoice {
    File(FileAction),
    Context(ContextAction, i32, i32),
    Symbol(usize),
    PickerSv(usize, usize),
    PickerHue(usize),
    PickerAccept,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HitBox(Rect);

impl HitBox {
    fn contains(self, mouse: Mouse) -> bool {
        let x = i32::from(mouse.x);
        let y = i32::from(mouse.y);
        x >= self.0.x
            && y >= self.0.y
            && x < self.0.x + self.0.width
            && y < self.0.y + self.0.height
    }
}

#[derive(Debug, Default)]
struct UiHits {
    file_button: HitBox,
    symbol_button: HitBox,
    tools: Vec<(HitBox, Tool)>,
    foreground: HitBox,
    background: HitBox,
    swap: HitBox,
    recent: Vec<(HitBox, Rgb)>,
    canvas: HitBox,
    overlay: Vec<(HitBox, OverlayChoice)>,
}

struct App {
    canvas: Canvas,
    tool: Tool,
    foreground: Rgb,
    background: Rgb,
    color_target: ColorTarget,
    symbol: char,
    recent: VecDeque<Rgb>,
    overlay: Overlay,
    stroke: Option<(StrokeButton, (i32, i32))>,
    path: String,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            canvas: Canvas::default(),
            tool: Tool::Pencil,
            foreground: Rgb::WHITE,
            background: Rgb::BLACK,
            color_target: ColorTarget::Foreground,
            symbol: '█',
            recent: VecDeque::from([Rgb::WHITE, Rgb::BLACK]),
            overlay: Overlay::None,
            stroke: None,
            path: DEFAULT_PATH.into(),
            status: "Ready — left draws, right erases, middle opens menu".into(),
        }
    }
}

impl App {
    fn selected_cell(&self) -> PaintCell {
        PaintCell {
            symbol: self.symbol,
            foreground: self.foreground,
            background: self.background,
        }
    }

    fn choose_color(&mut self, color: Rgb) {
        self.set_color(self.color_target, color);
        self.recent.retain(|recent| *recent != color);
        self.recent.push_front(color);
        self.recent.truncate(5);
        self.status = format!("{} color: {}", self.color_target.name(), color.hex());
    }

    fn set_color(&mut self, target: ColorTarget, color: Rgb) {
        match target {
            ColorTarget::Foreground => self.foreground = color,
            ColorTarget::Background => self.background = color,
        }
    }

    fn open_color_picker(&mut self, target: ColorTarget) {
        self.color_target = target;
        let original = match target {
            ColorTarget::Foreground => self.foreground,
            ColorTarget::Background => self.background,
        };
        let (hue, saturation, value) = picker_indices(original);
        self.overlay = Overlay::ColorPicker {
            target,
            hue,
            saturation,
            value,
            original,
        };
    }

    fn update_picker(&mut self, hue: usize, saturation: usize, value: usize) {
        let target = match &self.overlay {
            Overlay::ColorPicker { target, .. } => *target,
            _ => return,
        };
        self.set_color(target, picker_color(hue, saturation, value));
        if let Overlay::ColorPicker {
            hue: picker_hue,
            saturation: picker_saturation,
            value: picker_value,
            ..
        } = &mut self.overlay
        {
            *picker_hue = hue;
            *picker_saturation = saturation;
            *picker_value = value;
        }
    }

    fn accept_picker(&mut self) {
        let (target, color) = match &self.overlay {
            Overlay::ColorPicker {
                target,
                hue,
                saturation,
                value,
                ..
            } => (*target, picker_color(*hue, *saturation, *value)),
            _ => return,
        };
        self.color_target = target;
        self.choose_color(color);
        self.overlay = Overlay::None;
    }

    fn cancel_picker(&mut self) {
        if let Overlay::ColorPicker {
            target, original, ..
        } = &self.overlay
        {
            self.set_color(*target, *original);
        }
        self.overlay = Overlay::None;
    }

    fn swap_colors(&mut self) {
        std::mem::swap(&mut self.foreground, &mut self.background);
        self.status = "Foreground and background swapped".into();
    }

    fn begin_file_action(&mut self, action: FileAction) {
        self.overlay = Overlay::PathDialog {
            action,
            input: self.path.clone(),
        };
    }

    fn finish_file_action(&mut self, action: FileAction, input: String) {
        let path = Path::new(&input);
        let result = match action {
            FileAction::Open => Canvas::open(path).map(|canvas| self.canvas = canvas),
            FileAction::Save => self.canvas.save(path),
        };
        match result {
            Ok(()) => {
                self.path = input;
                self.status = format!(
                    "{} {}",
                    action.name().split('…').next().unwrap_or("Done"),
                    self.path
                );
            }
            Err(error) => {
                self.status = format!(
                    "{} failed: {error}",
                    action.name().split('…').next().unwrap_or("File")
                )
            }
        }
        self.overlay = Overlay::None;
    }

    fn use_context_action(&mut self, action: ContextAction, canvas_point: Option<(i32, i32)>) {
        match action {
            ContextAction::Pencil => self.tool = Tool::Pencil,
            ContextAction::Eraser => self.tool = Tool::Eraser,
            ContextAction::Fill => {
                self.tool = Tool::Fill;
                if let Some((x, y)) = canvas_point {
                    self.canvas.paint(x, y, Tool::Fill, self.selected_cell());
                }
            }
            ContextAction::Clear => self.canvas.clear(),
        }
        self.status = format!("{} selected", action.name());
        self.overlay = Overlay::None;
    }
}

fn layout_error(error: LayoutError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn content_hit(rect: Rect, x: i32, y: i32, width: i32, height: i32) -> HitBox {
    HitBox(Rect::new(rect.x + x + 1, rect.y + y + 1, width, height))
}

fn configure_window(window: &mut Window, title: &str) {
    window.border = 0;
    window.filling = true;
    window.name = Some(format!(" {title} "));
}

fn add_swatch(window: &mut Window, y: i32, x: i32, color: Rgb) {
    window.posprint_styled(y, x, "  ", CellStyle::background(color));
}

fn contrast_color(color: Rgb) -> Rgb {
    if u32::from(color.r) * 299 + u32::from(color.g) * 587 + u32::from(color.b) * 114 > 127_500 {
        Rgb::BLACK
    } else {
        Rgb::WHITE
    }
}

fn render(
    terminal: &mut Terminal,
    windows: &mut WindowManager,
    app: &mut App,
    height: u16,
    width: u16,
) -> io::Result<UiHits> {
    windows.del_all_windows();
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        let id = windows.new_window(0);
        let window = windows.get_mut(id).expect("new window exists");
        window.set_rect(Rect::from_terminal(height, width));
        window.border = -1;
        window.posprint(
            0,
            0,
            format!("Paint needs at least {MIN_WIDTH}x{MIN_HEIGHT}; q quits"),
        );
        windows.render_windows(terminal)?;
        return Ok(UiHits::default());
    }

    let menu_id = windows.new_window(1);
    let body_id = windows.new_window(0);
    let status_id = windows.new_window(1);
    let tools_id = windows.new_window(1);
    let canvas_id = windows.new_window(1);
    let colors_id = windows.new_window(1);
    windows.get_mut(body_id).expect("body exists").visible = false;

    let screen = Rect::from_terminal(height, width);
    let rows = Layout::vertical([13, 74, 13])
        .apply(screen, windows, &[menu_id, body_id, status_id])
        .map_err(layout_error)?;
    let columns = Layout::horizontal([15, 67, 18])
        .apply(rows[1], windows, &[tools_id, canvas_id, colors_id])
        .map_err(layout_error)?;
    let menu_rect = rows[0];
    let tools_rect = columns[0];
    let canvas_rect = columns[1];
    let colors_rect = columns[2];

    let menu = windows.get_mut(menu_id).expect("menu exists");
    configure_window(menu, "Termutils Paint");
    menu.posprint(0, 1, "[ File ]");
    menu.posprint(0, 12, format!("[ Symbol: {} ]", app.symbol));
    menu.posprint(0, 30, "O Open  S Save  Q Quit");

    let tools = windows.get_mut(tools_id).expect("tools exist");
    configure_window(tools, "Tools");
    let mut hits = UiHits {
        file_button: content_hit(menu_rect, 1, 0, 8, 1),
        symbol_button: content_hit(menu_rect, 12, 0, 13, 1),
        canvas: HitBox(Rect::new(
            canvas_rect.x + 1,
            canvas_rect.y + 1,
            (canvas_rect.width - 2).max(0),
            (canvas_rect.height - 2).max(0),
        )),
        ..UiHits::default()
    };
    for (index, tool) in Tool::ALL.into_iter().enumerate() {
        let y = index as i32;
        let marker = if app.tool == tool { '>' } else { ' ' };
        tools.posprint(y, 0, format!("{marker}{} {}", tool.shortcut(), tool.name()));
        hits.tools.push((
            content_hit(tools_rect, 0, y, (tools_rect.width - 2).max(0), 1),
            tool,
        ));
    }
    tools.posprint(4, 0, "L draw");
    tools.posprint(5, 0, "R erase");
    tools.posprint(6, 0, "M menu");

    let canvas = windows.get_mut(canvas_id).expect("canvas exists");
    configure_window(canvas, "Canvas");
    app.canvas.resize(
        (canvas_rect.width - 2).max(0),
        (canvas_rect.height - 2).max(0),
    );
    for ((x, y), cell) in &app.canvas.cells {
        canvas.posstyled(
            *y,
            *x,
            cell.symbol,
            CellStyle::new(Some(cell.foreground), Some(cell.background)),
        );
    }

    let colors = windows.get_mut(colors_id).expect("colors exist");
    configure_window(colors, "Colors");
    let fg_marker = if app.color_target == ColorTarget::Foreground {
        '>'
    } else {
        ' '
    };
    let bg_marker = if app.color_target == ColorTarget::Background {
        '>'
    } else {
        ' '
    };
    colors.posprint(0, 0, format!("{fg_marker}FG"));
    add_swatch(colors, 0, 4, app.foreground);
    colors.posprint(1, 0, format!("{bg_marker}BG"));
    add_swatch(colors, 1, 4, app.background);
    colors.posprint(2, 0, "[X] Swap");
    hits.foreground = content_hit(colors_rect, 0, 0, (colors_rect.width - 2).max(0), 1);
    hits.background = content_hit(colors_rect, 0, 1, (colors_rect.width - 2).max(0), 1);
    hits.swap = content_hit(colors_rect, 0, 2, 9, 1);

    colors.posprint(4, 0, "Recent");
    for (index, color) in app.recent.iter().copied().enumerate() {
        let x = index as i32 * 2;
        add_swatch(colors, 5, x, color);
        hits.recent
            .push((content_hit(colors_rect, x, 5, 2, 1), color));
    }

    let status = windows.get_mut(status_id).expect("status exists");
    configure_window(status, "Status");
    status.posprint(
        0,
        1,
        format!(
            "{} | '{}' | FG {} | BG {} | {}",
            app.tool.name(),
            app.symbol,
            app.foreground.hex(),
            app.background.hex(),
            app.status
        ),
    );

    render_overlay(windows, app, screen, &mut hits)?;
    windows.render_windows(terminal)?;
    Ok(hits)
}

fn popup_rect(screen: Rect, x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect::new(
        x.clamp(0, (screen.width - width).max(0)),
        y.clamp(0, (screen.height - height).max(0)),
        width.min(screen.width),
        height.min(screen.height),
    )
}

fn render_overlay(
    windows: &mut WindowManager,
    app: &App,
    screen: Rect,
    hits: &mut UiHits,
) -> io::Result<()> {
    match &app.overlay {
        Overlay::None => {}
        Overlay::FileMenu { selected } => {
            let rect = popup_rect(screen, 2, 2, FILE_MENU_WIDTH, 4);
            let id = windows.new_window(100);
            let window = windows.get_mut(id).expect("popup exists");
            window.set_rect(rect);
            configure_window(window, "File");
            for (index, action) in FileAction::ALL.into_iter().enumerate() {
                let marker = if index == *selected { '>' } else { ' ' };
                window.posprint(index as i32, 0, format!("{marker} {}", action.name()));
                hits.overlay.push((
                    content_hit(rect, 0, index as i32, rect.width - 2, 1),
                    OverlayChoice::File(action),
                ));
            }
        }
        Overlay::ContextMenu {
            x,
            y,
            canvas_x,
            canvas_y,
            selected,
        } => {
            let rect = popup_rect(screen, *x, *y, CONTEXT_MENU_WIDTH, 6);
            let id = windows.new_window(100);
            let window = windows.get_mut(id).expect("popup exists");
            window.set_rect(rect);
            configure_window(window, "Context");
            for (index, action) in ContextAction::ALL.into_iter().enumerate() {
                let marker = if index == *selected { '>' } else { ' ' };
                window.posprint(index as i32, 0, format!("{marker} {}", action.name()));
                hits.overlay.push((
                    content_hit(rect, 0, index as i32, rect.width - 2, 1),
                    OverlayChoice::Context(action, *canvas_x, *canvas_y),
                ));
            }
        }
        Overlay::SymbolMenu { selected } => {
            let rect = popup_rect(screen, 13, 2, SYMBOL_MENU_WIDTH, 6);
            let id = windows.new_window(100);
            let window = windows.get_mut(id).expect("popup exists");
            window.set_rect(rect);
            configure_window(window, "Choose symbol");
            for (index, symbol) in SYMBOLS.into_iter().enumerate() {
                let x = 1 + (index % 6) as i32 * 4;
                let y = (index / 6) as i32 * 2;
                let marker = if index == *selected { '[' } else { ' ' };
                let ending = if index == *selected { ']' } else { ' ' };
                window.posprint(y, x, format!("{marker}{symbol}{ending}"));
                hits.overlay
                    .push((content_hit(rect, x, y, 3, 1), OverlayChoice::Symbol(index)));
            }
        }
        Overlay::ColorPicker {
            target,
            hue,
            saturation,
            value,
            ..
        } => {
            let rect = popup_rect(
                screen,
                (screen.width - PICKER_WIDTH) / 2,
                (screen.height - PICKER_HEIGHT) / 2,
                PICKER_WIDTH,
                PICKER_HEIGHT,
            );
            let id = windows.new_window(110);
            let window = windows.get_mut(id).expect("picker exists");
            window.set_rect(rect);
            configure_window(window, "Color picker");
            let chosen = picker_color(*hue, *saturation, *value);
            window.posprint(0, 1, format!("{}  #{}", target.name(), rgb_hex(chosen)));

            for row in 0..PICKER_ROWS {
                for column in 0..PICKER_COLUMNS {
                    let color = picker_color(*hue, column, row);
                    let x = 1 + column as i32 * 2;
                    let y = 1 + row as i32;
                    add_swatch(window, y, x, color);
                    if column == *saturation && row == *value {
                        window.posstyled(
                            y,
                            x,
                            '◆',
                            CellStyle::new(Some(contrast_color(color)), Some(color)),
                        );
                    }
                    hits.overlay.push((
                        content_hit(rect, x, y, 2, 1),
                        OverlayChoice::PickerSv(column, row),
                    ));
                }
            }

            for step in 0..HUE_STEPS {
                let color = picker_color(step, PICKER_COLUMNS - 1, 0);
                let y = 1 + step as i32;
                add_swatch(window, y, 29, color);
                if step == *hue {
                    window.posstyled(
                        y,
                        29,
                        '◆',
                        CellStyle::new(Some(contrast_color(color)), Some(color)),
                    );
                }
                hits.overlay.push((
                    content_hit(rect, 29, y, 2, 1),
                    OverlayChoice::PickerHue(step),
                ));
            }
            let action_y = PICKER_ROWS as i32 + 2;
            window.posprint(action_y, 1, "[ OK ]   Esc cancel");
            hits.overlay.push((
                content_hit(rect, 1, action_y, 6, 1),
                OverlayChoice::PickerAccept,
            ));
        }
        Overlay::PathDialog { action, input } => {
            let rect = popup_rect(
                screen,
                (screen.width - 64) / 2,
                (screen.height - 5) / 2,
                64,
                5,
            );
            let id = windows.new_window(110);
            let window = windows.get_mut(id).expect("dialog exists");
            window.set_rect(rect);
            configure_window(window, action.name().split('…').next().unwrap_or("File"));
            window.posprint(0, 1, "Path:");
            window.posprint(1, 1, format!("> {input}_"));
            window.posprint(2, 1, "Enter confirm  Esc cancel  Backspace delete");
        }
    }
    Ok(())
}

fn mouse_canvas_point(hits: &UiHits, mouse: Mouse) -> Option<(i32, i32)> {
    hits.canvas.contains(mouse).then(|| {
        (
            i32::from(mouse.x) - hits.canvas.0.x,
            i32::from(mouse.y) - hits.canvas.0.y,
        )
    })
}

fn handle_mouse(app: &mut App, hits: &UiHits, mouse: Mouse) {
    if mouse.event == MouseEvent::IDLE {
        app.stroke = None;
        return;
    }

    if !matches!(app.overlay, Overlay::None) {
        app.stroke = None;
        if matches!(mouse.event, MouseEvent::LEFT | MouseEvent::SPAMLEFT) {
            if let Some(choice) = hits
                .overlay
                .iter()
                .find_map(|(hit, choice)| hit.contains(mouse).then_some(*choice))
            {
                activate_overlay_choice(app, choice);
            } else if mouse.event == MouseEvent::LEFT
                && !matches!(app.overlay, Overlay::PathDialog { .. })
            {
                if matches!(app.overlay, Overlay::ColorPicker { .. }) {
                    app.cancel_picker();
                } else {
                    app.overlay = Overlay::None;
                }
            }
        }
        return;
    }

    if mouse.event == MouseEvent::MIDDLE {
        app.stroke = None;
        if let Some((canvas_x, canvas_y)) = mouse_canvas_point(hits, mouse) {
            app.overlay = Overlay::ContextMenu {
                x: i32::from(mouse.x),
                y: i32::from(mouse.y),
                canvas_x,
                canvas_y,
                selected: 0,
            };
        }
        return;
    }

    if !matches!(
        mouse.event,
        MouseEvent::LEFT | MouseEvent::SPAMLEFT | MouseEvent::RIGHT | MouseEvent::SPAMRIGHT
    ) {
        app.stroke = None;
        return;
    }

    if matches!(mouse.event, MouseEvent::RIGHT | MouseEvent::SPAMRIGHT) {
        if let Some(point) = mouse_canvas_point(hits, mouse) {
            let from = match app.stroke {
                Some((StrokeButton::Erase, previous)) => previous,
                _ => point,
            };
            let cell = app.selected_cell();
            app.canvas.paint_line(from, point, Tool::Eraser, cell);
            app.stroke = Some((StrokeButton::Erase, point));
            app.status = "Erasing with right mouse button".into();
        } else {
            app.stroke = None;
        }
        return;
    }

    if mouse.event == MouseEvent::LEFT && hits.file_button.contains(mouse) {
        app.stroke = None;
        app.overlay = Overlay::FileMenu { selected: 0 };
    } else if mouse.event == MouseEvent::LEFT && hits.symbol_button.contains(mouse) {
        app.stroke = None;
        let selected = SYMBOLS
            .iter()
            .position(|symbol| *symbol == app.symbol)
            .unwrap_or(0);
        app.overlay = Overlay::SymbolMenu { selected };
    } else if let Some(tool) = hits
        .tools
        .iter()
        .find_map(|(hit, tool)| hit.contains(mouse).then_some(*tool))
    {
        app.stroke = None;
        app.tool = tool;
        app.status = format!("{} selected", tool.name());
    } else if hits.foreground.contains(mouse) {
        app.stroke = None;
        app.open_color_picker(ColorTarget::Foreground);
    } else if hits.background.contains(mouse) {
        app.stroke = None;
        app.open_color_picker(ColorTarget::Background);
    } else if hits.swap.contains(mouse) {
        app.stroke = None;
        app.swap_colors();
    } else if let Some(color) = hits
        .recent
        .iter()
        .find_map(|(hit, color)| hit.contains(mouse).then_some(*color))
    {
        app.stroke = None;
        app.choose_color(color);
    } else if let Some(point) = mouse_canvas_point(hits, mouse) {
        let cell = app.selected_cell();
        if app.tool == Tool::Fill {
            if mouse.event == MouseEvent::LEFT {
                app.canvas.paint(point.0, point.1, Tool::Fill, cell);
            }
            app.stroke = None;
        } else {
            let from = match app.stroke {
                Some((StrokeButton::Draw, previous)) => previous,
                _ => point,
            };
            app.canvas.paint_line(from, point, app.tool, cell);
            app.stroke = Some((StrokeButton::Draw, point));
        }
    } else {
        app.stroke = None;
    }
}

fn activate_overlay_choice(app: &mut App, choice: OverlayChoice) {
    match choice {
        OverlayChoice::File(action) => app.begin_file_action(action),
        OverlayChoice::Context(action, x, y) => app.use_context_action(action, Some((x, y))),
        OverlayChoice::Symbol(index) => {
            app.symbol = SYMBOLS[index];
            app.status = format!("Drawing symbol: {}", app.symbol);
            app.overlay = Overlay::None;
        }
        OverlayChoice::PickerSv(saturation, value) => {
            if let Overlay::ColorPicker { hue, .. } = &app.overlay {
                app.update_picker(*hue, saturation, value);
            }
        }
        OverlayChoice::PickerHue(hue) => {
            if let Overlay::ColorPicker {
                saturation, value, ..
            } = &app.overlay
            {
                app.update_picker(hue, *saturation, *value);
            }
        }
        OverlayChoice::PickerAccept => app.accept_picker(),
    }
}

fn move_selection(selected: &mut usize, count: usize, direction: i32) {
    *selected = (*selected as i32 + direction).rem_euclid(count as i32) as usize;
}

fn handle_key(app: &mut App, key: i32) -> bool {
    if let Overlay::PathDialog { action, input } = &mut app.overlay {
        match key {
            27 => app.overlay = Overlay::None,
            10 | 13 => {
                let action = *action;
                let input = input.clone();
                app.finish_file_action(action, input);
            }
            8 | 127 => {
                input.pop();
            }
            32..=126 => input.push(char::from_u32(key as u32).unwrap_or_default()),
            _ => {}
        }
        return true;
    }

    match app.overlay.clone() {
        Overlay::FileMenu { mut selected } => match key {
            KEY_UP => {
                move_selection(&mut selected, FileAction::ALL.len(), -1);
                app.overlay = Overlay::FileMenu { selected };
            }
            KEY_DOWN => {
                move_selection(&mut selected, FileAction::ALL.len(), 1);
                app.overlay = Overlay::FileMenu { selected };
            }
            10 | 13 => app.begin_file_action(FileAction::ALL[selected]),
            27 => app.overlay = Overlay::None,
            _ => {}
        },
        Overlay::ContextMenu {
            x,
            y,
            canvas_x,
            canvas_y,
            mut selected,
        } => match key {
            KEY_UP => {
                move_selection(&mut selected, ContextAction::ALL.len(), -1);
                app.overlay = Overlay::ContextMenu {
                    x,
                    y,
                    canvas_x,
                    canvas_y,
                    selected,
                };
            }
            KEY_DOWN => {
                move_selection(&mut selected, ContextAction::ALL.len(), 1);
                app.overlay = Overlay::ContextMenu {
                    x,
                    y,
                    canvas_x,
                    canvas_y,
                    selected,
                };
            }
            10 | 13 => {
                app.use_context_action(ContextAction::ALL[selected], Some((canvas_x, canvas_y)))
            }
            27 => app.overlay = Overlay::None,
            _ => {}
        },
        Overlay::SymbolMenu { mut selected } => match key {
            KEY_LEFT => {
                move_selection(&mut selected, SYMBOLS.len(), -1);
                app.overlay = Overlay::SymbolMenu { selected };
            }
            KEY_RIGHT => {
                move_selection(&mut selected, SYMBOLS.len(), 1);
                app.overlay = Overlay::SymbolMenu { selected };
            }
            KEY_UP => {
                move_selection(&mut selected, SYMBOLS.len(), -6);
                app.overlay = Overlay::SymbolMenu { selected };
            }
            KEY_DOWN => {
                move_selection(&mut selected, SYMBOLS.len(), 6);
                app.overlay = Overlay::SymbolMenu { selected };
            }
            10 | 13 => {
                app.symbol = SYMBOLS[selected];
                app.status = format!("Drawing symbol: {}", app.symbol);
                app.overlay = Overlay::None;
            }
            27 => app.overlay = Overlay::None,
            _ => {}
        },
        Overlay::ColorPicker {
            hue,
            saturation,
            value,
            ..
        } => match key {
            KEY_LEFT => app.update_picker(hue, saturation.saturating_sub(1), value),
            KEY_RIGHT => app.update_picker(hue, (saturation + 1).min(PICKER_COLUMNS - 1), value),
            KEY_UP => app.update_picker(hue, saturation, value.saturating_sub(1)),
            KEY_DOWN => app.update_picker(hue, saturation, (value + 1).min(PICKER_ROWS - 1)),
            91 => app.update_picker((hue + HUE_STEPS - 1) % HUE_STEPS, saturation, value),
            93 => app.update_picker((hue + 1) % HUE_STEPS, saturation, value),
            10 | 13 => app.accept_picker(),
            27 => app.cancel_picker(),
            _ => {}
        },
        Overlay::None => match key {
            113 | 81 => return false,
            111 | 79 => app.begin_file_action(FileAction::Open),
            115 | 83 => app.begin_file_action(FileAction::Save),
            112 | 80 => app.tool = Tool::Pencil,
            101 | 69 => app.tool = Tool::Eraser,
            102 | 70 => app.tool = Tool::Fill,
            120 | 88 => app.swap_colors(),
            _ => {}
        },
        Overlay::PathDialog { .. } => unreachable!(),
    }
    true
}

fn main() -> io::Result<()> {
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let mut app = App::default();

    loop {
        let (height, width) = terminal.getsize();
        let hits = render(&mut terminal, &mut windows, &mut app, height, width)?;
        let key = terminal.get_key(FRAME_MS, true)?;
        if key == MOUSE_EVENT {
            handle_mouse(&mut app, &hits, terminal.getmouse());
        } else if !handle_key(&mut app, key) {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(symbol: char, foreground: Rgb, background: Rgb) -> PaintCell {
        PaintCell {
            symbol,
            foreground,
            background,
        }
    }

    #[test]
    fn canvas_format_round_trips_symbols_and_colors() {
        let mut canvas = Canvas::default();
        canvas.resize(20, 10);
        canvas.paint(
            3,
            4,
            Tool::Pencil,
            cell('★', Rgb::new(1, 2, 3), Rgb::new(250, 240, 230)),
        );
        let loaded = Canvas::deserialize(&canvas.serialize()).unwrap();
        assert_eq!((loaded.width, loaded.height), (20, 10));
        assert_eq!(loaded.cells, canvas.cells);
    }

    #[test]
    fn eraser_removes_a_painted_cell() {
        let mut canvas = Canvas::default();
        canvas.resize(3, 3);
        let painted = cell('#', Rgb::WHITE, Rgb::BLACK);
        canvas.paint(1, 1, Tool::Pencil, painted);
        canvas.paint(1, 1, Tool::Eraser, painted);
        assert!(canvas.cells.is_empty());
    }

    #[test]
    fn fast_pointer_motion_produces_a_continuous_line() {
        let mut canvas = Canvas::default();
        canvas.resize(8, 5);
        let painted = cell('#', Rgb::WHITE, Rgb::BLACK);
        canvas.paint_line((0, 0), (7, 3), Tool::Pencil, painted);

        assert_eq!(canvas.cells.len(), 8);
        for x in 0..8 {
            assert!(canvas.cells.keys().any(|(painted_x, _)| *painted_x == x));
        }
    }

    #[test]
    fn right_mouse_erases_without_switching_the_selected_tool() {
        let mut app = App::default();
        app.canvas.resize(5, 2);
        let painted = app.selected_cell();
        app.canvas.paint(2, 0, Tool::Pencil, painted);
        let hits = UiHits {
            canvas: HitBox(Rect::new(10, 10, 5, 2)),
            ..UiHits::default()
        };

        handle_mouse(
            &mut app,
            &hits,
            Mouse {
                x: 12,
                y: 10,
                event: MouseEvent::RIGHT,
            },
        );

        assert!(app.canvas.cells.is_empty());
        assert_eq!(app.tool, Tool::Pencil);
    }

    #[test]
    fn middle_mouse_opens_the_canvas_context_menu() {
        let mut app = App::default();
        let hits = UiHits {
            canvas: HitBox(Rect::new(10, 10, 5, 2)),
            ..UiHits::default()
        };

        handle_mouse(
            &mut app,
            &hits,
            Mouse {
                x: 12,
                y: 11,
                event: MouseEvent::MIDDLE,
            },
        );

        assert!(matches!(
            app.overlay,
            Overlay::ContextMenu {
                canvas_x: 2,
                canvas_y: 1,
                ..
            }
        ));
    }

    #[test]
    fn fill_stops_at_cells_with_another_style() {
        let mut canvas = Canvas::default();
        canvas.resize(3, 3);
        let wall = cell('#', Rgb::WHITE, Rgb::BLACK);
        let fill = cell('.', Rgb::new(255, 0, 0), Rgb::BLACK);
        for y in 0..3 {
            canvas.paint(1, y, Tool::Pencil, wall);
        }
        canvas.paint(0, 0, Tool::Fill, fill);
        assert_eq!(canvas.cells.len(), 6);
        assert_eq!(canvas.cells.get(&(2, 0)), None);
    }

    #[test]
    fn recent_colors_are_unique_and_bounded() {
        let mut app = App::default();
        for value in 0..8 {
            app.choose_color(Rgb::new(value, 0, 0));
        }
        app.choose_color(Rgb::new(7, 0, 0));
        assert_eq!(app.recent.len(), 5);
        assert_eq!(app.recent.front(), Some(&Rgb::new(7, 0, 0)));
        assert_eq!(
            app.recent
                .iter()
                .filter(|color| **color == Rgb::new(7, 0, 0))
                .count(),
            1
        );
    }

    #[test]
    fn context_fill_uses_the_original_canvas_point() {
        let mut app = App::default();
        app.canvas.resize(3, 2);
        activate_overlay_choice(&mut app, OverlayChoice::Context(ContextAction::Fill, 1, 1));
        assert_eq!(app.canvas.cells.len(), 6);
        assert_eq!(app.canvas.cells.get(&(1, 1)), Some(&app.selected_cell()));
    }
}
