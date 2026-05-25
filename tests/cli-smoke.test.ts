import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const repoRoot = join(import.meta.dir, "..");
const ditox = join(repoRoot, "zig-out", "bin", "ditox");

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
    expect(run(["clear", "text"])).toBe("1");

    expect(run(["add", "gamma"])).toBe("3");
    expect(run(["clear", "all"])).toBe("1");

    const repair = JSON.parse(run(["repair"]));
    expect(repair.ok).toBe(true);

    run(["pause", "100"]);
    status = JSON.parse(run(["status"]));
    expect(status.watcher.paused).toBe(true);
    expect(run(["resume"])).toBe("resumed");
  });

  test("verifies Hyprland paste-back and launch wiring with fake tools", () => {
    const fakeBin = join(tempDir, "bin");
    const fakeClipboard = join(tempDir, "fake-clipboard.txt");
    const hyprLog = join(tempDir, "hypr.log");
    const launchTarget = join(tempDir, "launch-target.txt");
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
        "  printf '{\"address\":\"0xabc\",\"class\":\"fake-app\",\"title\":\"fake-title\"}'",
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
    expect(readFileSync(launchTarget, "utf8")).toBe("0xabc");
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
