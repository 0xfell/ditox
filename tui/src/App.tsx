import { For, Show, createSignal, onMount } from "solid-js";
import { render, useKeyboard } from "@opentui/solid";
import {
  clampSelection,
  formatAge,
  initialState,
  moveSelection,
  nextFilter,
  previewLines,
  selectedEntry,
  toggleSelectedId,
  type UiState,
} from "./state";
import { copyEntry, deleteEntry, favoriteEntry, listEntries, pasteEntry } from "./rpc";
import type { Entry } from "./types";

export function App() {
  const [state, setState] = createSignal<UiState>(initialState());

  async function refresh(next?: Partial<UiState>) {
    const current = { ...state(), ...next };
    setState(current);
    try {
      const result = await listEntries(current.query, current.filter);
      setState((previous) =>
        clampSelection({
          ...previous,
          ...next,
          entries: result.entries,
          status: `${result.entries.length} entries`,
        }),
      );
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function copySelected(paste: boolean) {
    const entry = selectedEntry(state());
    if (!entry) return;
    try {
      if (paste) await pasteEntry(entry.id, Bun.env.DITOX_TARGET_WINDOW);
      else await copyEntry(entry.id);
      setState((previous) => ({ ...previous, status: paste ? "pasted" : "copied" }));
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function toggleFavorite() {
    const entry = selectedEntry(state());
    if (!entry) return;
    await favoriteEntry(entry.id, !entry.favorite);
    await refresh();
  }

  async function confirmDelete() {
    const entry = selectedEntry(state());
    if (!entry) return;
    await deleteEntry(entry.id);
    await refresh({ mode: "browse" });
  }

  function handleSearchKey(event: any) {
    const key = keyName(event);
    if (key === "escape") setState((previous) => ({ ...previous, mode: "browse" }));
    else if (key === "backspace") setState((previous) => ({ ...previous, query: previous.query.slice(0, -1) }));
    else if (key === "enter") refresh({ mode: "browse" });
    else if (typeof event.sequence === "string" && event.sequence.length === 1) {
      setState((previous) => ({ ...previous, query: previous.query + event.sequence }));
    }
  }

  function handleBrowseKey(event: any) {
    const key = keyName(event);
    if (event.ctrl && key === "c") process.exit(0);
    if (key === "escape" || key === "q") process.exit(0);
    if (key === "up" || key === "k") setState((previous) => moveSelection(previous, -1));
    if (key === "down" || key === "j") setState((previous) => moveSelection(previous, 1));
    if (key === "tab") refresh({ filter: nextFilter(state().filter), selectedIndex: 0 });
    if (key === "/") setState((previous) => ({ ...previous, mode: "search", query: "" }));
    if (key === "space") setState(toggleSelectedId);
    if (key === "p") toggleFavorite();
    if (key === "d") setState((previous) => ({ ...previous, mode: "confirm-delete" }));
    if (key === "enter") copySelected(true);
    if (event.ctrl && key === "y") copySelected(false);
    if (key === "?") setState((previous) => ({ ...previous, mode: "help" }));
  }

  useKeyboard((event: any) => {
    const mode = state().mode;
    if (mode === "search") return handleSearchKey(event);
    if (mode === "help" && keyName(event) !== "") return setState((previous) => ({ ...previous, mode: "browse" }));
    if (mode === "confirm-delete") {
      const key = keyName(event);
      if (key === "y") return confirmDelete();
      if (key === "n" || key === "escape") return setState((previous) => ({ ...previous, mode: "browse" }));
      return;
    }
    handleBrowseKey(event);
  });

  onMount(() => refresh());

  return (
    <box flexDirection="column" width="100%" height="100%">
      <box height={3} border>
        <text>
          Ditox  filter:{state().filter}  query:{state().query || "-"}  {state().status}
        </text>
      </box>
      <box flexDirection="row" flexGrow={1}>
        <box width="45%" border>
          <For each={state().entries}>
            {(entry, index) => <EntryRow entry={entry} index={index()} selected={index() === state().selectedIndex} marked={state().selectedIds.has(entry.id)} />}
          </For>
          <Show when={state().entries.length === 0}>
            <text>No history</text>
          </Show>
        </box>
        <box width="55%" border>
          <For each={previewLines(selectedEntry(state()))}>{(line) => <text>{line}</text>}</For>
        </box>
      </box>
      <Show when={state().mode === "search"}>
        <box height={3} border>
          <text>/{state().query}</text>
        </box>
      </Show>
      <Show when={state().mode === "confirm-delete"}>
        <box height={3} border>
          <text>Delete selected entry? y/n</text>
        </box>
      </Show>
      <Show when={state().mode === "help"}>
        <box height={7} border>
          <text>j/k move  / search  tab filter  enter paste  ctrl+y copy</text>
          <text>space select  p favorite  d delete  esc quit</text>
        </box>
      </Show>
    </box>
  );
}

function EntryRow(props: { entry: Entry; index: number; selected: boolean; marked: boolean }) {
  const entry = () => props.entry;
  return (
    <text>
      {props.selected ? ">" : " "} {props.marked ? "*" : " "} {entry().favorite ? "p" : " "} {entry().kind} #{entry().id} {formatAge(entry().created_at_ms)} {entry().preview}
    </text>
  );
}

function keyName(event: any): string {
  return String(event.name ?? event.key ?? event.baseCode ?? event.sequence ?? "").toLowerCase();
}

export async function run() {
  await render(() => <App />);
}
