import type { Entry, EntryFilter } from "./types";

export type UiState = {
  entries: Entry[];
  selectedIndex: number;
  selectedIds: Set<number>;
  query: string;
  filter: EntryFilter;
  mode: "browse" | "search" | "help" | "confirm-delete";
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
    return [
      "Image entry",
      `MIME: ${entry.mime}`,
      `Bytes: ${entry.byte_len}`,
      `Hash: ${entry.hash}`,
      entry.blob_path ? `Path: ${entry.blob_path}` : "Path: not stored",
    ];
  }
  const lines = entry.content.split(/\r?\n/);
  return lines.length === 0 ? [""] : lines.slice(0, 24);
}

