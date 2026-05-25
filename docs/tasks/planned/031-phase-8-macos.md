# Task: macOS Port

## Status

planned

## Summary

Add macOS support for the TUI/CLI and native clipboard behavior.

## Scope

- NSPasteboard capture and write support for text, images, rich text, and file URLs.
- Accessibility-permission paste-back path.
- macOS config/data path handling.
- Homebrew packaging.
- Terminal-first install and usage documentation.

## Acceptance Criteria

- `ditox watch` captures common macOS clipboard formats.
- `ditox copy` writes text and image entries back to the clipboard.
- TUI browsing/search/copy workflows work in common terminals.
- macOS-specific permission errors are surfaced clearly.

