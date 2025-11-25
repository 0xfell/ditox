# Iced 0.14 Migration - Complete

## Summary

Successfully migrated ditox-gui from iced 0.13 to 0.14. All other dependency updates (rusqlite 0.32→0.37, directories 5→6, sysinfo 0.33→0.37, crossterm 0.28→0.29, ratatui-image 4→8) were API-compatible and required no code changes.

## Changes Made

### 1. Style Struct `snap` Fields
Added `snap: false` to all `container::Style` and `button::Style` structs (18 instances total).

### 2. Keyboard Subscription API
```rust
// Old (0.13)
iced::keyboard::on_key_press(|key, modifiers| { ... })

// New (0.14)
event::listen_with(|event, _status, _window| {
    if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
        // handle key
    }
})
```

### 3. Focus API
```rust
// Old (0.13)
iced::widget::text_input::focus(id)

// New (0.14)
iced::widget::operation::focus(id)
```

### 4. event::listen_with Signature
```rust
// Old (0.13): 2 arguments
|event, status| { ... }

// New (0.14): 3 arguments
|event, status, window_id| { ... }
```

### 5. Application Boot Closure
The `iced::application()` boot function now requires `Fn` instead of `FnOnce`. Solved by storing config in `OnceLock<Config>` and opening database fresh in boot function.

### 6. Space Widget API
```rust
// Old (0.13)
Space::with_width(Length::Fill)
Space::with_height(16)

// New (0.14)
Space::new().width(Length::Fill)
Space::new().height(16)
```

### 7. scrollable::Style auto_scroll
```rust
// Old (0.13)
auto_scroll: None

// New (0.14)
auto_scroll: scrollable::AutoScroll {
    background: Background::Color(Color::TRANSPARENT),
    border: Border::default(),
    shadow: Shadow::default(),
    icon: colors::TEXT_MUTED,
}
```

### 8. Stream Channel Type Annotations
Added explicit `Sender<Message>` type annotations to `iced::stream::channel` closures:
```rust
|mut sender: iced::futures::channel::mpsc::Sender<Message>| async move { ... }
```

## Dependency Updates (API-compatible, no changes needed)

| Package | Old | New |
|---------|-----|-----|
| rusqlite | 0.32 | 0.37.0 |
| toml | 0.8 | 0.9.8 |
| directories | 5 | 6.0.0 |
| wl-clipboard-rs | 0.8 | 0.9.2 |
| sysinfo | 0.33 | 0.37.2 |
| crossterm | 0.28 | 0.29.0 |
| ratatui-image | 4 | 8.0.2 |
| tray-icon | 0.19 | 0.21.2 |
| global-hotkey | 0.6 | 0.7.0 |
