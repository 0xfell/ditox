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
            .watcher = core.app.watcherStatus(opened.cfg),
            .stats = try opened.store.stats(),
        }, .{}, stdout);
        try stdout.writeByte('\n');
    } else if (std.mem.eql(u8, command, "repair")) {
        try std.json.Stringify.value(try opened.store.repair(), .{}, stdout);
        try stdout.writeByte('\n');
    } else if (std.mem.eql(u8, command, "pause")) {
        const duration = if (args.len > 2) args[2] else "0";
        try stdout.print("paused_for_ms={s}\n", .{duration});
    } else if (std.mem.eql(u8, command, "launch")) {
        try launchTui(allocator, init, opened.cfg.terminal_command);
    } else if (std.mem.eql(u8, command, "tui")) {
        try launchTui(allocator, init, "bun run --cwd tui start");
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

fn launchTui(allocator: std.mem.Allocator, init: std.process.Init, command: []const u8) !void {
    _ = core.clipboard.activeHyprlandAddress(allocator, init) catch null;
    var child = try std.process.spawn(init.io, .{
        .argv = &.{ "sh", "-c", command },
        .stdin = .inherit,
        .stdout = .inherit,
        .stderr = .inherit,
    });
    _ = try child.wait(init.io);
}

