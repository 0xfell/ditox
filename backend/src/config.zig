const std = @import("std");

pub const Config = struct {
    allocator: std.mem.Allocator,
    config_path: []const u8,
    data_dir: []const u8,
    db_path: []const u8,
    image_dir: []const u8,
    max_entries: u32 = 1000,
    delete_after_seconds: u32 = 0,
    allow_duplicates: bool = false,
    poll_interval_ms: u32 = 250,
    paste_enabled: bool = true,
    paste_buffer_ms: u32 = 120,
    auto_paste_enabled: bool = false,
    auto_paste_keybind: []const u8,
    auto_paste_buffer_ms: u32 = 10,
    max_preview_chars: u32 = 160,
    terminal_command: []const u8,
    excluded_apps: []const []const u8 = &.{},
    excluded_windows: []const []const u8 = &.{},

    pub fn load(allocator: std.mem.Allocator, init: std.process.Init) !Config {
        const env = init.environ_map.*;
        const home = env.get("HOME") orelse ".";

        const config_home = try ownedPath(allocator, env.get("XDG_CONFIG_HOME"), try std.fmt.allocPrint(allocator, "{s}/.config", .{home}));
        defer allocator.free(config_home);
        const data_home = try ownedPath(allocator, env.get("XDG_DATA_HOME"), try std.fmt.allocPrint(allocator, "{s}/.local/share", .{home}));
        defer allocator.free(data_home);

        const config_path = try ownedPath(allocator, env.get("DITOX_CONFIG"), try std.fmt.allocPrint(allocator, "{s}/ditox/config.toml", .{config_home}));
        const data_dir = try ownedPath(allocator, env.get("DITOX_DATA_DIR"), try std.fmt.allocPrint(allocator, "{s}/ditox", .{data_home}));
        const db_path = try ownedPath(allocator, env.get("DITOX_DB"), try std.fmt.allocPrint(allocator, "{s}/ditox-v2.db", .{data_dir}));
        const image_dir = try std.fmt.allocPrint(allocator, "{s}/images-v2", .{data_dir});
        const terminal_command = try allocator.dupe(u8, env.get("DITOX_TERMINAL_COMMAND") orelse "foot --app-id ditox -e ditox tui");
        const auto_paste_keybind = try allocator.dupe(u8, env.get("DITOX_AUTO_PASTE_KEYBIND") orelse "ctrl+v");
        const excluded_apps = try defaultExcludedApps(allocator);
        errdefer freeStringList(allocator, excluded_apps);
        const excluded_windows = try allocator.alloc([]const u8, 0);
        errdefer allocator.free(excluded_windows);

        try std.Io.Dir.cwd().createDirPath(init.io, std.fs.path.dirname(config_path) orelse ".");

        var cfg = Config{
            .allocator = allocator,
            .config_path = config_path,
            .data_dir = data_dir,
            .db_path = db_path,
            .image_dir = image_dir,
            .terminal_command = terminal_command,
            .auto_paste_keybind = auto_paste_keybind,
            .excluded_apps = excluded_apps,
            .excluded_windows = excluded_windows,
        };

        if (std.Io.Dir.cwd().readFileAlloc(init.io, config_path, allocator, .limited(1024 * 1024))) |bytes| {
            defer allocator.free(bytes);
            try cfg.applyConfigFile(bytes, home, config_home, data_home);
        } else |_| {}

        try std.Io.Dir.cwd().createDirPath(init.io, cfg.data_dir);
        try std.Io.Dir.cwd().createDirPath(init.io, std.fs.path.dirname(cfg.db_path) orelse ".");
        try std.Io.Dir.cwd().createDirPath(init.io, cfg.image_dir);

        return cfg;
    }

    pub fn deinit(self: Config) void {
        self.allocator.free(self.config_path);
        self.allocator.free(self.data_dir);
        self.allocator.free(self.db_path);
        self.allocator.free(self.image_dir);
        self.allocator.free(self.terminal_command);
        self.allocator.free(self.auto_paste_keybind);
        freeStringList(self.allocator, self.excluded_apps);
        freeStringList(self.allocator, self.excluded_windows);
    }

    fn applyConfigFile(self: *Config, bytes: []const u8, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        const trimmed = std.mem.trim(u8, bytes, " \t\r\n");
        if (trimmed.len > 0 and trimmed[0] == '{') {
            try self.applyJsonConfig(bytes, home, config_home, data_home);
            return;
        }
        try self.applyTomlSubset(bytes, home, config_home, data_home);
    }

    fn applyJsonConfig(self: *Config, bytes: []const u8, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        var parsed = try std.json.parseFromSlice(std.json.Value, self.allocator, bytes, .{});
        defer parsed.deinit();
        if (parsed.value != .object) return;

        const object = parsed.value.object;
        try self.applyJsonObject(object, home, config_home, data_home);
    }

    fn applyJsonObject(self: *Config, object: std.json.ObjectMap, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        if (jsonU32(object, "max_entries") orelse jsonU32(object, "maxHistory")) |value| self.max_entries = value;
        if (jsonU32(object, "delete_after_seconds") orelse jsonU32(object, "delete_after") orelse jsonU32(object, "deleteAfter")) |value| {
            self.delete_after_seconds = value;
        }
        if (jsonBool(object, "allow_duplicates") orelse jsonBool(object, "allowDuplicates")) |value| self.allow_duplicates = value;
        if (jsonU32(object, "poll_interval_ms") orelse jsonU32(object, "pollInterval")) |value| self.poll_interval_ms = value;
        if (jsonU32(object, "max_preview_chars") orelse jsonU32(object, "maxEntryLength")) |value| self.max_preview_chars = value;

        if (jsonString(object, "historyFile") orelse jsonString(object, "db_path")) |value| {
            try self.replaceConfiguredPath(&self.db_path, value, home, config_home, data_home);
        }
        if (jsonString(object, "tempDir") orelse jsonString(object, "image_dir")) |value| {
            try self.replaceConfiguredPath(&self.image_dir, value, home, config_home, data_home);
        }
        if (jsonString(object, "terminal_command") orelse jsonString(object, "terminalCommand")) |value| {
            self.allocator.free(self.terminal_command);
            self.terminal_command = try self.allocator.dupe(u8, value);
        }

        if (try jsonStringArray(self.allocator, object, "excluded_apps")) |value| {
            freeStringList(self.allocator, self.excluded_apps);
            self.excluded_apps = value;
        } else if (try jsonStringArray(self.allocator, object, "excludedApps")) |value| {
            freeStringList(self.allocator, self.excluded_apps);
            self.excluded_apps = value;
        }
        if (try jsonStringArray(self.allocator, object, "excluded_windows")) |value| {
            freeStringList(self.allocator, self.excluded_windows);
            self.excluded_windows = value;
        } else if (try jsonStringArray(self.allocator, object, "excludedWindows")) |value| {
            freeStringList(self.allocator, self.excluded_windows);
            self.excluded_windows = value;
        }

        if (jsonObject(object, "paste")) |paste| {
            if (jsonBool(paste, "enabled")) |value| self.paste_enabled = value;
            if (jsonU32(paste, "buffer_ms") orelse jsonU32(paste, "buffer")) |value| self.paste_buffer_ms = value;
        }
        if (jsonObject(object, "auto_paste") orelse jsonObject(object, "autoPaste")) |auto_paste| {
            if (jsonBool(auto_paste, "enabled")) |value| self.auto_paste_enabled = value;
            if (jsonString(auto_paste, "keybind")) |value| {
                self.allocator.free(self.auto_paste_keybind);
                self.auto_paste_keybind = try self.allocator.dupe(u8, value);
            }
            if (jsonU32(auto_paste, "buffer_ms") orelse jsonU32(auto_paste, "buffer")) |value| self.auto_paste_buffer_ms = value;
        }
        if (jsonObject(object, "history")) |history| {
            try self.applyJsonObject(history, home, config_home, data_home);
        }
        if (jsonObject(object, "watch")) |watch| {
            if (jsonU32(watch, "poll_interval_ms") orelse jsonU32(watch, "pollInterval")) |value| self.poll_interval_ms = value;
        }
        if (jsonObject(object, "ui")) |ui| {
            if (jsonU32(ui, "max_preview_chars") orelse jsonU32(ui, "maxEntryLength")) |value| self.max_preview_chars = value;
            if (jsonString(ui, "terminal_command") orelse jsonString(ui, "terminalCommand")) |value| {
                self.allocator.free(self.terminal_command);
                self.terminal_command = try self.allocator.dupe(u8, value);
            }
        }
        if (jsonObject(object, "capture")) |capture| {
            if (try jsonStringArray(self.allocator, capture, "excluded_apps")) |value| {
                freeStringList(self.allocator, self.excluded_apps);
                self.excluded_apps = value;
            } else if (try jsonStringArray(self.allocator, capture, "excludedApps")) |value| {
                freeStringList(self.allocator, self.excluded_apps);
                self.excluded_apps = value;
            }
            if (try jsonStringArray(self.allocator, capture, "excluded_windows")) |value| {
                freeStringList(self.allocator, self.excluded_windows);
                self.excluded_windows = value;
            } else if (try jsonStringArray(self.allocator, capture, "excludedWindows")) |value| {
                freeStringList(self.allocator, self.excluded_windows);
                self.excluded_windows = value;
            }
        }
    }

    fn applyTomlSubset(self: *Config, bytes: []const u8, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        var section: []const u8 = "";
        var lines = std.mem.splitScalar(u8, bytes, '\n');
        while (lines.next()) |raw_line| {
            var line = std.mem.trim(u8, raw_line, " \t\r");
            if (line.len == 0 or line[0] == '#') continue;
            if (line[0] == '[' and line[line.len - 1] == ']') {
                section = std.mem.trim(u8, line[1 .. line.len - 1], " \t");
                continue;
            }

            const eq = std.mem.indexOfScalar(u8, line, '=') orelse continue;
            const key = std.mem.trim(u8, line[0..eq], " \t");
            const value = stripComment(std.mem.trim(u8, line[eq + 1 ..], " \t"));

            if (std.mem.eql(u8, section, "")) {
                try self.applyTopLevelAlias(key, value, home, config_home, data_home);
            } else if (std.mem.eql(u8, section, "history")) {
                try self.applyHistoryValue(key, value, home, config_home, data_home);
            } else if (std.mem.eql(u8, section, "watch")) {
                if (std.mem.eql(u8, key, "poll_interval_ms") or std.mem.eql(u8, key, "pollInterval")) self.poll_interval_ms = parseU32(value, self.poll_interval_ms);
            } else if (std.mem.eql(u8, section, "paste")) {
                if (std.mem.eql(u8, key, "enabled")) self.paste_enabled = parseBool(value, self.paste_enabled);
                if (std.mem.eql(u8, key, "buffer_ms")) self.paste_buffer_ms = parseU32(value, self.paste_buffer_ms);
            } else if (std.mem.eql(u8, section, "auto_paste") or std.mem.eql(u8, section, "autoPaste")) {
                if (std.mem.eql(u8, key, "enabled")) self.auto_paste_enabled = parseBool(value, self.auto_paste_enabled);
                if (std.mem.eql(u8, key, "keybind")) {
                    self.allocator.free(self.auto_paste_keybind);
                    self.auto_paste_keybind = try self.allocator.dupe(u8, stripQuotes(value));
                }
                if (std.mem.eql(u8, key, "buffer_ms") or std.mem.eql(u8, key, "buffer")) self.auto_paste_buffer_ms = parseU32(value, self.auto_paste_buffer_ms);
            } else if (std.mem.eql(u8, section, "ui")) {
                if (std.mem.eql(u8, key, "max_preview_chars") or std.mem.eql(u8, key, "maxEntryLength")) self.max_preview_chars = parseU32(value, self.max_preview_chars);
                if (std.mem.eql(u8, key, "terminal_command")) {
                    self.allocator.free(self.terminal_command);
                    self.terminal_command = try self.allocator.dupe(u8, stripQuotes(value));
                }
            } else if (std.mem.eql(u8, section, "capture")) {
                try self.applyCaptureValue(key, value);
            }
        }
    }

    fn applyTopLevelAlias(self: *Config, key: []const u8, value: []const u8, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        try self.applyHistoryValue(key, value, home, config_home, data_home);
        try self.applyCaptureValue(key, value);
        if (std.mem.eql(u8, key, "pollInterval")) self.poll_interval_ms = parseU32(value, self.poll_interval_ms);
        if (std.mem.eql(u8, key, "maxEntryLength")) self.max_preview_chars = parseU32(value, self.max_preview_chars);
        if (std.mem.eql(u8, key, "tempDir")) try self.replaceConfiguredPath(&self.image_dir, value, home, config_home, data_home);
    }

    fn applyHistoryValue(self: *Config, key: []const u8, value: []const u8, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        if (std.mem.eql(u8, key, "max_entries") or std.mem.eql(u8, key, "maxHistory")) self.max_entries = parseU32(value, self.max_entries);
        if (std.mem.eql(u8, key, "delete_after_seconds") or std.mem.eql(u8, key, "delete_after") or std.mem.eql(u8, key, "deleteAfter")) {
            self.delete_after_seconds = parseU32(value, self.delete_after_seconds);
        }
        if (std.mem.eql(u8, key, "allow_duplicates") or std.mem.eql(u8, key, "allowDuplicates")) self.allow_duplicates = parseBool(value, self.allow_duplicates);
        if (std.mem.eql(u8, key, "historyFile")) try self.replaceConfiguredPath(&self.db_path, value, home, config_home, data_home);
    }

    fn applyCaptureValue(self: *Config, key: []const u8, value: []const u8) !void {
        if (std.mem.eql(u8, key, "excluded_apps") or std.mem.eql(u8, key, "excludedApps")) {
            freeStringList(self.allocator, self.excluded_apps);
            self.excluded_apps = try parseStringArray(self.allocator, value);
        }
        if (std.mem.eql(u8, key, "excluded_windows") or std.mem.eql(u8, key, "excludedWindows")) {
            freeStringList(self.allocator, self.excluded_windows);
            self.excluded_windows = try parseStringArray(self.allocator, value);
        }
    }

    fn replaceConfiguredPath(self: *Config, slot: *[]const u8, value: []const u8, home: []const u8, config_home: []const u8, data_home: []const u8) !void {
        const raw = std.mem.trim(u8, stripQuotes(value), " \t");
        if (raw.len == 0) return;
        const path = try resolveConfiguredPath(self.allocator, self.config_path, raw, home, config_home, data_home);
        self.allocator.free(slot.*);
        slot.* = path;
    }
};

pub fn testConfig(allocator: std.mem.Allocator, data_dir: []const u8) !Config {
    const db_path = try std.fmt.allocPrint(allocator, "{s}/ditox-v2.db", .{data_dir});
    errdefer allocator.free(db_path);
    const image_dir = try std.fmt.allocPrint(allocator, "{s}/images-v2", .{data_dir});
    errdefer allocator.free(image_dir);
    return .{
        .allocator = allocator,
        .config_path = try allocator.dupe(u8, "test-config.toml"),
        .data_dir = try allocator.dupe(u8, data_dir),
        .db_path = db_path,
        .image_dir = image_dir,
        .terminal_command = try allocator.dupe(u8, "ditox-tui"),
        .auto_paste_keybind = try allocator.dupe(u8, "ctrl+v"),
        .excluded_apps = try allocator.alloc([]const u8, 0),
        .excluded_windows = try allocator.alloc([]const u8, 0),
    };
}

fn ownedPath(allocator: std.mem.Allocator, env_value: ?[]const u8, fallback_owned: []u8) ![]const u8 {
    if (env_value) |value| {
        allocator.free(fallback_owned);
        return allocator.dupe(u8, value);
    }
    return fallback_owned;
}

fn resolveConfiguredPath(
    allocator: std.mem.Allocator,
    config_path: []const u8,
    raw: []const u8,
    home: []const u8,
    config_home: []const u8,
    data_home: []const u8,
) ![]const u8 {
    if (std.fs.path.isAbsolute(raw)) return allocator.dupe(u8, raw);
    if (std.mem.eql(u8, raw, "~")) return allocator.dupe(u8, home);
    if (std.mem.startsWith(u8, raw, "~/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ home, raw[2..] });
    if (std.mem.startsWith(u8, raw, "$HOME/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ home, raw["$HOME/".len..] });
    if (std.mem.startsWith(u8, raw, "${HOME}/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ home, raw["${HOME}/".len..] });
    if (std.mem.startsWith(u8, raw, "$XDG_CONFIG_HOME/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ config_home, raw["$XDG_CONFIG_HOME/".len..] });
    if (std.mem.startsWith(u8, raw, "${XDG_CONFIG_HOME}/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ config_home, raw["${XDG_CONFIG_HOME}/".len..] });
    if (std.mem.startsWith(u8, raw, "$XDG_DATA_HOME/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ data_home, raw["$XDG_DATA_HOME/".len..] });
    if (std.mem.startsWith(u8, raw, "${XDG_DATA_HOME}/")) return std.fmt.allocPrint(allocator, "{s}/{s}", .{ data_home, raw["${XDG_DATA_HOME}/".len..] });
    const config_dir = std.fs.path.dirname(config_path) orelse ".";
    return std.fs.path.join(allocator, &.{ config_dir, raw });
}

fn stripComment(value: []const u8) []const u8 {
    const idx = std.mem.indexOfScalar(u8, value, '#') orelse return value;
    return std.mem.trim(u8, value[0..idx], " \t");
}

fn stripQuotes(value: []const u8) []const u8 {
    if (value.len >= 2 and value[0] == '"' and value[value.len - 1] == '"') return value[1 .. value.len - 1];
    return value;
}

fn parseStringArray(allocator: std.mem.Allocator, value: []const u8) ![]const []const u8 {
    const trimmed = std.mem.trim(u8, value, " \t");
    if (trimmed.len < 2 or trimmed[0] != '[' or trimmed[trimmed.len - 1] != ']') {
        return allocator.alloc([]const u8, 0);
    }
    var list: std.ArrayList([]const u8) = .empty;
    errdefer {
        for (list.items) |item| allocator.free(item);
        list.deinit(allocator);
    }
    var parts = std.mem.splitScalar(u8, trimmed[1 .. trimmed.len - 1], ',');
    while (parts.next()) |part| {
        const item = stripQuotes(std.mem.trim(u8, part, " \t"));
        if (item.len > 0) try list.append(allocator, try allocator.dupe(u8, item));
    }
    const owned: []const []const u8 = try list.toOwnedSlice(allocator);
    return owned;
}

fn defaultExcludedApps(allocator: std.mem.Allocator) ![]const []const u8 {
    const defaults = [_][]const u8{ "1Password", "Bitwarden", "KeePassXC", "LastPass", "Dashlane", "Password Safe", "Keychain Access" };
    var list = try allocator.alloc([]const u8, defaults.len);
    errdefer allocator.free(list);
    for (defaults, 0..) |value, index| {
        list[index] = try allocator.dupe(u8, value);
    }
    return list;
}

fn freeStringList(allocator: std.mem.Allocator, list: []const []const u8) void {
    for (list) |item| allocator.free(item);
    allocator.free(list);
}

fn parseU32(value: []const u8, fallback: u32) u32 {
    return std.fmt.parseUnsigned(u32, value, 10) catch fallback;
}

fn parseBool(value: []const u8, fallback: bool) bool {
    if (std.mem.eql(u8, value, "true")) return true;
    if (std.mem.eql(u8, value, "false")) return false;
    return fallback;
}

fn jsonObject(object: std.json.ObjectMap, key: []const u8) ?std.json.ObjectMap {
    const value = object.get(key) orelse return null;
    return if (value == .object) value.object else null;
}

fn jsonString(object: std.json.ObjectMap, key: []const u8) ?[]const u8 {
    const value = object.get(key) orelse return null;
    return if (value == .string) value.string else null;
}

fn jsonBool(object: std.json.ObjectMap, key: []const u8) ?bool {
    const value = object.get(key) orelse return null;
    return if (value == .bool) value.bool else null;
}

fn jsonU32(object: std.json.ObjectMap, key: []const u8) ?u32 {
    const value = object.get(key) orelse return null;
    return switch (value) {
        .integer => |integer| if (integer >= 0 and integer <= std.math.maxInt(u32)) @intCast(integer) else null,
        else => null,
    };
}

fn jsonStringArray(allocator: std.mem.Allocator, object: std.json.ObjectMap, key: []const u8) !?[]const []const u8 {
    const value = object.get(key) orelse return null;
    if (value != .array) return null;
    var list: std.ArrayList([]const u8) = .empty;
    errdefer {
        for (list.items) |item| allocator.free(item);
        list.deinit(allocator);
    }
    for (value.array.items) |item| {
        if (item != .string) continue;
        try list.append(allocator, try allocator.dupe(u8, item.string));
    }
    const owned: []const []const u8 = try list.toOwnedSlice(allocator);
    return owned;
}
