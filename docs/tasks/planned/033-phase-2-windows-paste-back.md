# Task: Windows Paste-Back

## Status

planned

## Summary

Implement Windows foreground tracking and keystroke synthesis for terminal-led
paste-back.

## Scope

- Add a Win32 foreground tracker using focused window handles and process names.
- Restore focus safely before paste.
- Add a `SendInput` synthesizer with stuck-modifier protection.
- Preserve the paste sentinel so self-recapture is skipped.

## Acceptance Criteria

- TUI copy-and-paste-back can target the previously focused Windows app.
- Clipboard-only fallback remains available when focus restore or synthesis is
  unavailable.
- Tests cover command generation, foreground filtering, and sentinel behavior.

