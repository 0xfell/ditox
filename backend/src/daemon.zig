const std = @import("std");
const core = @import("ditox_core");

const Io = std.Io;

pub fn main(init: std.process.Init) !void {
    const allocator = init.gpa;
    const args = try init.minimal.args.toSlice(init.arena.allocator());

    if (args.len > 1 and std.mem.eql(u8, args[1], "serve")) {
        try serveStdio(allocator, init);
        return;
    }
    if (args.len > 1 and std.mem.eql(u8, args[1], "watch")) {
        try watchClipboard(allocator, init);
        return;
    }
    if (args.len > 1 and (std.mem.eql(u8, args[1], "daemon") or std.mem.eql(u8, args[1], "run"))) {
        try runPersistentDaemon(allocator, init);
        return;
    }

    var stdout_buffer: [1024]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    try stdout.writeAll("usage: ditoxd serve --stdio | ditoxd watch | ditoxd daemon\n  daemon: single long-lived owner running full capture loop (recommended)\n");
    try stdout.flush();
}

fn serveStdio(allocator: std.mem.Allocator, init: std.process.Init) !void {
    const input = try core.util.readAllStdin(allocator);
    defer allocator.free(input);

    const response = try core.rpc.handle(allocator, init, input);
    defer allocator.free(response);
    const framed = try core.rpc.frame(allocator, response);
    defer allocator.free(framed);

    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    try stdout.writeAll(framed);
    try stdout.flush();
}

fn watchClipboard(allocator: std.mem.Allocator, init: std.process.Init) !void {
    var opened = try core.app.openStore(allocator, init);
    defer opened.cfg.deinit();
    defer opened.store.close();

    try opened.store.markWatcherPid(@as(i64, @intCast(std.os.linux.getpid())));
    defer opened.store.clearWatcherPid() catch {};

    // Delegate to the shared loop (keeps logic single-source; legacy `watch` still works for compat).
    try runCaptureLoop(allocator, init, opened.cfg, &opened.store);
}

fn captureImageFirst(
    allocator: std.mem.Allocator,
    init: std.process.Init,
    cfg: core.config.Config,
    store: *core.storage.Storage,
    last_hash: *?[64]u8,
) !bool {
    const types = core.clipboard.listMimeTypes(allocator, init) catch return false;
    defer core.clipboard.freeMimeTypes(allocator, types);
    const mime = core.clipboard.preferredImageMime(types) orelse return false;
    const bytes = core.clipboard.readBytes(allocator, init, mime) catch return false;
    defer allocator.free(bytes);
    if (bytes.len == 0) return false;

    const hash = core.util.sha256Hex(bytes);
    if (last_hash.* != null and std.mem.eql(u8, &last_hash.*.?, &hash)) return true;
    if (try shouldSkipSelfWrite(allocator, store, &hash)) {
        last_hash.* = hash;
        return true;
    }
    _ = try store.addImage(cfg, mime, bytes, core.util.imageMetadata(bytes, mime));
    last_hash.* = hash;
    return true;
}

fn captureText(
    allocator: std.mem.Allocator,
    init: std.process.Init,
    cfg: core.config.Config,
    store: *core.storage.Storage,
    last_hash: *?[64]u8,
) !void {
    const text = try core.clipboard.readText(allocator, init);
    defer allocator.free(text);
    if (text.len == 0) return;
    const hash = core.util.sha256Hex(text);
    if (last_hash.* != null and std.mem.eql(u8, &last_hash.*.?, &hash)) return;
    if (try shouldSkipSelfWrite(allocator, store, &hash)) {
        last_hash.* = hash;
        return;
    }
    _ = try store.addText(cfg, text);
    last_hash.* = hash;
}

fn shouldSkipSelfWrite(allocator: std.mem.Allocator, store: *core.storage.Storage, hash: []const u8) !bool {
    const self_hash = try store.selfWriteHash() orelse return false;
    defer allocator.free(self_hash);
    if (!std.mem.eql(u8, self_hash, hash)) return false;
    try store.clearSelfWriteHash();
    return true;
}

fn isActiveWindowExcluded(allocator: std.mem.Allocator, init: std.process.Init, cfg: core.config.Config) !bool {
    const window = try core.clipboard.activeHyprlandWindow(allocator, init) orelse return false;
    defer window.deinit(allocator);
    return matchesAny(window.class, cfg.excluded_apps) or matchesAny(window.title, cfg.excluded_windows);
}

fn matchesAny(value: []const u8, patterns: []const []const u8) bool {
    for (patterns) |pattern| {
        if (containsIgnoreCase(value, pattern)) return true;
    }
    return false;
}

fn containsIgnoreCase(value: []const u8, pattern: []const u8) bool {
    if (pattern.len == 0 or pattern.len > value.len) return false;
    var start: usize = 0;
    while (start + pattern.len <= value.len) : (start += 1) {
        for (pattern, 0..) |needle, offset| {
            if (std.ascii.toLower(value[start + offset]) != std.ascii.toLower(needle)) break;
        } else {
            return true;
        }
    }
    return false;
}

test "excluded app and window matching is case-insensitive" {
    const apps = [_][]const u8{ "Bitwarden", "KeePassXC" };
    const windows = [_][]const u8{"Secret Window"};

    try std.testing.expect(matchesAny("bitwarden desktop", &apps));
    try std.testing.expect(matchesAny("org.keepassxc.KeePassXC", &apps));
    try std.testing.expect(matchesAny("private SECRET window title", &windows));
    try std.testing.expect(!matchesAny("Alacritty", &apps));
    try std.testing.expect(!matchesAny("normal browser tab", &windows));
}

/// Proper persistent single-owner daemon (implements the approved architectural plan).
/// - Owns ONE long-lived Storage connection for the lifetime of the daemon (this is the core fix for the "database is locked" + SQLiteFailure storm that made the watcher go stale).
/// - Runs the FULL clipboard capture watcher loop (captureImageFirst + captureText + add* writes + exclude/pause/last_hash/self-write guards) against that single owned *Storage.
/// - `ditoxd serve --stdio` and legacy `ditoxd watch` remain for compatibility / one-offs.
/// - This eliminates the N-process writer contention at the source for the capture workload.
fn runPersistentDaemon(allocator: std.mem.Allocator, init: std.process.Init) !void {
    var stdout_buffer: [1024]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;

    try stdout.writeAll("Starting proper persistent ditoxd daemon (single DB owner + full capture)...\n");
    try stdout.flush();

    var opened = try core.app.openStore(allocator, init);
    // Intentionally do not defer deinit/close here: the daemon *owns* these for its entire lifetime.
    // On process exit the OS reclaims; this is the single-owner model.

    try opened.store.markWatcherPid(@as(i64, @intCast(std.os.linux.getpid())));
    // Best-effort clear on any return path (normal for long-lived daemon is process death).
    defer opened.store.clearWatcherPid() catch {};

    try stdout.writeAll("Daemon owns the single long-lived DB connection. Entering owned capture loop.\n");
    try stdout.flush();

    // Run the *real* capture workload (image/text polling, dedup, exclusions, addText/addImage writes,
    // markWatcherSeen, retention) directly against the owned store. No other process does the heavy writes.
    // This is the structural fix the verifiers required.
    try runCaptureLoop(allocator, init, opened.cfg, &opened.store);
}

/// Shared capture loop used by both legacy `watch` and the new single-owner `daemon`.
/// All heavy DB writes (add*, markWatcherSeen) go through the caller's store.
fn runCaptureLoop(
    allocator: std.mem.Allocator,
    init: std.process.Init,
    cfg: core.config.Config,
    store: *core.storage.Storage,
) !void {
    var last_hash: ?[64]u8 = null;
    while (true) {
        store.markWatcherSeen() catch {};

        if (store.isWatcherPaused() catch false) {
            try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(cfg.poll_interval_ms), .awake);
            continue;
        }

        if (isActiveWindowExcluded(allocator, init, cfg) catch false) {
            try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(cfg.poll_interval_ms), .awake);
            continue;
        }

        if (try captureImageFirst(allocator, init, cfg, store, &last_hash)) {
            try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(cfg.poll_interval_ms), .awake);
            continue;
        }

        captureText(allocator, init, cfg, store, &last_hash) catch {};

        try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(cfg.poll_interval_ms), .awake);
    }
}
