import { describe, expect, test } from "bun:test";
import {
  copyOrPasteEntry,
  estimatedImagePreviewRows,
  fullPreviewReservedRows,
  imageProtocolCapabilities,
  runtimeKeysForBinding,
  runtimeSequenceKey,
  shutdownTui,
  terminalExitResetSequence,
  writeTerminalExitReset,
} from "./App";
import { createTestKeymap } from "@opentui/keymap/testing";
import { resolveTuiConfig } from "./tui-config";
import type { Entry } from "./types";

const imageEntry: Entry = {
  id: 7,
  kind: "image",
  mime: "image/png",
  content: "abc",
  preview: "Image entry",
  hash: "abc",
  favorite: false,
  created_at_ms: 1000,
  last_used_at_ms: null,
  byte_len: 11,
  source_app: null,
  blob_path: "/tmp/ditox-image.png",
  image_width: 2,
  image_height: 2,
};

describe("App helpers", () => {
  test("expands Enter bindings for OpenTUI return key events", () => {
    expect(runtimeKeysForBinding(["enter"])).toEqual(["enter", "return"]);
    expect(runtimeKeysForBinding(["return"])).toEqual(["enter", "return"]);
    expect(runtimeKeysForBinding(["ctrl+return"])).toEqual(["ctrl+enter", "ctrl+return"]);
    expect(runtimeKeysForBinding(["enter", "return"])).toEqual(["enter", "return"]);
  });

  test("collapses multi-stroke chord whitespace into contiguous keymap sequences", () => {
    // Regression: "c a" was handed to @opentui/keymap verbatim, which reads the
    // space as the `space` key (strokes [c, space, a]), so the clear chords never
    // fired. Each stroke must be normalized and concatenated.
    expect(runtimeSequenceKey("c a")).toBe("ca");
    expect(runtimeSequenceKey("c t")).toBe("ct");
    expect(runtimeSequenceKey("c i")).toBe("ci");
    expect(runtimeSequenceKey("c x")).toBe("cx");
    expect(runtimeSequenceKey("ctrl+x s")).toBe("ctrl+xs");
    // Single strokes (including the literal space preview key) pass through.
    expect(runtimeSequenceKey("space")).toBe("space");
    expect(runtimeSequenceKey(" ")).toBe("space");
    expect(runtimeSequenceKey("shift+tab")).toBe("shift+tab");
    expect(runtimeSequenceKey("enter")).toBe("enter");
    expect(runtimeKeysForBinding(["c a"])).toEqual(["ca"]);
  });

  test("default clear chords parse into two-stroke sequences in the real keymap", () => {
    const config = resolveTuiConfig();
    const km = createTestKeymap({ defaultKeys: true });
    const parseSequence = (km.keymap as unknown as { parseKeySequence: (key: string) => unknown }).parseKeySequence.bind(km.keymap);
    type SeqPart = { display?: string; stroke?: { name?: string } };
    const strokeNames = (key: string): string[] => {
      const parsed = parseSequence(key) as SeqPart[] | { parts?: SeqPart[] };
      const parts = Array.isArray(parsed) ? parsed : (parsed.parts ?? []);
      return parts.map((part) => part.display ?? part.stroke?.name ?? "?");
    };
    for (const binding of [config.keyBindings.clearAll, config.keyBindings.clearText, config.keyBindings.clearImages, config.keyBindings.clearAllIncludingPinned]) {
      const runtime = runtimeKeysForBinding(binding);
      expect(runtime.length).toBeGreaterThan(0);
      // Each default clear chord is "c <letter>" -> two strokes [c, <letter>].
      const names = strokeNames(runtime[0]!);
      expect(names.length).toBe(2);
      expect(names[0]).toBe("c");
      expect(names).not.toContain("space");
    }
    km.cleanup();
  });

  test("uses copy-only when paste is requested without a target window", async () => {
    const calls: string[] = [];
    const config = resolveTuiConfig();

    const result = await copyOrPasteEntry(4, {
      paste: true,
      labels: config.labels,
      rpc: {
        copyEntry: async (id) => {
          calls.push(`copy:${id}`);
          return { copied: true };
        },
        pasteEntry: async (id) => {
          calls.push(`paste:${id}`);
          return { pasted: true };
        },
      },
    });

    expect(result).toBe("copied");
    expect(calls).toEqual(["copy:4"]);
  });

  test("falls back to copy when Hyprland paste-back fails", async () => {
    const calls: string[] = [];
    const config = resolveTuiConfig();

    const result = await copyOrPasteEntry(5, {
      paste: true,
      targetWindow: "0xabc",
      labels: config.labels,
      rpc: {
        copyEntry: async (id) => {
          calls.push(`copy:${id}`);
          return { copied: true };
        },
        pasteEntry: async (id) => {
          calls.push(`paste:${id}`);
          throw new Error(config.labels.errorPasteBackFailed);
        },
      },
    });

    expect(result).toBe("copied");
    expect(calls).toEqual(["paste:5", "copy:5"]);
  });

  test("does not hide clipboard write failures behind paste fallback", async () => {
    const config = resolveTuiConfig();

    await expect(
      copyOrPasteEntry(6, {
        paste: true,
        targetWindow: "0xabc",
        labels: config.labels,
        rpc: {
          copyEntry: async () => ({ copied: true }),
          pasteEntry: async () => {
            throw new Error(config.labels.errorClipboardWriteFailed);
          },
        },
      }),
    ).rejects.toThrow(config.labels.errorClipboardWriteFailed);
  });

  test("maps OpenTUI terminal capabilities into image protocol support", () => {
    const capabilities = {
      kitty_keyboard: true,
      kitty_graphics: true,
      rgb: true,
      ansi256: true,
      unicode: "unicode",
      sgr_pixels: false,
      color_scheme_updates: false,
      explicit_width: true,
      scaled_text: false,
      sixel: false,
      focus_tracking: true,
      sync: true,
      bracketed_paste: true,
      hyperlinks: true,
      osc52: true,
      notifications: false,
      explicit_cursor_positioning: true,
      in_tmux: false,
      terminal: { name: "kitty", version: "0.40.0", from_xtversion: true },
    } as const;

    expect(imageProtocolCapabilities(capabilities)).toEqual({ kittyGraphics: true, sixel: false, nativeRenderer: false });
    expect(imageProtocolCapabilities(capabilities, { width: 1200, height: 800 })).toEqual({ kittyGraphics: true, sixel: false, nativeRenderer: true });
  });

  test("reserves full preview image notice spacing for scroll capacity", () => {
    const config = resolveTuiConfig({
      layout: {
        fullPreviewImageMode: "blocks",
        fullPreviewImageMaxRows: 4,
        fullPreviewImageRowInset: 0,
        fullPreviewImageNoticeVisibility: "always",
        fullPreviewImageNoticeSpacing: 2,
        fullPreviewMetaHeight: 1,
        showFullPreviewMetadata: true,
      },
    });

    expect(estimatedImagePreviewRows(imageEntry, config, 6)).toBe(7);
    expect(fullPreviewReservedRows(imageEntry, config, 6)).toBe(8);
  });

  test("does not reserve notice spacing when the full preview notice is hidden", () => {
    const config = resolveTuiConfig({
      layout: {
        fullPreviewImageMode: "blocks",
        fullPreviewImageMaxRows: 4,
        fullPreviewImageRowInset: 0,
        fullPreviewImageNoticeVisibility: "never",
        fullPreviewImageNoticeSpacing: 3,
        showFullPreviewMetadata: false,
      },
    });

    expect(fullPreviewReservedRows(imageEntry, config, 6)).toBe(4);
  });

  test("writes terminal reset escapes and destroys renderer before exit", () => {
    const writes: string[] = [];
    const exits: number[] = [];
    const calls: string[] = [];
    shutdownTui(
      {
        destroy: () => calls.push("destroy"),
      },
      {
        stdout: {
          isTTY: true,
          write: (chunk) => writes.push(chunk),
        },
        afterDestroy: () => calls.push("after"),
        exit: (code) => exits.push(code),
        schedule: (callback) => callback(),
      },
    );

    expect(calls).toEqual(["destroy", "after"]);
    expect(writes).toEqual([terminalExitResetSequence]);
    expect(terminalExitResetSequence).toContain("\x1b[?1006l");
    expect(terminalExitResetSequence).toContain("\x1b[?1003l");
    expect(exits).toEqual([0]);
  });

  test("does not write terminal reset escapes to non-tty stdout", () => {
    const writes: string[] = [];
    writeTerminalExitReset({ isTTY: false, write: (chunk) => writes.push(chunk) });
    expect(writes).toEqual([]);
  });
});
