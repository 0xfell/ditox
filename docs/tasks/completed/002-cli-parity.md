# Task: CLI Parity

> **Status:** completed
> **Priority:** high
> **Created:** 2024-11-27
> **Completed:** 2024-11-27

## Description

Add missing CLI commands to achieve feature parity with TUI. This enables full scripting and automation capabilities.

## Requirements

- [x] `ditox get <target> [--json]` - Get full content of entry
- [x] `ditox search <query> [--json] [--limit N]` - Fuzzy search from CLI
- [x] `ditox delete <target>` - Delete entry by index or ID
- [x] `ditox pin <target>` - Toggle pin status
- [x] `ditox count` - Print entry count (for scripts)

## Implementation Notes

### Approach
1. Add new variants to `Commands` enum in `src/cli.rs`
2. Add handlers in `main.rs` match statement
3. Reuse existing `db.rs` methods where possible

### Files Modified
- `src/cli.rs` - Added command definitions (Get, Search, Delete, Pin, Count; also added `--pinned` flag to List)
- `src/main.rs` - Added command handlers using existing db methods and nucleo_matcher for fuzzy search

### Additional Improvements
- Enhanced `ditox list` with `--pinned` flag to filter pinned-only entries
- Enhanced `ditox list` output to show pin status column
- Refactored `cmd_copy` to use shared `resolve_target` helper and added `db.touch()` call to update last_used timestamp
- Fixed image copy in `cmd_copy` to actually copy images to clipboard

## Testing

```bash
# All commands tested successfully:
ditox get 1                    # Prints raw content (for piping)
ditox get 1 --json             # Prints full entry as JSON
ditox search "fn"              # Fuzzy search
ditox search "test" --json --limit 5
ditox delete 1                 # Delete by index
ditox pin 1                    # Toggle pin
ditox count                    # Print count (for scripts)
ditox list --pinned            # Show only pinned entries
```

## Work Log

- **2024-11-27**: Implemented all CLI parity commands
  - Added 5 new commands: `get`, `search`, `delete`, `pin`, `count`
  - Enhanced `list` with `--pinned` flag and pin column
  - Refactored `copy` to use shared helper and properly copy images
  - All 132 tests pass
