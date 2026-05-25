const std = @import("std");

pub fn sha256Hex(input: []const u8) [64]u8 {
    var digest: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash(input, &digest, .{});
    return std.fmt.bytesToHex(digest, .lower);
}

pub fn sanitizeText(allocator: std.mem.Allocator, input: []const u8) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(allocator);

    var i: usize = 0;
    while (i < input.len) {
        const c = input[i];
        if (c == 0x1b and i + 1 < input.len and input[i + 1] == '[') {
            i += 2;
            while (i < input.len and !isAnsiFinal(input[i])) : (i += 1) {}
            if (i < input.len) i += 1;
            continue;
        }

        if (c == '\n' or c == '\r' or c == '\t' or c >= 0x20) {
            try out.append(allocator, c);
        }
        i += 1;
    }

    return out.toOwnedSlice(allocator);
}

pub fn preview(allocator: std.mem.Allocator, input: []const u8, limit: usize) ![]u8 {
    const sanitized = try sanitizeText(allocator, input);
    defer allocator.free(sanitized);

    if (sanitized.len <= limit) return allocator.dupe(u8, sanitized);
    var out = try allocator.alloc(u8, limit + 3);
    @memcpy(out[0..limit], sanitized[0..limit]);
    @memcpy(out[limit .. limit + 3], "...");
    return out;
}

pub fn dirname(path: []const u8) []const u8 {
    return std.fs.path.dirname(path) orelse ".";
}

fn isAnsiFinal(c: u8) bool {
    return c >= 0x40 and c <= 0x7e;
}

pub fn readAllStdin(allocator: std.mem.Allocator) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(allocator);

    var buf: [4096]u8 = undefined;
    while (true) {
        const n = try std.posix.read(std.posix.STDIN_FILENO, &buf);
        if (n == 0) break;
        try out.appendSlice(allocator, buf[0..n]);
    }

    return out.toOwnedSlice(allocator);
}

test "sanitizeText strips ANSI and controls" {
    const allocator = std.testing.allocator;
    const cleaned = try sanitizeText(allocator, "\x1b[31mred\x1b[0m\x00\nok");
    defer allocator.free(cleaned);
    try std.testing.expectEqualStrings("red\nok", cleaned);
}

test "sha256Hex" {
    const hash = sha256Hex("abc");
    try std.testing.expectEqualStrings("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", &hash);
}
