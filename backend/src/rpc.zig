const std = @import("std");
const app = @import("app.zig");

const Request = struct {
    jsonrpc: []const u8 = "2.0",
    id: std.json.Value = .null,
    method: []const u8,
    params: ?std.json.Value = null,
};

pub fn handle(allocator: std.mem.Allocator, init: std.process.Init, input: []const u8) ![]u8 {
    const body = requestBody(input);
    const parsed = std.json.parseFromSlice(Request, allocator, body, .{ .ignore_unknown_fields = true }) catch |err| {
        return errorResponseOwned(allocator, .null, -32700, @errorName(err));
    };
    defer parsed.deinit();

    var opened = app.openStore(allocator, init) catch |err| {
        return errorResponseOwned(allocator, parsed.value.id, -32000, @errorName(err));
    };
    defer opened.cfg.deinit();
    defer opened.store.close();

    const params = parsed.value.params;
    const method = parsed.value.method;

    return dispatch(allocator, init, parsed.value.id, method, params, &opened) catch |err| {
        return errorResponseOwned(allocator, parsed.value.id, -32000, @errorName(err));
    };
}

fn dispatch(allocator: std.mem.Allocator, init: std.process.Init, id_value: std.json.Value, method: []const u8, params: ?std.json.Value, opened: anytype) ![]u8 {
    if (std.mem.eql(u8, method, "health.check")) {
        return successResponseOwned(allocator, id_value, app.Health{
            .ok = true,
            .name = "ditoxd",
            .version = "0.1.0",
            .storage = "sqlite",
        });
    }
    if (std.mem.eql(u8, method, "config.get")) {
        return successResponseOwned(allocator, id_value, app.configView(opened.cfg));
    }
    if (std.mem.eql(u8, method, "watcher.status")) {
        return successResponseOwned(allocator, id_value, try app.watcherStatus(&opened.store, opened.cfg));
    }
    if (std.mem.eql(u8, method, "entries.list") or std.mem.eql(u8, method, "entries.search")) {
        const query = getString(params, "query") orelse "";
        const filter = getString(params, "filter") orelse "all";
        const limit = getU32(params, "limit") orelse 50;
        const entries = try opened.store.list(query, filter, limit);
        defer {
            for (entries) |entry| entry.deinit(allocator);
            allocator.free(entries);
        }
        return successResponseOwned(allocator, id_value, .{ .entries = entries });
    }
    if (std.mem.eql(u8, method, "entries.get")) {
        const id = getI64(params, "id") orelse return errorResponseOwned(allocator, id_value, -32602, "missing id");
        const entry = (try opened.store.get(id)) orelse return errorResponseOwned(allocator, id_value, -32004, "entry not found");
        defer entry.deinit(allocator);
        return successResponseOwned(allocator, id_value, .{ .entry = entry });
    }
    if (std.mem.eql(u8, method, "entries.add")) {
        const content = getString(params, "content") orelse return errorResponseOwned(allocator, id_value, -32602, "missing content");
        const id = try opened.store.addText(opened.cfg, content);
        return successResponseOwned(allocator, id_value, .{ .id = id });
    }
    if (std.mem.eql(u8, method, "entries.copy")) {
        const id = getI64(params, "id") orelse return errorResponseOwned(allocator, id_value, -32602, "missing id");
        return successResponseOwned(allocator, id_value, .{ .copied = try app.copyEntry(allocator, init, &opened.store, id) });
    }
    if (std.mem.eql(u8, method, "entries.bulk_copy")) {
        const ids = (try getI64Array(allocator, params, "ids")) orelse return errorResponseOwned(allocator, id_value, -32602, "missing ids");
        defer allocator.free(ids);
        return successResponseOwned(allocator, id_value, .{ .copied = try app.copyEntries(allocator, init, &opened.store, ids) });
    }
    if (std.mem.eql(u8, method, "entries.output")) {
        const ids = (try getI64Array(allocator, params, "ids")) orelse return errorResponseOwned(allocator, id_value, -32602, "missing ids");
        defer allocator.free(ids);
        const content = try opened.store.selectedContents(ids);
        defer allocator.free(content);
        return successResponseOwned(allocator, id_value, .{ .content = content });
    }
    if (std.mem.eql(u8, method, "entries.paste")) {
        const id = getI64(params, "id") orelse return errorResponseOwned(allocator, id_value, -32602, "missing id");
        const target_window = getString(params, "target_window");
        return successResponseOwned(allocator, id_value, .{ .pasted = try app.pasteEntry(allocator, init, opened.cfg, &opened.store, id, target_window) });
    }
    if (std.mem.eql(u8, method, "entries.delete")) {
        const id = getI64(params, "id") orelse return errorResponseOwned(allocator, id_value, -32602, "missing id");
        return successResponseOwned(allocator, id_value, .{ .deleted = try opened.store.delete(id) });
    }
    if (std.mem.eql(u8, method, "entries.favorite")) {
        const id = getI64(params, "id") orelse return errorResponseOwned(allocator, id_value, -32602, "missing id");
        const favorite = getBool(params, "favorite") orelse true;
        return successResponseOwned(allocator, id_value, .{ .updated = try opened.store.favorite(id, favorite) });
    }
    if (std.mem.eql(u8, method, "entries.clear")) {
        const kind = getString(params, "kind") orelse "all";
        return successResponseOwned(allocator, id_value, .{ .deleted = try opened.store.clear(kind) });
    }
    if (std.mem.eql(u8, method, "history.pause")) {
        const duration_ms = getU32(params, "duration_ms") orelse 0;
        try opened.store.pauseWatcher(duration_ms);
        return successResponseOwned(allocator, id_value, .{ .paused_for_ms = duration_ms });
    }
    if (std.mem.eql(u8, method, "history.resume")) {
        try opened.store.resumeWatcher();
        return successResponseOwned(allocator, id_value, .{ .resumed = true });
    }
    if (std.mem.eql(u8, method, "repair.run")) {
        return successResponseOwned(allocator, id_value, try opened.store.repair());
    }
    if (std.mem.eql(u8, method, "stats.get")) {
        return successResponseOwned(allocator, id_value, try opened.store.stats());
    }

    return errorResponseOwned(allocator, id_value, -32601, "method not found");
}

pub fn frame(allocator: std.mem.Allocator, json_body: []const u8) ![]u8 {
    return std.fmt.allocPrint(allocator, "Content-Length: {}\r\n\r\n{s}", .{ json_body.len, json_body });
}

fn requestBody(input: []const u8) []const u8 {
    if (std.mem.indexOf(u8, input, "\r\n\r\n")) |idx| return std.mem.trim(u8, input[idx + 4 ..], " \t\r\n");
    if (std.mem.indexOf(u8, input, "\n\n")) |idx| return std.mem.trim(u8, input[idx + 2 ..], " \t\r\n");
    return std.mem.trim(u8, input, " \t\r\n");
}

fn successResponseOwned(allocator: std.mem.Allocator, id: std.json.Value, result: anytype) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();
    const w = &out.writer;
    try w.writeAll("{\"jsonrpc\":\"2.0\",\"id\":");
    try std.json.Stringify.value(id, .{}, w);
    try w.writeAll(",\"result\":");
    try std.json.Stringify.value(result, .{}, w);
    try w.writeAll("}");
    return out.toOwnedSlice();
}

fn errorResponseOwned(allocator: std.mem.Allocator, id: std.json.Value, code: i32, message: []const u8) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();
    const w = &out.writer;
    try w.writeAll("{\"jsonrpc\":\"2.0\",\"id\":");
    try std.json.Stringify.value(id, .{}, w);
    try w.print(",\"error\":{{\"code\":{},\"message\":", .{code});
    try std.json.Stringify.value(message, .{}, w);
    try w.writeAll("}}");
    return out.toOwnedSlice();
}

fn getString(params: ?std.json.Value, key: []const u8) ?[]const u8 {
    const object = getObject(params) orelse return null;
    const value = object.get(key) orelse return null;
    return if (value == .string) value.string else null;
}

fn getI64(params: ?std.json.Value, key: []const u8) ?i64 {
    const object = getObject(params) orelse return null;
    const value = object.get(key) orelse return null;
    return switch (value) {
        .integer => |i| i,
        else => null,
    };
}

fn getU32(params: ?std.json.Value, key: []const u8) ?u32 {
    const value = getI64(params, key) orelse return null;
    if (value < 0) return null;
    return @intCast(value);
}

fn getBool(params: ?std.json.Value, key: []const u8) ?bool {
    const object = getObject(params) orelse return null;
    const value = object.get(key) orelse return null;
    return if (value == .bool) value.bool else null;
}

fn getI64Array(allocator: std.mem.Allocator, params: ?std.json.Value, key: []const u8) !?[]i64 {
    const object = getObject(params) orelse return null;
    const value = object.get(key) orelse return null;
    if (value != .array) return null;
    var ids: std.ArrayList(i64) = .empty;
    errdefer ids.deinit(allocator);
    for (value.array.items) |item| {
        if (item != .integer) return null;
        try ids.append(allocator, item.integer);
    }
    return try ids.toOwnedSlice(allocator);
}

fn getObject(params: ?std.json.Value) ?std.json.ObjectMap {
    const value = params orelse return null;
    return if (value == .object) value.object else null;
}

test "requestBody accepts content-length frames" {
    try std.testing.expectEqualStrings("{\"ok\":true}", requestBody("Content-Length: 11\r\n\r\n{\"ok\":true}"));
}
