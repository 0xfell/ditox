const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const core = b.addModule("ditox_core", .{
        .root_source_file = b.path("backend/src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    wireSqlite(b, core);

    const cli = b.addExecutable(.{
        .name = "ditox",
        .root_module = b.createModule(.{
            .root_source_file = b.path("backend/src/cli.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{.{ .name = "ditox_core", .module = core }},
        }),
    });
    wireSqlite(b, cli.root_module);
    b.installArtifact(cli);

    const daemon = b.addExecutable(.{
        .name = "ditoxd",
        .root_module = b.createModule(.{
            .root_source_file = b.path("backend/src/daemon.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{.{ .name = "ditox_core", .module = core }},
        }),
    });
    wireSqlite(b, daemon.root_module);
    b.installArtifact(daemon);
    installTuiAssets(b);

    const test_step = b.step("test", "Run backend tests");
    const core_tests = b.addTest(.{ .root_module = core });
    const run_core_tests = b.addRunArtifact(core_tests);
    test_step.dependOn(&run_core_tests.step);
    const daemon_tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("backend/src/daemon.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{.{ .name = "ditox_core", .module = core }},
        }),
    });
    wireSqlite(b, daemon_tests.root_module);
    const run_daemon_tests = b.addRunArtifact(daemon_tests);
    test_step.dependOn(&run_daemon_tests.step);

    const run_step = b.step("run", "Run ditox");
    const run_cmd = b.addRunArtifact(cli);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| run_cmd.addArgs(args);
    run_step.dependOn(&run_cmd.step);
}

fn wireSqlite(b: *std.Build, module: *std.Build.Module) void {
    module.linkSystemLibrary("c", .{});
    module.linkSystemLibrary("sqlite3", .{});

    if (env(b, "DITOX_SQLITE3_INCLUDE_DIR") orelse env(b, "SQLITE3_INCLUDE_DIR")) |dir| {
        module.addIncludePath(.{ .cwd_relative = dir });
    }
    if (env(b, "DITOX_SQLITE3_LIB_DIR") orelse env(b, "SQLITE3_LIB_DIR")) |dir| {
        module.addLibraryPath(.{ .cwd_relative = dir });
        module.addRPath(.{ .cwd_relative = dir });
    }
}

fn installTuiAssets(b: *std.Build) void {
    if (pathExists(b, "tui/dist/index.js")) {
        b.installDirectory(.{
            .source_dir = b.path("tui/dist"),
            .install_dir = .prefix,
            .install_subdir = "share/ditox/tui/dist",
        });
    }
    b.installFile("tui/tui-config.example.json", "share/ditox/tui/tui-config.example.json");
    b.installFile("tui/tui-config.schema.json", "share/ditox/tui/tui-config.schema.json");
    b.installFile("tui/custom_theme.example.json", "share/ditox/tui/custom_theme.example.json");
}

fn pathExists(b: *std.Build, path: []const u8) bool {
    const path_z = b.allocator.dupeZ(u8, path) catch return false;
    defer b.allocator.free(path_z);
    return std.os.linux.errno(std.os.linux.access(path_z.ptr, std.os.linux.F_OK)) == .SUCCESS;
}

fn env(b: *std.Build, name: []const u8) ?[]const u8 {
    return b.graph.environ_map.get(name);
}
