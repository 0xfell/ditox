const std = @import("std");
const core = @import("ditox_core");

const Io = std.Io;

pub fn main(init: std.process.Init) !void {
    const allocator = init.gpa;
    const args = try init.minimal.args.toSlice(init.arena.allocator());

    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;

    if (args.len <= 1 or std.mem.eql(u8, args[1], "help") or std.mem.eql(u8, args[1], "--help")) {
        try printHelp(stdout);
        try stdout.flush();
        return;
    }

    const command = args[1];
    var opened = try core.app.openStore(allocator, init);
    defer opened.cfg.deinit();
    defer opened.store.close();

    if (std.mem.eql(u8, command, "add")) {
        const content = if (args.len > 2) try joinArgs(allocator, args[2..]) else try core.util.readAllStdin(allocator);
        defer allocator.free(content);
        const id = try opened.store.addText(opened.cfg, content);
        try stdout.print("{}\n", .{id});
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
        try stdout.print("{}\n", .{try opened.store.clear(kind)});
    } else if (std.mem.eql(u8, command, "status")) {
        try std.json.Stringify.value(.{
            .config = core.app.configView(opened.cfg),
            .watcher = try core.app.watcherStatus(&opened.store, opened.cfg),
            .stats = try opened.store.stats(),
        }, .{}, stdout);
        try stdout.writeByte('\n');
    } else if (std.mem.eql(u8, command, "repair")) {
        try std.json.Stringify.value(try opened.store.repair(), .{}, stdout);
        try stdout.writeByte('\n');
    } else if (std.mem.eql(u8, command, "pause")) {
        const duration = if (args.len > 2) try std.fmt.parseUnsigned(u32, args[2], 10) else 0;
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
    } else if (std.mem.eql(u8, command, "launch")) {
        try launchTui(allocator, init, opened.cfg.terminal_command);
    } else if (std.mem.eql(u8, command, "tui")) {
        const tui_command = try defaultTuiCommand(allocator, init);
        defer allocator.free(tui_command);
        try launchTui(allocator, init, tui_command);
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
        \\  list [--query q]           print recent entries as JSON
        \\  print <id>                 print entry content
        \\  copy <id>                  copy entry to clipboard
        \\  paste <id>                 copy and paste entry via Hyprland
        \\  delete <id>                delete entry
        \\  favorite <id>              pin/favorite entry
        \\  unfavorite <id>            remove favorite
        \\  clear [all|text|image]     clear history
        \\  status                     print config, watcher, and stats
        \\  repair                     run storage repair
        \\  pause [ms]                 pause capture, 0 pauses until resume
        \\  resume                     resume capture
        \\  output <id>...             print multiple text entries
        \\  launch                     open configured TUI terminal command
        \\  tui                        run local TUI command
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

fn parseIds(allocator: std.mem.Allocator, args: []const []const u8) ![]i64 {
    var ids: std.ArrayList(i64) = .empty;
    errdefer ids.deinit(allocator);
    for (args) |arg| {
        try ids.append(allocator, try std.fmt.parseInt(i64, arg, 10));
    }
    return ids.toOwnedSlice(allocator);
}

fn launchTui(allocator: std.mem.Allocator, init: std.process.Init, command: []const u8) !void {
    const target = core.clipboard.activeHyprlandAddress(allocator, init) catch null;
    defer if (target) |value| allocator.free(value);
    const command_with_env = if (target) |address|
        try std.fmt.allocPrint(allocator, "DITOX_TARGET_WINDOW={s} {s}", .{ address, command })
    else
        try allocator.dupe(u8, command);
    defer allocator.free(command_with_env);

    var child = try std.process.spawn(init.io, .{
        .argv = &.{ "sh", "-c", command_with_env },
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
