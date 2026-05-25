import type { Entry, EntryFilter } from "./types";

export type UiState = {
  entries: Entry[];
  selectedIndex: number;
  selectedIds: Set<number>;
  query: string;
  filter: EntryFilter;
  mode: "browse" | "search" | "help" | "confirm-delete" | "confirm-clear";
  clearKind: "all" | "text" | "images";
  status: string;
};

export function initialState(): UiState {
  return {
    entries: [],
    selectedIndex: 0,
    selectedIds: new Set(),
    query: "",
    filter: "all",
    mode: "browse",
    clearKind: "all",
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

export function selectedIdsOrCurrent(state: UiState): number[] {
  if (state.selectedIds.size > 0) return [...state.selectedIds];
  const entry = selectedEntry(state);
  return entry ? [entry.id] : [];
}

export function visibleEntries(entries: Entry[], selectedIndex: number, maxRows: number): Array<{ entry: Entry; index: number }> {
  if (maxRows <= 0) return [];
  const clamped = Math.min(Math.max(0, selectedIndex), Math.max(0, entries.length - 1));
  const half = Math.floor(maxRows / 2);
  const start = Math.min(Math.max(0, clamped - half), Math.max(0, entries.length - maxRows));
  return entries.slice(start, start + maxRows).map((entry, offset) => ({ entry, index: start + offset }));
}

export function nextFilter(filter: EntryFilter): EntryFilter {
  const filters: EntryFilter[] = ["all", "text", "images", "favorites", "today"];
  return filters[(filters.indexOf(filter) + 1) % filters.length] ?? "all";
}

export function formatAge(timestampMs: number, now = Date.now()): string {
  const seconds = Math.max(0, Math.floor((now - timestampMs) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

export function previewLines(entry: Entry | undefined): string[] {
  if (!entry) return ["No entry selected"];
  if (entry.kind === "image") {
    const dimensions =
      entry.image_width !== null && entry.image_height !== null ? `${entry.image_width}x${entry.image_height}` : "unknown";
    return [
      "Image entry",
      `MIME: ${entry.mime}`,
      `Bytes: ${entry.byte_len}`,
      `Hash: ${entry.hash}`,
      `Dimensions: ${dimensions}`,
      entry.blob_path ? `Path: ${entry.blob_path}` : "Path: not stored",
    ];
  }
  const lines = entry.content.split(/\r?\n/);
  return lines.length === 0 ? [""] : lines.slice(0, 24);
}

export function truncateText(value: string, width: number): string {
  if (width <= 0) return "";
  const clean = value.replace(/\s+/g, " ");
  if (clean.length <= width) return clean;
  if (width <= 3) return ".".repeat(width);
  return `${clean.slice(0, width - 3)}...`;
}
