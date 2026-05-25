const std = @import("std");
const clipboard = @import("clipboard.zig");
const config = @import("config.zig");
const storage = @import("storage.zig");
const models = @import("models.zig");

pub const Health = struct {
    ok: bool,
    name: []const u8,
    version: []const u8,
    storage: []const u8,
};

pub const ConfigView = struct {
    config_path: []const u8,
    data_dir: []const u8,
    db_path: []const u8,
    max_entries: u32,
    allow_duplicates: bool,
    poll_interval_ms: u32,
    paste_enabled: bool,
    paste_buffer_ms: u32,
    max_preview_chars: u32,
    terminal_command: []const u8,
    excluded_apps: []const []const u8,
    excluded_windows: []const []const u8,
};

pub fn configView(cfg: config.Config) ConfigView {
    return .{
        .config_path = cfg.config_path,
        .data_dir = cfg.data_dir,
        .db_path = cfg.db_path,
        .max_entries = cfg.max_entries,
        .allow_duplicates = cfg.allow_duplicates,
        .poll_interval_ms = cfg.poll_interval_ms,
        .paste_enabled = cfg.paste_enabled,
        .paste_buffer_ms = cfg.paste_buffer_ms,
        .max_preview_chars = cfg.max_preview_chars,
        .terminal_command = cfg.terminal_command,
        .excluded_apps = cfg.excluded_apps,
        .excluded_windows = cfg.excluded_windows,
    };
}

pub fn watcherStatus(store: *storage.Storage, cfg: config.Config) !models.WatcherStatus {
    const last_seen_ms = try store.watcherLastSeen();
    const running = if (last_seen_ms) |seen| seen + (@as(i64, cfg.poll_interval_ms) * 8) + 2000 >= try store.nowMs() else false;
    return .{
        .running = running,
        .paused = try store.isWatcherPaused(),
        .backend = "wl-clipboard",
        .poll_interval_ms = cfg.poll_interval_ms,
        .last_seen_ms = last_seen_ms,
    };
}

pub fn copyEntry(allocator: std.mem.Allocator, init: std.process.Init, store: *storage.Storage, id: i64) !bool {
    const entry = (try store.get(id)) orelse return false;
    defer entry.deinit(allocator);
    try clipboard.writeText(allocator, init, entry.content);
    try store.markSelfWrite(entry.hash);
    return true;
}

pub fn copyEntries(allocator: std.mem.Allocator, init: std.process.Init, store: *storage.Storage, ids: []const i64) !bool {
    const content = try store.selectedContents(ids);
    defer allocator.free(content);
    if (content.len == 0) return false;
    try clipboard.writeText(allocator, init, content);
    const hash = @import("util.zig").sha256Hex(content);
    try store.markSelfWrite(&hash);
    return true;
}

pub fn pasteEntry(
    allocator: std.mem.Allocator,
    init: std.process.Init,
    cfg: config.Config,
    store: *storage.Storage,
    id: i64,
    target_window: ?[]const u8,
) !bool {
    const entry = (try store.get(id)) orelse return false;
    defer entry.deinit(allocator);
    if (!cfg.paste_enabled) {
        try clipboard.writeText(allocator, init, entry.content);
        try store.markSelfWrite(entry.hash);
        return true;
    }
    try clipboard.pasteText(allocator, init, entry.content, target_window, cfg.paste_buffer_ms);
    try store.markSelfWrite(entry.hash);
    return true;
}

pub fn openStore(allocator: std.mem.Allocator, init: std.process.Init) !struct {
    cfg: config.Config,
    store: storage.Storage,
} {
    const cfg = try config.Config.load(allocator, init);
    errdefer cfg.deinit();
    const store = try storage.Storage.open(allocator, cfg, init.io);
    return .{ .cfg = cfg, .store = store };
}
