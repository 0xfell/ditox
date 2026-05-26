import { describe, expect, test } from "bun:test";
import {
  estimatedImagePreviewRows,
  fullPreviewReservedRows,
  imageProtocolCapabilities,
  shutdownTui,
  terminalExitResetSequence,
  writeTerminalExitReset,
} from "./App";
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
  byte_len: 11,
  source_app: null,
  blob_path: "/tmp/ditox-image.png",
  image_width: 2,
  image_height: 2,
};

describe("App helpers", () => {
  test("maps OpenTUI terminal capabilities into image protocol support", () => {
    expect(
      imageProtocolCapabilities({
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
      }),
    ).toEqual({ kittyGraphics: true, sixel: false, nativeRenderer: false });
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
