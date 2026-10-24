use crate::framebuffer::{FrameBuffer, PresentStats};
use crate::layout::Rect;
use crate::terminal::{Mouse, MouseEvent, Terminal};
use std::collections::BTreeMap;
use std::io::{self, Write};

pub type WindowId = u64;
pub type WindowUpdater = Box<dyn FnMut(&mut Window)>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Cell {
    pub sym: char,
    pub color: Option<String>,
    pub x: i32,
    pub y: i32,
}

pub struct Window {
    pub id: WindowId,
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub always_on_screen: bool,
    pub fullscreen: bool,
    saved_bounds: Option<(i32, i32, i32, i32)>,
    pub layer: i32,
    pub border: i32,
    pub filling: bool,
    pub visible: bool,
    content: BTreeMap<(i32, i32), Cell>,
    pub content_offset_x: i32,
    pub content_offset_y: i32,
    pub updater: Option<WindowUpdater>,
    pub dragable: bool,
    pub drag: bool,
    pub drag_offset: i32,
    pub mouseable: bool,
    pub mouse: bool,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_event: MouseEvent,
}

impl Window {
    pub fn new(layer: i32) -> Self {
        Self::with_id(0, layer)
    }

    fn with_id(id: WindowId, layer: i32) -> Self {
        Self {
            id,
            name: None,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            always_on_screen: true,
            fullscreen: false,
            saved_bounds: None,
            layer,
            border: 0,
            filling: true,
            visible: true,
            content: BTreeMap::new(),
            content_offset_x: 0,
            content_offset_y: 0,
            updater: None,
            dragable: false,
            drag: false,
            drag_offset: 0,
            mouseable: false,
            mouse: false,
            mouse_x: 0,
            mouse_y: 0,
            mouse_event: MouseEvent::IDLE,
        }
    }

    pub fn poswchar(&mut self, y: i32, x: i32, character: char) {
        self.content.insert(
            (y, x),
            Cell {
                sym: character,
                color: None,
                x,
                y,
            },
        );
    }

    pub fn posprint(&mut self, y: i32, x: i32, text: impl AsRef<str>) {
        for (offset, character) in text.as_ref().chars().enumerate() {
            self.poswchar(y, x + offset as i32, character);
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.x = rect.x;
        self.y = rect.y;
        self.width = rect.width;
        self.height = rect.height;
        self.fullscreen = false;
        self.saved_bounds = None;
    }

    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.content.values()
    }

    pub fn toggle_fullscreen(&mut self, screen_height: u16, screen_width: u16) {
        if self.fullscreen {
            self.fullscreen = false;
            if let Some((x, y, width, height)) = self.saved_bounds.take() {
                (self.x, self.y, self.width, self.height) = (x, y, width, height);
            }
        } else {
            self.saved_bounds = Some((self.x, self.y, self.width, self.height));
            self.fullscreen = true;
            self.apply_fullscreen(screen_height, screen_width);
        }
    }

    fn apply_fullscreen(&mut self, screen_height: u16, screen_width: u16) {
        self.x = -1;
        self.y = -1;
        self.width = i32::from(screen_width) + 3;
        self.height = i32::from(screen_height) + 3;
    }

    fn update_interaction(&mut self, mouse: Mouse) {
        self.mouse = false;
        if self.mouseable
            && matches!(mouse.event, MouseEvent::LEFT | MouseEvent::SPAMLEFT)
            && contains(self, mouse)
        {
            self.mouse = true;
            self.mouse_x = i32::from(mouse.x) - self.x - 1;
            self.mouse_y = i32::from(mouse.y) - self.y - 1;
            self.mouse_event = mouse.event;
        } else {
            self.mouse_event = MouseEvent::IDLE;
        }

        if self.dragable {
            if mouse.event == MouseEvent::LEFT
                && i32::from(mouse.y) == self.y
                && i32::from(mouse.x) >= self.x
                && i32::from(mouse.x) <= self.x + self.width
            {
                self.drag = true;
                self.drag_offset = i32::from(mouse.x) - self.x;
            }
            if self.drag {
                if mouse.event == MouseEvent::IDLE {
                    self.drag = false;
                } else {
                    self.x = i32::from(mouse.x) - self.drag_offset;
                    self.y = i32::from(mouse.y);
                }
            }
        }
    }

    fn clamp_to_screen(&mut self, screen_height: u16, screen_width: u16) {
        if !self.always_on_screen {
            return;
        }
        let max_x = (i32::from(screen_width) - self.width).max(0);
        let max_y = (i32::from(screen_height) - self.height).max(0);
        self.x = self.x.clamp(0, max_x);
        self.y = self.y.clamp(0, max_y);
    }

    fn run_updater(&mut self) {
        if let Some(mut updater) = self.updater.take() {
            updater(self);
            self.updater = Some(updater);
        }
    }
}

fn contains(window: &Window, mouse: Mouse) -> bool {
    let x = i32::from(mouse.x);
    let y = i32::from(mouse.y);
    y >= window.y
        && y <= window.y + window.height - 1
        && x >= window.x
        && x <= window.x + window.width - 1
}

#[derive(Default)]
pub struct WindowManager {
    windows: Vec<Window>,
    next_id: WindowId,
    framebuffer: FrameBuffer,
}

impl WindowManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_window(&mut self, layer: i32) -> WindowId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.windows.push(Window::with_id(id, layer));
        self.sort_layers();
        id
    }

    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|window| window.id == id)
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|window| window.id == id)
    }

    pub fn del_window(&mut self, id: WindowId) -> Option<Window> {
        let index = self.windows.iter().position(|window| window.id == id)?;
        Some(self.windows.remove(index))
    }

    pub fn del_all_windows(&mut self) {
        self.windows.clear();
        self.next_id = 0;
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }

    /// Forces the next render to repaint the complete terminal.
    ///
    /// Call this after writing directly to the terminal or clearing it outside
    /// of this manager.
    pub fn invalidate(&mut self) {
        self.framebuffer.invalidate();
    }

    fn sort_layers(&mut self) {
        self.windows.sort_by_key(|window| window.layer);
    }

    pub fn render_windows(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        self.render_windows_with_stats(terminal).map(|_| ())
    }

    pub fn render_windows_with_stats(
        &mut self,
        terminal: &mut Terminal,
    ) -> io::Result<PresentStats> {
        let (screen_height, screen_width) = terminal.getsize();
        let mouse = terminal.getmouse();
        let stats =
            self.render_to_with_stats(screen_height, screen_width, mouse, &mut terminal.stdout)?;
        terminal.refresh()?;
        Ok(stats)
    }

    pub fn render_to(
        &mut self,
        screen_height: u16,
        screen_width: u16,
        mouse: Mouse,
        writer: &mut impl Write,
    ) -> io::Result<()> {
        self.render_to_with_stats(screen_height, screen_width, mouse, writer)
            .map(|_| ())
    }

    pub fn render_to_with_stats(
        &mut self,
        screen_height: u16,
        screen_width: u16,
        mouse: Mouse,
        writer: &mut impl Write,
    ) -> io::Result<PresentStats> {
        self.sort_layers();
        self.framebuffer.resize(screen_width, screen_height);
        self.framebuffer.begin_frame();
        for window in &mut self.windows {
            window.update_interaction(mouse);
            if window.fullscreen {
                window.apply_fullscreen(screen_height, screen_width);
            } else {
                window.clamp_to_screen(screen_height, screen_width);
            }
            if window.visible {
                render_window(window, &mut self.framebuffer);
            }
            window.run_updater();
        }
        self.framebuffer.present(writer)
    }
}

fn render_window(window: &Window, framebuffer: &mut FrameBuffer) {
    if window.filling && window.width > 2 && window.height > 2 {
        for row in 0..window.height - 2 {
            for column in 0..window.width - 2 {
                framebuffer.put(window.y + row + 1, window.x + column + 1, ' ');
            }
        }
    }
    for cell in window.cells() {
        if cell.y >= 0 && cell.x >= 0 && cell.y < window.height - 2 && cell.x < window.width - 2 {
            framebuffer.put(window.y + cell.y + 1, window.x + cell.x + 1, cell.sym);
        }
    }
    if window.border != -1 {
        draw_box(
            framebuffer,
            window.y,
            window.x,
            window.height,
            window.width,
            window.border != 0,
        );
    }
    if let Some(name) = &window.name {
        framebuffer.put_text(window.y, window.x + 1, name);
    }
}

fn draw_box(framebuffer: &mut FrameBuffer, y: i32, x: i32, height: i32, width: i32, square: bool) {
    if height < 2 || width < 2 {
        return;
    }
    let (top_left, top_right, bottom_left, bottom_right) = if square {
        ('┌', '┐', '└', '┘')
    } else {
        ('╭', '╮', '╰', '╯')
    };
    for column in 1..width - 1 {
        framebuffer.put(y, x + column, '─');
        framebuffer.put(y + height - 1, x + column, '─');
    }
    for row in 1..height - 1 {
        framebuffer.put(y + row, x, '│');
        framebuffer.put(y + row, x + width - 1, '│');
    }
    framebuffer.put(y, x, top_left);
    framebuffer.put(y, x + width - 1, top_right);
    framebuffer.put(y + height - 1, x, bottom_left);
    framebuffer.put(y + height - 1, x + width - 1, bottom_right);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_is_updated_and_kept_in_coordinate_order() {
        let mut window = Window::new(0);
        window.poswchar(2, 1, 'a');
        window.poswchar(0, 2, 'b');
        window.poswchar(2, 1, 'c');
        let cells: Vec<_> = window.cells().cloned().collect();
        assert_eq!(cells.len(), 2);
        assert_eq!((cells[0].y, cells[0].x, cells[0].sym), (0, 2, 'b'));
        assert_eq!((cells[1].y, cells[1].x, cells[1].sym), (2, 1, 'c'));
    }

    #[test]
    fn manager_orders_layers_and_handles_mouse_and_drag() {
        let mut manager = WindowManager::new();
        let upper = manager.new_window(10);
        let lower = manager.new_window(0);
        {
            let window = manager.get_mut(upper).unwrap();
            window.width = 6;
            window.height = 4;
            window.name = Some("upper".into());
        }
        {
            let window = manager.get_mut(lower).unwrap();
            window.x = 2;
            window.y = 1;
            window.width = 6;
            window.height = 4;
            window.mouseable = true;
            window.dragable = true;
            window.name = Some("lower".into());
            window.posprint(0, 0, "ok");
        }
        let mut output = Vec::new();
        let stats = manager
            .render_to_with_stats(
                20,
                40,
                Mouse {
                    x: 3,
                    y: 1,
                    event: MouseEvent::LEFT,
                },
                &mut output,
            )
            .unwrap();
        let lower = manager.get(lower).unwrap();
        assert!(lower.mouse);
        assert!(lower.drag);
        assert_eq!(lower.mouse_x, 0);
        assert_eq!(stats.changed_cells, 20 * 40);
        assert_eq!(stats.cursor_moves, 20);
        assert_eq!(manager.framebuffer().get(0, 1).unwrap().symbol, 'u');
        assert_eq!(manager.framebuffer().get(2, 3).unwrap().symbol, ' ');
        assert!(!output.is_empty());
    }

    #[test]
    fn manager_emits_only_changed_cells_after_the_first_frame() {
        let mut manager = WindowManager::new();
        let id = manager.new_window(0);
        let window = manager.get_mut(id).unwrap();
        window.x = 1;
        window.y = 1;
        window.width = 5;
        window.height = 3;
        window.border = -1;
        window.filling = false;
        window.poswchar(0, 0, 'a');

        manager
            .render_to(5, 10, Mouse::default(), &mut Vec::new())
            .unwrap();

        let mut unchanged = Vec::new();
        let stats = manager
            .render_to_with_stats(5, 10, Mouse::default(), &mut unchanged)
            .unwrap();
        assert!(unchanged.is_empty());
        assert_eq!(stats, PresentStats::default());

        manager.get_mut(id).unwrap().poswchar(0, 0, 'b');
        let mut changed = Vec::new();
        let stats = manager
            .render_to_with_stats(5, 10, Mouse::default(), &mut changed)
            .unwrap();
        assert_eq!(String::from_utf8(changed).unwrap(), "\x1b[3;3Hb");
        assert_eq!(stats.changed_cells, 1);
        assert_eq!(stats.cursor_moves, 1);
    }

    #[test]
    fn fullscreen_restores_previous_bounds() {
        let mut window = Window::new(0);
        (window.x, window.y, window.width, window.height) = (4, 3, 20, 10);
        window.toggle_fullscreen(24, 80);
        assert_eq!(
            (window.x, window.y, window.width, window.height),
            (-1, -1, 83, 27)
        );
        window.toggle_fullscreen(24, 80);
        assert_eq!(
            (window.x, window.y, window.width, window.height),
            (4, 3, 20, 10)
        );
    }

    #[test]
    fn setting_a_layout_rect_leaves_fullscreen_mode() {
        let mut window = Window::new(0);
        window.toggle_fullscreen(24, 80);
        window.set_rect(Rect::new(2, 3, 40, 10));
        assert!(!window.fullscreen);
        assert_eq!(window.rect(), Rect::new(2, 3, 40, 10));
    }
}
