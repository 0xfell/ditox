const std = @import("std");
const models = @import("models.zig");
const util = @import("util.zig");
const config = @import("config.zig");

const c = @cImport({
    @cInclude("sqlite3.h");
    @cInclude("unistd.h");
});

pub const current_schema_version: i64 = 2;

pub const Stats = struct {
    entries: i64,
    text: i64,
    images: i64,
    favorites: i64,
};

pub const RepairResult = struct {
    ok: bool,
    pruned_blobs: i64,
    sanitized_text: i64,
    removed_missing_images: i64,
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

        try storage.registerSqlFunctions();
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

        _ = try self.applyRetention(cfg);
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

        _ = try self.applyRetention(cfg);
        return c.sqlite3_last_insert_rowid(self.db);
    }

    pub fn list(self: *Storage, query: []const u8, filter: []const u8, limit: u32) ![]models.Entry {
        var sql_writer = std.Io.Writer.Allocating.init(self.allocator);
        defer sql_writer.deinit();
        const w = &sql_writer.writer;

        var fts_query: ?[]u8 = null;
        defer if (fts_query) |value| self.allocator.free(value);
        var like_query: ?[]u8 = null;
        defer if (like_query) |value| self.allocator.free(value);
        var prefix_query: ?[]u8 = null;
        defer if (prefix_query) |value| self.allocator.free(value);
        const trimmed_query = std.mem.trim(u8, query, " \t\r\n");
        if (trimmed_query.len > 0) {
            fts_query = try buildFtsQuery(self.allocator, trimmed_query);
            like_query = try buildLikeContainsQuery(self.allocator, trimmed_query);
            prefix_query = try buildLikePrefixQuery(self.allocator, trimmed_query);
        }
        const has_query = fts_query != null and fts_query.?.len > 0;

        try w.writeAll(
            \\SELECT entries.id, entries.kind, entries.mime, entries.content, entries.preview, entries.hash, entries.favorite, entries.created_at_ms, entries.byte_len, entries.source_app, entries.blob_path, entries.image_width, entries.image_height
            \\FROM entries
        );

        try w.writeAll(" WHERE 1 = 1");
        if (has_query) try w.writeAll(
            \\ AND (
            \\   entries.id IN (SELECT rowid FROM entry_fts WHERE entry_fts MATCH ?)
            \\   OR LOWER(entries.content) LIKE LOWER(?) ESCAPE '\'
            \\   OR LOWER(entries.preview) LIKE LOWER(?) ESCAPE '\'
            \\   OR ditox_fuzzy_score(entries.content, ?) > 0
            \\   OR ditox_fuzzy_score(entries.preview, ?) > 0
            \\ )
        );
        if (std.mem.eql(u8, filter, "text")) try w.writeAll(" AND kind = 'text'");
        if (std.mem.eql(u8, filter, "images")) try w.writeAll(" AND kind = 'image'");
        if (std.mem.eql(u8, filter, "favorites")) try w.writeAll(" AND favorite = 1");
        if (std.mem.eql(u8, filter, "today")) try w.writeAll(" AND created_at_ms >= (CAST(strftime('%s','now','start of day') AS INTEGER) * 1000)");
        if (has_query) {
            try w.writeAll(
                \\ ORDER BY favorite DESC,
                \\ CASE
                \\   WHEN LOWER(entries.content) = LOWER(?) OR LOWER(entries.preview) = LOWER(?) THEN 0
                \\   WHEN LOWER(entries.content) LIKE LOWER(?) ESCAPE '\' OR LOWER(entries.preview) LIKE LOWER(?) ESCAPE '\' THEN 1
                \\   WHEN LOWER(entries.content) LIKE LOWER(?) ESCAPE '\' THEN 2
                \\   WHEN LOWER(entries.preview) LIKE LOWER(?) ESCAPE '\' THEN 3
                \\   WHEN ditox_fuzzy_score(entries.content, ?) > 0 OR ditox_fuzzy_score(entries.preview, ?) > 0 THEN 4
                \\   ELSE 5
                \\ END ASC,
                \\ max(ditox_fuzzy_score(entries.content, ?), ditox_fuzzy_score(entries.preview, ?)) DESC,
                \\ created_at_ms DESC, id DESC LIMIT ?
            );
        } else {
            try w.writeAll(" ORDER BY favorite DESC, created_at_ms DESC, id DESC LIMIT ?");
        }

        const stmt = try self.prepare(sql_writer.written());
        defer _ = c.sqlite3_finalize(stmt);

        var bind_index: c_int = 1;
        if (has_query) {
            try bindText(stmt, bind_index, fts_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, like_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, like_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, prefix_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, prefix_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, like_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, like_query.?);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
            bind_index += 1;
            try bindText(stmt, bind_index, trimmed_query);
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
        const changed = c.sqlite3_changes(self.db);
        if (changed > 0) _ = try self.prunePendingBlobs();
        return changed > 0;
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
        return self.clearWithOptions(kind, false);
    }

    pub fn clearWithOptions(self: *Storage, kind: []const u8, preserve_favorites: bool) !i64 {
        var stmt: *c.sqlite3_stmt = undefined;
        if (std.mem.eql(u8, kind, "text") or std.mem.eql(u8, kind, "image") or std.mem.eql(u8, kind, "images")) {
            stmt = if (preserve_favorites)
                try self.prepare("DELETE FROM entries WHERE kind = ? AND favorite = 0")
            else
                try self.prepare("DELETE FROM entries WHERE kind = ?");
            try bindText(stmt, 1, if (std.mem.eql(u8, kind, "images")) "image" else kind);
        } else {
            stmt = if (preserve_favorites)
                try self.prepare("DELETE FROM entries WHERE favorite = 0")
            else
                try self.prepare("DELETE FROM entries");
        }
        defer _ = c.sqlite3_finalize(stmt);
        try self.stepDone(stmt);
        const changed = c.sqlite3_changes(self.db);
        if (changed > 0) _ = try self.prunePendingBlobs();
        return changed;
    }

    pub fn stats(self: *Storage) !Stats {
        return .{
            .entries = try self.scalarCount("SELECT count(*) FROM entries"),
            .text = try self.scalarCount("SELECT count(*) FROM entries WHERE kind = 'text'"),
            .images = try self.scalarCount("SELECT count(*) FROM entries WHERE kind = 'image'"),
            .favorites = try self.scalarCount("SELECT count(*) FROM entries WHERE favorite = 1"),
        };
    }

    pub fn repair(self: *Storage, cfg: config.Config) !RepairResult {
        const sanitized_text = try self.sanitizeTextEntries(cfg);
        const removed_missing_images = try self.removeMissingImageEntries();
        return .{
            .ok = true,
            .pruned_blobs = try self.prunePendingBlobs(),
            .sanitized_text = sanitized_text,
            .removed_missing_images = removed_missing_images,
        };
    }

    pub fn schemaVersion(self: *Storage) !i64 {
        return try self.scalarCount("PRAGMA user_version");
    }

    pub fn applyRetention(self: *Storage, cfg: config.Config) !i64 {
        var changed: i64 = 0;
        if (cfg.delete_after_seconds > 0) changed += try self.deleteExpired(cfg.delete_after_seconds);
        changed += try self.prune(cfg.max_entries);
        if (changed > 0) _ = try self.prunePendingBlobs();
        return changed;
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

    pub fn markWatcherPid(self: *Storage, pid: i64) !void {
        try self.setRuntimeInt("watcher_pid", pid);
    }

    pub fn watcherPid(self: *Storage) !?i64 {
        return try self.getRuntimeInt("watcher_pid");
    }

    pub fn clearWatcherPid(self: *Storage) !void {
        try self.deleteRuntime("watcher_pid");
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

        const version = try self.schemaVersion();
        if (version > current_schema_version) return error.UnsupportedSchemaVersion;

        if (version < current_schema_version) {
            try self.exec("BEGIN IMMEDIATE");
            var committed = false;
            errdefer if (!committed) self.exec("ROLLBACK") catch {};

            if (version < 1) try self.migrateToV1();
            if (version < 2) try self.migrateToV2();
            try self.setSchemaVersion(current_schema_version);

            try self.exec("COMMIT");
            committed = true;
        }

        try self.exec(rest_schema);
        try self.exec("INSERT INTO entry_fts(entry_fts) VALUES('rebuild')");
    }

    fn migrateToV1(self: *Storage) !void {
        try self.exec(v1_schema);
        try self.ensureColumn("entries", "source_app", "ALTER TABLE entries ADD COLUMN source_app TEXT");
    }

    fn migrateToV2(self: *Storage) !void {
        try self.ensureColumn("entries", "blob_path", "ALTER TABLE entries ADD COLUMN blob_path TEXT");
        try self.ensureColumn("entries", "image_width", "ALTER TABLE entries ADD COLUMN image_width INTEGER");
        try self.ensureColumn("entries", "image_height", "ALTER TABLE entries ADD COLUMN image_height INTEGER");
        try self.resetSearchSchema();
    }

    fn resetSearchSchema(self: *Storage) !void {
        try self.exec(
            \\DROP TRIGGER IF EXISTS entries_ai;
            \\DROP TRIGGER IF EXISTS entries_ad;
            \\DROP TRIGGER IF EXISTS entries_au;
            \\DROP TABLE IF EXISTS entry_fts;
        );
    }

    fn setSchemaVersion(self: *Storage, version: i64) !void {
        const sql = try std.fmt.allocPrint(self.allocator, "PRAGMA user_version = {}", .{version});
        defer self.allocator.free(sql);
        try self.exec(sql);
    }

    fn registerSqlFunctions(self: *Storage) !void {
        const rc = c.sqlite3_create_function_v2(
            self.db,
            "ditox_fuzzy_score",
            2,
            c.SQLITE_UTF8 | c.SQLITE_DETERMINISTIC,
            null,
            fuzzyScoreSqlite,
            null,
            null,
            null,
        );
        if (rc != c.SQLITE_OK) return self.sqliteError();
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

    fn prune(self: *Storage, max_entries: u32) !i64 {
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
        return c.sqlite3_changes(self.db);
    }

    fn deleteExpired(self: *Storage, delete_after_seconds: u32) !i64 {
        const cutoff = try self.nowMs() - (@as(i64, delete_after_seconds) * 1000);
        const stmt = try self.prepare(
            \\DELETE FROM entries
            \\WHERE favorite = 0
            \\  AND created_at_ms < ?
        );
        defer _ = c.sqlite3_finalize(stmt);
        try bindInt64(stmt, 1, cutoff);
        try self.stepDone(stmt);
        return c.sqlite3_changes(self.db);
    }

    fn prunePendingBlobs(self: *Storage) !i64 {
        var paths: std.ArrayList([]const u8) = .empty;
        defer {
            for (paths.items) |path| self.allocator.free(path);
            paths.deinit(self.allocator);
        }

        {
            const stmt = try self.prepare("SELECT path FROM pending_blob_prunes ORDER BY created_at_ms ASC");
            defer _ = c.sqlite3_finalize(stmt);
            while (true) {
                const rc = c.sqlite3_step(stmt);
                if (rc == c.SQLITE_DONE) break;
                if (rc != c.SQLITE_ROW) return self.sqliteError();
                try paths.append(self.allocator, try columnTextDup(self.allocator, stmt, 0));
            }
        }

        var pruned: i64 = 0;
        for (paths.items) |path| {
            deleteFilePath(self.io, path);
            const stmt = try self.prepare("DELETE FROM pending_blob_prunes WHERE path = ?");
            defer _ = c.sqlite3_finalize(stmt);
            try bindText(stmt, 1, path);
            try self.stepDone(stmt);
            pruned += c.sqlite3_changes(self.db);
        }
        return pruned;
    }

    fn sanitizeTextEntries(self: *Storage, cfg: config.Config) !i64 {
        const Row = struct {
            id: i64,
            content: []const u8,
            preview: []const u8,
            hash: []const u8,
            byte_len: i64,

            fn deinit(row: @This(), allocator: std.mem.Allocator) void {
                allocator.free(row.content);
                allocator.free(row.preview);
                allocator.free(row.hash);
            }
        };

        var rows: std.ArrayList(Row) = .empty;
        defer {
            for (rows.items) |row| row.deinit(self.allocator);
            rows.deinit(self.allocator);
        }

        {
            const stmt = try self.prepare("SELECT id, content, preview, hash, byte_len FROM entries WHERE kind = 'text'");
            defer _ = c.sqlite3_finalize(stmt);
            while (true) {
                const rc = c.sqlite3_step(stmt);
                if (rc == c.SQLITE_DONE) break;
                if (rc != c.SQLITE_ROW) return self.sqliteError();
                try rows.append(self.allocator, .{
                    .id = c.sqlite3_column_int64(stmt, 0),
                    .content = try columnTextDup(self.allocator, stmt, 1),
                    .preview = try columnTextDup(self.allocator, stmt, 2),
                    .hash = try columnTextDup(self.allocator, stmt, 3),
                    .byte_len = c.sqlite3_column_int64(stmt, 4),
                });
            }
        }

        var changed: i64 = 0;
        for (rows.items) |row| {
            const content = try util.sanitizeText(self.allocator, row.content);
            defer self.allocator.free(content);
            const preview_text = try util.preview(self.allocator, content, cfg.max_preview_chars);
            defer self.allocator.free(preview_text);
            const hash = util.sha256Hex(content);
            const byte_len: i64 = @intCast(content.len);
            if (std.mem.eql(u8, row.content, content) and
                std.mem.eql(u8, row.preview, preview_text) and
                std.mem.eql(u8, row.hash, &hash) and
                row.byte_len == byte_len)
            {
                continue;
            }

            const stmt = try self.prepare(
                \\UPDATE entries
                \\SET content = ?, preview = ?, hash = ?, byte_len = ?
                \\WHERE id = ?
            );
            defer _ = c.sqlite3_finalize(stmt);
            try bindText(stmt, 1, content);
            try bindText(stmt, 2, preview_text);
            try bindText(stmt, 3, &hash);
            try bindInt64(stmt, 4, byte_len);
            try bindInt64(stmt, 5, row.id);
            try self.stepDone(stmt);
            changed += c.sqlite3_changes(self.db);
        }
        return changed;
    }

    fn removeMissingImageEntries(self: *Storage) !i64 {
        const Row = struct {
            id: i64,
            blob_path: ?[]const u8,

            fn deinit(row: @This(), allocator: std.mem.Allocator) void {
                if (row.blob_path) |path| allocator.free(path);
            }
        };

        var rows: std.ArrayList(Row) = .empty;
        defer {
            for (rows.items) |row| row.deinit(self.allocator);
            rows.deinit(self.allocator);
        }

        {
            const stmt = try self.prepare("SELECT id, blob_path FROM entries WHERE kind = 'image'");
            defer _ = c.sqlite3_finalize(stmt);
            while (true) {
                const rc = c.sqlite3_step(stmt);
                if (rc == c.SQLITE_DONE) break;
                if (rc != c.SQLITE_ROW) return self.sqliteError();
                try rows.append(self.allocator, .{
                    .id = c.sqlite3_column_int64(stmt, 0),
                    .blob_path = try columnTextDupOptional(self.allocator, stmt, 1),
                });
            }
        }

        var removed: i64 = 0;
        for (rows.items) |row| {
            const path = row.blob_path orelse "";
            if (path.len > 0 and fileExists(self.io, path)) continue;
            if (try self.delete(row.id)) removed += 1;
        }
        return removed;
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

fn buildLikeContainsQuery(allocator: std.mem.Allocator, query: []const u8) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();

    try out.writer.writeByte('%');
    for (query) |char| {
        if (char == '\\' or char == '%' or char == '_') try out.writer.writeByte('\\');
        try out.writer.writeByte(char);
    }
    try out.writer.writeByte('%');
    return out.toOwnedSlice();
}

fn buildLikePrefixQuery(allocator: std.mem.Allocator, query: []const u8) ![]u8 {
    var out = std.Io.Writer.Allocating.init(allocator);
    errdefer out.deinit();

    for (query) |char| {
        if (char == '\\' or char == '%' or char == '_') try out.writer.writeByte('\\');
        try out.writer.writeByte(char);
    }
    try out.writer.writeByte('%');
    return out.toOwnedSlice();
}

fn fuzzyScoreSqlite(context: ?*c.sqlite3_context, argc: c_int, argv: [*c]?*c.sqlite3_value) callconv(.c) void {
    if (context == null or argc != 2) return;
    const haystack = sqliteValueText(argv[0]) orelse {
        c.sqlite3_result_int64(context, 0);
        return;
    };
    const needle = sqliteValueText(argv[1]) orelse {
        c.sqlite3_result_int64(context, 0);
        return;
    };
    c.sqlite3_result_int64(context, fuzzyScore(haystack, needle));
}

fn sqliteValueText(value: ?*c.sqlite3_value) ?[]const u8 {
    const actual = value orelse return null;
    const ptr = c.sqlite3_value_text(actual) orelse return null;
    const len = c.sqlite3_value_bytes(actual);
    if (len <= 0) return "";
    return @as([*]const u8, @ptrCast(ptr))[0..@intCast(len)];
}

fn fuzzyScore(haystack: []const u8, needle_raw: []const u8) i64 {
    const needle = std.mem.trim(u8, needle_raw, " \t\r\n");
    if (haystack.len == 0 or needle.len == 0) return 0;
    return @max(fuzzyScoreMode(haystack, needle, false), fuzzyScoreMode(haystack, needle, true));
}

fn fuzzyScoreMode(haystack: []const u8, needle: []const u8, prefer_boundaries: bool) i64 {
    var search_start: usize = 0;
    var previous_match: ?usize = null;
    var first_match: ?usize = null;
    var last_match: ?usize = null;
    var run: i64 = 0;
    var longest_run: i64 = 0;
    var boundary_matches: i64 = 0;
    var score: i64 = 0;

    for (needle) |needle_byte| {
        const wanted = asciiLower(needle_byte);
        const found = findFuzzyMatch(haystack, wanted, search_start, previous_match, prefer_boundaries);

        const match_index = found orelse return 0;
        if (first_match == null) first_match = match_index;

        if (previous_match != null and match_index == previous_match.? + 1) {
            run += 1;
        } else {
            run = 1;
        }
        longest_run = @max(longest_run, run);

        score += 10 + (run * 12);
        const boundary_match = isMatchBoundary(haystack, match_index);
        if (boundary_match) {
            score += 18;
            boundary_matches += 1;
        }
        if (match_index < 8) score += @intCast(8 - match_index);
        if (previous_match) |previous| {
            const gap: i64 = @intCast(match_index - previous - 1);
            const gap_penalty = if (boundary_match) @min(gap * 2, 16) else @min(gap * 5, 32);
            score -= gap_penalty;
        }

        previous_match = match_index;
        last_match = match_index;
        search_start = match_index + 1;
    }

    if (first_match) |index| score -= @min(@as(i64, @intCast(index)), 24);
    if (first_match != null and last_match != null) {
        const span = last_match.? - first_match.? + 1;
        const span_extra: i64 = @intCast(span - needle.len);
        score -= @min(span_extra * 2, 40);
    }
    if (boundary_matches == @as(i64, @intCast(needle.len))) {
        score += 40 + (@as(i64, @intCast(needle.len)) * 4);
    }
    if (longest_run == @as(i64, @intCast(needle.len))) score += 80;
    score -= @min(@as(i64, @intCast(haystack.len / 24)), 12);
    return @max(score, 1);
}

fn findFuzzyMatch(haystack: []const u8, wanted: u8, search_start: usize, previous_match: ?usize, prefer_boundaries: bool) ?usize {
    var first: ?usize = null;
    var first_boundary: ?usize = null;
    var index = search_start;
    while (index < haystack.len) : (index += 1) {
        if (asciiLower(haystack[index]) != wanted) continue;
        if (previous_match) |previous| {
            if (index == previous + 1) return index;
        }
        if (first == null) first = index;
        if (prefer_boundaries and first_boundary == null and isMatchBoundary(haystack, index)) first_boundary = index;
    }
    return first_boundary orelse first;
}

fn asciiLower(byte: u8) u8 {
    return if (byte >= 'A' and byte <= 'Z') byte + ('a' - 'A') else byte;
}

fn isMatchBoundary(haystack: []const u8, index: usize) bool {
    if (index == 0) return true;
    const previous = haystack[index - 1];
    const current = haystack[index];
    return isWordBoundary(previous) or (isAsciiLower(previous) and isAsciiUpper(current));
}

fn isAsciiLower(byte: u8) bool {
    return byte >= 'a' and byte <= 'z';
}

fn isAsciiUpper(byte: u8) bool {
    return byte >= 'A' and byte <= 'Z';
}

fn isWordBoundary(byte: u8) bool {
    return byte == ' ' or byte == '\t' or byte == '\n' or byte == '\r' or byte == '-' or byte == '_' or byte == '/' or byte == '\\' or byte == '.' or byte == ':';
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

fn fileExists(io: std.Io, path: []const u8) bool {
    if (std.fs.path.isAbsolute(path)) {
        std.Io.Dir.accessAbsolute(io, path, .{}) catch return false;
    } else {
        std.Io.Dir.cwd().access(io, path, .{}) catch return false;
    }
    return true;
}

fn execRawSqlite(allocator: std.mem.Allocator, db_path: []const u8, sql: []const u8) !void {
    const path_z = try allocator.dupeZ(u8, db_path);
    defer allocator.free(path_z);
    var db: ?*c.sqlite3 = null;
    if (c.sqlite3_open(path_z.ptr, &db) != c.SQLITE_OK) return error.SQLiteOpenFailed;
    defer _ = c.sqlite3_close(db.?);

    const sql_z = try allocator.dupeZ(u8, sql);
    defer allocator.free(sql_z);
    if (c.sqlite3_exec(db.?, sql_z.ptr, null, null, null) != c.SQLITE_OK) {
        const message = std.mem.span(c.sqlite3_errmsg(db.?));
        std.debug.print("sqlite: {s}\n", .{message});
        return error.SQLiteFailure;
    }
}

fn expectSearchOrder(store: *Storage, query: []const u8, expected_prefix: []const i64) !void {
    const entries = try store.list(query, "all", 20);
    defer {
        for (entries) |entry| entry.deinit(store.allocator);
        store.allocator.free(entries);
    }
    try std.testing.expect(entries.len >= expected_prefix.len);
    for (expected_prefix, 0..) |expected_id, index| {
        try std.testing.expectEqual(expected_id, entries[index].id);
    }
}

const v1_schema =
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
    \\  source_app TEXT
    \\);
    \\CREATE INDEX IF NOT EXISTS idx_entries_hash ON entries(hash);
    \\CREATE INDEX IF NOT EXISTS idx_entries_created ON entries(created_at_ms DESC);
    \\CREATE INDEX IF NOT EXISTS idx_entries_favorite ON entries(favorite);
    \\CREATE TABLE IF NOT EXISTS runtime_state (
    \\  key TEXT PRIMARY KEY,
    \\  value TEXT NOT NULL,
    \\  updated_at_ms INTEGER NOT NULL
    \\);
;

const rest_schema =
    \\CREATE INDEX IF NOT EXISTS idx_entries_hash ON entries(hash);
    \\CREATE INDEX IF NOT EXISTS idx_entries_created ON entries(created_at_ms DESC);
    \\CREATE INDEX IF NOT EXISTS idx_entries_favorite ON entries(favorite);
    \\CREATE TABLE IF NOT EXISTS pending_blob_prunes (
    \\  path TEXT PRIMARY KEY,
    \\  created_at_ms INTEGER NOT NULL DEFAULT (CAST(strftime('%s','now') AS INTEGER) * 1000)
    \\);
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
    \\CREATE TRIGGER IF NOT EXISTS entries_blob_ad AFTER DELETE ON entries
    \\WHEN old.kind = 'image' AND old.blob_path IS NOT NULL AND old.blob_path != ''
    \\BEGIN
    \\  INSERT OR IGNORE INTO pending_blob_prunes(path) VALUES (old.blob_path);
    \\END;
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
        const entries = try store.list("archable wo", "all", 10);
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
    const unpinned_id = try store.addText(cfg, "unpinned clear target");
    try std.testing.expectEqual(@as(i64, 1), try store.clearWithOptions("text", true));
    try std.testing.expect((try store.get(unpinned_id)) == null);
    {
        const pinned_entry = (try store.get(text_id)) orelse return error.TestUnexpectedResult;
        defer pinned_entry.deinit(allocator);
        try std.testing.expect(pinned_entry.favorite);
    }
    try std.testing.expectEqual(@as(i64, 1), try store.clear("text"));
    _ = try store.addText(cfg, "final clear target");
    try std.testing.expectEqual(@as(i64, 1), try store.clear("all"));
    _ = try store.repair(cfg);
}

test "storage LIKE search escapes wildcard input" {
    const allocator = std.testing.allocator;
    const query = try buildLikeContainsQuery(allocator, "50%_\\done");
    defer allocator.free(query);
    try std.testing.expectEqualStrings("%50\\%\\_\\\\done%", query);

    const prefix = try buildLikePrefixQuery(allocator, "50%_\\done");
    defer allocator.free(prefix);
    try std.testing.expectEqualStrings("50\\%\\_\\\\done%", prefix);
}

test "storage migrates v1 database to current schema" {
    const allocator = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const data_dir = try std.fmt.allocPrint(allocator, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer allocator.free(data_dir);

    var cfg = try config.testConfig(allocator, data_dir);
    defer cfg.deinit();
    try std.Io.Dir.cwd().createDirPath(std.testing.io, data_dir);
    try std.Io.Dir.cwd().createDirPath(std.testing.io, cfg.image_dir);

    try execRawSqlite(allocator, cfg.db_path, v1_schema ++
        \\INSERT INTO entries(kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, source_app)
        \\VALUES ('text', 'text/plain;charset=utf-8', 'legacy searchable', 'legacy searchable', 'legacy-hash', 0, 1700000000000, 17, 'legacy-app');
        \\PRAGMA user_version = 1;
    );

    var store = try Storage.open(allocator, cfg, std.testing.io);
    defer store.close();

    try std.testing.expectEqual(current_schema_version, try store.schemaVersion());
    {
        const entry = (try store.get(1)) orelse return error.TestUnexpectedResult;
        defer entry.deinit(allocator);
        try std.testing.expectEqualStrings("legacy searchable", entry.content);
        try std.testing.expectEqualStrings("legacy-app", entry.source_app.?);
        try std.testing.expect(entry.blob_path == null);
        try std.testing.expect(entry.image_width == null);
        try std.testing.expect(entry.image_height == null);
    }
    {
        const entries = try store.list("legacy", "all", 10);
        defer {
            for (entries) |entry| entry.deinit(allocator);
            allocator.free(entries);
        }
        try std.testing.expectEqual(@as(usize, 1), entries.len);
        try std.testing.expectEqual(@as(i64, 1), entries[0].id);
    }

    const image_id = try store.addImage(cfg, "image/png", "migrated-image-bytes", .{ .width = 4, .height = 5 });
    const image = (try store.get(image_id)) orelse return error.TestUnexpectedResult;
    defer image.deinit(allocator);
    try std.testing.expectEqualStrings("image", image.kind);
    try std.testing.expect(image.blob_path != null);
    try std.testing.expectEqual(@as(?i64, 4), image.image_width);
    try std.testing.expectEqual(@as(?i64, 5), image.image_height);
}

test "storage search ranks exact and prefix matches before incidental matches" {
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

    const exact_id = try store.addText(cfg, "needle");
    const contains_id = try store.addText(cfg, "zzz needle newest");
    const prefix_id = try store.addText(cfg, "needle prefix match");

    const entries = try store.list("needle", "all", 10);
    defer {
        for (entries) |entry| entry.deinit(allocator);
        allocator.free(entries);
    }

    try std.testing.expectEqual(@as(usize, 3), entries.len);
    try std.testing.expectEqual(exact_id, entries[0].id);
    try std.testing.expectEqual(prefix_id, entries[1].id);
    try std.testing.expectEqual(contains_id, entries[2].id);
}

test "storage search includes fuzzy subsequence matches" {
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

    const fuzzy_id = try store.addText(cfg, "network latency error");
    _ = try store.addText(cfg, "totally separate");

    const entries = try store.list("nle", "all", 10);
    defer {
        for (entries) |entry| entry.deinit(allocator);
        allocator.free(entries);
    }

    try std.testing.expectEqual(@as(usize, 1), entries.len);
    try std.testing.expectEqual(fuzzy_id, entries[0].id);
}

test "storage search ranks acronym fuzzy matches before newer incidental subsequences" {
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

    const acronym_id = try store.addText(cfg, "network latency error");
    const incidental_id = try store.addText(cfg, "annular low effect");

    const entries = try store.list("nle", "all", 10);
    defer {
        for (entries) |entry| entry.deinit(allocator);
        allocator.free(entries);
    }

    try std.testing.expectEqual(@as(usize, 2), entries.len);
    try std.testing.expectEqual(acronym_id, entries[0].id);
    try std.testing.expectEqual(incidental_id, entries[1].id);
}

test "storage search ranks realistic clipboard history samples predictably" {
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

    const terminal_id = try store.addText(cfg, "foot --app-id ditox -e ditox tui");
    const preview_path_id = try store.addText(cfg, "src/components/PreviewPane.tsx");
    const incidental_path_id = try store.addText(cfg, "support pipeline target");
    const camel_id = try store.addText(cfg, "OpenTuiConfig.resolveThemeSurface");
    const incidental_camel_id = try store.addText(cfg, "operation trace collector");
    const stack_id = try store.addText(cfg, "error: SQLiteFailure\nbackend/src/storage.zig:245:9");
    const env_id = try store.addText(cfg, "DITOX_TUI_CONFIG=/home/friend/.config/ditox/tui.json");
    _ = try store.addText(cfg, "npm install @opentui/solid");

    try expectSearchOrder(&store, "ppt", &.{ preview_path_id, incidental_path_id });
    try expectSearchOrder(&store, "otc", &.{ camel_id, incidental_camel_id });
    try expectSearchOrder(&store, "sqlitefailure", &.{stack_id});
    try expectSearchOrder(&store, "tui config", &.{env_id});
    try expectSearchOrder(&store, "ditox tui", &.{ terminal_id, env_id });
}

test "fuzzyScore rewards contiguous, acronym, path, and camel-case matches" {
    try std.testing.expect(fuzzyScore("network latency error", "nle") > 0);
    try std.testing.expectEqual(@as(i64, 0), fuzzyScore("totally separate", "nle"));
    try std.testing.expect(fuzzyScore("needle", "nee") > fuzzyScore("n-e-e", "nee"));
    try std.testing.expect(fuzzyScore("network latency error", "nle") > fuzzyScore("annular low effect", "nle"));
    try std.testing.expect(fuzzyScore("src/components/PreviewPane.tsx", "ppt") > fuzzyScore("support pipeline target", "ppt"));
}

test "storage retention deletes expired non-pinned entries and prunes image blobs" {
    const allocator = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const data_dir = try std.fmt.allocPrint(allocator, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer allocator.free(data_dir);

    var cfg = try config.testConfig(allocator, data_dir);
    defer cfg.deinit();
    cfg.delete_after_seconds = 1;
    try std.Io.Dir.cwd().createDirPath(std.testing.io, cfg.image_dir);

    var store = try Storage.open(allocator, cfg, std.testing.io);
    defer store.close();

    const old_text_id = try store.addText(cfg, "old text");
    const pinned_id = try store.addText(cfg, "old pinned");
    try std.testing.expect(try store.favorite(pinned_id, true));
    const image_id = try store.addImage(cfg, "image/png", "not-really-png", .{ .width = 1, .height = 1 });
    const image = (try store.get(image_id)) orelse return error.TestUnexpectedResult;
    const blob_path = try allocator.dupe(u8, image.blob_path.?);
    image.deinit(allocator);
    defer allocator.free(blob_path);

    const cutoff = try store.nowMs() - 5000;
    {
        const stmt = try store.prepare("UPDATE entries SET created_at_ms = ? WHERE id IN (?, ?, ?)");
        defer _ = c.sqlite3_finalize(stmt);
        try bindInt64(stmt, 1, cutoff);
        try bindInt64(stmt, 2, old_text_id);
        try bindInt64(stmt, 3, pinned_id);
        try bindInt64(stmt, 4, image_id);
        try store.stepDone(stmt);
    }

    try std.testing.expectEqual(@as(i64, 2), try store.applyRetention(cfg));
    try std.testing.expect((try store.get(old_text_id)) == null);
    try std.testing.expect((try store.get(image_id)) == null);
    {
        const pinned = (try store.get(pinned_id)) orelse return error.TestUnexpectedResult;
        defer pinned.deinit(allocator);
        try std.testing.expect(pinned.favorite);
    }
    try std.testing.expectError(error.FileNotFound, std.Io.Dir.cwd().access(std.testing.io, blob_path, .{}));
}

test "storage repair sanitizes text and removes missing image rows" {
    const allocator = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const data_dir = try std.fmt.allocPrint(allocator, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer allocator.free(data_dir);

    var cfg = try config.testConfig(allocator, data_dir);
    defer cfg.deinit();
    cfg.max_preview_chars = 4;
    try std.Io.Dir.cwd().createDirPath(std.testing.io, cfg.image_dir);

    var store = try Storage.open(allocator, cfg, std.testing.io);
    defer store.close();

    const bad_hash = util.sha256Hex("bad");
    {
        const stmt = try store.prepare(
            \\INSERT INTO entries(kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, blob_path, image_width, image_height)
            \\VALUES ('text', 'text/plain;charset=utf-8', ?, 'stale', ?, 0, CAST(strftime('%s','now') AS INTEGER) * 1000, 999, NULL, NULL, NULL)
        );
        defer _ = c.sqlite3_finalize(stmt);
        try bindText(stmt, 1, "\x1b[31mred\x1b[0m\x00 ok");
        try bindText(stmt, 2, &bad_hash);
        try store.stepDone(stmt);
    }
    const missing_image_id = blk: {
        const hash = util.sha256Hex("missing-image");
        const stmt = try store.prepare(
            \\INSERT INTO entries(kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, blob_path, image_width, image_height)
            \\VALUES ('image', 'image/png', ?, 'missing image', ?, 0, CAST(strftime('%s','now') AS INTEGER) * 1000, 13, ?, 1, 1)
        );
        defer _ = c.sqlite3_finalize(stmt);
        try bindText(stmt, 1, &hash);
        try bindText(stmt, 2, &hash);
        try bindText(stmt, 3, ".zig-cache/tmp/does-not-exist.png");
        try store.stepDone(stmt);
        break :blk c.sqlite3_last_insert_rowid(store.db);
    };

    const result = try store.repair(cfg);
    try std.testing.expect(result.ok);
    try std.testing.expectEqual(@as(i64, 1), result.sanitized_text);
    try std.testing.expectEqual(@as(i64, 1), result.removed_missing_images);
    try std.testing.expect((try store.get(missing_image_id)) == null);

    const entry = (try store.get(1)) orelse return error.TestUnexpectedResult;
    defer entry.deinit(allocator);
    try std.testing.expectEqualStrings("red ok", entry.content);
    try std.testing.expectEqualStrings("red ...", entry.preview);
    try std.testing.expectEqual(@as(i64, 6), entry.byte_len);
    const expected_hash = util.sha256Hex("red ok");
    try std.testing.expectEqualStrings(&expected_hash, entry.hash);
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
    try store.markWatcherPid(1234);
    try std.testing.expectEqual(@as(?i64, 1234), try store.watcherPid());
    try store.clearWatcherPid();
    try std.testing.expect((try store.watcherPid()) == null);

    try store.markSelfWrite("abc");
    const hash = (try store.takeSelfWriteHash()) orelse return error.TestUnexpectedResult;
    defer allocator.free(hash);
    try std.testing.expectEqualStrings("abc", hash);
    try std.testing.expect((try store.takeSelfWriteHash()) == null);
}
