const std = @import("std");

pub fn readText(allocator: std.mem.Allocator, init: std.process.Init) ![]u8 {
    const result = try std.process.run(allocator, init.io, .{
        .argv = &.{ "wl-paste", "--no-newline", "--type", "text" },
        .stderr_limit = .limited(4096),
        .stdout_limit = .limited(16 * 1024 * 1024),
    });
    defer allocator.free(result.stderr);

    switch (result.term) {
        .exited => |code| if (code != 0) {
            allocator.free(result.stdout);
            return error.ClipboardReadFailed;
        },
        else => {
            allocator.free(result.stdout);
            return error.ClipboardReadFailed;
        },
    }

    return result.stdout;
}

pub fn writeText(allocator: std.mem.Allocator, init: std.process.Init, text: []const u8) !void {
    var child = try std.process.spawn(init.io, .{
        .argv = &.{"wl-copy"},
        .stdin = .pipe,
        .stdout = .ignore,
        .stderr = .pipe,
    });
    defer child.kill(init.io);

    var stdin_buffer: [4096]u8 = undefined;
    var stdin_writer = child.stdin.?.writer(init.io, &stdin_buffer);
    try stdin_writer.interface.writeAll(text);
    try stdin_writer.interface.flush();
    child.stdin.?.close(init.io);
    child.stdin = null;

    if (child.stderr) |stderr_file| {
        var stderr_buffer: [4096]u8 = undefined;
        var stderr_reader = stderr_file.reader(init.io, &stderr_buffer);
        _ = stderr_reader.interface.discardAll(4096) catch {};
    }

    const term = try child.wait(init.io);
    switch (term) {
        .exited => |code| if (code == 0) return,
        else => {},
    }
    _ = allocator;
    return error.ClipboardWriteFailed;
}

pub fn pasteText(allocator: std.mem.Allocator, init: std.process.Init, text: []const u8, target_window: ?[]const u8, buffer_ms: u32) !void {
    try writeText(allocator, init, text);
    try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(buffer_ms), .awake);

    if (target_window) |address| {
        const focus_arg = try std.fmt.allocPrint(allocator, "address:{s}", .{address});
        defer allocator.free(focus_arg);
        _ = std.process.run(allocator, init.io, .{
            .argv = &.{ "hyprctl", "dispatch", "focuswindow", focus_arg },
            .stdout_limit = .limited(4096),
            .stderr_limit = .limited(4096),
        }) catch {};
    }

    const paste = try std.process.run(allocator, init.io, .{
        .argv = &.{ "hyprctl", "dispatch", "sendshortcut", "CTRL,V," },
        .stdout_limit = .limited(4096),
        .stderr_limit = .limited(4096),
    });
    defer allocator.free(paste.stdout);
    defer allocator.free(paste.stderr);

    switch (paste.term) {
        .exited => |code| if (code == 0) return,
        else => {},
    }
    return error.PasteBackFailed;
}

pub fn activeHyprlandAddress(allocator: std.mem.Allocator, init: std.process.Init) !?[]u8 {
    const result = try std.process.run(allocator, init.io, .{
        .argv = &.{ "hyprctl", "-j", "activewindow" },
        .stdout_limit = .limited(1024 * 1024),
        .stderr_limit = .limited(4096),
    });
    defer allocator.free(result.stderr);

    switch (result.term) {
        .exited => |code| if (code != 0) {
            allocator.free(result.stdout);
            return null;
        },
        else => {
            allocator.free(result.stdout);
            return null;
        },
    }

    const parsed = std.json.parseFromSlice(std.json.Value, allocator, result.stdout, .{}) catch {
        allocator.free(result.stdout);
        return null;
    };
    defer parsed.deinit();
    allocator.free(result.stdout);

    const obj = parsed.value.object;
    const address = obj.get("address") orelse return null;
    if (address != .string) return null;
    return try allocator.dupe(u8, address.string);
}
