import { describe, expect, test } from "bun:test";
import {
  clampSelection,
  formatAge,
  initialState,
  moveSelection,
  nextFilter,
  previewLines,
  selectedIdsOrCurrent,
  toggleSelectedId,
  truncateText,
  visibleEntries,
} from "./state";
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
  image_width: null,
  image_height: null,
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

  test("uses marked entries or current entry for bulk operations", () => {
    expect(selectedIdsOrCurrent({ ...initialState(), entries: [entry] })).toEqual([7]);
    expect(selectedIdsOrCurrent(toggleSelectedId({ ...initialState(), entries: [entry] }))).toEqual([7]);
  });

  test("windows visible entries around the selection", () => {
    const entries = Array.from({ length: 10 }, (_, index) => ({ ...entry, id: index + 1 }));
    expect(visibleEntries(entries, 8, 3).map((row) => row.index)).toEqual([7, 8, 9]);
  });

  test("cycles filters", () => {
    expect(nextFilter("all")).toBe("text");
    expect(nextFilter("today")).toBe("all");
  });

  test("formats age", () => {
    expect(formatAge(0, 65_000)).toBe("1m");
  });

  test("previews text content", () => {
    expect(previewLines(entry)).toEqual(["  1: hello", "  2: world"]);
  });

  test("previews image metadata", () => {
    expect(previewLines({ ...entry, kind: "image", image_width: 2, image_height: 3 })[3]).toBe("dims: 2x3");
  });

  test("truncates long row text", () => {
    expect(truncateText("abcdef", 4)).toBe("a...");
  });
});
