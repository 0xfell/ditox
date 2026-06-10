const std = @import("std");
const root = @import("root.zig");
const app = @import("app.zig");

const Request = struct {
    jsonrpc: []const u8,
    id: std.json.Value,
    method: []const u8,
    params: ?std.json.Value = null,
};

/// One-shot entry point: opens (and closes) a store around a single request.
pub fn handle(allocator: std.mem.Allocator, init: std.process.Init, input: []const u8) ![]u8 {
    var opened = app.openStore(allocator, init) catch |err| {
        return errorResponseOwned(allocator, .null, -32000, @errorName(err));
    };
    defer opened.cfg.deinit();
    defer opened.store.close();
    return handleOpened(allocator, init, input, &opened);
}

/// Long-lived entry point: handles one request against an already-open store,
/// so a persistent `ditoxd serve --stdio` session pays config + DB setup once.
pub fn handleOpened(allocator: std.mem.Allocator, init: std.process.Init, input: []const u8, opened: anytype) ![]u8 {
    const body = requestBody(input);
    const parsed = std.json.parseFromSlice(Request, allocator, body, .{ .ignore_unknown_fields = true }) catch |err| {
        return errorResponseOwned(allocator, .null, -32700, @errorName(err));
    };
    defer parsed.deinit();

    const params = parsed.value.params;
    const method = parsed.value.method;
    if (!std.mem.eql(u8, parsed.value.jsonrpc, "2.0")) {
        return errorResponseOwned(allocator, parsed.value.id, -32600, "invalid jsonrpc version");
    }
    if (validateParams(method, params)) |message| {
        return errorResponseOwned(allocator, parsed.value.id, -32602, message);
    }

    return dispatch(allocator, init, parsed.value.id, method, params, opened) catch |err| {
        return errorResponseOwned(allocator, parsed.value.id, -32000, @errorName(err));
    };
}

fn dispatch(allocator: std.mem.Allocator, init: std.process.Init, id_value: std.json.Value, method: []const u8, params: ?std.json.Value, opened: anytype) ![]u8 {
    if (std.mem.eql(u8, method, "health.check")) {
        return successResponseOwned(allocator, id_value, app.Health{
            .ok = true,
            .name = "ditoxd",
            .version = root.version,
            .storage = "sqlite",
        });
    }
    if (std.mem.eql(u8, method, "config.get")) {
        return successResponseOwned(allocator, id_value, app.configView(opened.cfg));
    }
    if (std.mem.eql(u8, method, "watcher.status")) {
        const status = try app.watcherStatus(&opened.store, opened.cfg);
        defer if (status.last_error) |message| allocator.free(message);
        return successResponseOwned(allocator, id_value, status);
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
    if (std.mem.eql(u8, method, "entries.get_image")) {
        const id = getI64(params, "id") orelse return errorResponseOwned(allocator, id_value, -32602, "missing id");
        const entry = (try opened.store.get(id)) orelse return errorResponseOwned(allocator, id_value, -32004, "entry not found");
        defer entry.deinit(allocator);
        if (!std.mem.eql(u8, entry.kind, "image")) return errorResponseOwned(allocator, id_value, -32005, "entry is not an image");
        const bytes = app.readImageBlob(allocator, init, entry.blob_path) catch {
            return errorResponseOwned(allocator, id_value, -32006, "image blob is not stored");
        };
        defer allocator.free(bytes);
        const encoder = std.base64.standard.Encoder;
        const encoded = try allocator.alloc(u8, encoder.calcSize(bytes.len));
        defer allocator.free(encoded);
        _ = encoder.encode(encoded, bytes);
        return successResponseOwned(allocator, id_value, .{
            .mime = entry.mime,
            .data = encoded,
            .width = entry.image_width,
            .height = entry.image_height,
        });
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
        const preserve_favorites = getBool(params, "preserve_favorites") orelse false;
        return successResponseOwned(allocator, id_value, .{ .deleted = try opened.store.clearWithOptions(kind, preserve_favorites) });
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
        return successResponseOwned(allocator, id_value, try opened.store.repair(opened.cfg));
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

fn validateParams(method: []const u8, params: ?std.json.Value) ?[]const u8 {
    if (std.mem.eql(u8, method, "health.check") or
        std.mem.eql(u8, method, "config.get") or
        std.mem.eql(u8, method, "watcher.status") or
        std.mem.eql(u8, method, "history.resume") or
        std.mem.eql(u8, method, "repair.run") or
        std.mem.eql(u8, method, "stats.get"))
    {
        return validateNoParams(params);
    }
    if (std.mem.eql(u8, method, "entries.list") or std.mem.eql(u8, method, "entries.search")) return validateListParams(params);
    if (std.mem.eql(u8, method, "entries.get") or
        std.mem.eql(u8, method, "entries.get_image") or
        std.mem.eql(u8, method, "entries.copy") or
        std.mem.eql(u8, method, "entries.delete"))
    {
        return validateIdParams(params);
    }
    if (std.mem.eql(u8, method, "entries.add")) return validateAddParams(params);
    if (std.mem.eql(u8, method, "entries.bulk_copy") or std.mem.eql(u8, method, "entries.output")) return validateIdsParams(params);
    if (std.mem.eql(u8, method, "entries.paste")) return validatePasteParams(params);
    if (std.mem.eql(u8, method, "entries.favorite")) return validateFavoriteParams(params);
    if (std.mem.eql(u8, method, "entries.clear")) return validateClearParams(params);
    if (std.mem.eql(u8, method, "history.pause")) return validatePauseParams(params);
    return null;
}

fn validateNoParams(params: ?std.json.Value) ?[]const u8 {
    const value = params orelse return null;
    if (value != .object) return "params must be an object";
    if (value.object.count() != 0) return "unexpected params";
    return null;
}

fn validateListParams(params: ?std.json.Value) ?[]const u8 {
    if (validateOptionalKeys(params, &.{ "query", "filter", "limit" })) |message| return message;
    const object = getObject(params) orelse return null;
    if (object.get("query")) |value| if (value != .string) return "invalid query";
    if (object.get("filter")) |value| {
        if (value != .string or !validFilter(value.string)) return "invalid filter";
    }
    if (object.get("limit")) |value| {
        if (value != .integer or value.integer < 1 or value.integer > 500) return "invalid limit";
    }
    return null;
}

fn validateIdParams(params: ?std.json.Value) ?[]const u8 {
    if (validateRequiredKeys(params, &.{"id"}, &.{})) |message| return message;
    const object = getObject(params).?;
    if (object.get("id").? != .integer) return "invalid id";
    return null;
}

fn validateIdsParams(params: ?std.json.Value) ?[]const u8 {
    if (validateRequiredKeys(params, &.{"ids"}, &.{})) |message| return message;
    const value = getObject(params).?.get("ids").?;
    if (value != .array or value.array.items.len == 0) return "invalid ids";
    for (value.array.items) |item| {
        if (item != .integer) return "invalid ids";
    }
    return null;
}

fn validateAddParams(params: ?std.json.Value) ?[]const u8 {
    if (validateRequiredKeys(params, &.{"content"}, &.{})) |message| return message;
    if (getObject(params).?.get("content").? != .string) return "invalid content";
    return null;
}

fn validatePasteParams(params: ?std.json.Value) ?[]const u8 {
    if (validateRequiredKeys(params, &.{"id"}, &.{"target_window"})) |message| return message;
    const object = getObject(params).?;
    if (object.get("id").? != .integer) return "invalid id";
    if (object.get("target_window")) |value| if (value != .string) return "invalid target_window";
    return null;
}

fn validateFavoriteParams(params: ?std.json.Value) ?[]const u8 {
    if (validateRequiredKeys(params, &.{ "id", "favorite" }, &.{})) |message| return message;
    const object = getObject(params).?;
    if (object.get("id").? != .integer) return "invalid id";
    if (object.get("favorite").? != .bool) return "invalid favorite";
    return null;
}

fn validateClearParams(params: ?std.json.Value) ?[]const u8 {
    if (validateRequiredKeys(params, &.{"kind"}, &.{"preserve_favorites"})) |message| return message;
    const object = getObject(params).?;
    if (object.get("kind").? != .string or !validClearKind(object.get("kind").?.string)) return "invalid kind";
    if (object.get("preserve_favorites")) |value| if (value != .bool) return "invalid preserve_favorites";
    return null;
}

fn validatePauseParams(params: ?std.json.Value) ?[]const u8 {
    if (validateOptionalKeys(params, &.{"duration_ms"})) |message| return message;
    const object = getObject(params) orelse return null;
    if (object.get("duration_ms")) |value| {
        if (value != .integer or value.integer < 0) return "invalid duration_ms";
    }
    return null;
}

fn validateOptionalKeys(params: ?std.json.Value, optional: []const []const u8) ?[]const u8 {
    const value = params orelse return null;
    if (value != .object) return "params must be an object";
    for (value.object.keys()) |key| {
        if (!containsKey(optional, key)) return "unexpected params";
    }
    return null;
}

fn validateRequiredKeys(params: ?std.json.Value, required: []const []const u8, optional: []const []const u8) ?[]const u8 {
    const value = params orelse return "missing params";
    if (value != .object) return "params must be an object";
    for (required) |key| {
        if (!value.object.contains(key)) return "missing params";
    }
    for (value.object.keys()) |key| {
        if (!containsKey(required, key) and !containsKey(optional, key)) return "unexpected params";
    }
    return null;
}

fn containsKey(keys: []const []const u8, candidate: []const u8) bool {
    for (keys) |key| {
        if (std.mem.eql(u8, key, candidate)) return true;
    }
    return false;
}

fn validFilter(value: []const u8) bool {
    return std.mem.eql(u8, value, "all") or
        std.mem.eql(u8, value, "text") or
        std.mem.eql(u8, value, "images") or
        std.mem.eql(u8, value, "favorites") or
        std.mem.eql(u8, value, "today");
}

fn validClearKind(value: []const u8) bool {
    return std.mem.eql(u8, value, "all") or
        std.mem.eql(u8, value, "text") or
        std.mem.eql(u8, value, "image") or
        std.mem.eql(u8, value, "images");
}

test "requestBody accepts content-length frames" {
    try std.testing.expectEqualStrings("{\"ok\":true}", requestBody("Content-Length: 11\r\n\r\n{\"ok\":true}"));
}

test "validateParams enforces method-specific RPC params" {
    try expectValidParams("entries.paste", "{\"id\":1,\"target_window\":\"0xabc\"}");
    try expectValidParams("entries.get_image", "{\"id\":7}");
    try expectInvalidParams("entries.get_image", "{}");
    try expectValidParams("entries.list", "{\"query\":\"needle\",\"filter\":\"today\",\"limit\":500}");
    try expectValidParams("history.pause", "{}");
    try expectValidParams("stats.get", "{}");
    try expectInvalidParams("entries.favorite", "{\"id\":1}");
    try expectInvalidParams("entries.clear", "{\"kind\":\"everything\"}");
    try expectInvalidParams("entries.list", "{\"query\":\"ok\",\"extra\":1}");
    try expectInvalidParams("entries.bulk_copy", "{\"ids\":[]}");
    try expectInvalidParams("stats.get", "{\"unexpected\":true}");
}

fn expectValidParams(method: []const u8, json: []const u8) !void {
    var parsed = try std.json.parseFromSlice(std.json.Value, std.testing.allocator, json, .{});
    defer parsed.deinit();
    try std.testing.expect(validateParams(method, parsed.value) == null);
}

fn expectInvalidParams(method: []const u8, json: []const u8) !void {
    var parsed = try std.json.parseFromSlice(std.json.Value, std.testing.allocator, json, .{});
    defer parsed.deinit();
    try std.testing.expect(validateParams(method, parsed.value) != null);
}
