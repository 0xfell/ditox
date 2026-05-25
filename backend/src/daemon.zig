const std = @import("std");
const core = @import("ditox_core");

const Io = std.Io;

pub fn main(init: std.process.Init) !void {
    const allocator = init.gpa;
    const args = try init.minimal.args.toSlice(init.arena.allocator());

    if (args.len > 1 and std.mem.eql(u8, args[1], "serve")) {
        try serveStdio(allocator, init);
        return;
    }
    if (args.len > 1 and std.mem.eql(u8, args[1], "watch")) {
        try watchClipboard(allocator, init);
        return;
    }

    var stdout_buffer: [1024]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    try stdout.writeAll("usage: ditoxd serve --stdio | ditoxd watch\n");
    try stdout.flush();
}

fn serveStdio(allocator: std.mem.Allocator, init: std.process.Init) !void {
    const input = try core.util.readAllStdin(allocator);
    defer allocator.free(input);

    const response = try core.rpc.handle(allocator, init, input);
    defer allocator.free(response);
    const framed = try core.rpc.frame(allocator, response);
    defer allocator.free(framed);

    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    try stdout.writeAll(framed);
    try stdout.flush();
}

fn watchClipboard(allocator: std.mem.Allocator, init: std.process.Init) !void {
    var opened = try core.app.openStore(allocator, init);
    defer opened.cfg.deinit();
    defer opened.store.close();

    var last_hash: ?[64]u8 = null;
    while (true) {
        if (core.clipboard.readText(allocator, init)) |text| {
            defer allocator.free(text);
            if (text.len > 0) {
                const hash = core.util.sha256Hex(text);
                if (last_hash == null or !std.mem.eql(u8, &last_hash.?, &hash)) {
                    _ = opened.store.addText(opened.cfg, text) catch {};
                    last_hash = hash;
                }
            }
        } else |_| {}

        try std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(opened.cfg.poll_interval_ms), .awake);
    }
}
