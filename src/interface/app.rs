use crate::{
    error::Error,
    interface::{Display, THEME},
    VERSION,
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Debug)]
/// The Terminal User Interface (TUI) Application.
pub struct App {
    /// The display component of the `App`.
    display: Arc<Mutex<Display>>,

    /// Is the application running ?
    running: Arc<AtomicBool>,

    /// The currently running build process.
    current_build_process: Arc<Mutex<Option<std::process::Child>>>,
}
impl App {
    /// Create a new instance of `App`.
    pub fn new(
        display: Arc<Mutex<Display>>,
        build_process: Arc<Mutex<Option<std::process::Child>>>,
    ) -> Self {
        Self {
            display,
            current_build_process: build_process,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Run the application, which draws the interface.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Error> {
        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    /// Render the interface frames into display.
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    /// Handle the queue of user events triggered in the `App`.
    fn handle_events(&mut self) -> Result<(), Error> {
        if crossterm::event::poll(Duration::from_millis(100))? {
            // Read and handle the next event in queue
            let first_event = crossterm::event::read()?;
            self.handle_event(first_event)?;

            // Drain all remaining events in queue without timeout
            while crossterm::event::poll(Duration::from_millis(0))? {
                let ev = crossterm::event::read()?;
                self.handle_event(ev)?;
            }
        }

        Ok(())
    }

    /// Handle a specifc user event triggered in the `App`.
    fn handle_event(&mut self, event: crossterm::event::Event) -> Result<(), Error> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                // Handle close app event
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.running.store(false, Ordering::SeqCst);
                    self.shutdown();
                }
                // Handle scroll up event
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Ok(mut display) = self.display.lock() {
                        display.prev_row()
                    }
                }
                // Handle scroll down event
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Ok(mut display) = self.display.lock() {
                        display.next_row()
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    /// Render the application title bar within the display interface.
    fn render_title_bar(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::horizontal([Constraint::Min(0)]);
        let [title_area] = layout.areas(area);

        let title_span = Span::styled(format!("Unlimited Ammo {VERSION}"), THEME.app_title);

        Paragraph::new(title_span)
            .alignment(ratatui::layout::Alignment::Center)
            .render(title_area, buf);
    }

    /// Render the main display, which is a table of log messages captured
    /// by the build/run processes triggered on file changes by `Watcher`.
    fn render_selected_tab(&self, area: Rect, buf: &mut Buffer) {
        // TODO: Handle this unwrap
        let mut display = self.display.lock().unwrap();
        display.render(area, buf);
    }

    /// Render the command bar within the display interface.
    fn render_command_bar(area: Rect, buf: &mut Buffer) {
        let keys = [("K/↑", "Up"), ("J/↓", "Down"), ("Q/Esc", "Quit")];

        let spans: Vec<Span<'_>> = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), THEME.key_binding.key);
                let desc = Span::styled(format!(" {desc} "), THEME.key_binding.description);
                [key, desc]
            })
            .collect();

        // TODO: Stylize using theme.rs
        Line::from(spans)
            .centered()
            .style((Color::Indexed(236), Color::Indexed(232)))
            .render(area, buf);
    }

    /// Handle the cleaning up of the application before shutdown.
    fn shutdown(&mut self) {
        if let Ok(mut build_process) = self.current_build_process.lock() {
            if let Some(process) = build_process.as_mut() {
                // TODO: Handle logging of failing to kill build process
                let _ = process.kill();
            }
        }
    }
}
/// Implement the ratatui::Widget trait for a reference to `App`
impl Widget for &App {
    /// Render the application
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [title_bar, tab, bottom_bar] = vertical.areas(area);

        Block::new().style(THEME.root).render(area, buf);
        self.render_title_bar(title_bar, buf);
        self.render_selected_tab(tab, buf);
        App::render_command_bar(bottom_bar, buf);
    }
}
