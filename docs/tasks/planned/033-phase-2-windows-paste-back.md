# Task: Phase 2 — Windows paste-back (foreground tracker + synthesizer)

> **Status:** planned
> **Priority:** high (blocks Windows Phase 2 parity)
> **Phase:** 2 — Paste-back UX (carry-over)
> **Created:** 2026-04-26
> **Spawned from:** task 024 sub-tasks 2.2 + 2.5
> **Estimated:** 1-2 weeks (Windows-side work)

## Why this is its own task

Task 024 closed on 2026-04-26 with the Linux paste-back MVP working
end-to-end on Hyprland (verified live: `hyprctl activewindow -j` for
foreground tracking, `hyprctl dispatch sendshortcut , ctrl+v,
address:<addr>` for synthesis). Sub-tasks 2.2 (Win32 foreground
tracker) and 2.5 (Win32 synthesizer via `SendInput`) were deferred
because validating them needs a Windows machine.

The two ship together because they share the same Windows hardening
pass — a stuck-modifier guard that walks all 256 VK codes and
releases any pressed by the user before synthesising the paste, plus
the `force_restore_window` Win32 sequence already partially
implemented in `ditox-gui/src/app.rs:148-498` that 2.2 will refactor
into a tracker.

## Description

Implement the two missing platform backends for the
`ForegroundTracker` and `Synthesizer` traits already defined in
`ditox-core`:

- `ForegroundTracker` trait: `ditox-core/src/foreground.rs:60-92`.
  Hyprland impl at `ditox-core/src/foreground/hyprctl.rs` (live).
- `Synthesizer` trait: `ditox-core/src/paste/synthesize.rs:80-100`.
  Linux impls (`Hyprctl` / `Wtype` / `Ydotool` / `Off`) live; Windows
  impl absent.

`build_default_tracker()` in `ditox-core/src/foreground.rs:240` and
`pick_chain(platform)` in `ditox-core/src/paste/synthesize.rs:280`
already cfg-branch on platform — they each return a stub
(`NoopForegroundTracker` / `OffSynthesizer`) under `#[cfg(windows)]`
today; these calls become the registration sites for the new types.

## Architecture

### 2.2 `Win32ForegroundTracker`

```rust
// ditox-core/src/foreground/win32.rs
pub struct Win32ForegroundTracker;

impl ForegroundTracker for Win32ForegroundTracker {
    fn name(&self) -> &str { "win32" }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() { return Ok(None); }

        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid as *mut _) };

        let title = read_window_title_w(hwnd)?;
        let basename = process_basename_via_query_full_image_name(pid)?;

        Ok(Some(ForegroundSnapshot {
            identifier: ForegroundId::Win32 { hwnd: hwnd as isize, pid },
            process_basename: basename,
            title,
            captured_at: SystemTime::now(),
        }))
    }

    fn restore(&self, snap: &ForegroundSnapshot) -> Result<()> {
        let ForegroundId::Win32 { hwnd, .. } = snap.identifier else {
            return Err(ForegroundError::WrongIdentifierVariant);
        };
        // Reuse the focus-recovery sequence already proven for Win+D
        // in ditox-gui/src/app.rs::force_restore_window:
        //   - SetForegroundWindow
        //   - SetWindowPos(HWND_TOPMOST) + SetWindowPos(HWND_NOTOPMOST)
        //   - AttachThreadInput trick if SetForegroundWindow fails
        force_restore_win32(hwnd as HWND)
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        // Phase 4 work: register a SetWinEventHook(EVENT_SYSTEM_FOREGROUND)
        // hook on a dedicated thread, post each transition to the channel.
        // Phase 2 stub: returns Err(()) (snapshot-on-demand is sufficient
        // for the launcher).
        Err(...)
    }

    fn shutdown(&mut self) -> Result<()> { Ok(()) }
}
```

`force_restore_win32` is essentially the existing code in
`ditox-gui/src/app.rs::force_restore_window` (Win+D recovery), pulled
into `ditox-core` and made tracker-agnostic.

`process_basename_via_query_full_image_name` opens the process via
`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, ..., pid)` and calls
`QueryFullProcessImageNameW`; the basename is derived from
`Path::file_name`. Falls back to `"unknown.exe"` on access denied
(common for system processes).

### 2.5 `Win32Synthesizer`

```rust
// ditox-core/src/paste/synthesize/win32.rs
pub struct Win32Synthesizer;

impl Synthesizer for Win32Synthesizer {
    fn name(&self) -> &str { "win32-sendinput" }
    fn is_available(&self) -> bool { true }  // always on Windows

    fn paste(
        &self,
        target: &ForegroundSnapshot,
        sequence: &KeystrokeSequence,
    ) -> Result<()> {
        // Stuck-modifier guard (Ditto's defence against the user
        // releasing Ctrl mid-summon): walk VK_LCONTROL .. VK_RMENU
        // (and the SHIFT/ALT family), check GetAsyncKeyState. If
        // any are reported pressed, send a KEYUP first.
        release_stuck_modifiers()?;

        // Translate `KeystrokeSequence` → `INPUT[]`.
        // For each chord:
        //   - press each modifier (KEYDOWN, no KEYUP_FLAG)
        //   - press the key
        //   - release the key
        //   - release the modifiers (LIFO)
        let inputs = build_inputs_for_sequence(sequence)?;
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
        if sent != inputs.len() as u32 {
            return Err(SynthError::SendInputPartial { sent, expected: inputs.len() });
        }
        Ok(())
    }
}
```

VK code translation:

- `Modifier::Ctrl`  → `VK_CONTROL`
- `Modifier::Shift` → `VK_SHIFT`
- `Modifier::Alt`   → `VK_MENU`
- `Modifier::Super` → `VK_LWIN`
- `SpecialKey::Enter` → `VK_RETURN`, etc. — mirrors the ydotool keycode
  table in `ditox-core/src/paste/synthesize.rs::ydotool_keycode_for`.
- `Key::Char(c)` → `VkKeyScanW(c)` to get the VK + shift state for the
  current keyboard layout.

### Wiring

`ditox-core/src/foreground.rs::build_default_tracker`:
```rust
#[cfg(windows)]
{
    Box::new(ForegroundFilter::new(
        Win32ForegroundTracker,
        ForegroundFilter::default_self_names(),
    ))
}
```

`ditox-core/src/paste/synthesize.rs::pick_chain`:
```rust
Platform::Windows => vec![
    Box::new(Win32Synthesizer::new()),
    Box::new(OffSynthesizer::new()),
],
```

## Acceptance criteria

- [ ] `ditox-gui` on Windows: Ctrl+Shift+V → click entry → text appears
      in Notepad/Word/Firefox via `SendInput`.
- [ ] Foreground correctness: ditox-gui's own clicks never become the
      "previous foreground" (existing `ForegroundFilter` already
      handles this; verify the basename comparison works on
      `ditox-gui.exe`).
- [ ] Stuck-modifier guard: hold Shift, summon ditox-gui via
      Ctrl+Shift+V, release Shift while the launcher is open, click
      entry — pasted text should NOT carry the stuck Shift (i.e. no
      uppercase / wrong character).
- [ ] Per-app override exercised: configure
      `[paste.keystrokes].gvim = "\"+gp"` and verify `SendInput`
      synthesises the four chords sequentially.
- [ ] No re-capture loop: pasting via ditox-gui doesn't add a new
      entry to its own history (sentinel hashes match what the
      Windows watcher computes — i.e. canonical UTF-16 → UTF-8 via
      `String::from_utf16_lossy`).

## Risks

- **Risk:** `SetForegroundWindow` legitimately fails on Windows 10/11
  when the calling process isn't itself foreground (anti-stealing
  policy). Mitigation: existing `AttachThreadInput` trick in
  `force_restore_window`; document the rare failure case in the
  tracker's `restore()` doc comment.
- **Risk:** `QueryFullProcessImageNameW` fails on system processes
  (e.g. SearchHost.exe) because of access restrictions. Mitigation:
  fall back to `"unknown.exe"` basename — the per-app keystroke
  override gracefully degrades to default `ctrl+v` for unknown
  basenames.
- **Risk:** `VkKeyScanW` returns layout-dependent VK codes; a French
  keyboard's `'a'` is on a different physical key than US. Mitigation:
  always use `VkKeyScanW(c)` (not hardcoded VK_A), call
  `MapVirtualKeyW` to translate to scancode if needed, set
  `KEYEVENTF_SCANCODE` flag for layout-independence.

## Implementation Notes

`ditox-gui/src/app.rs::force_restore_window` is the proven Win32
focus-recovery sequence — should be lifted into
`ditox-core/src/foreground/win32.rs` (since the tracker now needs it)
and the GUI can call it via the tracker's `restore()` method.

Tests live next to the impl files. Win32 calls are obviously hard to
mock; tests focus on:
- VK translation tables (`KeystrokeSequence` → `Vec<INPUT>` shape).
- `force_restore_win32` argv shape (extract pure helpers).
- Basename normalisation (`C:\Program Files\Foo\bar.exe` → `bar.exe`,
  case-folded).

A small Windows VM CI job would let us test end-to-end via WSH
automation (`SendKeys` to a Notepad instance, assert clipboard
state) — out of scope for the initial landing, mark as TODO.
