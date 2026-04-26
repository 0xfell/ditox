//! Path A1 spike: ditox-style launcher via `iced_layershell`.
//!
//! Prototype goals (from `docs/tasks/planned/022-foundation-layer-shell-spike.md`):
//! - Render an iced 0.14 column with 5 hard-coded entries.
//! - Anchor to bottom-left of the active output.
//! - Receive keyboard events.
//! - Close on `Esc`.
//! - Run on Hyprland and Sway.
//!
//! How to run:
//!     cd spike/a1-iced-layershell
//!     cargo run --release
//!
//! Expected behaviour on a wlr compositor (Hyprland / Sway / River):
//! - 420x520 panel appears bottom-left of the active monitor
//! - Esc quits the process
//! - Up/Down move selection (visual highlight)
//! - Enter prints the selected entry's text and quits
//!
//! Compositor compatibility:
//! - Hyprland: yes (wlr-layer-shell native)
//! - Sway: yes
//! - River / Wayfire: yes
//! - KDE Plasma 5.27+: yes (via Plasma's wlr-layer-shell impl)
//! - GNOME / Mutter: NO — Mutter does not implement wlr-layer-shell.
//!   This is a documented limitation of Path A1 and Path A2 alike;
//!   GNOME falls back to xdg_toplevel.

use iced::event::{self, Event};
use iced::widget::{column, container, row, text};
use iced::{Color, Element, Length, Task};
use iced_layershell::build_pattern::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::Settings;
use iced_layershell::to_layer_message;

/// Five hard-coded entries to mimic ditox's history list. Matches
/// `docs/tasks/planned/022-foundation-layer-shell-spike.md`'s spec
/// of "5 hard-coded entries" so the prototype's purpose is visually
/// obvious.
const ENTRIES: &[&str] = &[
    "1. Hello, world!",
    "2. https://github.com/0xfell/ditox",
    "3. cargo build --workspace",
    "4. The quick brown fox jumps over the lazy dog",
    "5. ditox: cross-platform clipboard manager",
];

#[derive(Default)]
struct App {
    selected: usize,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    KeyPressed(iced::keyboard::Key),
    Selected(usize),
}

fn namespace() -> String {
    String::from("ditox-spike")
}

fn subscription(_app: &App) -> iced::Subscription<Message> {
    event::listen_with(|evt, _status, _window| match evt {
        Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
            Some(Message::KeyPressed(key))
        }
        _ => None,
    })
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::KeyPressed(key) => match key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                println!("Esc pressed → exiting");
                std::process::exit(0);
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                if app.selected + 1 < ENTRIES.len() {
                    app.selected += 1;
                }
                Task::none()
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                if app.selected > 0 {
                    app.selected -= 1;
                }
                Task::none()
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                println!("Enter → would copy: {}", ENTRIES[app.selected]);
                std::process::exit(0);
            }
            _ => Task::none(),
        },
        Message::Selected(i) => {
            app.selected = i;
            Task::none()
        }
        // `#[to_layer_message]` adds extra variants we don't use here.
        _ => Task::none(),
    }
}

fn view(app: &App) -> Element<'_, Message> {
    let header = container(text("ditox · spike A1 (iced_layershell)").size(14))
        .padding(8)
        .width(Length::Fill);

    let mut list = column![].spacing(2).padding(8);
    for (i, entry) in ENTRIES.iter().enumerate() {
        let row_text = text(*entry).size(13);
        let selected = i == app.selected;
        let row_container = container(row![row_text].padding(6))
            .width(Length::Fill)
            .style(move |_theme| {
                let bg = if selected {
                    Color::from_rgba(0.2, 0.4, 0.8, 0.4)
                } else {
                    Color::TRANSPARENT
                };
                container::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: Some(Color::WHITE),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });
        list = list.push(row_container);
    }

    let footer = container(text("Esc=quit  ↑↓=move  Enter=copy").size(11))
        .padding(8)
        .width(Length::Fill);

    container(column![header, list, footer].spacing(4))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.1, 0.1, 0.13, 0.95,
            ))),
            text_color: Some(Color::WHITE),
            ..Default::default()
        })
        .into()
}

fn style(_app: &App, theme: &iced::Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: theme.palette().text,
    }
}

pub fn main() -> Result<(), iced_layershell::Error> {
    application(App::default, namespace, update, view)
        .style(style)
        .subscription(subscription)
        .settings(Settings {
            layer_settings: LayerShellSettings {
                size: Some((420, 520)),
                anchor: Anchor::Bottom | Anchor::Left,
                start_mode: StartMode::Active,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                margin: (0, 0, 24, 24),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
