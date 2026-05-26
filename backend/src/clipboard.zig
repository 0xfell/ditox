const std = @import("std");

pub const ActiveWindow = struct {
    address: []const u8,
    class: []const u8,
    title: []const u8,

    pub fn deinit(self: ActiveWindow, allocator: std.mem.Allocator) void {
        allocator.free(self.address);
        allocator.free(self.class);
        allocator.free(self.title);
    }
};

pub fn listMimeTypes(allocator: std.mem.Allocator, init: std.process.Init) ![][]const u8 {
    const result = try std.process.run(allocator, init.io, .{
        .argv = &.{ "wl-paste", "--list-types" },
        .stderr_limit = .limited(4096),
        .stdout_limit = .limited(1024 * 1024),
    });
    defer allocator.free(result.stdout);
    defer allocator.free(result.stderr);

    switch (result.term) {
        .exited => |code| if (code != 0) return error.ClipboardReadFailed,
        else => return error.ClipboardReadFailed,
    }

    var list: std.ArrayList([]const u8) = .empty;
    errdefer {
        for (list.items) |item| allocator.free(item);
        list.deinit(allocator);
    }

    var lines = std.mem.splitScalar(u8, result.stdout, '\n');
    while (lines.next()) |line_raw| {
        const line = std.mem.trim(u8, line_raw, " \t\r\n");
        if (line.len == 0) continue;
        try list.append(allocator, try allocator.dupe(u8, line));
    }
    return list.toOwnedSlice(allocator);
}

pub fn freeMimeTypes(allocator: std.mem.Allocator, types: [][]const u8) void {
    for (types) |mime| allocator.free(mime);
    allocator.free(types);
}

pub fn preferredImageMime(types: []const []const u8) ?[]const u8 {
    const preferred = [_][]const u8{ "image/png", "image/jpeg", "image/webp", "image/gif", "image/bmp" };
    for (preferred) |candidate| {
        for (types) |mime| {
            if (std.mem.eql(u8, mime, candidate)) return mime;
        }
    }
    for (types) |mime| {
        if (std.mem.startsWith(u8, mime, "image/")) return mime;
    }
    return null;
}

pub fn readText(allocator: std.mem.Allocator, init: std.process.Init) ![]u8 {
    if (init.environ_map.*.get("DITOX_CLIPBOARD_MOCK")) |path| {
        return try readMockClipboard(allocator, init, path);
    }

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

pub fn readBytes(allocator: std.mem.Allocator, init: std.process.Init, mime: []const u8) ![]u8 {
    const result = try std.process.run(allocator, init.io, .{
        .argv = &.{ "wl-paste", "--type", mime },
        .stderr_limit = .limited(4096),
        .stdout_limit = .limited(64 * 1024 * 1024),
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
    if (init.environ_map.*.get("DITOX_CLIPBOARD_MOCK")) |path| {
        try writeMockClipboard(init, path, text);
        return;
    }

    try writeBytesToClipboard(allocator, init, text, null);
}

pub fn writeBytes(allocator: std.mem.Allocator, init: std.process.Init, bytes: []const u8, mime: []const u8) !void {
    if (init.environ_map.*.get("DITOX_CLIPBOARD_MOCK")) |path| {
        try writeMockClipboard(init, path, bytes);
        const mime_path = try std.fmt.allocPrint(allocator, "{s}.mime", .{path});
        defer allocator.free(mime_path);
        try writeMockClipboard(init, mime_path, mime);
        return;
    }

    try writeBytesToClipboard(allocator, init, bytes, mime);
}

pub fn pasteText(allocator: std.mem.Allocator, init: std.process.Init, text: []const u8, target_window: ?[]const u8, buffer_ms: u32, keybind: []const u8) !void {
    try writeText(allocator, init, text);
    if (init.environ_map.*.get("DITOX_CLIPBOARD_MOCK") != null) return;
    try sendPasteShortcut(allocator, init, target_window, buffer_ms, keybind);
}

pub fn pasteBytes(allocator: std.mem.Allocator, init: std.process.Init, bytes: []const u8, mime: []const u8, target_window: ?[]const u8, buffer_ms: u32, keybind: []const u8) !void {
    try writeBytes(allocator, init, bytes, mime);
    if (init.environ_map.*.get("DITOX_CLIPBOARD_MOCK") != null) return;
    try sendPasteShortcut(allocator, init, target_window, buffer_ms, keybind);
}

fn writeBytesToClipboard(allocator: std.mem.Allocator, init: std.process.Init, bytes: []const u8, mime: ?[]const u8) !void {
    var argv_buffer: [3][]const u8 = undefined;
    const argv = if (mime) |value| blk: {
        argv_buffer = .{ "wl-copy", "--type", value };
        break :blk argv_buffer[0..3];
    } else blk: {
        argv_buffer[0] = "wl-copy";
        break :blk argv_buffer[0..1];
    };
    var child = try std.process.spawn(init.io, .{
        .argv = argv,
        .stdin = .pipe,
        .stdout = .ignore,
        .stderr = .pipe,
    });
    defer child.kill(init.io);

    var stdin_buffer: [4096]u8 = undefined;
    var stdin_writer = child.stdin.?.writer(init.io, &stdin_buffer);
    try stdin_writer.interface.writeAll(bytes);
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

pub fn sendPasteShortcut(allocator: std.mem.Allocator, init: std.process.Init, target_window: ?[]const u8, buffer_ms: u32, keybind: []const u8) !void {
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

    const shortcut = try hyprlandShortcut(allocator, keybind);
    defer allocator.free(shortcut);
    const paste = try std.process.run(allocator, init.io, .{
        .argv = &.{ "hyprctl", "dispatch", "sendshortcut", shortcut },
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

fn hyprlandShortcut(allocator: std.mem.Allocator, keybind: []const u8) ![]u8 {
    const lower = try std.ascii.allocLowerString(allocator, std.mem.trim(u8, keybind, " \t\r\n"));
    defer allocator.free(lower);
    if (lower.len == 0) return error.InvalidPasteKeybind;

    var mods: std.ArrayList([]const u8) = .empty;
    defer mods.deinit(allocator);
    var main_key: ?[]const u8 = null;

    var parts = std.mem.splitScalar(u8, lower, '+');
    while (parts.next()) |raw_part| {
        const part = std.mem.trim(u8, raw_part, " \t\r\n");
        if (part.len == 0) return error.InvalidPasteKeybind;
        if (pasteModifier(part)) |modifier| {
            try mods.append(allocator, modifier);
        } else {
            if (main_key != null) return error.InvalidPasteKeybind;
            main_key = try hyprlandKey(allocator, part);
        }
    }

    const key = main_key orelse return error.InvalidPasteKeybind;
    defer allocator.free(key);

    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();
    for (mods.items, 0..) |modifier, index| {
        if (index > 0) try out.writer.writeByte(' ');
        try out.writer.writeAll(modifier);
    }
    try out.writer.print(",{s},", .{key});
    return out.toOwnedSlice();
}

fn pasteModifier(part: []const u8) ?[]const u8 {
    if (std.mem.eql(u8, part, "ctrl") or std.mem.eql(u8, part, "control")) return "CTRL";
    if (std.mem.eql(u8, part, "shift")) return "SHIFT";
    if (std.mem.eql(u8, part, "alt") or std.mem.eql(u8, part, "option")) return "ALT";
    if (std.mem.eql(u8, part, "super") or std.mem.eql(u8, part, "meta") or std.mem.eql(u8, part, "cmd") or std.mem.eql(u8, part, "command") or std.mem.eql(u8, part, "win")) return "SUPER";
    return null;
}

fn hyprlandKey(allocator: std.mem.Allocator, part: []const u8) ![]const u8 {
    if (part.len == 1) {
        const upper = try allocator.alloc(u8, 1);
        upper[0] = std.ascii.toUpper(part[0]);
        return upper;
    }
    if (std.mem.eql(u8, part, "space")) return allocator.dupe(u8, "SPACE");
    if (std.mem.eql(u8, part, "enter") or std.mem.eql(u8, part, "return")) return allocator.dupe(u8, "RETURN");
    if (std.mem.eql(u8, part, "tab")) return allocator.dupe(u8, "TAB");
    if (std.mem.eql(u8, part, "escape") or std.mem.eql(u8, part, "esc")) return allocator.dupe(u8, "ESCAPE");
    return std.ascii.allocUpperString(allocator, part);
}

pub fn activeHyprlandAddress(allocator: std.mem.Allocator, init: std.process.Init) !?[]u8 {
    const window = try activeHyprlandWindow(allocator, init) orelse return null;
    defer window.deinit(allocator);
    return try allocator.dupe(u8, window.address);
}

pub fn activeHyprlandWindow(allocator: std.mem.Allocator, init: std.process.Init) !?ActiveWindow {
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
    const class = obj.get("class");
    const title = obj.get("title");
    return .{
        .address = try allocator.dupe(u8, address.string),
        .class = try allocator.dupe(u8, if (class != null and class.? == .string) class.?.string else ""),
        .title = try allocator.dupe(u8, if (title != null and title.? == .string) title.?.string else ""),
    };
}

fn readMockClipboard(allocator: std.mem.Allocator, init: std.process.Init, path: []const u8) ![]u8 {
    return std.Io.Dir.cwd().readFileAlloc(init.io, path, allocator, .limited(16 * 1024 * 1024)) catch |err| switch (err) {
        error.FileNotFound => allocator.dupe(u8, ""),
        else => err,
    };
}

fn writeMockClipboard(init: std.process.Init, path: []const u8, text: []const u8) !void {
    if (std.fs.path.dirname(path)) |parent| {
        if (parent.len > 0) try std.Io.Dir.cwd().createDirPath(init.io, parent);
    }
    try std.Io.Dir.cwd().writeFile(init.io, .{ .sub_path = path, .data = text });
}

test "preferredImageMime chooses image types before text" {
    const types = [_][]const u8{ "text/plain", "image/jpeg", "image/png" };
    try std.testing.expectEqualStrings("image/png", preferredImageMime(&types).?);
}
