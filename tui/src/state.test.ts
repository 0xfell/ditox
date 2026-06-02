import { describe, expect, test } from "bun:test";
import {
  clampSelection,
  applySearch,
  cancelSearch,
  formatAge,
  initialState,
  moveEnd,
  moveHome,
  movePage,
  movePreview,
  moveSelection,
  nextFilter,
  openSearch,
  previewScrollCapacity,
  visibleFullPreviewLineCapacity,
  previewLines,
  selectRange,
  selectSingle,
  selectThroughIndex,
  selectedIdsOrCurrent,
  selectedSetIncludesPinned,
  togglePinnedOnly,
  toggleSelectedId,
  truncateText,
  updateSearchQuery,
  visibleEntries,
  visibleEntryCapacity,
  visiblePreviewLineCapacity,
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
  last_used_at_ms: null,
  byte_len: 11,
  source_app: null,
  blob_path: null,
  image_width: null,
  image_height: null,
};

describe("state", () => {
  test("creates configurable initial browse state", () => {
    expect(initialState({ filter: "images", pinnedOnly: true, query: "logo" })).toMatchObject({
      filter: "images",
      pinnedOnly: true,
      query: "logo",
      mode: "browse",
      selectedIndex: 0,
    });
  });

  test("selection is clamped to available entries", () => {
    const state = clampSelection({ ...initialState(), entries: [entry], selectedIndex: 5 });
    expect(state.selectedIndex).toBe(0);
  });

  test("moves selection", () => {
    const state = moveSelection({ ...initialState(), entries: [entry, { ...entry, id: 8 }], selectedIndex: 0 }, 1);
    expect(state.selectedIndex).toBe(1);
  });

  test("moves by page and boundaries", () => {
    const entries = Array.from({ length: 10 }, (_, index) => ({ ...entry, id: index + 1 }));
    const state = { ...initialState(), entries, selectedIndex: 4 };
    expect(movePage(state, 3, 1).selectedIndex).toBe(7);
    expect(movePage(state, 8, -1).selectedIndex).toBe(0);
    expect(moveHome(state).selectedIndex).toBe(0);
    expect(moveEnd(state).selectedIndex).toBe(9);
  });

  test("moves preview offset within rendered line bounds", () => {
    const state = { ...initialState(), previewOffset: 2 };
    expect(movePreview(state, 2, 10, 4).previewOffset).toBe(4);
    expect(movePreview(state, 99, 10, 4).previewOffset).toBe(6);
    expect(movePreview(state, -99, 10, 4).previewOffset).toBe(0);
  });

  test("toggles multiselect", () => {
    const selected = toggleSelectedId({ ...initialState(), entries: [entry] });
    expect(selected.selectedIds.has(7)).toBe(true);
  });

  test("selects single and range", () => {
    const entries = [entry, { ...entry, id: 8 }, { ...entry, id: 9 }];
    const single = selectSingle({ ...initialState(), entries, selectedIndex: 1 });
    expect([...single.selectedIds]).toEqual([8]);
    const ranged = selectRange(single, 1);
    expect(ranged.selectedIndex).toBe(2);
    expect([...ranged.selectedIds].sort()).toEqual([8, 9]);
  });

  test("selects through an arbitrary index for mouse shift selection", () => {
    const entries = [entry, { ...entry, id: 8 }, { ...entry, id: 9 }, { ...entry, id: 10 }];
    const ranged = selectThroughIndex({ ...initialState(), entries, selectedIndex: 1 }, 3);
    expect(ranged.selectedIndex).toBe(3);
    expect([...ranged.selectedIds].sort((left, right) => left - right)).toEqual([8, 9, 10]);
  });

  test("toggles pinned-only view and resets transient list state", () => {
    const state = togglePinnedOnly({ ...initialState(), selectedIndex: 3, selectedIds: new Set([7]), previewOffset: 5 });
    expect(state.pinnedOnly).toBe(true);
    expect(state.selectedIndex).toBe(0);
    expect(state.previewOffset).toBe(0);
    expect(state.selectedIds.size).toBe(0);
  });

  test("uses marked entries or current entry for bulk operations", () => {
    expect(selectedIdsOrCurrent({ ...initialState(), entries: [entry] })).toEqual([7]);
    expect(selectedIdsOrCurrent(toggleSelectedId({ ...initialState(), entries: [entry] }))).toEqual([7]);
  });

  test("detects pinned entries in the active selected set", () => {
    const pinned = { ...entry, id: 8, favorite: true };
    const plain = { ...entry, id: 9, favorite: false };
    expect(selectedSetIncludesPinned({ ...initialState(), entries: [pinned], selectedIndex: 0 })).toBe(true);
    expect(selectedSetIncludesPinned({ ...initialState(), entries: [pinned, plain], selectedIds: new Set([9]) })).toBe(false);
    expect(selectedSetIncludesPinned({ ...initialState(), entries: [pinned, plain], selectedIds: new Set([8, 9]) })).toBe(true);
  });

  test("tracks configurable search lifecycle state", () => {
    const base = { ...initialState({ query: "old" }), entries: [entry], selectedIndex: 3, previewOffset: 9 };
    const opened = openSearch(base);
    expect(opened.mode).toBe("search");
    expect(opened.query).toBe("");
    expect(opened.queryBeforeSearch).toBe("old");
    expect(opened.previewOffset).toBe(0);

    const kept = openSearch(base, false);
    expect(kept.query).toBe("old");

    const typed = updateSearchQuery(opened, "new");
    expect(typed.query).toBe("new");
    expect(typed.selectedIndex).toBe(0);

    expect(cancelSearch(typed).query).toBe("old");
    expect(cancelSearch(typed, false).query).toBe("new");
    expect(applySearch(typed)).toMatchObject({ mode: "browse", query: "new", queryBeforeSearch: null, selectedIndex: 0 });
  });

  test("windows visible entries around the selection", () => {
    const entries = Array.from({ length: 10 }, (_, index) => ({ ...entry, id: index + 1 }));
    expect(visibleEntries(entries, 8, 3).map((row) => row.index)).toEqual([7, 8, 9]);
  });

  test("calculates visible entry capacity with row spacing", () => {
    expect(visibleEntryCapacity(5, 0)).toBe(5);
    expect(visibleEntryCapacity(5, 1)).toBe(3);
    expect(visibleEntryCapacity(5, 2)).toBe(2);
    expect(visibleEntryCapacity(1, 3)).toBe(1);
    expect(visibleEntryCapacity(0, 3)).toBe(0);
  });

  test("keeps full-preview scroll capacity independent from list row spacing", () => {
    expect(visibleEntryCapacity(9, 2)).toBe(3);
    expect(previewScrollCapacity(9, 2)).toBe(7);
    expect(previewScrollCapacity(1, 4)).toBe(1);
    expect(visibleFullPreviewLineCapacity(9, 0, 2, 3)).toBe(4);
    expect(visibleFullPreviewLineCapacity(8, 1, 1, 2)).toBe(2);
  });

  test("calculates visible preview line capacity with configured spacing", () => {
    expect(visiblePreviewLineCapacity(6, 0)).toBe(6);
    expect(visiblePreviewLineCapacity(5, 1)).toBe(2);
    expect(visiblePreviewLineCapacity(6, 1)).toBe(3);
    expect(visiblePreviewLineCapacity(6, 2)).toBe(2);
  });

  test("cycles filters", () => {
    expect(nextFilter("all")).toBe("text");
    expect(nextFilter("today")).toBe("all");
    expect(nextFilter("images", ["images", "all"])).toBe("all");
    expect(nextFilter("today", ["images", "all"])).toBe("images");
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
