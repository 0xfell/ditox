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
    image_dir: []const u8,
    max_entries: u32,
    delete_after_seconds: u32,
    allow_duplicates: bool,
    poll_interval_ms: u32,
    paste_enabled: bool,
    paste_buffer_ms: u32,
    auto_paste_enabled: bool,
    auto_paste_keybind: []const u8,
    auto_paste_buffer_ms: u32,
    max_preview_chars: u32,
    terminal_command: []const u8,
    excluded_apps: []const []const u8,
    excluded_windows: []const []const u8,
};

pub const KillWatcherResult = struct {
    killed: bool,
    pid: ?i64,
};

pub fn configView(cfg: config.Config) ConfigView {
    return .{
        .config_path = cfg.config_path,
        .data_dir = cfg.data_dir,
        .db_path = cfg.db_path,
        .image_dir = cfg.image_dir,
        .max_entries = cfg.max_entries,
        .delete_after_seconds = cfg.delete_after_seconds,
        .allow_duplicates = cfg.allow_duplicates,
        .poll_interval_ms = cfg.poll_interval_ms,
        .paste_enabled = cfg.paste_enabled,
        .paste_buffer_ms = cfg.paste_buffer_ms,
        .auto_paste_enabled = cfg.auto_paste_enabled,
        .auto_paste_keybind = cfg.auto_paste_keybind,
        .auto_paste_buffer_ms = cfg.auto_paste_buffer_ms,
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

pub fn killWatcher(store: *storage.Storage) !KillWatcherResult {
    const pid = try store.watcherPid() orelse return .{ .killed = false, .pid = null };
    if (pid <= 0 or pid == @as(i64, @intCast(std.os.linux.getpid()))) {
        try store.clearWatcherPid();
        return .{ .killed = false, .pid = pid };
    }
    std.posix.kill(@as(std.posix.pid_t, @intCast(pid)), .TERM) catch |err| switch (err) {
        error.ProcessNotFound => {
            try store.clearWatcherPid();
            return .{ .killed = false, .pid = pid };
        },
        else => return err,
    };
    try store.clearWatcherPid();
    return .{ .killed = true, .pid = pid };
}

pub fn copyEntry(allocator: std.mem.Allocator, init: std.process.Init, store: *storage.Storage, id: i64) !bool {
    const entry = (try store.get(id)) orelse return false;
    defer entry.deinit(allocator);
    if (std.mem.eql(u8, entry.kind, "image")) {
        const bytes = try readImageBlob(allocator, init, entry.blob_path);
        defer allocator.free(bytes);
        try clipboard.writeBytes(allocator, init, bytes, entry.mime);
    } else {
        try clipboard.writeText(allocator, init, entry.content);
    }
    try store.markSelfWrite(entry.hash);
    _ = try store.markUsed(entry.id);
    return true;
}

pub fn copyEntries(allocator: std.mem.Allocator, init: std.process.Init, store: *storage.Storage, ids: []const i64) !bool {
    const content = try store.selectedContents(ids);
    defer allocator.free(content);
    if (content.len == 0) return false;
    try clipboard.writeText(allocator, init, content);
    const hash = @import("util.zig").sha256Hex(content);
    try store.markSelfWrite(&hash);
    for (ids) |id| _ = try store.markUsed(id);
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
    if (std.mem.eql(u8, entry.kind, "image")) {
        const bytes = try readImageBlob(allocator, init, entry.blob_path);
        defer allocator.free(bytes);
        if (!cfg.paste_enabled) {
            try clipboard.writeBytes(allocator, init, bytes, entry.mime);
        } else {
            try clipboard.pasteBytes(allocator, init, bytes, entry.mime, target_window, cfg.paste_buffer_ms, cfg.auto_paste_keybind);
        }
        try store.markSelfWrite(entry.hash);
        _ = try store.markUsed(entry.id);
        return true;
    }
    if (!cfg.paste_enabled) {
        try clipboard.writeText(allocator, init, entry.content);
        try store.markSelfWrite(entry.hash);
        _ = try store.markUsed(entry.id);
        return true;
    }
    try clipboard.pasteText(allocator, init, entry.content, target_window, cfg.paste_buffer_ms, cfg.auto_paste_keybind);
    try store.markSelfWrite(entry.hash);
    _ = try store.markUsed(entry.id);
    return true;
}

pub fn autoPaste(allocator: std.mem.Allocator, init: std.process.Init, cfg: config.Config, target_window: ?[]const u8) !void {
    try clipboard.sendPasteShortcut(allocator, init, target_window, cfg.auto_paste_buffer_ms, cfg.auto_paste_keybind);
}

pub fn openStore(allocator: std.mem.Allocator, init: std.process.Init) !struct {
    cfg: config.Config,
    store: storage.Storage,
} {
    const cfg = try config.Config.load(allocator, init);
    errdefer cfg.deinit();
    var store = try storage.Storage.open(allocator, cfg, init.io);
    errdefer store.close();
    _ = try store.applyRetention(cfg);
    return .{ .cfg = cfg, .store = store };
}

fn readImageBlob(allocator: std.mem.Allocator, init: std.process.Init, blob_path: ?[]const u8) ![]u8 {
    const path = blob_path orelse return error.ImageBlobMissing;
    if (path.len == 0) return error.ImageBlobMissing;
    return try std.Io.Dir.cwd().readFileAlloc(init.io, path, allocator, .limited(64 * 1024 * 1024));
}
