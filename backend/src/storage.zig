const std = @import("std");
const models = @import("models.zig");
const util = @import("util.zig");
const config = @import("config.zig");

const c = @cImport({
    @cInclude("sqlite3.h");
});

pub const Stats = struct {
    entries: i64,
    text: i64,
    images: i64,
    favorites: i64,
};

pub const Storage = struct {
    allocator: std.mem.Allocator,
    db: *c.sqlite3,

    pub fn open(allocator: std.mem.Allocator, cfg: config.Config) !Storage {
        var db: ?*c.sqlite3 = null;
        const path_z = try allocator.dupeZ(u8, cfg.db_path);
        defer allocator.free(path_z);

        if (c.sqlite3_open(path_z.ptr, &db) != c.SQLITE_OK) return error.SQLiteOpenFailed;
        var storage = Storage{ .allocator = allocator, .db = db.? };
        errdefer storage.close();

        try storage.exec(schema);
        return storage;
    }

    pub fn close(self: *Storage) void {
        _ = c.sqlite3_close(self.db);
    }

    pub fn addText(self: *Storage, cfg: config.Config, text: []const u8) !i64 {
        const content = try util.sanitizeText(self.allocator, text);
        defer self.allocator.free(content);
        const entry_preview = try util.preview(self.allocator, content, cfg.max_preview_chars);
        defer self.allocator.free(entry_preview);

        const hash = util.sha256Hex(content);
        if (!cfg.allow_duplicates) {
            if (try self.findIdByHash(&hash)) |existing| return existing;
        }

        const stmt = try self.prepare(
            \\INSERT INTO entries
            \\  (kind, mime, content, preview, hash, favorite, created_at_ms, byte_len)
            \\VALUES
            \\  ('text', 'text/plain;charset=utf-8', ?, ?, ?, 0, CAST(strftime('%s','now') AS INTEGER) * 1000, ?)
        );
        defer _ = c.sqlite3_finalize(stmt);

        try bindText(stmt, 1, content);
        try bindText(stmt, 2, entry_preview);
        try bindText(stmt, 3, &hash);
        try bindInt64(stmt, 4, @intCast(content.len));
        try self.stepDone(stmt);

        try self.prune(cfg.max_entries);
        return c.sqlite3_last_insert_rowid(self.db);
    }

    pub fn list(self: *Storage, query: []const u8, filter: []const u8, limit: u32) ![]models.Entry {
        var sql_writer = std.Io.Writer.Allocating.init(self.allocator);
        defer sql_writer.deinit();
        const w = &sql_writer.writer;

        try w.writeAll(
            \\SELECT id, kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, source_app, blob_path
            \\FROM entries
            \\WHERE 1 = 1
        );

        const has_query = query.len > 0;
        if (has_query) try w.writeAll(" AND (content LIKE ? OR preview LIKE ?)");
        if (std.mem.eql(u8, filter, "text")) try w.writeAll(" AND kind = 'text'");
        if (std.mem.eql(u8, filter, "images")) try w.writeAll(" AND kind = 'image'");
        if (std.mem.eql(u8, filter, "favorites")) try w.writeAll(" AND favorite = 1");
        if (std.mem.eql(u8, filter, "today")) try w.writeAll(" AND created_at_ms >= (CAST(strftime('%s','now','start of day') AS INTEGER) * 1000)");
        try w.writeAll(" ORDER BY favorite DESC, created_at_ms DESC, id DESC LIMIT ?");

        const stmt = try self.prepare(sql_writer.written());
        defer _ = c.sqlite3_finalize(stmt);

        var bind_index: c_int = 1;
        var like_query: ?[]u8 = null;
        defer if (like_query) |value| self.allocator.free(value);
        if (has_query) {
            like_query = try std.fmt.allocPrint(self.allocator, "%{s}%", .{query});
            try bindText(stmt, bind_index, like_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, like_query.?);
            bind_index += 1;
        }
        try bindInt64(stmt, bind_index, limit);

        var entries: std.ArrayList(models.Entry) = .empty;
        errdefer {
            for (entries.items) |entry| entry.deinit(self.allocator);
            entries.deinit(self.allocator);
        }

        while (true) {
            const rc = c.sqlite3_step(stmt);
            if (rc == c.SQLITE_DONE) break;
            if (rc != c.SQLITE_ROW) return self.sqliteError();
            try entries.append(self.allocator, try self.readEntry(stmt));
        }

        return entries.toOwnedSlice(self.allocator);
    }

    pub fn get(self: *Storage, id: i64) !?models.Entry {
        const stmt = try self.prepare(
            \\SELECT id, kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, source_app, blob_path
            \\FROM entries WHERE id = ?
        );
        defer _ = c.sqlite3_finalize(stmt);
        try bindInt64(stmt, 1, id);

        const rc = c.sqlite3_step(stmt);
        if (rc == c.SQLITE_DONE) return null;
        if (rc != c.SQLITE_ROW) return self.sqliteError();
        return try self.readEntry(stmt);
    }

    pub fn delete(self: *Storage, id: i64) !bool {
        const stmt = try self.prepare("DELETE FROM entries WHERE id = ?");
        defer _ = c.sqlite3_finalize(stmt);
        try bindInt64(stmt, 1, id);
        try self.stepDone(stmt);
        return c.sqlite3_changes(self.db) > 0;
    }

    pub fn favorite(self: *Storage, id: i64, value: bool) !bool {
        const stmt = try self.prepare("UPDATE entries SET favorite = ? WHERE id = ?");
        defer _ = c.sqlite3_finalize(stmt);
        try bindInt64(stmt, 1, if (value) 1 else 0);
        try bindInt64(stmt, 2, id);
        try self.stepDone(stmt);
        return c.sqlite3_changes(self.db) > 0;
    }

    pub fn clear(self: *Storage, kind: []const u8) !i64 {
        var stmt: *c.sqlite3_stmt = undefined;
        if (std.mem.eql(u8, kind, "text") or std.mem.eql(u8, kind, "image")) {
            stmt = try self.prepare("DELETE FROM entries WHERE kind = ?");
            try bindText(stmt, 1, kind);
        } else {
            stmt = try self.prepare("DELETE FROM entries");
        }
        defer _ = c.sqlite3_finalize(stmt);
        try self.stepDone(stmt);
        return c.sqlite3_changes(self.db);
    }

    pub fn stats(self: *Storage) !Stats {
        return .{
            .entries = try self.scalarCount("SELECT count(*) FROM entries"),
            .text = try self.scalarCount("SELECT count(*) FROM entries WHERE kind = 'text'"),
            .images = try self.scalarCount("SELECT count(*) FROM entries WHERE kind = 'image'"),
            .favorites = try self.scalarCount("SELECT count(*) FROM entries WHERE favorite = 1"),
        };
    }

    pub fn repair(self: *Storage) !struct { ok: bool, pruned_blobs: i64 } {
        try self.exec("INSERT INTO pending_blob_prunes(path) SELECT blob_path FROM entries WHERE kind = 'image' AND blob_path IS NOT NULL AND blob_path = ''");
        return .{ .ok = true, .pruned_blobs = 0 };
    }

    fn prune(self: *Storage, max_entries: u32) !void {
        const stmt = try self.prepare(
            \\DELETE FROM entries
            \\WHERE favorite = 0
            \\  AND id NOT IN (
            \\    SELECT id FROM entries ORDER BY favorite DESC, created_at_ms DESC, id DESC LIMIT ?
            \\  )
        );
        defer _ = c.sqlite3_finalize(stmt);
        try bindInt64(stmt, 1, max_entries);
        try self.stepDone(stmt);
    }

    fn findIdByHash(self: *Storage, hash: []const u8) !?i64 {
        const stmt = try self.prepare("SELECT id FROM entries WHERE hash = ? ORDER BY id DESC LIMIT 1");
        defer _ = c.sqlite3_finalize(stmt);
        try bindText(stmt, 1, hash);

        const rc = c.sqlite3_step(stmt);
        if (rc == c.SQLITE_DONE) return null;
        if (rc != c.SQLITE_ROW) return self.sqliteError();
        return c.sqlite3_column_int64(stmt, 0);
    }

    fn scalarCount(self: *Storage, sql: []const u8) !i64 {
        const stmt = try self.prepare(sql);
        defer _ = c.sqlite3_finalize(stmt);
        const rc = c.sqlite3_step(stmt);
        if (rc != c.SQLITE_ROW) return self.sqliteError();
        return c.sqlite3_column_int64(stmt, 0);
    }

    fn readEntry(self: *Storage, stmt: *c.sqlite3_stmt) !models.Entry {
        return .{
            .id = c.sqlite3_column_int64(stmt, 0),
            .kind = try columnTextDup(self.allocator, stmt, 1),
            .mime = try columnTextDup(self.allocator, stmt, 2),
            .content = try columnTextDup(self.allocator, stmt, 3),
            .preview = try columnTextDup(self.allocator, stmt, 4),
            .hash = try columnTextDup(self.allocator, stmt, 5),
            .favorite = c.sqlite3_column_int(stmt, 6) != 0,
            .created_at_ms = c.sqlite3_column_int64(stmt, 7),
            .byte_len = c.sqlite3_column_int64(stmt, 8),
            .source_app = try columnTextDupOptional(self.allocator, stmt, 9),
            .blob_path = try columnTextDupOptional(self.allocator, stmt, 10),
        };
    }

    fn exec(self: *Storage, sql: []const u8) !void {
        const sql_z = try self.allocator.dupeZ(u8, sql);
        defer self.allocator.free(sql_z);
        if (c.sqlite3_exec(self.db, sql_z.ptr, null, null, null) != c.SQLITE_OK) return self.sqliteError();
    }

    fn prepare(self: *Storage, sql: []const u8) !*c.sqlite3_stmt {
        const sql_z = try self.allocator.dupeZ(u8, sql);
        defer self.allocator.free(sql_z);
        var stmt: ?*c.sqlite3_stmt = null;
        if (c.sqlite3_prepare_v2(self.db, sql_z.ptr, @intCast(sql.len), &stmt, null) != c.SQLITE_OK) return self.sqliteError();
        return stmt.?;
    }

    fn stepDone(self: *Storage, stmt: *c.sqlite3_stmt) !void {
        const rc = c.sqlite3_step(stmt);
        if (rc != c.SQLITE_DONE) return self.sqliteError();
    }

    fn sqliteError(self: *Storage) error{SQLiteFailure} {
        const message = std.mem.span(c.sqlite3_errmsg(self.db));
        std.debug.print("sqlite: {s}\n", .{message});
        return error.SQLiteFailure;
    }
};

fn bindText(stmt: *c.sqlite3_stmt, index: c_int, value: []const u8) !void {
    if (c.sqlite3_bind_text(stmt, index, value.ptr, @intCast(value.len), c.SQLITE_TRANSIENT) != c.SQLITE_OK) {
        return error.SQLiteFailure;
    }
}

fn bindInt64(stmt: *c.sqlite3_stmt, index: c_int, value: i64) !void {
    if (c.sqlite3_bind_int64(stmt, index, value) != c.SQLITE_OK) {
        return error.SQLiteFailure;
    }
}

fn columnTextDup(allocator: std.mem.Allocator, stmt: *c.sqlite3_stmt, index: c_int) ![]const u8 {
    const bytes = c.sqlite3_column_bytes(stmt, index);
    const ptr = c.sqlite3_column_text(stmt, index) orelse return allocator.dupe(u8, "");
    return allocator.dupe(u8, @as([*]const u8, @ptrCast(ptr))[0..@intCast(bytes)]);
}

fn columnTextDupOptional(allocator: std.mem.Allocator, stmt: *c.sqlite3_stmt, index: c_int) !?[]const u8 {
    if (c.sqlite3_column_type(stmt, index) == c.SQLITE_NULL) return null;
    return try columnTextDup(allocator, stmt, index);
}

const schema =
    \\PRAGMA journal_mode = WAL;
    \\PRAGMA foreign_keys = ON;
    \\CREATE TABLE IF NOT EXISTS entries (
    \\  id INTEGER PRIMARY KEY AUTOINCREMENT,
    \\  kind TEXT NOT NULL CHECK (kind IN ('text', 'image')),
    \\  mime TEXT NOT NULL,
    \\  content TEXT NOT NULL,
    \\  preview TEXT NOT NULL,
    \\  hash TEXT NOT NULL,
    \\  favorite INTEGER NOT NULL DEFAULT 0,
    \\  created_at_ms INTEGER NOT NULL,
    \\  byte_len INTEGER NOT NULL,
    \\  source_app TEXT,
    \\  blob_path TEXT
    \\);
    \\CREATE INDEX IF NOT EXISTS idx_entries_hash ON entries(hash);
    \\CREATE INDEX IF NOT EXISTS idx_entries_created ON entries(created_at_ms DESC);
    \\CREATE INDEX IF NOT EXISTS idx_entries_favorite ON entries(favorite);
    \\CREATE VIRTUAL TABLE IF NOT EXISTS entry_fts USING fts5(content, preview, source_app, content='entries', content_rowid='id');
    \\CREATE TRIGGER IF NOT EXISTS entries_ai AFTER INSERT ON entries BEGIN
    \\  INSERT INTO entry_fts(rowid, content, preview, source_app) VALUES (new.id, new.content, new.preview, new.source_app);
    \\END;
    \\CREATE TRIGGER IF NOT EXISTS entries_ad AFTER DELETE ON entries BEGIN
    \\  INSERT INTO entry_fts(entry_fts, rowid, content, preview, source_app) VALUES ('delete', old.id, old.content, old.preview, old.source_app);
    \\END;
    \\CREATE TRIGGER IF NOT EXISTS entries_au AFTER UPDATE ON entries BEGIN
    \\  INSERT INTO entry_fts(entry_fts, rowid, content, preview, source_app) VALUES ('delete', old.id, old.content, old.preview, old.source_app);
    \\  INSERT INTO entry_fts(rowid, content, preview, source_app) VALUES (new.id, new.content, new.preview, new.source_app);
    \\END;
    \\CREATE TABLE IF NOT EXISTS pending_blob_prunes (
    \\  path TEXT PRIMARY KEY,
    \\  created_at_ms INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER) * 1000)
    \\);
;

