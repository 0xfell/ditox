import { onCleanup, onMount } from "solid-js";
import { useRenderer } from "@opentui/solid";
import { useBindings, useKeymap } from "@opentui/keymap/solid";
import { fullPreviewVisibleRows } from "./layout";
import { previewModel } from "./presentation";
import {
  applySearch,
  cancelSearch,
  moveEnd,
  moveHome,
  movePage,
  movePreview,
  moveSelection,
  nextFilter,
  openSearch,
  selectRange,
  selectSingle,
  selectedEntry,
  toggleSelectedId,
  updateSearchQuery,
  type UiState,
} from "./state";
import { shutdownTui } from "./terminal-lifecycle";
import { normalizeKey, type ResolvedTuiConfig } from "./tui-config";

export type DitoxKeymapActions = {
  state: () => UiState;
  setState: (updater: UiState | ((previous: UiState) => UiState)) => void;
  refresh: (next?: Partial<UiState>) => Promise<void>;
  copySelected: (paste: boolean) => Promise<void>;
  bulkCopySelected: () => Promise<void>;
  copySearchMatches: () => Promise<void>;
  outputSelected: () => Promise<void>;
  toggleFavorite: () => Promise<void>;
  confirmDelete: () => Promise<void>;
  confirmClear: () => Promise<void>;
  togglePinnedView: () => Promise<void>;
  openClear: (kind: UiState["clearKind"], preserveFavorites?: boolean) => void;
  config: ResolvedTuiConfig;
  browsePageRows: () => number;
  previewRows: () => number;
  previewWrapWidth: () => number;
};

export function useDitoxKeymap(actions: DitoxKeymapActions) {
  const keymap = useKeymap();
  const renderer = useRenderer();

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
      const specialKeys = [
        ...actions.config.keyBindings.searchCancel,
        ...actions.config.keyBindings.searchBackspace,
        ...actions.config.keyBindings.searchApply,
        ...actions.config.keyBindings.searchCopyMatches,
      ];
      if (matchesAnyKey(event, specialKeys) || matchesAnyKey(event, ["escape", "backspace", "enter"])) return;
      if (typeof event.sequence === "string" && event.sequence.length === 1 && event.sequence >= " ") {
        actions.setState((previous) => updateSearchQuery(previous, previous.query + event.sequence));
        ctx.consume();
      }
    });
    onCleanup(dispose);
  });

  useBindings(() => {
    const mode = actions.state().mode;
    const keys = actions.config.keyBindings;
    if (mode === "search") {
      return {
        priority: 100,
        bindings: [
          ...bind(keys.searchCancel, () => actions.setState((previous) => cancelSearch(previous, actions.config.behavior.restoreQueryOnSearchCancel))),
          ...bind(keys.searchBackspace, () => actions.setState((previous) => updateSearchQuery(previous, previous.query.slice(0, -1)))),
          ...bind(keys.searchApply, () => actions.refresh(applySearch(actions.state()))),
          ...bind(keys.searchCopyMatches, () => actions.copySearchMatches()),
        ],
      };
    }
    if (mode === "preview") {
      const totalLines = () =>
        previewModel(
          selectedEntry(actions.state()),
          actions.config.layout.maxFullPreviewLines,
          actions.config.labels,
          actions.config.layout,
          actions.previewWrapWidth(),
        ).length;
      return {
        priority: 100,
        bindings: [
          ...bind(keys.previewBack, () => actions.setState((previous) => ({ ...previous, mode: "browse", previewOffset: 0 }))),
          ...bind(keys.previewUp, () => actions.setState((previous) => movePreview(previous, -1, totalLines(), previewVisibleRows(actions)))),
          ...bind(keys.previewDown, () => actions.setState((previous) => movePreview(previous, 1, totalLines(), previewVisibleRows(actions)))),
          ...bind(keys.previewPageUp, () => actions.setState((previous) => movePreview(previous, -previewVisibleRows(actions), totalLines(), previewVisibleRows(actions)))),
          ...bind(keys.previewPageDown, () => actions.setState((previous) => movePreview(previous, previewVisibleRows(actions), totalLines(), previewVisibleRows(actions)))),
          ...bind(keys.copyPaste, () => actions.copySelected(true)),
          ...bind(keys.copyOnly, () => actions.copySelected(false)),
          ...bind(keys.forceQuit, () => shutdownTui(renderer)),
        ],
      };
    }
    if (mode === "confirm-delete") {
      return {
        priority: 100,
        bindings: [
          ...bind(keys.confirmYes, () => actions.confirmDelete()),
          ...bind(keys.confirmNo, () => actions.setState((previous) => ({ ...previous, mode: "browse" }))),
          ...bind(keys.searchCancel, () => actions.setState((previous) => ({ ...previous, mode: "browse" }))),
        ],
      };
    }
    if (mode === "confirm-clear") {
      return {
        priority: 100,
        bindings: [
          ...bind(keys.confirmYes, () => actions.confirmClear()),
          ...bind(keys.confirmNo, () => actions.setState((previous) => ({ ...previous, mode: "browse" }))),
          ...bind(keys.searchCancel, () => actions.setState((previous) => ({ ...previous, mode: "browse" }))),
        ],
      };
    }
    return {
      priority: 100,
      bindings: [
        ...bind(keys.forceQuit, () => shutdownTui(renderer)),
        ...bind(keys.quit, () => shutdownTui(renderer)),
        ...bind(keys.up, () => actions.setState((previous) => moveSelection(previous, -1))),
        ...bind(keys.down, () => actions.setState((previous) => moveSelection(previous, 1))),
        ...bind(keys.pageUp, () => actions.setState((previous) => movePage(previous, actions.browsePageRows(), -1))),
        ...bind(keys.pageDown, () => actions.setState((previous) => movePage(previous, actions.browsePageRows(), 1))),
        ...bind(keys.home, () => actions.setState(moveHome)),
        ...bind(keys.end, () => actions.setState(moveEnd)),
        ...bind(keys.nextFilter, () =>
          actions.refresh({ filter: nextFilter(actions.state().filter, actions.config.filterOrder), pinnedOnly: false, selectedIndex: 0, previewOffset: 0 }),
        ),
        ...bind(keys.search, () => actions.setState((previous) => openSearch(previous, actions.config.behavior.clearQueryOnSearchOpen))),
        ...bind(keys.selectToggle, () => actions.setState(toggleSelectedId)),
        ...bind(keys.selectSingle, () => actions.setState(selectSingle)),
        ...bind(keys.clearSelection, () => actions.setState((previous) => ({ ...previous, selectedIds: new Set() }))),
        ...bind(keys.selectUp, () => actions.setState((previous) => selectRange(previous, -1))),
        ...bind(keys.selectDown, () => actions.setState((previous) => selectRange(previous, 1))),
        ...bind(keys.toggleFavorite, () => actions.toggleFavorite()),
        ...bind(keys.togglePinnedView, () => actions.togglePinnedView()),
        ...bind(keys.preview, () => actions.setState((previous) => ({ ...previous, mode: "preview", previewOffset: 0 }))),
        ...bind(keys.delete, () => actions.setState((previous) => ({ ...previous, mode: "confirm-delete" }))),
        ...bind(keys.copyPaste, () => actions.copySelected(true)),
        ...bind(keys.copyOnly, () => actions.copySelected(false)),
        ...bind(keys.bulkCopy, () => actions.bulkCopySelected()),
        ...bind(keys.output, () => actions.outputSelected()),
        ...bind(keys.help, () => actions.setState((previous) => ({ ...previous, mode: "help" }))),
        ...bind(keys.clearAll, () => actions.openClear("all")),
        ...bind(keys.clearText, () => actions.openClear("text")),
        ...bind(keys.clearImages, () => actions.openClear("images")),
        ...bind(keys.clearAllIncludingPinned, () => actions.openClear("all", false)),
      ],
    };
  });
}

function bind(keys: string[], cmd: () => void | Promise<void>): Array<{ key: string; cmd: () => void | Promise<void> }> {
  return runtimeKeysForBinding(keys).map((key) => ({ key, cmd }));
}

export function runtimeKeysForBinding(keys: string[]): string[] {
  const expanded: string[] = [];
  const seen = new Set<string>();
  for (const rawKey of keys) {
    const key = runtimeSequenceKey(rawKey);
    for (const candidate of [key, openTuiRuntimeAlias(key)]) {
      if (candidate === null || seen.has(candidate)) continue;
      seen.add(candidate);
      expanded.push(candidate);
    }
  }
  return expanded;
}

// Ditox spells multi-stroke chords with spaces (e.g. "c a", "ctrl+x s") for
// readability, but @opentui/keymap reads a literal space as the `space` key and
// otherwise matches strokes greedily from contiguous characters. Normalize each
// stroke and concatenate so "c a" -> "ca" (strokes [c, a]) and "ctrl+x s" ->
// "ctrl+xs" (strokes [ctrl+x, s]). Single-stroke bindings (including the literal
// "space"/" " preview key) have no inter-stroke whitespace and pass through.
export function runtimeSequenceKey(rawKey: string): string {
  const strokes = rawKey.trim().split(/\s+/).filter(Boolean);
  if (strokes.length <= 1) return normalizeKey(rawKey);
  return strokes.map((stroke) => normalizeKey(stroke)).join("");
}

function openTuiRuntimeAlias(key: string): string | null {
  const parts = key.split("+");
  if (parts[parts.length - 1] !== "enter") return null;
  parts[parts.length - 1] = "return";
  return parts.join("+");
}

function matchesAnyKey(event: any, keys: string[]): boolean {
  const candidates = [
    normalizeKey(String(event.name ?? "")),
    normalizeKey(String(event.key ?? "")),
    normalizeKey(String(event.baseCode ?? "")),
    normalizeKey(String(event.sequence ?? "")),
  ];
  return keys.some((key) => candidates.includes(normalizeKey(key)));
}

function previewVisibleRows(actions: DitoxKeymapActions): number {
  return fullPreviewVisibleRows(selectedEntry(actions.state()), actions.config, actions.previewRows());
}
