const std = @import("std");

pub const EntryKind = enum {
    text,
    image,

    pub fn fromSlice(value: []const u8) ?EntryKind {
        if (std.mem.eql(u8, value, "text")) return .text;
        if (std.mem.eql(u8, value, "image")) return .image;
        return null;
    }

    pub fn string(kind: EntryKind) []const u8 {
        return switch (kind) {
            .text => "text",
            .image => "image",
        };
    }
};

pub const Entry = struct {
    id: i64,
    kind: []const u8,
    mime: []const u8,
    content: []const u8,
    preview: []const u8,
    hash: []const u8,
    favorite: bool,
    created_at_ms: i64,
    byte_len: i64,
    source_app: ?[]const u8 = null,
    blob_path: ?[]const u8 = null,
    image_width: ?i64 = null,
    image_height: ?i64 = null,

    pub fn deinit(self: Entry, allocator: std.mem.Allocator) void {
        allocator.free(self.kind);
        allocator.free(self.mime);
        allocator.free(self.content);
        allocator.free(self.preview);
        allocator.free(self.hash);
        if (self.source_app) |value| allocator.free(value);
        if (self.blob_path) |value| allocator.free(value);
    }
};

pub const WatcherStatus = struct {
    running: bool,
    paused: bool,
    backend: []const u8,
    poll_interval_ms: u32,
    last_seen_ms: ?i64 = null,
    last_error: ?[]const u8 = null,
};

pub const ImageMetadata = struct {
    width: ?i64 = null,
    height: ?i64 = null,
};
