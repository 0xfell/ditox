# Task: Windows Multi-Format Capture

## Status

planned

## Summary

Replace the polling-only Windows clipboard path with event-driven capture and
format enumeration.

## Scope

- Use `AddClipboardFormatListener` for change events.
- Enumerate available Win32 clipboard formats and map them into canonical
  Ditox formats.
- Preserve text/image priority and deduplication behavior.
- Keep `arboard` where it remains useful for write-back.

## Acceptance Criteria

- Capturing plain text, HTML, RTF, file lists, and images works on Windows.
- Empty or locked clipboard reads degrade without crashing the watcher.
- Existing CLI and TUI tests pass on Windows.

