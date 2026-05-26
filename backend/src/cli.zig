const std = @import("std");
const core = @import("ditox_core");

const Io = std.Io;

const TuiLaunchOptions = struct {
    keep_open: bool = false,
    realtime: bool = false,
};

pub fn main(init: std.process.Init) !void {
    const allocator = init.gpa;
    const args = try init.minimal.args.toSlice(init.arena.allocator());

    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;

    if (args.len <= 1) {
        var opened = try core.app.openStore(allocator, init);
        defer opened.cfg.deinit();
        defer opened.store.close();
        try launchTui(allocator, init, opened.cfg.terminal_command, .{});
        try stdout.flush();
        return;
    }

    if (std.mem.eql(u8, args[1], "help") or std.mem.eql(u8, args[1], "--help")) {
        try printHelp(stdout);
        try stdout.flush();
        return;
    }

    const command = args[1];
    var opened = try core.app.openStore(allocator, init);
    defer opened.cfg.deinit();
    defer opened.store.close();

    if (isAddCommand(command)) {
        const content = if (args.len > 2) try joinArgs(allocator, args[2..]) else try core.util.readAllStdin(allocator);
        defer allocator.free(content);
        const id = try opened.store.addText(opened.cfg, content);
        try stdout.print("{}\n", .{id});
    } else if (isCopyInputCommand(command)) {
        const content = if (args.len > 2) try joinArgs(allocator, args[2..]) else try core.util.readAllStdin(allocator);
        defer allocator.free(content);
        try core.clipboard.writeText(allocator, init, content);
        const hash = core.util.sha256Hex(content);
        try opened.store.markSelfWrite(&hash);
    } else if (isPrintClipboardCommand(command)) {
        const content = try core.clipboard.readText(allocator, init);
        defer allocator.free(content);
        try stdout.writeAll(content);
    } else if (isWlStoreCommand(command)) {
        try storeWlData(allocator, init, opened.cfg, &opened.store);
    } else if (std.mem.eql(u8, command, "list")) {
        const query = optionValue(args, "--query") orelse "";
        const filter = optionValue(args, "--filter") orelse "all";
        const entries = try opened.store.list(query, filter, 50);
        defer {
            for (entries) |entry| entry.deinit(allocator);
            allocator.free(entries);
        }
        try std.json.Stringify.value(.{ .entries = entries }, .{}, stdout);
        try stdout.writeByte('\n');
    } else if (std.mem.eql(u8, command, "print")) {
        const id = try parseRequiredId(args);
        const entry = (try opened.store.get(id)) orelse return error.EntryNotFound;
        defer entry.deinit(allocator);
        try stdout.print("{s}\n", .{entry.content});
    } else if (std.mem.eql(u8, command, "copy")) {
        const id = try parseRequiredId(args);
        _ = try core.app.copyEntry(allocator, init, &opened.store, id);
    } else if (std.mem.eql(u8, command, "paste")) {
        const id = try parseRequiredId(args);
        const target = optionValue(args, "--target-window");
        _ = try core.app.pasteEntry(allocator, init, opened.cfg, &opened.store, id, target);
    } else if (isAutoPasteCommand(command)) {
        const target = optionValue(args, "--target-window");
        try core.app.autoPaste(allocator, init, opened.cfg, target);
    } else if (std.mem.eql(u8, command, "delete")) {
        const id = try parseRequiredId(args);
        try stdout.print("{}\n", .{try opened.store.delete(id)});
    } else if (std.mem.eql(u8, command, "favorite")) {
        const id = try parseRequiredId(args);
        try stdout.print("{}\n", .{try opened.store.favorite(id, true)});
    } else if (std.mem.eql(u8, command, "unfavorite")) {
        const id = try parseRequiredId(args);
        try stdout.print("{}\n", .{try opened.store.favorite(id, false)});
    } else if (std.mem.eql(u8, command, "clear")) {
        const kind = if (args.len > 2) args[2] else "all";
        try stdout.print("{}\n", .{try opened.store.clearWithOptions(kind, hasFlag(args, "--keep-pinned"))});
    } else if (isClipseClearPinnedCommand(command)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("all", true)});
    } else if (isClipseClearAllCommand(command)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("all", false)});
    } else if (isClipseClearImagesCommand(command)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("images", false)});
    } else if (isClipseClearTextCommand(command)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("text", false)});
    } else if (std.mem.eql(u8, command, "status")) {
        try std.json.Stringify.value(.{
            .config = core.app.configView(opened.cfg),
            .watcher = try core.app.watcherStatus(&opened.store, opened.cfg),
            .stats = try opened.store.stats(),
        }, .{}, stdout);
        try stdout.writeByte('\n');
    } else if (isRepairCommand(command)) {
        try std.json.Stringify.value(try opened.store.repair(opened.cfg), .{}, stdout);
        try stdout.writeByte('\n');
    } else if (isKillCommand(command)) {
        try std.json.Stringify.value(try core.app.killWatcher(&opened.store), .{}, stdout);
        try stdout.writeByte('\n');
    } else if (isPauseCommand(command)) {
        const duration = try parsePauseDuration(command, args);
        try opened.store.pauseWatcher(duration);
        try stdout.print("paused_for_ms={}\n", .{duration});
    } else if (std.mem.eql(u8, command, "resume")) {
        try opened.store.resumeWatcher();
        try stdout.writeAll("resumed\n");
    } else if (std.mem.eql(u8, command, "output")) {
        const ids = try parseIds(allocator, args[2..]);
        defer allocator.free(ids);
        const contents = try opened.store.selectedContents(ids);
        defer allocator.free(contents);
        try stdout.writeAll(contents);
        if (contents.len > 0 and contents[contents.len - 1] != '\n') try stdout.writeByte('\n');
    } else if (isOutputAllCommand(command)) {
        const format = if (args.len > 2) args[2] else "unescaped";
        try outputAllText(&opened.store, opened.cfg.max_entries, format, stdout);
    } else if (isUnsupportedPlatformListenCommand(command)) {
        try stdout.print("{s} is not supported yet; Ditox currently supports Wayland listening through -listen and -listen-shell.\n", .{command});
    } else if (isListenShellCommand(command)) {
        try runDaemon(allocator, init, args[0], true);
    } else if (isListenCommand(command)) {
        try runDaemon(allocator, init, args[0], false);
    } else if (isKeepCommand(command)) {
        try launchTui(allocator, init, opened.cfg.terminal_command, .{ .keep_open = true });
    } else if (isEnableRealtimeCommand(command)) {
        try launchTui(allocator, init, opened.cfg.terminal_command, .{ .realtime = true });
    } else if (std.mem.eql(u8, command, "launch")) {
        try launchTui(allocator, init, opened.cfg.terminal_command, .{ .keep_open = hasFlag(args, "--keep"), .realtime = hasFlag(args, "--enable-real-time") });
    } else if (std.mem.eql(u8, command, "tui")) {
        const tui_command = try defaultTuiCommand(allocator, init);
        defer allocator.free(tui_command);
        try launchTui(allocator, init, tui_command, .{ .keep_open = hasFlag(args, "--keep"), .realtime = hasFlag(args, "--enable-real-time") });
    } else {
        try stdout.print("unknown command: {s}\n", .{command});
        try printHelp(stdout);
    }

    try stdout.flush();
}

fn printHelp(stdout: *Io.Writer) !void {
    try stdout.writeAll(
        \\ditox commands:
        \\  add [text]                 add text or stdin to history
        \\  -a [text]                  Clipse alias for add
        \\  -c [text]                  copy text or stdin directly to clipboard
        \\  -p                         print current text clipboard
        \\  --wl-store                 store wl-paste --watch stdin
        \\  list [--query q]           print recent entries as JSON
        \\  print <id>                 print entry content
        \\  copy <id>                  copy entry to clipboard
        \\  paste <id> [--target-window addr]
        \\                             copy and paste entry via Hyprland
        \\  --auto-paste [--target-window addr]
        \\                             send configured paste shortcut
        \\  delete <id>                delete entry
        \\  favorite <id>              pin/favorite entry
        \\  unfavorite <id>            remove favorite
        \\  clear [all|text|image] [--keep-pinned]
        \\                             clear history
        \\  -clear                     clear unpinned history
        \\  -clear-all|-clear-text|-clear-images
        \\                             Clipse clear aliases
        \\  status                     print config, watcher, and stats
        \\  repair                     run storage repair
        \\  -clean                     Clipse alias for repair
        \\  -kill                      kill stored watcher process
        \\  pause [ms]                 pause capture, 0 pauses until resume
        \\  -pause <duration>          Clipse alias, accepts ms/s/m/h
        \\  resume                     resume capture
        \\  output <id>...             print multiple text entries
        \\  --output-all raw|unescaped print all text entries
        \\  -listen-shell              run watcher in this shell
        \\  -listen                    start watcher process
        \\  -listen-x11|-listen-darwin recognized Clipse listener aliases; unsupported
        \\  keep                       open TUI and keep it open after paste
        \\  -enable-real-time          open TUI with live polling enabled
        \\  launch [--keep]            open configured TUI terminal command
        \\  (no args)                  open configured TUI terminal command
        \\  tui [--keep]               run local TUI command
        \\
    );
}

fn joinArgs(allocator: std.mem.Allocator, args: []const []const u8) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();
    for (args, 0..) |arg, index| {
        if (index > 0) try out.writer.writeByte(' ');
        try out.writer.writeAll(arg);
    }
    return out.toOwnedSlice();
}

fn parseRequiredId(args: []const []const u8) !i64 {
    if (args.len < 3) return error.MissingId;
    return std.fmt.parseInt(i64, args[2], 10);
}

fn optionValue(args: []const []const u8, name: []const u8) ?[]const u8 {
    var i: usize = 0;
    while (i + 1 < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], name)) return args[i + 1];
    }
    return null;
}

fn hasFlag(args: []const []const u8, name: []const u8) bool {
    for (args[2..]) |arg| {
        if (std.mem.eql(u8, arg, name)) return true;
    }
    return false;
}

fn parseIds(allocator: std.mem.Allocator, args: []const []const u8) ![]i64 {
    var ids: std.ArrayList(i64) = .empty;
    errdefer ids.deinit(allocator);
    for (args) |arg| {
        try ids.append(allocator, try std.fmt.parseInt(i64, arg, 10));
    }
    return ids.toOwnedSlice(allocator);
}

fn parsePauseDuration(command: []const u8, args: []const []const u8) !u32 {
    const raw = pauseDurationValue(command, args) orelse return 0;
    return parseDurationMs(raw);
}

fn pauseDurationValue(command: []const u8, args: []const []const u8) ?[]const u8 {
    if (std.mem.startsWith(u8, command, "-pause=")) return command["-pause=".len..];
    if (std.mem.startsWith(u8, command, "--pause=")) return command["--pause=".len..];
    if (args.len > 2) return args[2];
    return null;
}

fn parseDurationMs(raw: []const u8) !u32 {
    const value = std.mem.trim(u8, raw, " \t\r\n");
    if (value.len == 0) return error.InvalidDuration;
    const suffixes = [_]struct {
        suffix: []const u8,
        multiplier: u64,
    }{
        .{ .suffix = "ms", .multiplier = 1 },
        .{ .suffix = "s", .multiplier = 1000 },
        .{ .suffix = "m", .multiplier = 60 * 1000 },
        .{ .suffix = "h", .multiplier = 60 * 60 * 1000 },
    };
    for (suffixes) |item| {
        if (std.mem.endsWith(u8, value, item.suffix)) {
            const number = value[0 .. value.len - item.suffix.len];
            return checkedDurationMs(number, item.multiplier);
        }
    }
    return checkedDurationMs(value, 1);
}

fn checkedDurationMs(number_raw: []const u8, multiplier: u64) !u32 {
    const number = std.mem.trim(u8, number_raw, " \t\r\n");
    if (number.len == 0) return error.InvalidDuration;
    const parsed = try std.fmt.parseUnsigned(u64, number, 10);
    const millis = parsed *| multiplier;
    if (millis > std.math.maxInt(u32)) return error.DurationTooLarge;
    return @intCast(millis);
}

fn storeWlData(allocator: std.mem.Allocator, init: std.process.Init, cfg: core.config.Config, store: anytype) !void {
    if (init.environ_map.*.get("CLIPBOARD_STATE")) |state| {
        if (std.mem.eql(u8, state, "sensitive")) return;
    }

    const input = try core.util.readAllStdin(allocator);
    defer allocator.free(input);
    if (input.len == 0) return;

    if (detectImageMime(input)) |mime| {
        _ = try store.addImage(cfg, mime, input, core.util.imageMetadata(input, mime));
        return;
    }

    _ = try store.addText(cfg, input);
}

fn detectImageMime(bytes: []const u8) ?[]const u8 {
    if (isPng(bytes)) return "image/png";
    if (isJpeg(bytes)) return "image/jpeg";
    if (isGif(bytes)) return "image/gif";
    if (isWebp(bytes)) return "image/webp";
    if (isBmp(bytes)) return "image/bmp";
    return null;
}

fn isPng(bytes: []const u8) bool {
    return bytes.len >= 8 and std.mem.eql(u8, bytes[0..8], "\x89PNG\r\n\x1a\n");
}

fn isJpeg(bytes: []const u8) bool {
    return bytes.len >= 3 and bytes[0] == 0xff and bytes[1] == 0xd8 and bytes[2] == 0xff;
}

fn isGif(bytes: []const u8) bool {
    return bytes.len >= 6 and (std.mem.eql(u8, bytes[0..6], "GIF87a") or std.mem.eql(u8, bytes[0..6], "GIF89a"));
}

fn isWebp(bytes: []const u8) bool {
    return bytes.len >= 12 and std.mem.eql(u8, bytes[0..4], "RIFF") and std.mem.eql(u8, bytes[8..12], "WEBP");
}

fn isBmp(bytes: []const u8) bool {
    return bytes.len >= 2 and std.mem.eql(u8, bytes[0..2], "BM");
}

fn outputAllText(store: anytype, limit: u32, format: []const u8, stdout: *Io.Writer) !void {
    if (!std.mem.eql(u8, format, "raw") and !std.mem.eql(u8, format, "unescaped")) return error.InvalidOutputFormat;
    const entries = try store.list("", "text", limit);
    defer {
        for (entries) |entry| entry.deinit(store.allocator);
        store.allocator.free(entries);
    }
    for (entries) |entry| {
        if (std.mem.eql(u8, format, "raw")) {
            try std.json.Stringify.value(entry.content, .{}, stdout);
        } else {
            try stdout.writeAll(entry.content);
        }
        try stdout.writeByte('\n');
    }
}

fn runDaemon(allocator: std.mem.Allocator, init: std.process.Init, self_path: []const u8, wait: bool) !void {
    const daemon = try daemonPath(allocator, init, self_path);
    defer allocator.free(daemon);

    var child = try std.process.spawn(init.io, .{
        .argv = &.{ daemon, "watch" },
        .stdin = if (wait) .inherit else .ignore,
        .stdout = if (wait) .inherit else .ignore,
        .stderr = if (wait) .inherit else .ignore,
    });
    if (wait) {
        _ = try child.wait(init.io);
    }
}

fn daemonPath(allocator: std.mem.Allocator, init: std.process.Init, self_path: []const u8) ![]const u8 {
    if (init.environ_map.*.get("DITOXD")) |value| return allocator.dupe(u8, value);
    if (std.fs.path.dirname(self_path)) |dir| {
        const candidate = try std.fs.path.join(allocator, &.{ dir, "ditoxd" });
        if (std.Io.Dir.cwd().access(init.io, candidate, .{})) {
            return candidate;
        } else |_| {
            allocator.free(candidate);
        }
    }
    return allocator.dupe(u8, "ditoxd");
}

fn isAddCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "add") or std.mem.eql(u8, command, "-a") or std.mem.eql(u8, command, "--add");
}

fn isCopyInputCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-c") or std.mem.eql(u8, command, "--copy-input");
}

fn isPrintClipboardCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-p") or std.mem.eql(u8, command, "--paste") or std.mem.eql(u8, command, "--print-clipboard");
}

fn isWlStoreCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-wl-store") or std.mem.eql(u8, command, "--wl-store");
}

fn isAutoPasteCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-auto-paste") or std.mem.eql(u8, command, "--auto-paste");
}

fn isClipseClearPinnedCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-clear") or std.mem.eql(u8, command, "--clear");
}

fn isClipseClearAllCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-clear-all") or std.mem.eql(u8, command, "--clear-all");
}

fn isClipseClearImagesCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-clear-images") or std.mem.eql(u8, command, "--clear-images");
}

fn isClipseClearTextCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-clear-text") or std.mem.eql(u8, command, "--clear-text");
}

fn isRepairCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "repair") or std.mem.eql(u8, command, "-clean") or std.mem.eql(u8, command, "--clean");
}

fn isKillCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-kill") or std.mem.eql(u8, command, "--kill");
}

fn isPauseCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "pause") or
        std.mem.eql(u8, command, "-pause") or
        std.mem.eql(u8, command, "--pause") or
        std.mem.startsWith(u8, command, "-pause=") or
        std.mem.startsWith(u8, command, "--pause=");
}

fn isOutputAllCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-output-all") or std.mem.eql(u8, command, "--output-all");
}

fn isEnableRealtimeCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-enable-real-time") or std.mem.eql(u8, command, "--enable-real-time");
}

fn isUnsupportedPlatformListenCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-listen-x11") or
        std.mem.eql(u8, command, "--listen-x11") or
        std.mem.eql(u8, command, "-listen-darwin") or
        std.mem.eql(u8, command, "--listen-darwin");
}

fn isListenShellCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-listen-shell") or std.mem.eql(u8, command, "--listen-shell");
}

fn isListenCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "-listen") or std.mem.eql(u8, command, "--listen");
}

fn isKeepCommand(command: []const u8) bool {
    return std.mem.eql(u8, command, "keep") or std.mem.eql(u8, command, "--keep");
}

fn launchTui(allocator: std.mem.Allocator, init: std.process.Init, command: []const u8, options: TuiLaunchOptions) !void {
    const target = core.clipboard.activeHyprlandAddress(allocator, init) catch null;
    defer if (target) |value| allocator.free(value);

    var env = try init.environ_map.clone(allocator);
    defer env.deinit();
    if (target) |address| {
        try env.put("DITOX_TARGET_WINDOW", address);
    } else {
        _ = env.swapRemove("DITOX_TARGET_WINDOW");
    }
    if (options.keep_open) try env.put("DITOX_TUI_EXIT_AFTER_PASTE", "false");
    if (options.realtime) try env.put("DITOX_TUI_REFRESH_MS", "250");

    var child = try std.process.spawn(init.io, .{
        .argv = &.{ "sh", "-c", command },
        .environ_map = &env,
        .stdin = .inherit,
        .stdout = .inherit,
        .stderr = .inherit,
    });
    _ = try child.wait(init.io);
}

fn defaultTuiCommand(allocator: std.mem.Allocator, init: std.process.Init) ![]const u8 {
    const env = init.environ_map.*;
    if (env.get("DITOX_TUI_COMMAND")) |command| return allocator.dupe(u8, command);
    if (env.get("DITOX_TUI_DIR")) |dir| return std.fmt.allocPrint(allocator, "bun run --cwd {s} start", .{dir});

    const source_dir = std.fs.path.dirname(@src().file) orelse ".";
    const backend_dir = std.fs.path.dirname(source_dir) orelse source_dir;
    const repo_dir = std.fs.path.dirname(backend_dir) orelse backend_dir;
    const tui_dir = try std.fs.path.join(allocator, &.{ repo_dir, "tui" });
    defer allocator.free(tui_dir);
    const bundled_entry = try std.fs.path.join(allocator, &.{ tui_dir, "dist", "index.js" });
    defer allocator.free(bundled_entry);
    if (std.Io.Dir.cwd().access(init.io, bundled_entry, .{})) {
        return std.fmt.allocPrint(allocator, "bun {s}", .{bundled_entry});
    } else |_| {}
    return std.fmt.allocPrint(allocator, "bun run --cwd {s} start", .{tui_dir});
}
