import { describe, expect, test } from "bun:test";
import { clampSelection, formatAge, initialState, moveSelection, nextFilter, previewLines, toggleSelectedId } from "./state";
import type { Entry } from "./types";

const entry: Entry = {
  id: 7,
  kind: "text",
  mime: "text/plain;charset=utf-8",
  content: "hello\nworld",
  preview: "hello",
  hash: "abc",
  favorite: false,
  created_at_ms: 1000,
  byte_len: 11,
  source_app: null,
  blob_path: null,
};

describe("state", () => {
  test("selection is clamped to available entries", () => {
    const state = clampSelection({ ...initialState(), entries: [entry], selectedIndex: 5 });
    expect(state.selectedIndex).toBe(0);
  });

  test("moves selection", () => {
    const state = moveSelection({ ...initialState(), entries: [entry, { ...entry, id: 8 }], selectedIndex: 0 }, 1);
    expect(state.selectedIndex).toBe(1);
  });

  test("toggles multiselect", () => {
    const selected = toggleSelectedId({ ...initialState(), entries: [entry] });
    expect(selected.selectedIds.has(7)).toBe(true);
  });

  test("cycles filters", () => {
    expect(nextFilter("all")).toBe("text");
    expect(nextFilter("today")).toBe("all");
  });

  test("formats age", () => {
    expect(formatAge(0, 65_000)).toBe("1m");
  });

  test("previews text content", () => {
    expect(previewLines(entry)).toEqual(["hello", "world"]);
  });
});

