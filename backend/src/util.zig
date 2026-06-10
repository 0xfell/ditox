const std = @import("std");
const models = @import("models.zig");

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

/// Sniffs the image format from magic bytes. This is the single source of
/// truth for capture-format detection: both the CLI `--wl-store` path and the
/// watcher daemon verify clipboard payloads with it, so an announced MIME can
/// never mislabel stored bytes.
pub fn detectImageMime(bytes: []const u8) ?[]const u8 {
    if (isPng(bytes)) return "image/png";
    if (isJpeg(bytes)) return "image/jpeg";
    if (isGif(bytes)) return "image/gif";
    if (isWebp(bytes)) return "image/webp";
    if (isBmp(bytes)) return "image/bmp";
    return null;
}

/// True when `detectImageMime` is able to recognize this MIME from magic
/// bytes. Formats outside this set (e.g. image/svg+xml) cannot be verified.
pub fn isSniffableImageMime(mime: []const u8) bool {
    return std.mem.eql(u8, mime, "image/png") or
        std.mem.eql(u8, mime, "image/jpeg") or
        std.mem.eql(u8, mime, "image/jpg") or
        std.mem.eql(u8, mime, "image/gif") or
        std.mem.eql(u8, mime, "image/webp") or
        std.mem.eql(u8, mime, "image/bmp");
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

pub fn imageMetadata(bytes: []const u8, mime: []const u8) models.ImageMetadata {
    if (std.mem.eql(u8, mime, "image/png")) return pngMetadata(bytes);
    if (std.mem.eql(u8, mime, "image/jpeg")) return jpegMetadata(bytes);
    return .{};
}

fn pngMetadata(bytes: []const u8) models.ImageMetadata {
    const signature = "\x89PNG\r\n\x1a\n";
    if (bytes.len < 24 or !std.mem.eql(u8, bytes[0..8], signature)) return .{};
    if (!std.mem.eql(u8, bytes[12..16], "IHDR")) return .{};
    return .{
        .width = @intCast(std.mem.readInt(u32, bytes[16..][0..4], .big)),
        .height = @intCast(std.mem.readInt(u32, bytes[20..][0..4], .big)),
    };
}

fn jpegMetadata(bytes: []const u8) models.ImageMetadata {
    if (bytes.len < 4 or bytes[0] != 0xff or bytes[1] != 0xd8) return .{};
    var index: usize = 2;
    while (index + 9 < bytes.len) {
        if (bytes[index] != 0xff) {
            index += 1;
            continue;
        }
        while (index < bytes.len and bytes[index] == 0xff) index += 1;
        if (index >= bytes.len) return .{};
        const marker = bytes[index];
        index += 1;
        if (marker == 0xd9 or marker == 0xda) return .{};
        if (index + 2 > bytes.len) return .{};
        const segment_len = std.mem.readInt(u16, bytes[index..][0..2], .big);
        if (segment_len < 2 or index + segment_len > bytes.len) return .{};
        if (isJpegSof(marker) and segment_len >= 7) {
            return .{
                .height = @intCast(std.mem.readInt(u16, bytes[index + 3 ..][0..2], .big)),
                .width = @intCast(std.mem.readInt(u16, bytes[index + 5 ..][0..2], .big)),
            };
        }
        index += segment_len;
    }
    return .{};
}

fn isJpegSof(marker: u8) bool {
    return switch (marker) {
        0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf => true,
        else => false,
    };
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

test "detectImageMime sniffs known formats and rejects other bytes" {
    try std.testing.expectEqualStrings("image/png", detectImageMime("\x89PNG\r\n\x1a\n12345678").?);
    try std.testing.expectEqualStrings("image/jpeg", detectImageMime("\xff\xd8\xff\xe0rest").?);
    try std.testing.expectEqualStrings("image/gif", detectImageMime("GIF89a-rest").?);
    try std.testing.expectEqualStrings("image/webp", detectImageMime("RIFF\x00\x00\x00\x00WEBPVP8 ").?);
    try std.testing.expectEqualStrings("image/bmp", detectImageMime("BM\x00\x00").?);
    try std.testing.expectEqual(@as(?[]const u8, null), detectImageMime("<svg></svg>"));
    try std.testing.expectEqual(@as(?[]const u8, null), detectImageMime("https://example.com/cat.png"));

    try std.testing.expect(isSniffableImageMime("image/png"));
    try std.testing.expect(isSniffableImageMime("image/jpg"));
    try std.testing.expect(!isSniffableImageMime("image/svg+xml"));
    try std.testing.expect(!isSniffableImageMime("text/plain"));
}

test "imageMetadata reads png dimensions" {
    const png =
        "\x89PNG\r\n\x1a\n" ++
        "\x00\x00\x00\x0dIHDR" ++
        "\x00\x00\x00\x02" ++
        "\x00\x00\x00\x03" ++
        "\x08\x06\x00\x00\x00";
    const meta = imageMetadata(png, "image/png");
    try std.testing.expectEqual(@as(?i64, 2), meta.width);
    try std.testing.expectEqual(@as(?i64, 3), meta.height);
}
