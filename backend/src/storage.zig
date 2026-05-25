const std = @import("std");
const models = @import("models.zig");
const util = @import("util.zig");
const config = @import("config.zig");

const c = @cImport({
    @cInclude("sqlite3.h");
    @cInclude("unistd.h");
});

pub const Stats = struct {
    entries: i64,
    text: i64,
    images: i64,
    favorites: i64,
};

pub const Storage = struct {
    allocator: std.mem.Allocator,
    io: std.Io,
    db: *c.sqlite3,

    pub fn open(allocator: std.mem.Allocator, cfg: config.Config, io: std.Io) !Storage {
        var db: ?*c.sqlite3 = null;
        const path_z = try allocator.dupeZ(u8, cfg.db_path);
        defer allocator.free(path_z);

        if (c.sqlite3_open(path_z.ptr, &db) != c.SQLITE_OK) return error.SQLiteOpenFailed;
        var storage = Storage{ .allocator = allocator, .io = io, .db = db.? };
        errdefer storage.close();

        try storage.migrate();
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

    pub fn addImage(self: *Storage, cfg: config.Config, mime: []const u8, bytes: []const u8, metadata: models.ImageMetadata) !i64 {
        const hash = util.sha256Hex(bytes);
        if (!cfg.allow_duplicates) {
            if (try self.findIdByHash(&hash)) |existing| return existing;
        }

        const ext = imageExtension(mime);
        const blob_path = try self.writeImageBlob(cfg, &hash, ext, bytes);
        defer self.allocator.free(blob_path);

        const entry_preview = try imagePreview(self.allocator, mime, bytes.len, metadata);
        defer self.allocator.free(entry_preview);

        const stmt = try self.prepare(
            \\INSERT INTO entries
            \\  (kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, blob_path, image_width, image_height)
            \\VALUES
            \\  ('image', ?, ?, ?, ?, 0, CAST(strftime('%s','now') AS INTEGER) * 1000, ?, ?, ?, ?)
        );
        defer _ = c.sqlite3_finalize(stmt);

        try bindText(stmt, 1, mime);
        try bindText(stmt, 2, &hash);
        try bindText(stmt, 3, entry_preview);
        try bindText(stmt, 4, &hash);
        try bindInt64(stmt, 5, @intCast(bytes.len));
        try bindText(stmt, 6, blob_path);
        try bindNullableInt64(stmt, 7, metadata.width);
        try bindNullableInt64(stmt, 8, metadata.height);
        try self.stepDone(stmt);

        try self.prune(cfg.max_entries);
        return c.sqlite3_last_insert_rowid(self.db);
    }

    pub fn list(self: *Storage, query: []const u8, filter: []const u8, limit: u32) ![]models.Entry {
        var sql_writer = std.Io.Writer.Allocating.init(self.allocator);
        defer sql_writer.deinit();
        const w = &sql_writer.writer;

        var fts_query: ?[]u8 = null;
        defer if (fts_query) |value| self.allocator.free(value);
        const trimmed_query = std.mem.trim(u8, query, " \t\r\n");
        if (trimmed_query.len > 0) {
            fts_query = try buildFtsQuery(self.allocator, trimmed_query);
        }
        const has_query = fts_query != null and fts_query.?.len > 0;

        try w.writeAll(
            \\SELECT entries.id, entries.kind, entries.mime, entries.content, entries.preview, entries.hash, entries.favorite, entries.created_at_ms, entries.byte_len, entries.source_app, entries.blob_path, entries.image_width, entries.image_height
            \\FROM entries
        );

        if (has_query) try w.writeAll(" JOIN entry_fts ON entry_fts.rowid = entries.id");
        try w.writeAll(" WHERE 1 = 1");
        if (has_query) try w.writeAll(" AND entry_fts MATCH ?");
        if (std.mem.eql(u8, filter, "text")) try w.writeAll(" AND kind = 'text'");
        if (std.mem.eql(u8, filter, "images")) try w.writeAll(" AND kind = 'image'");
        if (std.mem.eql(u8, filter, "favorites")) try w.writeAll(" AND favorite = 1");
        if (std.mem.eql(u8, filter, "today")) try w.writeAll(" AND created_at_ms >= (CAST(strftime('%s','now','start of day') AS INTEGER) * 1000)");
        try w.writeAll(" ORDER BY favorite DESC, created_at_ms DESC, id DESC LIMIT ?");

        const stmt = try self.prepare(sql_writer.written());
        defer _ = c.sqlite3_finalize(stmt);

        var bind_index: c_int = 1;
        if (has_query) {
            try bindText(stmt, bind_index, fts_query.?);
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
            \\SELECT id, kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, source_app, blob_path, image_width, image_height
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
        if (std.mem.eql(u8, kind, "text") or std.mem.eql(u8, kind, "image") or std.mem.eql(u8, kind, "images")) {
            stmt = try self.prepare("DELETE FROM entries WHERE kind = ?");
            try bindText(stmt, 1, if (std.mem.eql(u8, kind, "images")) "image" else kind);
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

    pub fn pauseWatcher(self: *Storage, duration_ms: u32) !void {
        if (duration_ms == 0) {
            try self.setRuntimeInt("paused_until_ms", -1);
            return;
        }
        try self.setRuntimeInt("paused_until_ms", try self.nowMs() + duration_ms);
    }

    pub fn resumeWatcher(self: *Storage) !void {
        try self.setRuntimeInt("paused_until_ms", 0);
    }

    pub fn isWatcherPaused(self: *Storage) !bool {
        const paused_until = try self.getRuntimeInt("paused_until_ms") orelse return false;
        if (paused_until < 0) return true;
        return paused_until > try self.nowMs();
    }

    pub fn markWatcherSeen(self: *Storage) !void {
        try self.setRuntimeInt("watcher_last_seen_ms", try self.nowMs());
    }

    pub fn watcherLastSeen(self: *Storage) !?i64 {
        return try self.getRuntimeInt("watcher_last_seen_ms");
    }

    pub fn markSelfWrite(self: *Storage, hash: []const u8) !void {
        try self.setRuntimeText("last_self_hash", hash);
    }

    pub fn selfWriteHash(self: *Storage) !?[]const u8 {
        return try self.getRuntimeText("last_self_hash");
    }

    pub fn clearSelfWriteHash(self: *Storage) !void {
        try self.deleteRuntime("last_self_hash");
    }

    pub fn takeSelfWriteHash(self: *Storage) !?[]const u8 {
        const value = try self.getRuntimeText("last_self_hash");
        if (value != null) try self.deleteRuntime("last_self_hash");
        return value;
    }

    pub fn selectedContents(self: *Storage, ids: []const i64) ![]u8 {
        var out = std.Io.Writer.Allocating.init(self.allocator);
        errdefer out.deinit();
        var written: usize = 0;
        for (ids) |id| {
            const entry = (try self.get(id)) orelse continue;
            defer entry.deinit(self.allocator);
            if (entry.kind.len != 4 or !std.mem.eql(u8, entry.kind, "text")) continue;
            if (written > 0) try out.writer.writeByte('\n');
            try out.writer.writeAll(entry.content);
            written += 1;
        }
        return out.toOwnedSlice();
    }

    fn migrate(self: *Storage) !void {
        try self.exec("PRAGMA journal_mode = WAL");
        try self.exec("PRAGMA foreign_keys = ON");
        try self.exec(create_entries_schema);
        try self.ensureColumn("entries", "source_app", "ALTER TABLE entries ADD COLUMN source_app TEXT");
        try self.ensureColumn("entries", "blob_path", "ALTER TABLE entries ADD COLUMN blob_path TEXT");
        try self.ensureColumn("entries", "image_width", "ALTER TABLE entries ADD COLUMN image_width INTEGER");
        try self.ensureColumn("entries", "image_height", "ALTER TABLE entries ADD COLUMN image_height INTEGER");
        try self.exec(rest_schema);
        try self.exec("INSERT INTO entry_fts(entry_fts) VALUES('rebuild')");
        try self.exec("PRAGMA user_version = 2");
    }

    fn ensureColumn(self: *Storage, table: []const u8, column: []const u8, sql: []const u8) !void {
        if (!try self.columnExists(table, column)) try self.exec(sql);
    }

    fn columnExists(self: *Storage, table: []const u8, column: []const u8) !bool {
        const sql = try std.fmt.allocPrint(self.allocator, "PRAGMA table_info({s})", .{table});
        defer self.allocator.free(sql);
        const stmt = try self.prepare(sql);
        defer _ = c.sqlite3_finalize(stmt);

        while (true) {
            const rc = c.sqlite3_step(stmt);
            if (rc == c.SQLITE_DONE) return false;
            if (rc != c.SQLITE_ROW) return self.sqliteError();
            const name = try columnTextDup(self.allocator, stmt, 1);
            defer self.allocator.free(name);
            if (std.mem.eql(u8, name, column)) return true;
        }
    }

    fn writeImageBlob(self: *Storage, cfg: config.Config, hash: []const u8, ext: []const u8, bytes: []const u8) ![]u8 {
        const shard = hash[0..2];
        const dir_path = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ cfg.image_dir, shard });
        defer self.allocator.free(dir_path);
        try std.Io.Dir.cwd().createDirPath(self.io, dir_path);

        const final_path = try std.fmt.allocPrint(self.allocator, "{s}/{s}.{s}", .{ dir_path, hash, ext });
        errdefer self.allocator.free(final_path);
        const tmp_path = try std.fmt.allocPrint(self.allocator, "{s}.tmp-{}", .{ final_path, try self.nowMs() });
        defer self.allocator.free(tmp_path);
        errdefer deleteFilePath(self.io, tmp_path);

        {
            var file = try std.Io.Dir.createFileAbsolute(self.io, tmp_path, .{ .exclusive = true });
            defer file.close(self.io);
            try file.writeStreamingAll(self.io, bytes);
            try file.sync(self.io);
        }

        if (std.fs.path.isAbsolute(tmp_path)) {
            try std.Io.Dir.renameAbsolute(tmp_path, final_path, self.io);
        } else {
            const cwd = std.Io.Dir.cwd();
            try cwd.rename(tmp_path, cwd, final_path, self.io);
        }

        var parent_dir = if (std.fs.path.isAbsolute(dir_path))
            try std.Io.Dir.openDirAbsolute(self.io, dir_path, .{})
        else
            try std.Io.Dir.cwd().openDir(self.io, dir_path, .{});
        defer parent_dir.close(self.io);
        _ = c.fsync(parent_dir.handle);
        return final_path;
    }

    fn setRuntimeText(self: *Storage, key: []const u8, value: []const u8) !void {
        const stmt = try self.prepare(
            \\INSERT INTO runtime_state(key, value, updated_at_ms)
            \\VALUES (?, ?, CAST(strftime('%s','now') AS INTEGER) * 1000)
            \\ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at_ms = excluded.updated_at_ms
        );
        defer _ = c.sqlite3_finalize(stmt);
        try bindText(stmt, 1, key);
        try bindText(stmt, 2, value);
        try self.stepDone(stmt);
    }

    fn setRuntimeInt(self: *Storage, key: []const u8, value: i64) !void {
        const text = try std.fmt.allocPrint(self.allocator, "{}", .{value});
        defer self.allocator.free(text);
        try self.setRuntimeText(key, text);
    }

    fn getRuntimeText(self: *Storage, key: []const u8) !?[]const u8 {
        const stmt = try self.prepare("SELECT value FROM runtime_state WHERE key = ?");
        defer _ = c.sqlite3_finalize(stmt);
        try bindText(stmt, 1, key);
        const rc = c.sqlite3_step(stmt);
        if (rc == c.SQLITE_DONE) return null;
        if (rc != c.SQLITE_ROW) return self.sqliteError();
        return try columnTextDup(self.allocator, stmt, 0);
    }

    fn getRuntimeInt(self: *Storage, key: []const u8) !?i64 {
        const text = try self.getRuntimeText(key) orelse return null;
        defer self.allocator.free(text);
        return std.fmt.parseInt(i64, text, 10) catch null;
    }

    fn deleteRuntime(self: *Storage, key: []const u8) !void {
        const stmt = try self.prepare("DELETE FROM runtime_state WHERE key = ?");
        defer _ = c.sqlite3_finalize(stmt);
        try bindText(stmt, 1, key);
        try self.stepDone(stmt);
    }

    pub fn nowMs(self: *Storage) !i64 {
        return try self.scalarCount("SELECT CAST(strftime('%s','now') AS INTEGER) * 1000");
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
            .image_width = columnNullableInt64(stmt, 11),
            .image_height = columnNullableInt64(stmt, 12),
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

fn bindNullableInt64(stmt: *c.sqlite3_stmt, index: c_int, value: ?i64) !void {
    if (value) |actual| return bindInt64(stmt, index, actual);
    if (c.sqlite3_bind_null(stmt, index) != c.SQLITE_OK) return error.SQLiteFailure;
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

fn columnNullableInt64(stmt: *c.sqlite3_stmt, index: c_int) ?i64 {
    if (c.sqlite3_column_type(stmt, index) == c.SQLITE_NULL) return null;
    return c.sqlite3_column_int64(stmt, index);
}

fn buildFtsQuery(allocator: std.mem.Allocator, query: []const u8) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();

    var parts = std.mem.tokenizeAny(u8, query, " \t\r\n");
    var first = true;
    while (parts.next()) |raw_token| {
        const token = std.mem.trim(u8, raw_token, "\"'");
        if (token.len == 0) continue;
        if (!first) try out.writer.writeAll(" AND ");
        first = false;
        try out.writer.writeByte('"');
        for (token) |char| {
            if (char == '"') {
                try out.writer.writeAll("\"\"");
            } else {
                try out.writer.writeByte(char);
            }
        }
        try out.writer.writeByte('"');
    }

    return out.toOwnedSlice();
}

fn imageExtension(mime: []const u8) []const u8 {
    if (std.mem.eql(u8, mime, "image/png")) return "png";
    if (std.mem.eql(u8, mime, "image/jpeg")) return "jpg";
    if (std.mem.eql(u8, mime, "image/webp")) return "webp";
    if (std.mem.eql(u8, mime, "image/gif")) return "gif";
    if (std.mem.eql(u8, mime, "image/bmp")) return "bmp";
    return "bin";
}

fn imagePreview(allocator: std.mem.Allocator, mime: []const u8, byte_len: usize, metadata: models.ImageMetadata) ![]u8 {
    if (metadata.width != null and metadata.height != null) {
        return std.fmt.allocPrint(allocator, "Image {s} {} bytes {}x{}", .{ mime, byte_len, metadata.width.?, metadata.height.? });
    }
    return std.fmt.allocPrint(allocator, "Image {s} {} bytes", .{ mime, byte_len });
}

fn deleteFilePath(io: std.Io, path: []const u8) void {
    if (std.fs.path.isAbsolute(path)) {
        std.Io.Dir.deleteFileAbsolute(io, path) catch {};
    } else {
        std.Io.Dir.cwd().deleteFile(io, path) catch {};
    }
}

const create_entries_schema =
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
    \\  blob_path TEXT,
    \\  image_width INTEGER,
    \\  image_height INTEGER
    \\);
;

const rest_schema =
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
    \\CREATE TABLE IF NOT EXISTS runtime_state (
    \\  key TEXT PRIMARY KEY,
    \\  value TEXT NOT NULL,
    \\  updated_at_ms INTEGER NOT NULL
    \\);
;

test "storage covers add list search get favorite delete clear repair and images" {
    const allocator = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const data_dir = try std.fmt.allocPrint(allocator, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer allocator.free(data_dir);

    var cfg = try config.testConfig(allocator, data_dir);
    defer cfg.deinit();
    try std.Io.Dir.cwd().createDirPath(std.testing.io, cfg.image_dir);

    var store = try Storage.open(allocator, cfg, std.testing.io);
    defer store.close();

    const text_id = try store.addText(cfg, "hello searchable world");
    const second_id = try store.addText(cfg, "another entry");
    try std.testing.expect(text_id > 0);
    try std.testing.expect(second_id > text_id);

    {
        const entries = try store.list("", "all", 10);
        defer {
            for (entries) |entry| entry.deinit(allocator);
            allocator.free(entries);
        }
        try std.testing.expectEqual(@as(usize, 2), entries.len);
    }

    {
        const entries = try store.list("searchable", "all", 10);
        defer {
            for (entries) |entry| entry.deinit(allocator);
            allocator.free(entries);
        }
        try std.testing.expectEqual(@as(usize, 1), entries.len);
        try std.testing.expectEqual(text_id, entries[0].id);
    }

    {
        const entry = (try store.get(text_id)) orelse return error.TestUnexpectedResult;
        defer entry.deinit(allocator);
        try std.testing.expectEqualStrings("hello searchable world", entry.content);
    }

    try std.testing.expect(try store.favorite(text_id, true));
    {
        const stats = try store.stats();
        try std.testing.expectEqual(@as(i64, 1), stats.favorites);
    }

    const image_id = try store.addImage(cfg, "image/png", "not-really-png", .{ .width = 2, .height = 3 });
    {
        const entry = (try store.get(image_id)) orelse return error.TestUnexpectedResult;
        defer entry.deinit(allocator);
        try std.testing.expectEqualStrings("image", entry.kind);
        try std.testing.expectEqual(@as(?i64, 2), entry.image_width);
        try std.testing.expect(entry.blob_path != null);
    }

    try std.testing.expect(try store.delete(second_id));
    try std.testing.expectEqual(@as(i64, 1), try store.clear("images"));
    try std.testing.expectEqual(@as(i64, 1), try store.clear("text"));
    _ = try store.addText(cfg, "final clear target");
    try std.testing.expectEqual(@as(i64, 1), try store.clear("all"));
    _ = try store.repair();
}

test "storage runtime state supports watcher pause and self-write guard" {
    const allocator = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const data_dir = try std.fmt.allocPrint(allocator, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer allocator.free(data_dir);

    var cfg = try config.testConfig(allocator, data_dir);
    defer cfg.deinit();
    try std.Io.Dir.cwd().createDirPath(std.testing.io, cfg.image_dir);

    var store = try Storage.open(allocator, cfg, std.testing.io);
    defer store.close();

    try std.testing.expect(!try store.isWatcherPaused());
    try store.pauseWatcher(60_000);
    try std.testing.expect(try store.isWatcherPaused());
    try store.resumeWatcher();
    try std.testing.expect(!try store.isWatcherPaused());

    try store.markWatcherSeen();
    try std.testing.expect((try store.watcherLastSeen()) != null);

    try store.markSelfWrite("abc");
    const hash = (try store.takeSelfWriteHash()) orelse return error.TestUnexpectedResult;
    defer allocator.free(hash);
    try std.testing.expectEqualStrings("abc", hash);
    try std.testing.expect((try store.takeSelfWriteHash()) == null);
}
