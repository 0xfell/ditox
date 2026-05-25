const std = @import("std");

pub const Config = struct {
    allocator: std.mem.Allocator,
    config_path: []const u8,
    data_dir: []const u8,
    db_path: []const u8,
    image_dir: []const u8,
    max_entries: u32 = 1000,
    allow_duplicates: bool = false,
    poll_interval_ms: u32 = 250,
    paste_enabled: bool = true,
    paste_buffer_ms: u32 = 120,
    max_preview_chars: u32 = 160,
    terminal_command: []const u8,

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

        try std.Io.Dir.cwd().createDirPath(init.io, std.fs.path.dirname(config_path) orelse ".");
        try std.Io.Dir.cwd().createDirPath(init.io, data_dir);
        try std.Io.Dir.cwd().createDirPath(init.io, image_dir);

        var cfg = Config{
            .allocator = allocator,
            .config_path = config_path,
            .data_dir = data_dir,
            .db_path = db_path,
            .image_dir = image_dir,
            .terminal_command = terminal_command,
        };

        if (std.Io.Dir.cwd().readFileAlloc(init.io, config_path, allocator, .limited(1024 * 1024))) |bytes| {
            defer allocator.free(bytes);
            try cfg.applyTomlSubset(bytes);
        } else |_| {}

        return cfg;
    }

    pub fn deinit(self: Config) void {
        self.allocator.free(self.config_path);
        self.allocator.free(self.data_dir);
        self.allocator.free(self.db_path);
        self.allocator.free(self.image_dir);
        self.allocator.free(self.terminal_command);
    }

    fn applyTomlSubset(self: *Config, bytes: []const u8) !void {
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

            if (std.mem.eql(u8, section, "history")) {
                if (std.mem.eql(u8, key, "max_entries")) self.max_entries = parseU32(value, self.max_entries);
                if (std.mem.eql(u8, key, "allow_duplicates")) self.allow_duplicates = parseBool(value, self.allow_duplicates);
            } else if (std.mem.eql(u8, section, "watch")) {
                if (std.mem.eql(u8, key, "poll_interval_ms")) self.poll_interval_ms = parseU32(value, self.poll_interval_ms);
            } else if (std.mem.eql(u8, section, "paste")) {
                if (std.mem.eql(u8, key, "enabled")) self.paste_enabled = parseBool(value, self.paste_enabled);
                if (std.mem.eql(u8, key, "buffer_ms")) self.paste_buffer_ms = parseU32(value, self.paste_buffer_ms);
            } else if (std.mem.eql(u8, section, "ui")) {
                if (std.mem.eql(u8, key, "max_preview_chars")) self.max_preview_chars = parseU32(value, self.max_preview_chars);
                if (std.mem.eql(u8, key, "terminal_command")) {
                    self.allocator.free(self.terminal_command);
                    self.terminal_command = try self.allocator.dupe(u8, stripQuotes(value));
                }
            }
        }
    }
};

fn ownedPath(allocator: std.mem.Allocator, env_value: ?[]const u8, fallback_owned: []u8) ![]const u8 {
    if (env_value) |value| {
        allocator.free(fallback_owned);
        return allocator.dupe(u8, value);
    }
    return fallback_owned;
}

fn stripComment(value: []const u8) []const u8 {
    const idx = std.mem.indexOfScalar(u8, value, '#') orelse return value;
    return std.mem.trim(u8, value[0..idx], " \t");
}

fn stripQuotes(value: []const u8) []const u8 {
    if (value.len >= 2 and value[0] == '"' and value[value.len - 1] == '"') return value[1 .. value.len - 1];
    return value;
}

fn parseU32(value: []const u8, fallback: u32) u32 {
    return std.fmt.parseUnsigned(u32, value, 10) catch fallback;
}

fn parseBool(value: []const u8, fallback: bool) bool {
    if (std.mem.eql(u8, value, "true")) return true;
    if (std.mem.eql(u8, value, "false")) return false;
    return fallback;
}
