use ansi_to_tui::IntoText;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Margin, Rect},
    widgets::{
        Block, Cell, Clear, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Table, TableState, Widget,
    },
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default, Clone, Debug)]
/// The display interface for the table
/// of log messages produced from `Watcher`
/// building/running the project on changes.
pub struct Display {
    /// The current log messages in display
    pub logs: Arc<Mutex<Vec<String>>>,

    /// The current state of the table which
    /// contains the log messages being displayed
    pub state: TableState,

    /// The currently selected log row in the
    /// display table.
    ///
    /// NOTE: This is different from self.logs
    /// index, because we wrap long log messages
    /// into new rows in the display table.
    pub selected_visual_idx: usize,

    /// The total number of log rows in the
    /// display table.
    ///
    /// NOTE: This is different from the length
    /// of self.logs, because we wrap long log
    /// messages into new rows in the display table.
    pub n_visual_rows: usize,

    /// Do we need to jump to the most recent (last)
    /// log message row in the table.
    ///
    /// NOTE: This is set to true after each new
    /// log is added, then set back to false after
    /// jumping to the latest log row.
    pub jump_to_latest: bool,

    /// Does the display need to be redrew ?
    pub needs_redraw: Arc<AtomicBool>,
}
impl Display {
    /// Create a new `Display` instance
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            needs_redraw: Arc::new(AtomicBool::new(false)),
            selected_visual_idx: 0,
            n_visual_rows: 0,
            jump_to_latest: false,
            state,
        }
    }

    /// Add a log message to the display
    pub fn add_log(&mut self, log: String) {
        let mut logs = self.logs.lock().unwrap();
        logs.push(log);

        // Jump to the most recent log, which is
        // this log we just added to the display
        self.jump_to_latest = true;
    }

    /// Trigger a redraw of the display
    pub fn trigger_redraw(&self) {
        self.needs_redraw.store(true, Ordering::SeqCst);
    }

    /// Should the display be redrew
    pub fn should_redraw(&self) -> bool {
        self.needs_redraw.load(Ordering::SeqCst)
    }

    /// Go to the next log message in the display table.
    pub fn next_row(&mut self) {
        if self.n_visual_rows == 0 {
            return;
        }

        self.selected_visual_idx = (self.selected_visual_idx + 1) % self.n_visual_rows;
    }

    /// Go to the previous log message in the display table.
    pub fn prev_row(&mut self) {
        if self.n_visual_rows == 0 {
            return;
        }

        self.selected_visual_idx =
            (self.selected_visual_idx + self.n_visual_rows - 1) % self.n_visual_rows;
    }

    /// Render the display
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Render the display area
        let area = area.inner(Margin {
            vertical: 0,
            horizontal: 0,
        });
        Clear.render(area, buf);
        Block::new().render(area, buf);
        let available_width = area.width.saturating_sub(2) as usize;

        // Create table rows for the log messages to be
        // displayed within.
        let logs = self.logs.lock().unwrap();
        let mut visual_rows = vec![];
        let mut visual_idx_map = vec![];

        for (log_idx, raw_log) in logs.iter().enumerate() {
            let text = raw_log.into_text().unwrap_or_default();

            for line in text.lines {
                let mut current_line = ratatui::text::Line::default();
                let mut current_width = 0;

                for span in line.spans {
                    let content = span.content;
                    let style = span.style;

                    for g in content.graphemes(true) {
                        let g_width = ratatui::text::Line::from(g).width();

                        // If the log message is longer than the available width
                        // then split it up into multiple display table rows
                        if current_width + g_width > available_width && current_width > 0 {
                            visual_rows.push(Row::new(vec![Cell::from(current_line.clone())]));
                            visual_idx_map.push((log_idx, visual_rows.len()));
                            current_line = ratatui::text::Line::default();
                            current_width = 0;
                        }

                        current_line
                            .spans
                            .push(ratatui::text::Span::styled(g.to_string(), style));

                        current_width += g_width;
                    }
                }

                if !current_line.spans.is_empty() {
                    visual_rows.push(Row::new(vec![Cell::from(current_line.clone())]));
                    visual_idx_map.push((log_idx, visual_rows.len()));
                }
            }
        }

        // Update the visual rows being displayed
        self.n_visual_rows = visual_rows.len();
        if self.jump_to_latest {
            self.selected_visual_idx = self.n_visual_rows.saturating_sub(1);
            self.jump_to_latest = false;
        }

        // Create and render the display table
        let mut table_state = TableState::default();
        table_state.select(Some(self.selected_visual_idx));

        StatefulWidget::render(
            Table::new(visual_rows, [Constraint::Percentage(100)]).row_highlight_style(
                ratatui::style::Style::default().bg(ratatui::style::Color::DarkGray),
            ),
            area,
            buf,
            &mut table_state,
        );

        // Handle the display tables scroll bar
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(self.n_visual_rows)
            .position(self.selected_visual_idx);

        let scrollbar_area = Rect {
            width: area.width + 1,
            y: area.y + 3,
            height: area.height.saturating_sub(4),
            x: area.x,
        };

        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalLeft)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("▌")
            .render(scrollbar_area, buf, &mut scrollbar_state);
    }
}
