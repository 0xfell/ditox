# Task: OpenTUI Terminal Frontend

## Status

planned

## Summary

Build the next-generation Ditox terminal UI with Bun, TypeScript, SolidJS, and
OpenTUI. The OpenTUI frontend should talk to the Rust native core through a
small local API or JSON command bridge rather than reading SQLite directly.

## Goals

- Preserve the current TUI workflows: browse, search, preview, copy, paste-back,
  delete, favorite, notes, collections, tags, multi-select, and sync status.
- Improve interaction quality with OpenTUI layout, keymap, focus, and component
  primitives.
- Keep native OS behavior in Rust: clipboard capture/write, paste synthesis,
  image storage, config, and sync.

## Acceptance Criteria

- A Bun/TypeScript/OpenTUI app can list entries, search, preview text, and copy
  an entry through the Rust side.
- The bridge contract is documented and covered by fixture or contract tests.
- The existing Rust TUI remains available until the OpenTUI frontend reaches
  feature parity.

