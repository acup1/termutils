use crate::window::{WindowId, WindowManager};
use std::error::Error;
use std::fmt;

/// A rectangular terminal region, measured in character cells.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width: width.max(0),
            height: height.max(0),
        }
    }

    pub fn from_terminal(height: u16, width: u16) -> Self {
        Self::new(0, 0, i32::from(width), i32::from(height))
    }

    pub fn inner(self, margin: u16) -> Self {
        let horizontal = i32::from(margin).min(self.width.max(0) / 2);
        let vertical = i32::from(margin).min(self.height.max(0) / 2);
        Self::new(
            self.x + horizontal,
            self.y + vertical,
            self.width - horizontal * 2,
            self.height - vertical * 2,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

/// Splits a region into percentage-sized rows or columns.
///
/// Reapply the layout with a new [`Rect`] after a terminal resize.
///
/// # Examples
///
/// ```
/// use termutils::{Layout, Rect, WindowManager};
///
/// let mut windows = WindowManager::new();
/// let sidebar = windows.new_window(0);
/// let content = windows.new_window(0);
/// let screen = Rect::from_terminal(24, 80);
///
/// let regions = Layout::horizontal([30, 70])
///     .gap(1)
///     .apply(screen, &mut windows, &[sidebar, content])?;
/// let content_rows = Layout::vertical([20, 80]).split(regions[1])?;
///
/// # Ok::<(), termutils::LayoutError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Layout {
    direction: LayoutDirection,
    percentages: Vec<u16>,
    margin: u16,
    gap: u16,
}

impl Layout {
    pub fn new(direction: LayoutDirection, percentages: impl IntoIterator<Item = u16>) -> Self {
        Self {
            direction,
            percentages: percentages.into_iter().collect(),
            margin: 0,
            gap: 0,
        }
    }

    pub fn horizontal(percentages: impl IntoIterator<Item = u16>) -> Self {
        Self::new(LayoutDirection::Horizontal, percentages)
    }

    pub fn vertical(percentages: impl IntoIterator<Item = u16>) -> Self {
        Self::new(LayoutDirection::Vertical, percentages)
    }

    pub fn margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn direction(&self) -> LayoutDirection {
        self.direction
    }

    pub fn percentages(&self) -> &[u16] {
        &self.percentages
    }

    pub fn split(&self, area: Rect) -> Result<Vec<Rect>, LayoutError> {
        if self.percentages.is_empty() {
            return Err(LayoutError::Empty);
        }

        let total = self.percentages.iter().map(|value| u32::from(*value)).sum();
        if total != 100 {
            return Err(LayoutError::InvalidPercentageTotal { total });
        }

        let area = area.inner(self.margin);
        let section_count = self.percentages.len();
        let gap_count = section_count.saturating_sub(1) as i32;
        let axis_length = match self.direction {
            LayoutDirection::Horizontal => area.width,
            LayoutDirection::Vertical => area.height,
        }
        .max(0);
        let gap = if gap_count == 0 {
            0
        } else {
            i32::from(self.gap).min(axis_length / gap_count)
        };
        let available = axis_length.saturating_sub(gap * gap_count);
        let mut sizes = Vec::with_capacity(section_count);
        let mut remainders = Vec::with_capacity(section_count);

        for percentage in &self.percentages {
            let product = i64::from(available) * i64::from(*percentage);
            sizes.push((product / 100) as i32);
            remainders.push((product % 100) as i32);
        }

        let assigned: i32 = sizes.iter().sum();
        let mut remainder_order: Vec<_> = (0..section_count).collect();
        remainder_order.sort_by_key(|index| (std::cmp::Reverse(remainders[*index]), *index));
        for index in remainder_order
            .into_iter()
            .take((available - assigned) as usize)
        {
            sizes[index] += 1;
        }

        let mut cursor = match self.direction {
            LayoutDirection::Horizontal => area.x,
            LayoutDirection::Vertical => area.y,
        };
        let mut regions = Vec::with_capacity(section_count);
        for size in sizes {
            let region = match self.direction {
                LayoutDirection::Horizontal => Rect::new(cursor, area.y, size, area.height),
                LayoutDirection::Vertical => Rect::new(area.x, cursor, area.width, size),
            };
            regions.push(region);
            cursor += size + gap;
        }
        Ok(regions)
    }

    /// Applies the calculated regions to the supplied windows in the same order.
    ///
    /// Validation happens before any window is changed, so an error never leaves
    /// the manager with a partially applied layout.
    pub fn apply(
        &self,
        area: Rect,
        windows: &mut WindowManager,
        window_ids: &[WindowId],
    ) -> Result<Vec<Rect>, LayoutError> {
        let regions = self.split(area)?;
        if regions.len() != window_ids.len() {
            return Err(LayoutError::WindowCount {
                sections: regions.len(),
                windows: window_ids.len(),
            });
        }
        for (index, id) in window_ids.iter().enumerate() {
            if window_ids[..index].contains(id) {
                return Err(LayoutError::DuplicateWindow(*id));
            }
            if windows.get(*id).is_none() {
                return Err(LayoutError::UnknownWindow(*id));
            }
        }
        for (id, region) in window_ids.iter().zip(regions.iter().copied()) {
            windows
                .get_mut(*id)
                .expect("window ids were validated")
                .set_rect(region);
        }
        Ok(regions)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutError {
    Empty,
    InvalidPercentageTotal { total: u32 },
    WindowCount { sections: usize, windows: usize },
    UnknownWindow(WindowId),
    DuplicateWindow(WindowId),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a layout needs at least one section"),
            Self::InvalidPercentageTotal { total } => {
                write!(formatter, "layout percentages must total 100, got {total}")
            }
            Self::WindowCount { sections, windows } => write!(
                formatter,
                "layout has {sections} sections but received {windows} windows"
            ),
            Self::UnknownWindow(id) => write!(formatter, "window {id} does not exist"),
            Self::DuplicateWindow(id) => {
                write!(formatter, "window {id} was supplied more than once")
            }
        }
    }
}

impl Error for LayoutError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_percentages_cover_the_whole_area() {
        let regions = Layout::horizontal([25, 75])
            .split(Rect::new(0, 0, 80, 24))
            .unwrap();
        assert_eq!(regions, [Rect::new(0, 0, 20, 24), Rect::new(20, 0, 60, 24)]);
    }

    #[test]
    fn rounding_is_distributed_without_losing_cells() {
        let regions = Layout::horizontal([50, 50])
            .split(Rect::new(0, 0, 11, 5))
            .unwrap();
        assert_eq!(regions, [Rect::new(0, 0, 6, 5), Rect::new(6, 0, 5, 5)]);
    }

    #[test]
    fn margin_and_gap_are_removed_before_splitting() {
        let regions = Layout::horizontal([30, 70])
            .margin(2)
            .gap(1)
            .split(Rect::new(0, 0, 100, 40))
            .unwrap();
        assert_eq!(regions, [Rect::new(2, 2, 29, 36), Rect::new(32, 2, 66, 36)]);
    }

    #[test]
    fn gaps_shrink_to_fit_tiny_areas() {
        let regions = Layout::horizontal([20, 30, 50])
            .gap(5)
            .split(Rect::new(0, 0, 2, 4))
            .unwrap();
        assert_eq!(
            regions,
            [
                Rect::new(0, 0, 0, 4),
                Rect::new(1, 0, 0, 4),
                Rect::new(2, 0, 0, 4),
            ]
        );
    }

    #[test]
    fn regions_can_be_split_again_for_nested_interfaces() {
        let rows = Layout::vertical([20, 80])
            .split(Rect::new(0, 0, 100, 50))
            .unwrap();
        let columns = Layout::horizontal([30, 70]).split(rows[1]).unwrap();
        assert_eq!(rows[0], Rect::new(0, 0, 100, 10));
        assert_eq!(columns[0], Rect::new(0, 10, 30, 40));
        assert_eq!(columns[1], Rect::new(30, 10, 70, 40));
    }

    #[test]
    fn percentages_must_total_one_hundred() {
        assert_eq!(
            Layout::horizontal([20, 20]).split(Rect::new(0, 0, 10, 10)),
            Err(LayoutError::InvalidPercentageTotal { total: 40 })
        );
    }

    #[test]
    fn apply_sets_window_rectangles() {
        let mut windows = WindowManager::new();
        let sidebar = windows.new_window(0);
        let content = windows.new_window(0);
        let regions = Layout::horizontal([30, 70])
            .apply(Rect::new(0, 0, 100, 30), &mut windows, &[sidebar, content])
            .unwrap();
        assert_eq!(windows.get(sidebar).unwrap().rect(), regions[0]);
        assert_eq!(windows.get(content).unwrap().rect(), regions[1]);
    }

    #[test]
    fn apply_validates_all_windows_before_changing_any() {
        let mut windows = WindowManager::new();
        let existing = windows.new_window(0);
        windows
            .get_mut(existing)
            .unwrap()
            .set_rect(Rect::new(1, 2, 3, 4));
        let result = Layout::horizontal([50, 50]).apply(
            Rect::new(0, 0, 100, 20),
            &mut windows,
            &[existing, 999],
        );
        assert_eq!(result, Err(LayoutError::UnknownWindow(999)));
        assert_eq!(windows.get(existing).unwrap().rect(), Rect::new(1, 2, 3, 4));
    }
}
