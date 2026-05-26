import { describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { formatDitoxdMissing, formatProcessError, formatRpcError, listEntries, parseFrame } from "./rpc";

describe("rpc presentation", () => {
  test("formats transport errors with defaults", () => {
    expect(formatDitoxdMissing("ditoxd")).toBe("ditoxd not found or not executable");
    expect(formatProcessError("entries.copy", "error: FileNotFound", 1)).toBe("wl-copy was not found or could not be started");
    expect(formatProcessError("entries.paste", "error: FileNotFound", 1)).toBe("wl-copy or hyprctl was not found");
    expect(formatRpcError("entries.paste", "PasteBackFailed")).toBe("failed to paste through Hyprland");
    expect(formatProcessError("watcher.status", "boom", 12)).toBe("boom");
    expect(formatRpcError("watcher.status", "StrangeError")).toBe("StrangeError");
    expect(formatProcessError("watcher.status", "", null)).toBe("ditoxd exited with unknown status");
  });

  test("formats transport errors with configured labels", () => {
    const labels = {
      errorDitoxdMissing: "backend missing: {binary}",
      errorClipboardToolMissing: "copy transport missing",
      errorPasteToolMissing: "paste transport missing",
      errorClipboardWriteFailed: "copy write failed",
      errorPasteBackFailed: "paste transport failed",
      errorDitoxdExited: "backend exited: {status}",
      errorUnknownStatus: "mystery status",
      errorProcessTemplate: "{method} failed[{status}]: {message}",
      errorRpcTemplate: "{method} rpc failed: {message}",
    };

    expect(formatDitoxdMissing("./bin/ditoxd", labels)).toBe("backend missing: ./bin/ditoxd");
    expect(formatProcessError("entries.copy", "error: FileNotFound", 1, labels)).toBe("copy transport missing");
    expect(formatProcessError("entries.paste", "error: FileNotFound", 1, labels)).toBe("paste transport missing");
    expect(formatProcessError("entries.copy", "ClipboardWriteFailed", 1, labels)).toBe("copy write failed");
    expect(formatRpcError("entries.paste", "PasteBackFailed", labels)).toBe("paste transport failed");
    expect(formatProcessError("watcher.status", "boom", 7, labels)).toBe("watcher.status failed[7]: boom");
    expect(formatRpcError("watcher.status", "StrangeError", labels)).toBe("watcher.status rpc failed: StrangeError");
    expect(formatProcessError("watcher.status", "", 42, labels)).toBe("watcher.status failed[42]: backend exited: 42");
    expect(formatProcessError("watcher.status", "", null, labels)).toBe("watcher.status failed[mystery status]: backend exited: mystery status");
  });

  test("parses content-length framed JSON", () => {
    expect(parseFrame<{ ok: boolean }>('Content-Length: 11\r\n\r\n{"ok":true}')).toEqual({ ok: true });
  });

  test("sends configured list limit to the backend", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-rpc-"));
    const fakeDitoxd = join(dir, "ditoxd");
    const capture = join(dir, "stdin.frame");
    writeFileSync(
      fakeDitoxd,
      [
        "#!/bin/sh",
        `cat > '${capture}'`,
        'body=\'{"jsonrpc":"2.0","id":"fake","result":{"entries":[]}}\'',
        'printf "Content-Length: %s\\r\\n\\r\\n%s" "${#body}" "$body"',
        "",
      ].join("\n"),
    );
    chmodSync(fakeDitoxd, 0o755);

    const previousDitoxd = Bun.env.DITOXD;
    Bun.env.DITOXD = fakeDitoxd;
    try {
      await listEntries("needle", "images", 321);
      const request = parseFrame<{ method: string; params: { query: string; filter: string; limit: number } }>(readFileSync(capture, "utf8"));
      expect(request.method).toBe("entries.list");
      expect(request.params).toEqual({ query: "needle", filter: "images", limit: 321 });
    } finally {
      if (previousDitoxd === undefined) delete Bun.env.DITOXD;
      else Bun.env.DITOXD = previousDitoxd;
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("contract exposes method-specific request and success schemas", () => {
    const schema = JSON.parse(readFileSync(join(import.meta.dir, "../../contracts/rpc.schema.json"), "utf8"));
    const defs = schema.$defs;

    expect(defs.entriesPasteRequest.required).toEqual(["jsonrpc", "id", "method", "params"]);
    expect(defs.entriesPasteRequest.properties.method.const).toBe("entries.paste");
    expect(defs.entriesPasteRequest.properties.params.$ref).toBe("#/$defs/pasteParams");
    expect(defs.entriesPasteSuccess.properties.result.$ref).toBe("#/$defs/pasteResult");
    expect(defs.entriesFavoriteRequest.properties.params.$ref).toBe("#/$defs/favoriteParams");
    expect(defs.request.oneOf.some((item: { $ref: string }) => item.$ref === "#/$defs/entriesPasteRequest")).toBe(true);
    expect(defs.success.oneOf.some((item: { $ref: string }) => item.$ref === "#/$defs/entriesPasteSuccess")).toBe(true);
    expect(JSON.stringify(defs.entriesPasteRequest)).not.toContain("baseRequest");
  });
});
