use serde::Deserialize;
use ratatui::widgets::BorderType;

#[derive(Debug, Clone, Copy)]
pub struct Caps {
    pub color_depth: u16, // 0 (never), 16, 256, 24
    pub unicode: bool,
    pub no_color: bool,
}

pub fn detect_caps() -> Caps {
    // Basic heuristics; keep fast and conservative
    let no_color = std::env::var_os("NO_COLOR").is_some()
        || matches!(
            std::env::var("DITOX_TUI_COLOR").ok().as_deref(),
            Some("never")
        );
    let color_depth = if no_color {
        0
    } else if matches!(
        std::env::var("DITOX_TUI_COLOR").ok().as_deref(),
        Some("always")
    ) {
        // Assume truecolor when forced
        24
    } else if std::env::var("COLORTERM")
        .map(|v| v.contains("truecolor"))
        .unwrap_or(false)
    {
        24
    } else if std::env::var("TERM")
        .map(|v| v.contains("256"))
        .unwrap_or(false)
    {
        256
    } else {
        16
    };
    let unicode = std::env::var("DITOX_TUI_ASCII").ok().as_deref() != Some("1");
    Caps {
        color_depth,
        unicode,
        no_color,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TuiTheme {
    pub highlight_fg: ratatui::style::Color,
    pub highlight_bg: ratatui::style::Color,
    pub border_fg: ratatui::style::Color,
    pub help_fg: ratatui::style::Color,
    pub title_fg: ratatui::style::Color,
    pub muted_fg: ratatui::style::Color,
    pub status_fg: ratatui::style::Color,
    pub status_bg: ratatui::style::Color,
    pub badge_fg: ratatui::style::Color,
    pub badge_bg: ratatui::style::Color,
    pub search_match_fg: ratatui::style::Color,
    pub search_match_bg: ratatui::style::Color,
    pub border_type: Option<BorderType>,
}

fn default_highlight_fg() -> ratatui::style::Color {
    ratatui::style::Color::Black
}
fn default_highlight_bg() -> ratatui::style::Color {
    ratatui::style::Color::Cyan
}
fn default_border_fg() -> ratatui::style::Color {
    ratatui::style::Color::Gray
}
fn default_help_fg() -> ratatui::style::Color {
    ratatui::style::Color::Yellow
}
fn default_title_fg() -> ratatui::style::Color { ratatui::style::Color::Gray }
fn default_muted_fg() -> ratatui::style::Color { ratatui::style::Color::Gray }
fn default_status_fg() -> ratatui::style::Color { ratatui::style::Color::White }
fn default_status_bg() -> ratatui::style::Color { ratatui::style::Color::Reset }
fn default_badge_fg() -> ratatui::style::Color { ratatui::style::Color::Black }
fn default_badge_bg() -> ratatui::style::Color { ratatui::style::Color::Yellow }
fn default_search_match_fg() -> ratatui::style::Color { ratatui::style::Color::Black }
fn default_search_match_bg() -> ratatui::style::Color { ratatui::style::Color::Yellow }

#[derive(Deserialize)]
struct RawTheme {
    highlight_fg: Option<String>,
    highlight_bg: Option<String>,
    border_fg: Option<String>,
    help_fg: Option<String>,
    title_fg: Option<String>,
    muted_fg: Option<String>,
    status_fg: Option<String>,
    status_bg: Option<String>,
    badge_fg: Option<String>,
    badge_bg: Option<String>,
    search_match_fg: Option<String>,
    search_match_bg: Option<String>,
    border_style: Option<String>,
}

pub fn load_tui_theme() -> TuiTheme {
    // Precedence: CLI env DITOX_TUI_THEME -> settings.tui.theme -> default built-in
    let theme_hint = std::env::var("DITOX_TUI_THEME")
        .ok()
        .or_else(|| crate::config::load_settings().tui.and_then(|t| t.theme));
    let caps = detect_caps();
    let from = theme_hint.as_deref().and_then(load_theme_from_hint);
    let raw = from.unwrap_or_else(builtin_theme_dark);
    // Map to TuiTheme, honoring no-color
    let map = |opt: Option<String>, def: fn() -> ratatui::style::Color| {
        if caps.no_color {
            ratatui::style::Color::Reset
        } else {
            opt.and_then(parse_color).unwrap_or_else(def)
        }
    };
    TuiTheme {
        highlight_fg: map(raw.highlight_fg.clone(), default_highlight_fg),
        highlight_bg: map(raw.highlight_bg.clone(), default_highlight_bg),
        border_fg: map(raw.border_fg.clone(), default_border_fg),
        help_fg: map(raw.help_fg.clone(), default_help_fg),
        title_fg: map(raw.title_fg.clone(), default_title_fg),
        muted_fg: map(raw.muted_fg.clone(), default_muted_fg),
        status_fg: map(raw.status_fg.clone(), default_status_fg),
        status_bg: map(raw.status_bg.clone(), default_status_bg),
        badge_fg: map(raw.badge_fg.clone(), default_badge_fg),
        badge_bg: map(raw.badge_bg.clone(), default_badge_bg),
        search_match_fg: map(raw.search_match_fg.clone(), default_search_match_fg),
        search_match_bg: map(raw.search_match_bg.clone(), default_search_match_bg),
        border_type: parse_border_type(raw.border_style.as_deref()),
    }
}

fn load_theme_from_hint(hint: &str) -> Option<RawTheme> {
    // If absolute or relative path exists, read; otherwise try built-ins and config/themes/<name>.toml
    let p = std::path::Path::new(hint);
    if p.exists() {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
    } else {
        match hint.to_ascii_lowercase().as_str() {
            "dark" => Some(builtin_theme_dark()),
            "high-contrast" | "high_contrast" | "hc" => Some(builtin_theme_high_contrast()),
            name => {
                let path = crate::config::config_dir()
                    .join("themes")
                    .join(format!("{}.toml", name));
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|s| toml::from_str::<RawTheme>(&s).ok())
            }
        }
    }
}

fn builtin_theme_dark() -> RawTheme {
    RawTheme {
        highlight_fg: Some("black".into()),
        highlight_bg: Some("#1f6feb".into()),
        border_fg: Some("gray".into()),
        help_fg: Some("yellow".into()),
        title_fg: Some("gray".into()),
        muted_fg: Some("gray".into()),
        status_fg: Some("white".into()),
        status_bg: Some("#30363d".into()),
        badge_fg: Some("black".into()),
        badge_bg: Some("#ffd866".into()),
        search_match_fg: Some("black".into()),
        search_match_bg: Some("yellow".into()),
        border_style: Some("plain".into()),
    }
}

fn builtin_theme_high_contrast() -> RawTheme {
    RawTheme {
        highlight_fg: Some("black".into()),
        highlight_bg: Some("white".into()),
        border_fg: Some("white".into()),
        help_fg: Some("white".into()),
        title_fg: Some("white".into()),
        muted_fg: Some("white".into()),
        status_fg: Some("black".into()),
        status_bg: Some("white".into()),
        badge_fg: Some("black".into()),
        badge_bg: Some("white".into()),
        search_match_fg: Some("black".into()),
        search_match_bg: Some("white".into()),
        border_style: Some("plain".into()),
    }
}

pub fn available_themes() -> Vec<String> {
    let mut v = vec!["dark".to_string(), "high-contrast".to_string()];
    let user = crate::config::config_dir().join("themes");
    if let Ok(rd) = std::fs::read_dir(&user) {
        for e in rd.flatten() {
            if let Some(ext) = e.path().extension() {
                if ext == "toml" {
                    if let Some(stem) = e.path().file_stem().and_then(|s| s.to_str()) {
                        v.push(stem.to_string());
                    }
                }
            }
        }
    }
    v.sort();
    v.dedup();
    v
}

fn parse_border_type(s: Option<&str>) -> Option<BorderType> {
    match s.map(|v| v.trim().to_ascii_lowercase()) {
        None => Some(BorderType::Plain),
        Some(ref k) if k == "plain" => Some(BorderType::Plain),
        Some(ref k) if k == "rounded" => Some(BorderType::Rounded),
        Some(ref k) if k == "double" => Some(BorderType::Double),
        Some(ref k) if k == "none" => None,
        Some(_) => Some(BorderType::Plain),
    }
}

// ======= Glyphs, Layouts, and ASCII preview =======

#[derive(Clone, Debug)]
pub struct Glyphs {
    pub favorite_on: String,
    pub favorite_off: String,
    pub selected: String,
    pub unselected: String,
    pub kind_text: String,
    pub kind_image: String,
    pub enter_label: String,
}

#[derive(Deserialize)]
struct RawGlyphs {
    favorite_on: Option<String>,
    favorite_off: Option<String>,
    selected: Option<String>,
    unselected: Option<String>,
    kind_text: Option<String>,
    kind_image: Option<String>,
    enter_label: Option<String>,
}

pub fn load_glyphs() -> Glyphs {
    let caps = detect_caps();
    if !caps.unicode {
        return builtin_glyphs_ascii();
    }
    let hint = std::env::var("DITOX_TUI_GLYPHS")
        .ok()
        .or_else(|| crate::config::load_settings().tui.and_then(|t| t.glyphs));
    let raw = hint
        .as_deref()
        .and_then(load_glyphs_from_hint)
        .unwrap_or_else(builtin_glyphs_unicode_raw);
    Glyphs {
        favorite_on: raw.favorite_on.unwrap_or_else(|| "★".into()),
        favorite_off: raw.favorite_off.unwrap_or_else(|| " ".into()),
        selected: raw.selected.unwrap_or_else(|| "•".into()),
        unselected: raw.unselected.unwrap_or_else(|| " ".into()),
        kind_text: raw.kind_text.unwrap_or_else(|| "".into()),
        kind_image: raw.kind_image.unwrap_or_else(|| "".into()),
        enter_label: raw.enter_label.unwrap_or_else(|| "⏎".into()),
    }
}

fn load_glyphs_from_hint(hint: &str) -> Option<RawGlyphs> {
    let p = std::path::Path::new(hint);
    if p.exists() {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
    } else {
        match hint.to_ascii_lowercase().as_str() {
            "ascii" => Some(builtin_glyphs_ascii_raw()),
            "unicode" | "nerdfont" | "nf" => Some(builtin_glyphs_unicode_raw()),
            name => {
                let path = crate::config::config_dir()
                    .join("glyphs")
                    .join(format!("{}.toml", name));
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|s| toml::from_str::<RawGlyphs>(&s).ok())
            }
        }
    }
}

fn builtin_glyphs_ascii_raw() -> RawGlyphs {
    RawGlyphs {
        favorite_on: Some("*".into()),
        favorite_off: Some(" ".into()),
        selected: Some(">".into()),
        unselected: Some(" ".into()),
        kind_text: Some("T".into()),
        kind_image: Some("IMG".into()),
        enter_label: Some("Enter".into()),
    }
}

fn builtin_glyphs_unicode_raw() -> RawGlyphs {
    RawGlyphs {
        favorite_on: Some("★".into()),
        favorite_off: Some(" ".into()),
        selected: Some("•".into()),
        unselected: Some(" ".into()),
        kind_text: Some("".into()),
        kind_image: Some("".into()),
        enter_label: Some("⏎".into()),
    }
}

fn builtin_glyphs_ascii() -> Glyphs {
    let raw = builtin_glyphs_ascii_raw();
    Glyphs {
        favorite_on: raw.favorite_on.unwrap(),
        favorite_off: raw.favorite_off.unwrap(),
        selected: raw.selected.unwrap(),
        unselected: raw.unselected.unwrap(),
        kind_text: raw.kind_text.unwrap(),
        kind_image: raw.kind_image.unwrap(),
        enter_label: raw.enter_label.unwrap(),
    }
}

pub fn available_glyph_packs() -> Vec<String> {
    let mut v = vec!["ascii".to_string(), "unicode".to_string()];
    let user = crate::config::config_dir().join("glyphs");
    if let Ok(rd) = std::fs::read_dir(&user) {
        for e in rd.flatten() {
            if let Some(ext) = e.path().extension() {
                if ext == "toml" {
                    if let Some(stem) = e.path().file_stem().and_then(|s| s.to_str()) {
                        v.push(stem.to_string());
                    }
                }
            }
        }
    }
    v.sort();
    v.dedup();
    v
}

#[derive(Clone, Debug)]
pub struct LayoutPack {
    pub help_footer: bool,
    pub search_bar_bottom: bool,
    pub list_line_height: u8,
    pub item_template: Option<String>,
    pub meta_template: Option<String>,
    pub list_title_template: Option<String>,
    pub footer_template: Option<String>,
    pub help_template: Option<String>,
    pub border_list: Option<BorderType>,
    pub border_search: Option<BorderType>,
    pub border_footer: Option<BorderType>,
    pub border_help: Option<BorderType>,
    pub show_list_pager: Option<bool>,
    pub pager_template: Option<String>,
}

#[derive(Deserialize)]
struct RawLayout {
    help: Option<String>,
    search_bar_position: Option<String>,
    list_line_height: Option<u8>,
    item_template: Option<String>,
    meta_template: Option<String>,
    list_title_template: Option<String>,
    footer_template: Option<String>,
    help_template: Option<String>,
    border_list: Option<String>,
    border_search: Option<String>,
    border_footer: Option<String>,
    border_help: Option<String>,
    show_list_pager: Option<bool>,
    pager_template: Option<String>,
}

pub fn load_layout() -> LayoutPack {
    let hint = std::env::var("DITOX_TUI_LAYOUT")
        .ok()
        .or_else(|| crate::config::load_settings().tui.and_then(|t| t.layout));
    let raw = hint
        .as_deref()
        .and_then(load_layout_from_hint)
        .unwrap_or(RawLayout { help: None, search_bar_position: None, list_line_height: None, item_template: None, meta_template: None, list_title_template: None, footer_template: None, help_template: None, border_list: None, border_search: None, border_footer: None, border_help: None, show_list_pager: None, pager_template: None });
    let hf = raw
        .help
        .as_deref()
        .unwrap_or("visible")
        .eq_ignore_ascii_case("visible");
    let sb = raw
        .search_bar_position
        .as_deref()
        .unwrap_or("top")
        .eq_ignore_ascii_case("bottom");
    let llh = raw.list_line_height.unwrap_or(2).clamp(1, 2);
    LayoutPack {
        help_footer: hf,
        search_bar_bottom: sb,
        list_line_height: llh,
        item_template: raw.item_template,
        meta_template: raw.meta_template,
        list_title_template: raw.list_title_template,
        footer_template: raw.footer_template,
        help_template: raw.help_template,
        border_list: parse_border_type(raw.border_list.as_deref()),
        border_search: parse_border_type(raw.border_search.as_deref()),
        border_footer: parse_border_type(raw.border_footer.as_deref()),
        border_help: parse_border_type(raw.border_help.as_deref()),
        show_list_pager: raw.show_list_pager,
        pager_template: raw.pager_template,
    }
}

fn load_layout_from_hint(hint: &str) -> Option<RawLayout> {
    let p = std::path::Path::new(hint);
    if p.exists() {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
    } else {
        match hint.to_ascii_lowercase().as_str() {
            "default" => Some(RawLayout { help: None, search_bar_position: None, list_line_height: None, item_template: None, meta_template: None, list_title_template: None, footer_template: None, help_template: None, border_list: None, border_search: None, border_footer: None, border_help: None, show_list_pager: None, pager_template: None }),
            name => {
                let path = crate::config::config_dir()
                    .join("layouts")
                    .join(format!("{}.toml", name));
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|s| toml::from_str::<RawLayout>(&s).ok())
            }
        }
    }
}

pub fn available_layouts() -> Vec<String> {
    let mut v = vec!["default".to_string()];
    let user = crate::config::config_dir().join("layouts");
    if let Ok(rd) = std::fs::read_dir(&user) {
        for e in rd.flatten() {
            if let Some(ext) = e.path().extension() {
                if ext == "toml" {
                    if let Some(stem) = e.path().file_stem().and_then(|s| s.to_str()) {
                        v.push(stem.to_string());
                    }
                }
            }
        }
    }
    v.sort();
    v.dedup();
    v
}

pub fn print_ascii_preview(theme: &str) {
    std::env::set_var("DITOX_TUI_ASCII", "1");
    std::env::set_var("DITOX_TUI_THEME", theme);
    let _th = load_tui_theme();
    let gp = load_glyphs();
    // No ANSI; just ASCII so previews are portable
    println!("Ditox Picker — Preview (theme: {})", theme);
    println!("----------------------------------------");
    println!(
        "1  [{}] {} First item                           ",
        gp.unselected, gp.kind_text
    );
    println!(
        "2  [{}] {} Second item (favorite)               ",
        gp.favorite_on, gp.kind_text
    );
    println!(
        "3  [{}] {} Third item                            ",
        gp.unselected, gp.kind_image
    );
    println!("----------------------------------------");
    println!(
        "Shortcuts: {} copy | x delete | p fav/unfav | Tab favorites | ? more",
        gp.enter_label
    );
}

pub fn parse_color(s: String) -> Option<ratatui::style::Color> {
    parse_color_str(&s)
}

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
            if let Some(hex) = k.strip_prefix('#') {
                return parse_hex(hex);
            }
            if let Some(rest) = k.strip_prefix("rgb(") {
                return parse_rgb_tuple(rest.to_string());
            }
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
    } else {
        None
    }
}

fn parse_rgb_tuple(rest: String) -> Option<ratatui::style::Color> {
    let t = rest.strip_suffix(')')?;
    let parts: Vec<_> = t.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].parse::<u8>().ok()?;
    let g = parts[1].parse::<u8>().ok()?;
    let b = parts[2].parse::<u8>().ok()?;
    Some(ratatui::style::Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_named_hex_rgb_colors() {
        assert!(matches!(
            parse_color("red".into()),
            Some(ratatui::style::Color::Red)
        ));
        assert!(matches!(
            parse_color("#112233".into()),
            Some(ratatui::style::Color::Rgb(0x11, 0x22, 0x33))
        ));
        assert!(matches!(
            parse_color("rgb(1, 2, 3)".into()),
            Some(ratatui::style::Color::Rgb(1, 2, 3))
        ));
        assert!(parse_color("rgb(1,2)".into()).is_none());
        assert!(parse_color("#abcd".into()).is_none());
    }
}
