import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { render, useRenderer, useTerminalDimensions } from "@opentui/solid";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import { KeymapProvider, useBindings, useKeymap } from "@opentui/keymap/solid";
import {
  clampSelection,
  formatAge,
  initialState,
  moveSelection,
  nextFilter,
  previewLines,
  selectedIdsOrCurrent,
  selectedEntry,
  truncateText,
  toggleSelectedId,
  visibleEntries,
  type UiState,
} from "./state";
import { bulkCopyEntries, clearEntries, copyEntry, deleteEntry, favoriteEntry, listEntries, outputEntries, pasteEntry } from "./rpc";
import type { Entry } from "./types";

function AppRoot() {
  const renderer = useRenderer();
  const keymap = createDefaultOpenTuiKeymap(renderer);
  return (
    <KeymapProvider keymap={keymap}>
      <App />
    </KeymapProvider>
  );
}

export function App() {
  const [state, setState] = createSignal<UiState>(initialState());
  const dimensions = useTerminalDimensions();
  const listRows = () => Math.max(1, dimensions().height - 7);
  const rowWidth = () => Math.max(24, Math.floor(dimensions().width * 0.45) - 2);

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

  async function bulkCopySelected() {
    const ids = selectedIdsOrCurrent(state());
    if (ids.length === 0) return;
    try {
      const result = ids.length === 1 ? await copyEntry(ids[0]!) : await bulkCopyEntries(ids);
      setState((previous) => ({ ...previous, status: result.copied ? `copied ${ids.length}` : "nothing copied" }));
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function outputSelected() {
    const ids = selectedIdsOrCurrent(state());
    if (ids.length === 0) return;
    try {
      const result = await outputEntries(ids);
      process.stdout.write(result.content);
      if (result.content.length > 0 && !result.content.endsWith("\n")) process.stdout.write("\n");
      process.exit(0);
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

  async function confirmClear() {
    const kind = state().clearKind;
    const result = await clearEntries(kind);
    await refresh({ mode: "browse", selectedIds: new Set(), status: `cleared ${result.deleted}` });
  }

  function openClear(kind: UiState["clearKind"]) {
    setState((previous) => ({ ...previous, mode: "confirm-clear", clearKind: kind }));
  }

  useDitoxKeymap({
    state,
    setState,
    refresh,
    copySelected,
    bulkCopySelected,
    outputSelected,
    toggleFavorite,
    confirmDelete,
    confirmClear,
    openClear,
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
          <For each={visibleEntries(state().entries, state().selectedIndex, listRows())}>
            {(row) => (
              <EntryRow
                entry={row.entry}
                index={row.index}
                selected={row.index === state().selectedIndex}
                marked={state().selectedIds.has(row.entry.id)}
                width={rowWidth()}
              />
            )}
          </For>
          <Show when={state().entries.length === 0}>
            <text>{state().query ? "No matches" : "No clipboard history"}</text>
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
      <Show when={state().mode === "confirm-clear"}>
        <box height={3} border>
          <text>Clear {state().clearKind}? y/n</text>
        </box>
      </Show>
      <Show when={state().mode === "help"}>
        <box height={7} border>
          <text>j/k move  / search  tab filter  enter paste  ctrl+y copy  y bulk copy</text>
          <text>space select  p favorite  d delete  ca clear all  ct clear text  ci clear images</text>
        </box>
      </Show>
    </box>
  );
}

function EntryRow(props: { entry: Entry; index: number; selected: boolean; marked: boolean; width: number }) {
  const entry = () => props.entry;
  const row = () =>
    `${props.selected ? ">" : " "} ${props.marked ? "*" : " "} ${entry().favorite ? "p" : " "} ${entry().kind} #${entry().id} ${formatAge(entry().created_at_ms)} ${entry().preview}`;
  return (
    <text>
      {truncateText(row(), props.width)}
    </text>
  );
}

type DitoxKeymapActions = {
  state: () => UiState;
  setState: (updater: UiState | ((previous: UiState) => UiState)) => void;
  refresh: (next?: Partial<UiState>) => Promise<void>;
  copySelected: (paste: boolean) => Promise<void>;
  bulkCopySelected: () => Promise<void>;
  outputSelected: () => Promise<void>;
  toggleFavorite: () => Promise<void>;
  confirmDelete: () => Promise<void>;
  confirmClear: () => Promise<void>;
  openClear: (kind: UiState["clearKind"]) => void;
};

function useDitoxKeymap(actions: DitoxKeymapActions) {
  const keymap = useKeymap();

  onMount(() => {
    const dispose = keymap.intercept("key", (ctx) => {
      const mode = actions.state().mode;
      if (mode === "help") {
        actions.setState((previous) => ({ ...previous, mode: "browse" }));
        ctx.consume();
        return;
      }
      if (mode !== "search") return;
      const event = ctx.event as any;
      const name = keyName(event);
      if (name === "escape" || name === "backspace" || name === "enter") return;
      if (typeof event.sequence === "string" && event.sequence.length === 1 && event.sequence >= " ") {
        actions.setState((previous) => ({ ...previous, query: previous.query + event.sequence }));
        ctx.consume();
      }
    });
    onCleanup(dispose);
  });

  useBindings(() => {
    const mode = actions.state().mode;
    if (mode === "search") {
      return {
        priority: 100,
        bindings: [
          { key: "escape", cmd: () => actions.setState((previous) => ({ ...previous, mode: "browse" })) },
          { key: "backspace", cmd: () => actions.setState((previous) => ({ ...previous, query: previous.query.slice(0, -1) })) },
          { key: "enter", cmd: () => actions.refresh({ mode: "browse" }) },
        ],
      };
    }
    if (mode === "confirm-delete") {
      return {
        priority: 100,
        bindings: [
          { key: "y", cmd: () => actions.confirmDelete() },
          { key: "n", cmd: () => actions.setState((previous) => ({ ...previous, mode: "browse" })) },
          { key: "escape", cmd: () => actions.setState((previous) => ({ ...previous, mode: "browse" })) },
        ],
      };
    }
    if (mode === "confirm-clear") {
      return {
        priority: 100,
        bindings: [
          { key: "y", cmd: () => actions.confirmClear() },
          { key: "n", cmd: () => actions.setState((previous) => ({ ...previous, mode: "browse" })) },
          { key: "escape", cmd: () => actions.setState((previous) => ({ ...previous, mode: "browse" })) },
        ],
      };
    }
    return {
      priority: 100,
      bindings: [
        { key: "ctrl+c", cmd: () => process.exit(0) },
        { key: "escape", cmd: () => process.exit(0) },
        { key: "q", cmd: () => process.exit(0) },
        { key: "up", cmd: () => actions.setState((previous) => moveSelection(previous, -1)) },
        { key: "k", cmd: () => actions.setState((previous) => moveSelection(previous, -1)) },
        { key: "down", cmd: () => actions.setState((previous) => moveSelection(previous, 1)) },
        { key: "j", cmd: () => actions.setState((previous) => moveSelection(previous, 1)) },
        { key: "tab", cmd: () => actions.refresh({ filter: nextFilter(actions.state().filter), selectedIndex: 0 }) },
        { key: "/", cmd: () => actions.setState((previous) => ({ ...previous, mode: "search", query: "" })) },
        { key: "space", cmd: () => actions.setState(toggleSelectedId) },
        { key: "p", cmd: () => actions.toggleFavorite() },
        { key: "d", cmd: () => actions.setState((previous) => ({ ...previous, mode: "confirm-delete" })) },
        { key: "enter", cmd: () => actions.copySelected(true) },
        { key: "ctrl+y", cmd: () => actions.copySelected(false) },
        { key: "y", cmd: () => actions.bulkCopySelected() },
        { key: "o", cmd: () => actions.outputSelected() },
        { key: "?", cmd: () => actions.setState((previous) => ({ ...previous, mode: "help" })) },
        { key: "c a", cmd: () => actions.openClear("all") },
        { key: "c t", cmd: () => actions.openClear("text") },
        { key: "c i", cmd: () => actions.openClear("images") },
      ],
    };
  });
}

function keyName(event: any): string {
  return String(event.name ?? event.key ?? event.baseCode ?? event.sequence ?? "").toLowerCase();
}

export async function run() {
  await render(() => <AppRoot />);
}
