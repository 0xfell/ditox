import type { Entry, EntryFilter, WatcherStatus } from "./types";
import { formatAge, previewModel, truncateText } from "./presentation";

export { formatAge, truncateText };

export type UiState = {
  entries: Entry[];
  selectedIndex: number;
  selectedIds: Set<number>;
  query: string;
  filter: EntryFilter;
  pinnedOnly: boolean;
  mode: "browse" | "search" | "help" | "confirm-delete" | "confirm-clear" | "preview";
  clearKind: "all" | "text" | "images";
  clearPreserveFavorites: boolean;
  previewOffset: number;
  queryBeforeSearch: string | null;
  watcher: WatcherStatus | null;
  status: string;
};

export type InitialStateOptions = {
  filter?: EntryFilter;
  pinnedOnly?: boolean;
  query?: string;
};

export function initialState(options: InitialStateOptions = {}): UiState {
  return {
    entries: [],
    selectedIndex: 0,
    selectedIds: new Set(),
    query: options.query ?? "",
    filter: options.filter ?? "all",
    pinnedOnly: options.pinnedOnly ?? false,
    mode: "browse",
    clearKind: "all",
    clearPreserveFavorites: true,
    previewOffset: 0,
    queryBeforeSearch: null,
    watcher: null,
    status: "",
  };
}

export function clampSelection(state: UiState): UiState {
  const max = Math.max(0, state.entries.length - 1);
  return { ...state, selectedIndex: Math.min(Math.max(0, state.selectedIndex), max) };
}

export function moveSelection(state: UiState, delta: number): UiState {
  return clampSelection({ ...state, selectedIndex: state.selectedIndex + delta });
}

export function movePage(state: UiState, rows: number, direction: -1 | 1): UiState {
  return moveSelection(state, Math.max(1, rows) * direction);
}

export function moveHome(state: UiState): UiState {
  return clampSelection({ ...state, selectedIndex: 0 });
}

export function moveEnd(state: UiState): UiState {
  return clampSelection({ ...state, selectedIndex: Math.max(0, state.entries.length - 1) });
}

export function movePreview(state: UiState, delta: number, totalLines: number, rows: number): UiState {
  const maxOffset = Math.max(0, totalLines - Math.max(1, rows));
  const previewOffset = Math.min(Math.max(0, state.previewOffset + delta), maxOffset);
  return { ...state, previewOffset };
}

export function togglePinnedOnly(state: UiState): UiState {
  return {
    ...state,
    pinnedOnly: !state.pinnedOnly,
    selectedIndex: 0,
    selectedIds: new Set(),
    previewOffset: 0,
  };
}

export function selectedEntry(state: UiState): Entry | undefined {
  return state.entries[state.selectedIndex];
}

export function toggleSelectedId(state: UiState): UiState {
  const entry = selectedEntry(state);
  if (!entry) return state;
  const selectedIds = new Set(state.selectedIds);
  if (selectedIds.has(entry.id)) selectedIds.delete(entry.id);
  else selectedIds.add(entry.id);
  return { ...state, selectedIds };
}

export function selectSingle(state: UiState): UiState {
  const entry = selectedEntry(state);
  return { ...state, selectedIds: entry ? new Set([entry.id]) : new Set() };
}

export function selectRange(state: UiState, delta: -1 | 1): UiState {
  const selectedIds = new Set(state.selectedIds);
  const current = selectedEntry(state);
  if (current) selectedIds.add(current.id);
  const next = moveSelection(state, delta);
  const entry = selectedEntry(next);
  if (entry) selectedIds.add(entry.id);
  return { ...next, selectedIds };
}

export function selectThroughIndex(state: UiState, index: number): UiState {
  const clamped = clampSelection({ ...state, selectedIndex: index });
  const start = Math.min(state.selectedIndex, clamped.selectedIndex);
  const end = Math.max(state.selectedIndex, clamped.selectedIndex);
  const selectedIds = new Set(state.selectedIds);
  for (let cursor = start; cursor <= end; cursor += 1) {
    const entry = state.entries[cursor];
    if (entry) selectedIds.add(entry.id);
  }
  return { ...clamped, selectedIds };
}

export function selectedIdsOrCurrent(state: UiState): number[] {
  if (state.selectedIds.size > 0) return [...state.selectedIds];
  const entry = selectedEntry(state);
  return entry ? [entry.id] : [];
}

export function selectedSetIncludesPinned(state: UiState): boolean {
  const ids = new Set(selectedIdsOrCurrent(state));
  return state.entries.some((entry) => ids.has(entry.id) && entry.favorite);
}

export function openSearch(state: UiState, clearQuery = true): UiState {
  return {
    ...state,
    mode: "search",
    queryBeforeSearch: state.query,
    query: clearQuery ? "" : state.query,
    previewOffset: 0,
  };
}

export function updateSearchQuery(state: UiState, query: string): UiState {
  return {
    ...state,
    query,
    selectedIndex: 0,
    previewOffset: 0,
  };
}

export function applySearch(state: UiState): UiState {
  return {
    ...state,
    mode: "browse",
    queryBeforeSearch: null,
    selectedIndex: 0,
    previewOffset: 0,
  };
}

export function cancelSearch(state: UiState, restoreQuery = true): UiState {
  return {
    ...state,
    mode: "browse",
    query: restoreQuery && state.queryBeforeSearch !== null ? state.queryBeforeSearch : state.query,
    queryBeforeSearch: null,
    selectedIndex: 0,
    previewOffset: 0,
  };
}

export function visibleEntries(entries: Entry[], selectedIndex: number, maxRows: number): Array<{ entry: Entry; index: number }> {
  if (maxRows <= 0) return [];
  const clamped = Math.min(Math.max(0, selectedIndex), Math.max(0, entries.length - 1));
  const half = Math.floor(maxRows / 2);
  const start = Math.min(Math.max(0, clamped - half), Math.max(0, entries.length - maxRows));
  return entries.slice(start, start + maxRows).map((entry, offset) => ({ entry, index: start + offset }));
}

export function visibleEntryCapacity(rows: number, rowSpacing = 0): number {
  return spacedItemCapacity(rows, rowSpacing);
}

export function visiblePreviewLineCapacity(rows: number, lineSpacing = 0): number {
  const availableRows = Math.max(0, Math.floor(rows));
  if (availableRows === 0) return 0;
  const blankRows = Math.max(0, Math.floor(lineSpacing));
  if (blankRows === 0) return availableRows;
  return Math.max(1, Math.floor(availableRows / (blankRows + 1)));
}

export function visibleFullPreviewLineCapacity(rows: number, lineSpacing = 0, insetRows = 0, metadataRows = 0): number {
  return visiblePreviewLineCapacity(previewScrollCapacity(rows, Math.max(0, Math.floor(insetRows)) + Math.max(0, Math.floor(metadataRows))), lineSpacing);
}

function spacedItemCapacity(rows: number, spacing: number): number {
  const availableRows = Math.max(0, Math.floor(rows));
  if (availableRows === 0) return 0;
  const blankRows = Math.max(0, Math.floor(spacing));
  return Math.max(1, Math.floor((availableRows + blankRows) / (blankRows + 1)));
}

export function previewScrollCapacity(rows: number, insetRows = 0): number {
  return Math.max(1, Math.max(0, Math.floor(rows)) - Math.max(0, Math.floor(insetRows)));
}

const defaultFilterOrder: EntryFilter[] = ["all", "text", "images", "favorites", "today"];

export function nextFilter(filter: EntryFilter, order: EntryFilter[] = defaultFilterOrder): EntryFilter {
  const filters: EntryFilter[] = order.length > 0 ? order : defaultFilterOrder;
  const index = filters.indexOf(filter);
  return filters[index >= 0 ? (index + 1) % filters.length : 0] ?? "all";
}

export function previewLines(entry: Entry | undefined): string[] {
  return previewModel(entry, 24).map((line) => (line.gutter ? `${line.gutter}: ${line.text}` : line.text));
}
