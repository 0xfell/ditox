import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { Database } from "bun:sqlite";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { deflateSync } from "node:zlib";

const repoRoot = join(import.meta.dir, "..");
const ditox = join(repoRoot, "zig-out", "bin", "ditox");
const ditoxd = join(repoRoot, "zig-out", "bin", "ditoxd");

let tempDir = "";
let env: Record<string, string> = {};

beforeEach(() => {
  tempDir = mkdtempSync(join(tmpdir(), "ditox-cli-"));
  env = {
    DITOX_DATA_DIR: join(tempDir, "data"),
    DITOX_CLIPBOARD_MOCK: join(tempDir, "clipboard.txt"),
  };
});

afterEach(() => {
  if (tempDir && existsSync(tempDir)) rmSync(tempDir, { recursive: true, force: true });
});

describe("ditox CLI smoke", () => {
  test("covers add list search print copy paste favorite delete clear status and repair", () => {
    expect(run(["add", "alpha searchable"])).toBe("1");
    expect(run(["add", "beta second"])).toBe("2");
    expect(run(["print", "1"])).toBe("alpha searchable");

    const listed = JSON.parse(run(["list", "--query", "searchable"]));
    expect(listed.entries).toHaveLength(1);
    expect(listed.entries[0].content).toBe("alpha searchable");

    run(["copy", "1"]);
    expect(readFileSync(env.DITOX_CLIPBOARD_MOCK!, "utf8")).toBe("alpha searchable");

    run(["paste", "2"]);
    expect(readFileSync(env.DITOX_CLIPBOARD_MOCK!, "utf8")).toBe("beta second");

    expect(run(["favorite", "1"])).toBe("true");
    let status = JSON.parse(run(["status"]));
    expect(status.stats.favorites).toBe(1);

    expect(run(["delete", "2"])).toBe("true");
    expect(run(["add", "clear me without touching pinned"])).toBe("3");
    expect(run(["clear", "text", "--keep-pinned"])).toBe("1");
    status = JSON.parse(run(["status"]));
    expect(status.stats.entries).toBe(1);
    expect(status.stats.favorites).toBe(1);
    expect(run(["clear", "text"])).toBe("1");

    expect(run(["add", "gamma"])).toBe("4");
    expect(run(["clear", "all"])).toBe("1");

    const repair = JSON.parse(run(["repair"]));
    expect(repair.ok).toBe(true);

    expect(run(["pause", "60000"])).toBe("paused_for_ms=60000");
    status = JSON.parse(run(["status"]));
    expect(status.watcher.paused).toBe(true);
    expect(run(["resume"])).toBe("resumed");
    expect(run(["-pause", "2s"])).toBe("paused_for_ms=2000");
    status = JSON.parse(run(["status"]));
    expect(status.watcher.paused).toBe(true);
    expect(run(["--pause=1m"])).toBe("paused_for_ms=60000");
    expect(run(["resume"])).toBe("resumed");
  });

  test("orders searched history by relevance before recency", () => {
    expect(run(["add", "needle"])).toBe("1");
    expect(run(["add", "zzz needle newest"])).toBe("2");
    expect(run(["add", "needle prefix match"])).toBe("3");

    const listed = JSON.parse(run(["list", "--query", "needle"]));
    expect(listed.entries.map((entry: { content: string }) => entry.content)).toEqual([
      "needle",
      "needle prefix match",
      "zzz needle newest",
    ]);
  });

  test("finds fuzzy subsequence search matches", () => {
    expect(run(["add", "network latency error"])).toBe("1");
    expect(run(["add", "totally separate"])).toBe("2");

    const listed = JSON.parse(run(["list", "--query", "nle"]));
    expect(listed.entries.map((entry: { content: string }) => entry.content)).toEqual(["network latency error"]);
  });

  test("accepts Clipse-style backend config aliases for duplicates and history limits", () => {
    env.DITOX_CONFIG = join(tempDir, "clipse-aliases.toml");
    writeFileSync(
      env.DITOX_CONFIG,
      [
        "maxHistory = 2",
        "allowDuplicates = true",
        "pollInterval = 25",
        "maxEntryLength = 8",
        "",
      ].join("\n"),
    );

    expect(run(["add", "duplicate value one"])).toBe("1");
    expect(run(["add", "duplicate value one"])).toBe("2");
    expect(run(["add", "newest value"])).toBe("3");

    const status = JSON.parse(run(["status"]));
    expect(status.config.max_entries).toBe(2);
    expect(status.config.allow_duplicates).toBe(true);
    expect(status.config.poll_interval_ms).toBe(25);
    expect(status.config.max_preview_chars).toBe(8);
    expect(status.stats.entries).toBe(2);

    const listed = JSON.parse(run(["list"]));
    expect(listed.entries.map((entry: { id: number }) => entry.id)).toEqual([3, 2]);
    expect(listed.entries[1].preview).toBe("duplicat...");
  });

  test("accepts Clipse-style backend path aliases relative to the config file", () => {
    const configDir = join(tempDir, "clipse-config");
    const expectedDb = join(configDir, "history", "clipboard.sqlite");
    const expectedImageDir = join(configDir, "tmp_files");
    mkdirSync(configDir, { recursive: true });
    env.DITOX_CONFIG = join(configDir, "configuration.toml");
    writeFileSync(
      env.DITOX_CONFIG,
      [
        'historyFile = "history/clipboard.sqlite"',
        'tempDir = "tmp_files"',
        "",
      ].join("\n"),
    );

    expect(run(["add", "stored in clipse history path"])).toBe("1");
    expect(existsSync(expectedDb)).toBe(true);
    let status = JSON.parse(run(["status"]));
    expect(status.config.db_path).toBe(expectedDb);
    expect(status.stats.entries).toBe(1);

    const png = rgbaPng(1, 1, [[255, 0, 0, 255]]);
    expect(runWithStdin(["--wl-store"], png)).toBe("");
    status = JSON.parse(run(["status"]));
    expect(status.stats.images).toBe(1);

    const listed = JSON.parse(run(["list", "--filter", "images"]));
    expect(listed.entries).toHaveLength(1);
    expect(listed.entries[0].blob_path.startsWith(`${expectedImageDir}/`)).toBe(true);
    expect(existsSync(listed.entries[0].blob_path)).toBe(true);
  });

  test("loads Clipse-style configuration.json backend settings", () => {
    const configDir = join(tempDir, "clipse-json");
    const expectedDb = join(configDir, "history", "clipboard.sqlite");
    const expectedImageDir = join(configDir, "tmp_files");
    mkdirSync(configDir, { recursive: true });
    env.DITOX_CONFIG = join(configDir, "configuration.json");
    writeFileSync(
      env.DITOX_CONFIG,
      JSON.stringify(
        {
          allowDuplicates: true,
          historyFile: "history/clipboard.sqlite",
          maxHistory: 2,
          deleteAfter: 3,
          pollInterval: 35,
          maxEntryLength: 7,
          tempDir: "tmp_files",
          excludedApps: ["Bitwarden"],
          excludedWindows: ["Secret Window"],
          autoPaste: {
            enabled: true,
            keybind: "ctrl+shift+v",
            buffer: 4,
          },
        },
        null,
        2,
      ),
    );

    expect(run(["add", "duplicate json value"])).toBe("1");
    expect(run(["add", "duplicate json value"])).toBe("2");
    expect(run(["add", "newest json value"])).toBe("3");

    const status = JSON.parse(run(["status"]));
    expect(status.config.db_path).toBe(expectedDb);
    expect(status.config.image_dir).toBe(expectedImageDir);
    expect(status.config.max_entries).toBe(2);
    expect(status.config.delete_after_seconds).toBe(3);
    expect(status.config.allow_duplicates).toBe(true);
    expect(status.config.poll_interval_ms).toBe(35);
    expect(status.config.max_preview_chars).toBe(7);
    expect(status.config.excluded_apps).toEqual(["Bitwarden"]);
    expect(status.config.excluded_windows).toEqual(["Secret Window"]);
    expect(status.config.auto_paste_enabled).toBe(true);
    expect(status.config.auto_paste_keybind).toBe("ctrl+shift+v");
    expect(status.config.auto_paste_buffer_ms).toBe(4);
    expect(status.stats.entries).toBe(2);
    expect(existsSync(expectedDb)).toBe(true);
    expect(existsSync(expectedImageDir)).toBe(true);

    const listed = JSON.parse(run(["list"]));
    expect(listed.entries.map((entry: { id: number }) => entry.id)).toEqual([3, 2]);
    expect(listed.entries[1].preview).toBe("duplica...");
  });

  test("accepts Clipse-style CLI aliases for supported operations", () => {
    expect(run(["-a", "alias first"])).toBe("1");
    expect(run(["-a", "alias second"])).toBe("2");

    expect(run(["--output-all", "unescaped"]).split("\n").sort()).toEqual(["alias first", "alias second"]);
    expect(run(["--output-all", "raw"]).split("\n").map((line) => JSON.parse(line)).sort()).toEqual(["alias first", "alias second"]);

    run(["-c", "direct clipboard"]);
    expect(readFileSync(env.DITOX_CLIPBOARD_MOCK!, "utf8")).toBe("direct clipboard");
    expect(run(["-p"])).toBe("direct clipboard");

    expect(run(["favorite", "1"])).toBe("true");
    expect(run(["-a", "clear me"])).toBe("3");
    expect(run(["-clear"])).toBe("2");
    let status = JSON.parse(run(["status"]));
    expect(status.stats.entries).toBe(1);
    expect(status.stats.favorites).toBe(1);

    expect(run(["-a", "clear pinned too"])).toBe("4");
    expect(run(["-clear-text"])).toBe("2");
    status = JSON.parse(run(["status"]));
    expect(status.stats.entries).toBe(0);

    expect(run(["-a", "wipe one"])).toBe("5");
    expect(run(["-a", "wipe two"])).toBe("6");
    expect(run(["favorite", "5"])).toBe("true");
    expect(run(["-clear-all"])).toBe("2");
    status = JSON.parse(run(["status"]));
    expect(status.stats.entries).toBe(0);

    const clean = JSON.parse(run(["--clean"]));
    expect(clean.ok).toBe(true);
    expect(clean.sanitized_text).toBe(0);
    expect(clean.removed_missing_images).toBe(0);
  });

  test("recognizes unsupported platform-specific Clipse listener aliases", () => {
    expect(run(["-listen-x11"])).toContain("currently supports Wayland listening");
    expect(run(["--listen-darwin"])).toContain("currently supports Wayland listening");
  });

  test("stores wl-paste watch stdin through the Clipse wl-store alias", () => {
    expect(runWithStdin(["--wl-store"], "stored from wl watch")).toBe("");
    let listed = JSON.parse(run(["list"]));
    expect(listed.entries).toHaveLength(1);
    expect(listed.entries[0].kind).toBe("text");
    expect(listed.entries[0].content).toBe("stored from wl watch");

    const png = rgbaPng(3, 2, [
      [255, 0, 0, 255],
      [0, 255, 0, 255],
      [0, 0, 255, 255],
      [255, 255, 255, 255],
      [16, 32, 48, 255],
      [64, 80, 96, 255],
    ]);
    expect(runWithStdin(["--wl-store"], png)).toBe("");

    listed = JSON.parse(run(["list", "--filter", "images"]));
    expect(listed.entries).toHaveLength(1);
    expect(listed.entries[0].kind).toBe("image");
    expect(listed.entries[0].mime).toBe("image/png");
    expect(listed.entries[0].image_width).toBe(3);
    expect(listed.entries[0].image_height).toBe(2);
    expect(existsSync(listed.entries[0].blob_path)).toBe(true);

    expect(runWithStdin(["--wl-store"], "do not store", { CLIPBOARD_STATE: "sensitive" })).toBe("");
    const status = JSON.parse(run(["status"]));
    expect(status.stats.entries).toBe(2);
  });

  test("verifies Hyprland paste-back and launch wiring with fake tools", () => {
    const fakeBin = join(tempDir, "bin");
    const fakeClipboard = join(tempDir, "fake-clipboard.txt");
    const hyprLog = join(tempDir, "hypr.log");
    const launchTarget = join(tempDir, "launch-target.txt");
    const keepTarget = join(tempDir, "keep-target.txt");
    const launchKeepTarget = join(tempDir, "launch-keep-target.txt");
    const noArgLaunchTarget = join(tempDir, "no-arg-launch-target.txt");
    const realtimeTarget = join(tempDir, "realtime-target.txt");
    const tuiRealtimeTarget = join(tempDir, "tui-realtime-target.txt");
    mkdirSync(fakeBin);

    writeFileSync(
      join(fakeBin, "wl-copy"),
      "#!/usr/bin/env sh\ncat > \"$DITOX_FAKE_CLIPBOARD\"\n",
    );
    writeFileSync(
      join(fakeBin, "hyprctl"),
      [
        "#!/usr/bin/env sh",
        "printf '%s\\n' \"$*\" >> \"$DITOX_HYPR_LOG\"",
        "if [ \"$1\" = \"-j\" ] && [ \"$2\" = \"activewindow\" ]; then",
        "  printf '{\"address\":\"0xabc;echo bad\",\"class\":\"fake-app\",\"title\":\"fake-title\"}'",
        "fi",
        "exit 0",
        "",
      ].join("\n"),
    );
    chmodSync(join(fakeBin, "wl-copy"), 0o755);
    chmodSync(join(fakeBin, "hyprctl"), 0o755);

    delete env.DITOX_CLIPBOARD_MOCK;
    env.PATH = `${fakeBin}:${process.env.PATH ?? ""}`;
    env.DITOX_FAKE_CLIPBOARD = fakeClipboard;
    env.DITOX_HYPR_LOG = hyprLog;

    expect(run(["add", "hypr paste target"])).toBe("1");
    run(["paste", "1", "--target-window", "0xabc"]);
    expect(readFileSync(fakeClipboard, "utf8")).toBe("hypr paste target");

    const log = readFileSync(hyprLog, "utf8");
    expect(log).toContain("dispatch focuswindow address:0xabc");
    expect(log).toContain("dispatch sendshortcut CTRL,V,");

    env.DITOX_TERMINAL_COMMAND = `sh -c 'printf "$DITOX_TARGET_WINDOW" > ${launchTarget}'`;
    run(["launch"]);
    expect(readFileSync(launchTarget, "utf8")).toBe("0xabc;echo bad");

    env.DITOX_TERMINAL_COMMAND = `sh -c 'printf "$DITOX_TARGET_WINDOW|$DITOX_TUI_EXIT_AFTER_PASTE" > ${keepTarget}'`;
    run(["keep"]);
    expect(readFileSync(keepTarget, "utf8")).toBe("0xabc;echo bad|false");

    env.DITOX_TERMINAL_COMMAND = `sh -c 'printf "$DITOX_TARGET_WINDOW|$DITOX_TUI_EXIT_AFTER_PASTE" > ${launchKeepTarget}'`;
    run(["launch", "--keep"]);
    expect(readFileSync(launchKeepTarget, "utf8")).toBe("0xabc;echo bad|false");

    env.DITOX_TERMINAL_COMMAND = `sh -c 'printf "$DITOX_TARGET_WINDOW" > ${noArgLaunchTarget}'`;
    run([]);
    expect(readFileSync(noArgLaunchTarget, "utf8")).toBe("0xabc;echo bad");

    env.DITOX_TERMINAL_COMMAND = `sh -c 'printf "$DITOX_TARGET_WINDOW|$DITOX_TUI_REFRESH_MS" > ${realtimeTarget}'`;
    run(["-enable-real-time"]);
    expect(readFileSync(realtimeTarget, "utf8")).toBe("0xabc;echo bad|250");

    env.DITOX_TUI_COMMAND = `sh -c 'printf "$DITOX_TARGET_WINDOW|$DITOX_TUI_REFRESH_MS" > ${tuiRealtimeTarget}'`;
    run(["tui", "--enable-real-time"]);
    expect(readFileSync(tuiRealtimeTarget, "utf8")).toBe("0xabc;echo bad|250");
    delete env.DITOX_TUI_COMMAND;

    env.DITOX_CONFIG = join(tempDir, "config.toml");
    writeFileSync(env.DITOX_CONFIG, "[auto_paste]\nenabled = true\nkeybind = \"ctrl+shift+v\"\nbuffer_ms = 0\n");
    run(["--auto-paste", "--target-window", "0xabc"]);
    const autoPasteLog = readFileSync(hyprLog, "utf8");
    expect(autoPasteLog).toContain("dispatch focuswindow address:0xabc");
    expect(autoPasteLog).toContain("dispatch sendshortcut CTRL SHIFT,V,");
    const status = JSON.parse(run(["status"]));
    expect(status.config.auto_paste_enabled).toBe(true);
    expect(status.config.auto_paste_keybind).toBe("ctrl+shift+v");
    expect(status.config.auto_paste_buffer_ms).toBe(0);
  });

  test("kills the stored watcher process through the Clipse kill alias", async () => {
    env.DITOX_CONFIG = join(tempDir, "watcher.toml");
    writeFileSync(env.DITOX_CONFIG, "[watch]\npoll_interval_ms = 50\n");

    const fakeBin = join(tempDir, "watcher-bin");
    mkdirSync(fakeBin);
    writeFileSync(join(fakeBin, "wl-paste"), "#!/usr/bin/env sh\nexit 1\n");
    writeFileSync(join(fakeBin, "hyprctl"), "#!/usr/bin/env sh\nexit 1\n");
    chmodSync(join(fakeBin, "wl-paste"), 0o755);
    chmodSync(join(fakeBin, "hyprctl"), 0o755);
    env.PATH = `${fakeBin}:${process.env.PATH ?? ""}`;

    const proc = Bun.spawn({
      cmd: [ditoxd, "watch"],
      env: { ...process.env, ...env },
      stdout: "pipe",
      stderr: "pipe",
    });

    try {
      await waitUntil(() => JSON.parse(run(["status"])).watcher.running === true, 2500);
      const result = JSON.parse(run(["--kill"]));
      expect(result.killed).toBe(true);
      expect(typeof result.pid).toBe("number");
      expect(await waitForExit(proc, 2500)).not.toBeNull();
      const stale = JSON.parse(run(["--kill"]));
      expect(stale.killed).toBe(false);
    } finally {
      proc.kill();
      await proc.exited.catch(() => {});
    }
  });

  test("skips excluded active apps and windows case-insensitively while watching", async () => {
    env.DITOX_CONFIG = join(tempDir, "excluded.toml");
    writeFileSync(
      env.DITOX_CONFIG,
      [
        'excludedApps = ["Bitwarden"]',
        'excludedWindows = ["Secret Window"]',
        "",
        "[watch]",
        "poll_interval_ms = 50",
        "",
      ].join("\n"),
    );

    const fakeBin = join(tempDir, "excluded-bin");
    mkdirSync(fakeBin);
    writeFileSync(
      join(fakeBin, "wl-paste"),
      [
        "#!/usr/bin/env sh",
        'if [ "$1" = "--list-types" ]; then',
        "  printf 'text/plain\\n'",
        "  exit 0",
        "fi",
        "printf 'excluded clipboard text'",
        "",
      ].join("\n"),
    );
    writeFileSync(
      join(fakeBin, "hyprctl"),
      [
        "#!/usr/bin/env sh",
        'if [ "$1" = "-j" ] && [ "$2" = "activewindow" ]; then',
        '  printf \'{"address":"0xabc","class":"bitwarden","title":"secret window"}\'',
        "fi",
        "exit 0",
        "",
      ].join("\n"),
    );
    chmodSync(join(fakeBin, "wl-paste"), 0o755);
    chmodSync(join(fakeBin, "hyprctl"), 0o755);
    delete env.DITOX_CLIPBOARD_MOCK;
    env.PATH = `${fakeBin}:${process.env.PATH ?? ""}`;

    const proc = Bun.spawn({
      cmd: [ditoxd, "watch"],
      env: { ...process.env, ...env },
      stdout: "pipe",
      stderr: "pipe",
    });

    try {
      await waitUntil(() => JSON.parse(run(["status"])).watcher.running === true, 2500);
      await sleep(180);
      const status = JSON.parse(run(["status"]));
      expect(status.config.excluded_apps).toEqual(["Bitwarden"]);
      expect(status.config.excluded_windows).toEqual(["Secret Window"]);
      expect(status.stats.entries).toBe(0);
      expect(JSON.parse(run(["list"])).entries).toHaveLength(0);
    } finally {
      proc.kill();
      await proc.exited.catch(() => {});
    }
  });

  test("copies and pastes image entries as image bytes with their MIME type", () => {
    JSON.parse(run(["status"]));

    const imageDir = join(env.DITOX_DATA_DIR!, "images-v2", "12");
    mkdirSync(imageDir, { recursive: true });
    const imagePath = join(imageDir, "123456.png");
    const png = rgbaPng(1, 1, [[16, 32, 48, 255]]);
    writeFileSync(imagePath, png);

    const db = new Database(join(env.DITOX_DATA_DIR!, "ditox-v2.db"));
    db.run(
      "INSERT INTO entries (kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, blob_path, image_width, image_height) VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?)",
      "image",
      "image/png",
      "123456",
      "image/png 1x1",
      "123456",
      Date.now(),
      png.length,
      imagePath,
      1,
      1,
    );
    db.close();

    const fakeBin = join(tempDir, "bin");
    const fakeClipboard = join(tempDir, "fake-image-clipboard.bin");
    const wlCopyArgs = join(tempDir, "wl-copy.args");
    const hyprLog = join(tempDir, "hypr-image.log");
    mkdirSync(fakeBin);
    writeFileSync(
      join(fakeBin, "wl-copy"),
      "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" > \"$DITOX_WL_COPY_ARGS\"\ncat > \"$DITOX_FAKE_CLIPBOARD\"\n",
    );
    writeFileSync(
      join(fakeBin, "hyprctl"),
      "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" >> \"$DITOX_HYPR_LOG\"\nexit 0\n",
    );
    chmodSync(join(fakeBin, "wl-copy"), 0o755);
    chmodSync(join(fakeBin, "hyprctl"), 0o755);

    delete env.DITOX_CLIPBOARD_MOCK;
    env.PATH = `${fakeBin}:${process.env.PATH ?? ""}`;
    env.DITOX_FAKE_CLIPBOARD = fakeClipboard;
    env.DITOX_WL_COPY_ARGS = wlCopyArgs;
    env.DITOX_HYPR_LOG = hyprLog;

    run(["copy", "1"]);
    expect(readFileSync(wlCopyArgs, "utf8").trim()).toBe("--type image/png");
    expect([...readFileSync(fakeClipboard)]).toEqual([...png]);

    run(["paste", "1", "--target-window", "0xdef"]);
    expect(readFileSync(wlCopyArgs, "utf8").trim()).toBe("--type image/png");
    expect([...readFileSync(fakeClipboard)]).toEqual([...png]);
    const log = readFileSync(hyprLog, "utf8");
    expect(log).toContain("dispatch focuswindow address:0xdef");
    expect(log).toContain("dispatch sendshortcut CTRL,V,");
  });

  test("renders PNG image history as terminal block cells in the TUI smoke path", () => {
    expect(run(["add", "text seed"])).toBe("1");

    const imageDir = join(env.DITOX_DATA_DIR!, "images-v2", "ab");
    mkdirSync(imageDir, { recursive: true });
    const imagePath = join(imageDir, "abcdef.png");
    const png = rgbaPng(2, 2, [
      [255, 0, 0, 255],
      [0, 255, 0, 255],
      [0, 0, 255, 255],
      [255, 255, 255, 255],
    ]);
    writeFileSync(imagePath, png);

    const db = new Database(join(env.DITOX_DATA_DIR!, "ditox-v2.db"));
    db.run(
      "INSERT INTO entries (kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, blob_path, image_width, image_height) VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?)",
      "image",
      "image/png",
      "abcdef",
      "image/png 2x2",
      "abcdef",
      Date.now() + 1000,
      png.length,
      imagePath,
      2,
      2,
    );
    db.close();

    const proc = Bun.spawnSync({
      cmd: ["timeout", "--foreground", "--kill-after=1s", "1s", "bun", "run", "--cwd", "tui", "start"],
      cwd: repoRoot,
      env: {
        ...process.env,
        ...env,
        DITOXD: ditoxd,
        DITOX_TUI_REFRESH_MS: "0",
        DITOX_TUI_IMAGE_PREVIEW: "blocks",
      },
      stdout: "pipe",
      stderr: "pipe",
    });

    expect(proc.exitCode).toBe(124);
    expect(`${proc.stdout?.toString() ?? ""}${proc.stderr?.toString() ?? ""}`).toContain("▙");
  });

  test("renders file-based TUI customization in the real OpenTUI smoke path", () => {
    expect(run(["add", "visual smoke entry"])).toBe("1");
    const tuiConfigPath = join(tempDir, "tui.json");
    writeFileSync(
      tuiConfigPath,
      JSON.stringify({
        layout: {
          refreshIntervalMs: 0,
          listWidthPercent: 50,
          showMetadata: true,
          imagePreviewMode: "metadata",
          mouseEnabled: false,
        },
        chrome: {
          panelBorderStyle: "double",
          selectedMarker: "!!",
          normalMarker: "..",
          statusSeparator: "::",
        },
        labels: {
          appTitle: "visual",
          brand: "VIS",
          historyTitle: "clips",
          previewTitle: "peek",
          watcherStopped: "daemon off",
          pinned: "SAVED",
        },
        keyBindings: {
          copyPaste: "ctrl+p",
          preview: "ctrl+o",
          help: "f1",
        },
      }),
    );

    const proc = Bun.spawnSync({
      cmd: ["timeout", "--foreground", "--kill-after=1s", "1s", "bun", "run", "--cwd", "tui", "start"],
      cwd: repoRoot,
      env: {
        ...process.env,
        ...env,
        DITOXD: ditoxd,
        DITOX_TUI_CONFIG: tuiConfigPath,
      },
      stdout: "pipe",
      stderr: "pipe",
    });

    expect(proc.exitCode).toBe(124);
    const output = `${proc.stdout?.toString() ?? ""}${proc.stderr?.toString() ?? ""}`;
    expect(output).toContain("VIS");
    expect(output).toContain("clips");
    expect(output).toContain("peek");
    expect(output).toContain("visual smoke entry");
    expect(output).toContain("!!");
    expect(output).toContain("ctrl+p paste");
    expect(output).toContain("ctrl+o preview");
    expect(output).toContain("::");
    expect(output).toContain("daemon off");
  });

  test("expires non-pinned entries by configured TTL and removes image blobs", () => {
    env.DITOX_CONFIG = join(tempDir, "config.toml");
    writeFileSync(env.DITOX_CONFIG, "[history]\ndelete_after_seconds = 1\n");

    expect(run(["add", "old unpinned"])).toBe("1");
    expect(run(["add", "old pinned"])).toBe("2");
    expect(run(["favorite", "2"])).toBe("true");

    const imageDir = join(env.DITOX_DATA_DIR!, "images-v2", "de");
    mkdirSync(imageDir, { recursive: true });
    const imagePath = join(imageDir, "deadbeef.png");
    writeFileSync(imagePath, rgbaPng(1, 1, [[255, 0, 0, 255]]));

    const db = new Database(join(env.DITOX_DATA_DIR!, "ditox-v2.db"));
    db.run(
      "INSERT INTO entries (kind, mime, content, preview, hash, favorite, created_at_ms, byte_len, blob_path, image_width, image_height) VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?)",
      "image",
      "image/png",
      "deadbeef",
      "image/png 1x1",
      "deadbeef",
      Date.now() - 10_000,
      70,
      imagePath,
      1,
      1,
    );
    db.run("UPDATE entries SET created_at_ms = ? WHERE id IN (1, 2)", Date.now() - 10_000);
    db.close();

    const status = JSON.parse(run(["status"]));
    expect(status.config.delete_after_seconds).toBe(1);
    expect(status.stats.entries).toBe(1);
    expect(status.stats.favorites).toBe(1);
    expect(existsSync(imagePath)).toBe(false);

    const listed = JSON.parse(run(["list"]));
    expect(listed.entries).toHaveLength(1);
    expect(listed.entries[0].content).toBe("old pinned");
  });
});

function run(args: string[]): string {
  const proc = Bun.spawnSync({
    cmd: [ditox, ...args],
    env: { ...process.env, ...env },
    stdout: "pipe",
    stderr: "pipe",
  });

  if (proc.exitCode !== 0) {
    throw new Error(`${args.join(" ")} failed: ${proc.stderr?.toString() ?? ""}`);
  }
  return (proc.stdout?.toString() ?? "").trim();
}

function runWithStdin(args: string[], stdin: string | Uint8Array, extraEnv: Record<string, string> = {}): string {
  const proc = Bun.spawnSync({
    cmd: [ditox, ...args],
    env: { ...process.env, ...env, ...extraEnv },
    stdin: typeof stdin === "string" ? Buffer.from(stdin) : Buffer.from(stdin),
    stdout: "pipe",
    stderr: "pipe",
  });

  if (proc.exitCode !== 0) {
    throw new Error(`${args.join(" ")} failed: ${proc.stderr?.toString() ?? ""}`);
  }
  return (proc.stdout?.toString() ?? "").trim();
}

async function waitUntil(predicate: () => boolean, timeoutMs: number): Promise<void> {
  const started = Date.now();
  let lastError: unknown;
  while (Date.now() - started < timeoutMs) {
    try {
      if (predicate()) return;
    } catch (error) {
      lastError = error;
    }
    await sleep(50);
  }
  if (lastError) throw lastError;
  throw new Error(`timed out after ${timeoutMs}ms`);
}

async function waitForExit(proc: Bun.Subprocess, timeoutMs: number): Promise<number | null> {
  return await Promise.race([proc.exited, sleep(timeoutMs).then(() => null)]);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function rgbaPng(width: number, height: number, pixels: Array<[number, number, number, number]>): Uint8Array {
  const raw = new Uint8Array(height * (1 + width * 4));
  let offset = 0;
  let pixel = 0;
  for (let y = 0; y < height; y += 1) {
    raw[offset++] = 0;
    for (let x = 0; x < width; x += 1) {
      raw.set(pixels[pixel++]!, offset);
      offset += 4;
    }
  }

  return concatBytes(
    Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", concatBytes(u32(width), u32(height), Uint8Array.from([8, 6, 0, 0, 0]))),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", new Uint8Array()),
  );
}

function chunk(type: string, data: Uint8Array): Uint8Array {
  const typeBytes = Uint8Array.from([...type].map((char) => char.charCodeAt(0)));
  return concatBytes(u32(data.length), typeBytes, data, Uint8Array.from([0, 0, 0, 0]));
}

function u32(value: number): Uint8Array {
  return Uint8Array.from([(value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff]);
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}
