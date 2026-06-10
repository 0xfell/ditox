//! Filesystem helpers for content-addressed image blobs. These are pure
//! (allocator, io)-parameterized functions; Storage wraps them with its own
//! state.

const std = @import("std");

const c = @cImport({
    @cInclude("unistd.h");
});

/// Writes `bytes` to `<image_dir>/<hash[0..2]>/<hash>.<ext>` atomically:
/// exclusive-create a uniquely suffixed tmp file, write + fsync, rename over
/// the final path, then fsync the parent directory. Returns the final path;
/// caller owns the returned slice.
pub fn writeImageBlob(
    allocator: std.mem.Allocator,
    io: std.Io,
    image_dir: []const u8,
    hash: []const u8,
    ext: []const u8,
    bytes: []const u8,
) ![]u8 {
    const shard = hash[0..2];
    const dir_path = try std.fmt.allocPrint(allocator, "{s}/{s}", .{ image_dir, shard });
    defer allocator.free(dir_path);
    try std.Io.Dir.cwd().createDirPath(io, dir_path);

    const final_path = try std.fmt.allocPrint(allocator, "{s}/{s}.{s}", .{ dir_path, hash, ext });
    errdefer allocator.free(final_path);
    // The tmp suffix must be unique per writer: a timestamp alone is only
    // second-granular here, and two concurrent writers of the same image
    // (daemon + --wl-store) would collide on the exclusive create.
    var tmp_suffix: [8]u8 = undefined;
    io.random(&tmp_suffix);
    const tmp_path = try std.fmt.allocPrint(allocator, "{s}.tmp-{x}", .{ final_path, std.mem.readInt(u64, &tmp_suffix, .little) });
    defer allocator.free(tmp_path);
    errdefer deleteFilePath(io, tmp_path);

    {
        var file = try std.Io.Dir.createFileAbsolute(io, tmp_path, .{ .exclusive = true });
        defer file.close(io);
        try file.writeStreamingAll(io, bytes);
        try file.sync(io);
    }

    if (std.fs.path.isAbsolute(tmp_path)) {
        try std.Io.Dir.renameAbsolute(tmp_path, final_path, io);
    } else {
        const cwd = std.Io.Dir.cwd();
        try cwd.rename(tmp_path, cwd, final_path, io);
    }

    var parent_dir = if (std.fs.path.isAbsolute(dir_path))
        try std.Io.Dir.openDirAbsolute(io, dir_path, .{})
    else
        try std.Io.Dir.cwd().openDir(io, dir_path, .{});
    defer parent_dir.close(io);
    _ = c.fsync(parent_dir.handle);
    return final_path;
}

/// Lists every regular file under `<image_dir>/<shard>/` as a full path built
/// the same way `writeImageBlob` builds stored paths, so results compare
/// byte-for-byte against `entries.blob_path`. Caller owns the slice and each
/// path. A missing image dir yields an empty list.
pub fn listBlobFiles(allocator: std.mem.Allocator, io: std.Io, image_dir: []const u8) ![][]u8 {
    var paths: std.ArrayList([]u8) = .empty;
    errdefer {
        for (paths.items) |path| allocator.free(path);
        paths.deinit(allocator);
    }

    var root = openIterableDir(io, image_dir) catch return paths.toOwnedSlice(allocator);
    defer root.close(io);

    var shards = root.iterate();
    while (try shards.next(io)) |shard| {
        if (shard.kind != .directory) continue;
        const shard_path = try std.fmt.allocPrint(allocator, "{s}/{s}", .{ image_dir, shard.name });
        defer allocator.free(shard_path);

        var shard_dir = openIterableDir(io, shard_path) catch continue;
        defer shard_dir.close(io);
        var files = shard_dir.iterate();
        while (try files.next(io)) |file| {
            if (file.kind != .file) continue;
            const path = try std.fmt.allocPrint(allocator, "{s}/{s}", .{ shard_path, file.name });
            errdefer allocator.free(path);
            try paths.append(allocator, path);
        }
    }
    return paths.toOwnedSlice(allocator);
}

fn openIterableDir(io: std.Io, path: []const u8) !std.Io.Dir {
    return if (std.fs.path.isAbsolute(path))
        std.Io.Dir.openDirAbsolute(io, path, .{ .iterate = true })
    else
        std.Io.Dir.cwd().openDir(io, path, .{ .iterate = true });
}

pub fn deleteFilePath(io: std.Io, path: []const u8) void {
    if (std.fs.path.isAbsolute(path)) {
        std.Io.Dir.deleteFileAbsolute(io, path) catch {};
    } else {
        std.Io.Dir.cwd().deleteFile(io, path) catch {};
    }
}

pub fn fileExists(io: std.Io, path: []const u8) bool {
    if (std.fs.path.isAbsolute(path)) {
        std.Io.Dir.accessAbsolute(io, path, .{}) catch return false;
    } else {
        std.Io.Dir.cwd().access(io, path, .{}) catch return false;
    }
    return true;
}

test "writeImageBlob writes a sharded content-addressed blob atomically" {
    const allocator = std.testing.allocator;
    const io = std.testing.io;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const image_dir = try std.fmt.allocPrint(allocator, ".zig-cache/tmp/{s}/images", .{tmp.sub_path});
    defer allocator.free(image_dir);
    try std.Io.Dir.cwd().createDirPath(io, image_dir);

    const hash = "abcdef0123456789";
    const path = try writeImageBlob(allocator, io, image_dir, hash, "png", "blob-bytes");
    defer allocator.free(path);

    const expected = try std.fmt.allocPrint(allocator, "{s}/ab/{s}.png", .{ image_dir, hash });
    defer allocator.free(expected);
    try std.testing.expectEqualStrings(expected, path);
    try std.testing.expect(fileExists(io, path));

    const contents = try std.Io.Dir.cwd().readFileAlloc(io, path, allocator, .limited(1024));
    defer allocator.free(contents);
    try std.testing.expectEqualStrings("blob-bytes", contents);

    // No tmp-suffixed leftovers survive a successful write.
    var shard_dir = try std.Io.Dir.cwd().openDir(io, std.fs.path.dirname(path).?, .{ .iterate = true });
    defer shard_dir.close(io);
    var iter = shard_dir.iterate();
    var count: usize = 0;
    while (try iter.next(io)) |entry| {
        try std.testing.expect(std.mem.indexOf(u8, entry.name, ".tmp-") == null);
        count += 1;
    }
    try std.testing.expectEqual(@as(usize, 1), count);

    deleteFilePath(io, path);
    try std.testing.expect(!fileExists(io, path));
    // Deleting a missing path is a no-op, not an error.
    deleteFilePath(io, path);
}
