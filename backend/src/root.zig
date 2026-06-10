pub const version = "0.1.0";

pub const app = @import("app.zig");
pub const blob = @import("blob.zig");
pub const clipboard = @import("clipboard.zig");
pub const config = @import("config.zig");
pub const fuzzy = @import("fuzzy.zig");
pub const models = @import("models.zig");
pub const rpc = @import("rpc.zig");
pub const storage = @import("storage.zig");
pub const util = @import("util.zig");

test {
    _ = app;
    _ = blob;
    _ = clipboard;
    _ = config;
    _ = fuzzy;
    _ = models;
    _ = rpc;
    _ = storage;
    _ = util;
}
