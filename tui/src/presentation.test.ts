import { describe, expect, test } from "bun:test";
import { entryMeta, formatBytes, previewModel, scrollbarCells, statusTone } from "./presentation";
import type { Entry } from "./types";

const entry: Entry = {
  id: 7,
  kind: "text",
  mime: "text/plain;charset=utf-8",
  content: "hello\nworld",
  preview: "hello",
  hash: "abc123",
  favorite: false,
  created_at_ms: 1000,
  byte_len: 11,
  source_app: null,
  blob_path: null,
  image_width: null,
  image_height: null,
};

describe("presentation", () => {
  test("formats byte sizes for row metadata", () => {
    expect(formatBytes(42)).toBe("42 B");
    expect(formatBytes(1536)).toBe("1.5 KiB");
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0 MiB");
  });

  test("builds stable compact entry metadata", () => {
    expect(entryMeta({ ...entry, favorite: true }, 66_000)).toBe("TXT 1m   11 B PIN");
  });

  test("models image preview rows with semantic gutters", () => {
    const rows = previewModel({ ...entry, kind: "image", image_width: 2, image_height: 3 }, 6);
    expect(rows.map((row) => `${row.gutter}:${row.text}`)).toContain("dims:2x3");
  });

  test("creates a bounded scrollbar thumb", () => {
    expect(scrollbarCells(10, 5, 4)).toEqual(["|", "|", "#", "|"]);
  });

  test("maps status text to semantic tones", () => {
    expect(statusTone("copied 2")).toBe("success");
    expect(statusTone("PasteBackFailed")).toBe("error");
    expect(statusTone("paused watcher")).toBe("warning");
  });
});
