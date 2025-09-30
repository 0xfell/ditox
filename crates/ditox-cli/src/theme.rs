use serde::Deserialize;

#[derive(Clone, Copy, Debug)]
pub struct TuiTheme {
    pub highlight_fg: ratatui::style::Color,
    pub highlight_bg: ratatui::style::Color,
    pub border_fg: ratatui::style::Color,
    pub help_fg: ratatui::style::Color,
}

fn default_highlight_fg() -> ratatui::style::Color { ratatui::style::Color::Black }
fn default_highlight_bg() -> ratatui::style::Color { ratatui::style::Color::Cyan }
fn default_border_fg() -> ratatui::style::Color { ratatui::style::Color::Gray }
fn default_help_fg() -> ratatui::style::Color { ratatui::style::Color::Yellow }

#[derive(Deserialize)]
struct RawTheme {
    highlight_fg: Option<String>,
    highlight_bg: Option<String>,
    border_fg: Option<String>,
    help_fg: Option<String>,
}

pub fn load_tui_theme() -> TuiTheme {
    let path = crate::config::config_dir().join("tui_theme.toml");
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(raw) = toml::from_str::<RawTheme>(&s) {
            return TuiTheme {
                highlight_fg: raw
                    .highlight_fg
                    .and_then(parse_color)
                    .unwrap_or_else(default_highlight_fg),
                highlight_bg: raw
                    .highlight_bg
                    .and_then(parse_color)
                    .unwrap_or_else(default_highlight_bg),
                border_fg: raw
                    .border_fg
                    .and_then(parse_color)
                    .unwrap_or_else(default_border_fg),
                help_fg: raw
                    .help_fg
                    .and_then(parse_color)
                    .unwrap_or_else(default_help_fg),
            };
        }
    }
    TuiTheme {
        highlight_fg: default_highlight_fg(),
        highlight_bg: default_highlight_bg(),
        border_fg: default_border_fg(),
        help_fg: default_help_fg(),
    }
}

pub fn parse_color(s: String) -> Option<ratatui::style::Color> { parse_color_str(&s) }

fn parse_color_str(s: &str) -> Option<ratatui::style::Color> {
    use ratatui::style::Color;
    let k = s.trim().to_ascii_lowercase();
    match k.as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        _ => {
            if let Some(hex) = k.strip_prefix('#') { return parse_hex(hex); }
            if let Some(rest) = k.strip_prefix("rgb(") { return parse_rgb_tuple(rest.to_string()); }
            None
        }
    }
}

fn parse_hex(hex: &str) -> Option<ratatui::style::Color> {
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(ratatui::style::Color::Rgb(r, g, b))
    } else { None }
}

fn parse_rgb_tuple(rest: String) -> Option<ratatui::style::Color> {
    let t = rest.strip_suffix(')')?;
    let parts: Vec<_> = t.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 { return None; }
    let r = parts[0].parse::<u8>().ok()?;
    let g = parts[1].parse::<u8>().ok()?;
    let b = parts[2].parse::<u8>().ok()?;
    Some(ratatui::style::Color::Rgb(r, g, b))
}
