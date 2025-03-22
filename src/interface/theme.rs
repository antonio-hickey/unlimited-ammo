use ratatui::style::{Color, Modifier, Style};

pub struct KeyBinding {
    pub key: Style,
    pub description: Style,
}

pub struct Theme {
    pub root: Style,
    pub app_title: Style,
    pub key_binding: KeyBinding,
}

pub const THEME: Theme = Theme {
    root: Style::new().bg(BLACK),
    app_title: Style::new()
        .fg(GREEN)
        .bg(BLACK)
        .add_modifier(Modifier::BOLD),
    key_binding: KeyBinding {
        key: Style::new().fg(BLACK).bg(DARK_GRAY),
        description: Style::new().fg(DARK_GRAY).bg(BLACK),
    },
};

const GREEN: Color = Color::Green;
const BLACK: Color = Color::Rgb(8, 8, 8);
const DARK_GRAY: Color = Color::Rgb(68, 68, 68);
