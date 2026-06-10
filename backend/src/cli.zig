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
    if (std.mem.eql(u8, args[1], "-v") or std.mem.eql(u8, args[1], "--version") or std.mem.eql(u8, args[1], "version")) {
        try stdout.print("ditox {s}\n", .{core.version});
        try stdout.flush();
        return;
    }

    const command = args[1];
    var opened = try core.app.openStore(allocator, init);
    defer opened.cfg.deinit();
    defer opened.store.close();

    if (matchesAlias(command, &command_aliases.add)) {
        const content = if (args.len > 2) try joinArgs(allocator, args[2..]) else try core.util.readAllStdin(allocator);
        defer allocator.free(content);
        const id = try opened.store.addText(opened.cfg, content);
        try stdout.print("{}\n", .{id});
    } else if (matchesAlias(command, &command_aliases.copy_input)) {
        const content = if (args.len > 2) try joinArgs(allocator, args[2..]) else try core.util.readAllStdin(allocator);
        defer allocator.free(content);
        try core.clipboard.writeText(allocator, init, content);
        const hash = core.util.sha256Hex(content);
        try opened.store.markSelfWrite(&hash);
    } else if (matchesAlias(command, &command_aliases.print_clipboard)) {
        const content = try core.clipboard.readText(allocator, init);
        defer allocator.free(content);
        try stdout.writeAll(content);
    } else if (matchesAlias(command, &command_aliases.wl_store)) {
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
    } else if (matchesAlias(command, &command_aliases.auto_paste)) {
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
    } else if (matchesAlias(command, &command_aliases.clear_pinned)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("all", true)});
    } else if (matchesAlias(command, &command_aliases.clear_all)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("all", false)});
    } else if (matchesAlias(command, &command_aliases.clear_images)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("images", false)});
    } else if (matchesAlias(command, &command_aliases.clear_text)) {
        try stdout.print("{}\n", .{try opened.store.clearWithOptions("text", false)});
    } else if (std.mem.eql(u8, command, "status")) {
        const watcher = try core.app.watcherStatus(&opened.store, opened.cfg);
        defer if (watcher.last_error) |message| allocator.free(message);
        try std.json.Stringify.value(.{
            .config = core.app.configView(opened.cfg),
            .watcher = watcher,
            .stats = try opened.store.stats(),
        }, .{}, stdout);
        try stdout.writeByte('\n');
    } else if (matchesAlias(command, &command_aliases.repair)) {
        try std.json.Stringify.value(try opened.store.repair(opened.cfg), .{}, stdout);
        try stdout.writeByte('\n');
    } else if (matchesAlias(command, &command_aliases.kill)) {
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
    } else if (matchesAlias(command, &command_aliases.output_all)) {
        const format = if (args.len > 2) args[2] else "unescaped";
        try outputAllText(&opened.store, opened.cfg.max_entries, format, stdout);
    } else if (matchesAlias(command, &command_aliases.unsupported_platform_listen)) {
        try stdout.print("{s} is not supported yet; Ditox currently supports Wayland listening through -listen and -listen-shell.\n", .{command});
    } else if (matchesAlias(command, &command_aliases.listen_shell)) {
        try runDaemon(allocator, init, args[0], true);
    } else if (matchesAlias(command, &command_aliases.listen)) {
        try runDaemon(allocator, init, args[0], false);
    } else if (matchesAlias(command, &command_aliases.keep)) {
        try launchTui(allocator, init, opened.cfg.terminal_command, .{ .keep_open = true });
    } else if (matchesAlias(command, &command_aliases.enable_realtime)) {
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
        \\  -v|--version|version       print version
        \\  add [text]                 add text or stdin to history
        \\  -a [text]                  short alias for add
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
        \\                             clear aliases
        \\  status                     print config, watcher, and stats
        \\  repair                     run storage repair
        \\  -clean                     short alias for repair
        \\  -kill                      kill stored watcher process
        \\  pause [ms]                 pause capture, 0 pauses until resume
        \\  -pause <duration>          alias, accepts ms/s/m/h
        \\  resume                     resume capture
        \\  output <id>...             print multiple text entries
        \\  --output-all raw|unescaped print all text entries
        \\  -listen-shell              run watcher in this shell
        \\  -listen                    start watcher process
        \\  -listen-x11|-listen-darwin recognized listener aliases; unsupported
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

    if (core.util.detectImageMime(input)) |mime| {
        _ = try store.addImage(cfg, mime, input, core.util.imageMetadata(input, mime));
        return;
    }

    _ = try store.addText(cfg, input);
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
        .argv = &.{ daemon, "daemon" },
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

// Accepted spellings for each aliased command, in one place instead of
// eighteen near-identical predicate functions. Spellings must stay exactly
// as the compat surface promises (see printHelp and the CLI smoke tests).
const command_aliases = struct {
    const add = [_][]const u8{ "add", "-a", "--add" };
    const copy_input = [_][]const u8{ "-c", "--copy-input" };
    const print_clipboard = [_][]const u8{ "-p", "--paste", "--print-clipboard" };
    const wl_store = [_][]const u8{ "-wl-store", "--wl-store" };
    const auto_paste = [_][]const u8{ "-auto-paste", "--auto-paste" };
    const clear_pinned = [_][]const u8{ "-clear", "--clear" };
    const clear_all = [_][]const u8{ "-clear-all", "--clear-all" };
    const clear_images = [_][]const u8{ "-clear-images", "--clear-images" };
    const clear_text = [_][]const u8{ "-clear-text", "--clear-text" };
    const repair = [_][]const u8{ "repair", "-clean", "--clean" };
    const kill = [_][]const u8{ "-kill", "--kill" };
    const pause = [_][]const u8{ "pause", "-pause", "--pause" };
    const pause_prefixes = [_][]const u8{ "-pause=", "--pause=" };
    const output_all = [_][]const u8{ "-output-all", "--output-all" };
    const enable_realtime = [_][]const u8{ "-enable-real-time", "--enable-real-time" };
    // Recognized listener spellings for platforms Ditox does not support yet.
    const unsupported_platform_listen = [_][]const u8{ "-listen-x11", "--listen-x11", "-listen-darwin", "--listen-darwin" };
    const listen_shell = [_][]const u8{ "-listen-shell", "--listen-shell" };
    const listen = [_][]const u8{ "-listen", "--listen" };
    const keep = [_][]const u8{ "keep", "--keep" };
};

fn matchesAlias(command: []const u8, aliases: []const []const u8) bool {
    for (aliases) |alias| {
        if (std.mem.eql(u8, command, alias)) return true;
    }
    return false;
}

fn matchesAnyPrefix(command: []const u8, prefixes: []const []const u8) bool {
    for (prefixes) |prefix| {
        if (std.mem.startsWith(u8, command, prefix)) return true;
    }
    return false;
}

fn isPauseCommand(command: []const u8) bool {
    return matchesAlias(command, &command_aliases.pause) or
        matchesAnyPrefix(command, &command_aliases.pause_prefixes);
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
    if (env.get("DITOX_TUI_DIR")) |dir| return sourceTuiCommand(allocator, dir);
    if (try installedTuiCommand(allocator, init)) |command| return command;

    const source_dir = std.fs.path.dirname(@src().file) orelse ".";
    const backend_dir = std.fs.path.dirname(source_dir) orelse source_dir;
    const repo_dir = std.fs.path.dirname(backend_dir) orelse backend_dir;
    const tui_dir = try std.fs.path.join(allocator, &.{ repo_dir, "tui" });
    defer allocator.free(tui_dir);
    const bundled_entry = try std.fs.path.join(allocator, &.{ tui_dir, "dist", "index.js" });
    defer allocator.free(bundled_entry);
    if (std.Io.Dir.cwd().access(init.io, bundled_entry, .{})) {
        return bundledTuiCommand(allocator, bundled_entry);
    } else |_| {}
    return sourceTuiCommand(allocator, tui_dir);
}

fn installedTuiCommand(allocator: std.mem.Allocator, init: std.process.Init) !?[]const u8 {
    const exe_dir = std.process.executableDirPathAlloc(init.io, allocator) catch return null;
    defer allocator.free(exe_dir);

    const candidates = [_][]const u8{
        "../share/ditox/tui/dist/index.js",
        "../share/ditox/tui/index.js",
        "tui/dist/index.js",
        "dist/index.js",
    };

    for (candidates) |candidate| {
        const entry = try std.fs.path.resolve(allocator, &.{ exe_dir, candidate });
        defer allocator.free(entry);
        if (std.Io.Dir.cwd().access(init.io, entry, .{})) {
            // When the bundle lives in the Nix install layout under
            // share/ditox/tui/dist, we have also shipped a matching
            // node_modules/@opentui tree (populated at build time from the
            // exact locked 0.2.15 packages). Use --no-install + --cwd so the
            // TUI process resolves its native FFI libs only from the store
            // path and never touches the user's global Bun cache. This is the
            // fix for the Super+V "ditox -enable-real-time" crash.
            if (std.mem.indexOf(u8, entry, "share/ditox/tui/dist/index.js")) |_| {
                const tui_root = std.fs.path.dirname(std.fs.path.dirname(entry) orelse entry) orelse entry;
                const root_quoted = try shellQuote(allocator, tui_root);
                defer allocator.free(root_quoted);
                return @as(?[]const u8, try std.fmt.allocPrint(allocator, "bun --no-install --cwd {s} ./dist/index.js", .{root_quoted}));
            }
            return @as(?[]const u8, try bundledTuiCommand(allocator, entry));
        } else |_| {}
    }

    return null;
}

fn bundledTuiCommand(allocator: std.mem.Allocator, entry: []const u8) ![]const u8 {
    const quoted = try shellQuote(allocator, entry);
    defer allocator.free(quoted);
    return std.fmt.allocPrint(allocator, "bun {s}", .{quoted});
}

fn sourceTuiCommand(allocator: std.mem.Allocator, dir: []const u8) ![]const u8 {
    const quoted = try shellQuote(allocator, dir);
    defer allocator.free(quoted);
    return std.fmt.allocPrint(allocator, "bun run --cwd {s} start", .{quoted});
}

fn shellQuote(allocator: std.mem.Allocator, value: []const u8) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();
    try out.writer.writeByte('\'');
    for (value) |byte| {
        if (byte == '\'') {
            try out.writer.writeAll("'\\''");
        } else {
            try out.writer.writeByte(byte);
        }
    }
    try out.writer.writeByte('\'');
    return out.toOwnedSlice();
}
