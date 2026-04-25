use ditox_core::config::ThemeConfig;
use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub border: Color,
    pub muted: Color,
    pub accent: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            selected_bg: Color::Rgb(59, 66, 97),
            selected_fg: Color::Rgb(192, 202, 245),
            border: Color::Rgb(86, 95, 137),
            muted: Color::Rgb(86, 95, 137),
            accent: Color::Rgb(122, 162, 247),
        }
    }
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        Self {
            bg: Color::Reset,
            fg: parse_color(&config.text).unwrap_or(Color::White),
            selected_bg: parse_color(&config.selected).unwrap_or(Color::Rgb(59, 66, 97)),
            selected_fg: Color::Rgb(192, 202, 245),
            border: parse_color(&config.border).unwrap_or(Color::Rgb(86, 95, 137)),
            muted: parse_color(&config.muted).unwrap_or(Color::Rgb(86, 95, 137)),
            accent: Color::Rgb(122, 162, 247),
        }
    }

    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.selected_fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for highlighted search matches
    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Style for highlighted search matches in selected row
    pub fn highlight_selected(&self) -> Style {
        Style::default()
            .fg(Color::Yellow)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }
}

fn parse_color(s: &str) -> Option<Color> {
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}
