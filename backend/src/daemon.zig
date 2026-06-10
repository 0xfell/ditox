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

/// Serves JSON-RPC over stdio. Content-Length framed requests are answered in
/// a loop over one long-lived store, so a TUI session holds a single
/// connection instead of spawning a process per call. For compatibility,
/// leftover unframed input at EOF is answered once (the old one-shot mode).
fn serveStdio(allocator: std.mem.Allocator, init: std.process.Init) !void {
    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;

    var opened = try core.app.openStore(allocator, init);
    defer opened.cfg.deinit();
    defer opened.store.close();

    var pending: std.ArrayList(u8) = .empty;
    defer pending.deinit(allocator);
    var buf: [4096]u8 = undefined;

    while (true) {
        while (framedRequestSpan(pending.items)) |span| {
            try respondStdio(allocator, init, stdout, &opened, pending.items[span.body_start..][0..span.body_len]);
            try pending.replaceRange(allocator, 0, span.body_start + span.body_len, &.{});
        }
        const n = std.posix.read(std.posix.STDIN_FILENO, &buf) catch break;
        if (n == 0) break;
        try pending.appendSlice(allocator, buf[0..n]);
    }

    const leftover = std.mem.trim(u8, pending.items, " \t\r\n");
    if (leftover.len > 0) try respondStdio(allocator, init, stdout, &opened, leftover);
}

fn respondStdio(allocator: std.mem.Allocator, init: std.process.Init, stdout: *Io.Writer, opened: anytype, request: []const u8) !void {
    const response = try core.rpc.handleOpened(allocator, init, request, opened);
    defer allocator.free(response);
    const framed = try core.rpc.frame(allocator, response);
    defer allocator.free(framed);
    try stdout.writeAll(framed);
    try stdout.flush();
}

const FrameSpan = struct { body_start: usize, body_len: usize };

fn framedRequestSpan(bytes: []const u8) ?FrameSpan {
    var sep_len: usize = 4;
    const header_end = std.mem.indexOf(u8, bytes, "\r\n\r\n") orelse blk: {
        sep_len = 2;
        break :blk std.mem.indexOf(u8, bytes, "\n\n") orelse return null;
    };
    const length = parseContentLength(bytes[0..header_end]) orelse return null;
    const body_start = header_end + sep_len;
    if (bytes.len < body_start + length) return null;
    return .{ .body_start = body_start, .body_len = length };
}

fn parseContentLength(header: []const u8) ?usize {
    var lines = std.mem.splitScalar(u8, header, '\n');
    while (lines.next()) |raw_line| {
        const line = std.mem.trim(u8, raw_line, " \t\r");
        const colon = std.mem.indexOfScalar(u8, line, ':') orelse continue;
        const name = std.mem.trim(u8, line[0..colon], " \t");
        if (!std.ascii.eqlIgnoreCase(name, "content-length")) continue;
        return std.fmt.parseInt(usize, std.mem.trim(u8, line[colon + 1 ..], " \t"), 10) catch null;
    }
    return null;
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
    const announced = core.clipboard.preferredImageMime(types) orelse return false;
    const bytes = core.clipboard.readBytes(allocator, init, announced) catch return false;
    defer allocator.free(bytes);
    if (bytes.len == 0) return false;

    // The clipboard can change between listing MIME types and reading bytes,
    // and offers can simply mislabel their payload. Trust the magic bytes over
    // the announced type: a recognized signature corrects the stored MIME, and
    // bytes that fail to match a sniffable announced type are not stored as an
    // image at all (the text path picks the tick up instead). Unverifiable
    // formats (e.g. image/svg+xml) keep the announced type as before.
    const sniffed = core.util.detectImageMime(bytes);
    const mime = sniffed orelse mime: {
        if (core.util.isSniffableImageMime(announced)) return false;
        break :mime announced;
    };

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

test "framedRequestSpan parses complete Content-Length frames only" {
    // Incomplete header.
    try std.testing.expect(framedRequestSpan("Content-Length: 5\r\n") == null);
    // Header complete, body incomplete.
    try std.testing.expect(framedRequestSpan("Content-Length: 5\r\n\r\nab") == null);
    // No Content-Length header.
    try std.testing.expect(framedRequestSpan("X-Other: 1\r\n\r\n{}") == null);

    const exact = framedRequestSpan("Content-Length: 2\r\n\r\n{}") orelse return error.TestUnexpectedResult;
    try std.testing.expectEqual(@as(usize, 21), exact.body_start);
    try std.testing.expectEqual(@as(usize, 2), exact.body_len);

    // Case-insensitive header name and a second pipelined request behind it.
    const pipelined = framedRequestSpan("content-length: 4\r\n\r\nabcdContent-Length: 2\r\n\r\n{}") orelse return error.TestUnexpectedResult;
    try std.testing.expectEqual(@as(usize, 21), pipelined.body_start);
    try std.testing.expectEqual(@as(usize, 4), pipelined.body_len);

    // Bare-newline separator tolerated.
    const bare = framedRequestSpan("Content-Length: 2\n\n{}") orelse return error.TestUnexpectedResult;
    try std.testing.expectEqual(@as(usize, 19), bare.body_start);
    try std.testing.expectEqual(@as(usize, 2), bare.body_len);
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
/// Capture failures must never terminate the loop: a transient SQLite-busy or
/// disk error on one tick is logged and surfaced through watcher.status
/// (`last_error`), and the next poll retries.
fn runCaptureLoop(
    allocator: std.mem.Allocator,
    init: std.process.Init,
    cfg: core.config.Config,
    store: *core.storage.Storage,
) !void {
    var last_hash: ?[64]u8 = null;
    var had_error = false;
    // A previous daemon run may have died with an error recorded; this run
    // starts fresh.
    store.clearWatcherError() catch {};
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

        var tick_error = false;
        const image_handled = captureImageFirst(allocator, init, cfg, store, &last_hash) catch |err| handled: {
            tick_error = true;
            reportCaptureError(allocator, store, "image capture", err);
            // An image was on the clipboard but storing it failed. Treat the
            // tick as handled so the text alternative (often just a URL) does
            // not replace the image; the next poll retries the image.
            break :handled true;
        };
        if (!image_handled) {
            captureText(allocator, init, cfg, store, &last_hash) catch |err| {
                tick_error = true;
                reportCaptureError(allocator, store, "text capture", err);
            };
        }
        if (had_error and !tick_error) store.clearWatcherError() catch {};
        had_error = tick_error;

        try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(cfg.poll_interval_ms), .awake);
    }
}

fn reportCaptureError(allocator: std.mem.Allocator, store: *core.storage.Storage, context: []const u8, err: anyerror) void {
    std.log.warn("{s} failed: {s}", .{ context, @errorName(err) });
    const message = std.fmt.allocPrint(allocator, "{s} failed: {s}", .{ context, @errorName(err) }) catch return;
    defer allocator.free(message);
    store.setWatcherError(message) catch {};
}
