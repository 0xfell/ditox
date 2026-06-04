import { createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { render, useRenderer, useTerminalDimensions } from "@opentui/solid";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import { KeymapProvider, useBindings, useKeymap } from "@opentui/keymap/solid";
import {
  CliRenderEvents,
  parseColor,
  type CliRenderer,
  type CliRendererConfig,
  type CursorStyleOptions,
  type KittyKeyboardOptions,
  type PixelResolution,
  type TerminalCapabilities,
} from "@opentui/core";
import {
  clampSelection,
  applySearch,
  cancelSearch,
  initialState,
  moveEnd,
  moveHome,
  movePage,
  movePreview,
  moveSelection,
  nextFilter,
  openSearch,
  selectRange,
  selectSingle,
  selectThroughIndex,
  selectedIdsOrCurrent,
  selectedEntry,
  selectedSetIncludesPinned,
  visibleFullPreviewLineCapacity,
  togglePinnedOnly,
  toggleSelectedId,
  updateSearchQuery,
  visibleEntryCapacity,
  type UiState,
} from "./state";
import { bulkCopyEntries, clearEntries, copyEntry, deleteEntry, favoriteEntry, getWatcherStatus, listEntries, outputEntries, pasteEntry } from "./rpc";
import { currentTuiConfig, normalizeKey, surface, type ResolvedTuiConfig, type TuiStatusHintMode } from "./tui-config";
import {
  formatClearedStatus,
  formatCopiedCountStatus,
  formatDeletedStatus,
  formatEntriesStatus,
  formatPinStatus,
  formatViewStatus,
  previewModel,
} from "./presentation";
import { Shell } from "./components/Shell";
import { HeaderBar } from "./components/HeaderBar";
import { EntryList } from "./components/EntryList";
import { PreviewPane } from "./components/PreviewPane";
import { StatusLine } from "./components/StatusLine";
import { ModeOverlay } from "./components/Overlay";
import { FullPreview } from "./components/FullPreview";
import { isBlockPreviewMime, type ImageProtocolCapabilities } from "./image-preview";
import { TerminalImageManager, type TerminalImageState } from "./terminal-image";
import type { Entry } from "./types";

type CopyPasteRpc = {
  copyEntry: typeof copyEntry;
  pasteEntry: typeof pasteEntry;
};

function AppRoot(props: { config: ResolvedTuiConfig }) {
  const renderer = useRenderer();
  const keymap = createDefaultOpenTuiKeymap(renderer);
  return (
    <KeymapProvider keymap={keymap}>
      <App config={props.config} />
    </KeymapProvider>
  );
}

export function App(props: { config?: ResolvedTuiConfig } = {}) {
  const config = props.config ?? currentTuiConfig();
  const renderer = useRenderer();
  const imageManager = new TerminalImageManager();
  const [state, setState] = createSignal<UiState>(initialState(config.startup));
  const [terminalCapabilities, setTerminalCapabilities] = createSignal<TerminalCapabilities | null>(renderer.capabilities);
  // The terminal answers the pixel-resolution query asynchronously, so
  // renderer.resolution is a plain (non-reactive) getter that is null on first
  // paint. Mirror it into a signal (synced in onMount) so image previews
  // re-render to full resolution the moment the resolution lands, instead of
  // staying on the low-res block fallback until the selection changes.
  const [resolution, setResolution] = createSignal<PixelResolution | null>(renderer.resolution);
  const dimensions = useTerminalDimensions();
  const imageTerminal = (): TerminalImageState =>
    terminalImageState(dimensions().width, dimensions().height, resolution(), terminalCapabilities());
  const contentWidth = () => Math.max(1, dimensions().width - config.layout.shellPaddingX * 2);
  const contentHeight = () => Math.max(1, dimensions().height - config.layout.shellPaddingY * 2);
  const overlayHeight = () => activeOverlayHeight(state(), config);
  const headerRows = () => (config.layout.showHeader ? config.layout.headerHeight : 0);
  const statusRows = () => (config.layout.showStatusLine ? config.layout.statusHeight : 0);
  const listRows = () => Math.max(1, contentHeight() - headerRows() - statusRows() - overlayHeight());
  const visibleListEntries = () => visibleEntryCapacity(listRows(), config.layout.rowSpacing);
  const splitPaneTotalWidth = () => Math.max(1, contentWidth());
  const splitPaneGap = () =>
    config.layout.showPreviewPane ? Math.min(config.layout.splitPaneGap, Math.max(0, splitPaneTotalWidth() - config.layout.minPaneWidth * 2)) : 0;
  const splitPaneAvailableWidth = () => Math.max(1, splitPaneTotalWidth() - splitPaneGap());
  const listPaneWidthPercent = () => (config.layout.showPreviewPane ? (config.layout.listWidthPercent * splitPaneAvailableWidth()) / splitPaneTotalWidth() : 100);
  const previewPaneWidthPercent = () => (config.layout.showPreviewPane ? (config.layout.previewWidthPercent * splitPaneAvailableWidth()) / splitPaneTotalWidth() : 0);
  const rowWidth = () =>
    config.layout.showPreviewPane
      ? Math.max(config.layout.minPaneWidth, Math.floor((splitPaneAvailableWidth() * config.layout.listWidthPercent) / 100) - config.layout.splitPaneWidthInset)
      : Math.max(config.layout.minPaneWidth, contentWidth());
  const previewWidth = () => Math.max(config.layout.minPaneWidth, Math.floor((splitPaneAvailableWidth() * config.layout.previewWidthPercent) / 100) - config.layout.splitPaneWidthInset);
  // Mirror FullPreview's own width math so wrapped-line counts used for scroll
  // bounds match what FullPreview actually renders.
  const fullPreviewWidth = () => Math.max(config.layout.minPaneWidth, contentWidth() - config.layout.fullPreviewWidthInset);
  const fullPreviewTextWidth = () =>
    Math.max(1, fullPreviewWidth() - (config.layout.showFullPreviewGutter ? config.layout.fullPreviewTextWidthInset : config.layout.fullPreviewPaddingX * 2));
  const previewVisibleRows = () =>
    visibleFullPreviewLineCapacity(
      listRows(),
      config.layout.fullPreviewLineSpacing,
      config.layout.fullPreviewScrollInsetRows,
      fullPreviewReservedRows(selectedEntry(state()), config, listRows()),
    );
  let lastLiveSearchKey = "";

  async function refresh(next?: Partial<UiState>) {
    const current = { ...state(), ...next };
    setState(current);
    try {
      const [result, watcher] = await Promise.all([
        listEntries(current.query, current.pinnedOnly ? "favorites" : current.filter, config.layout.historyLimit, config.labels),
        getWatcherStatus(config.labels),
      ]);
      setState((previous) =>
        clampSelection({
          ...previous,
          ...next,
          entries: result.entries,
          watcher,
          status: next?.status ?? formatEntriesStatus(result.entries.length, config.labels),
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
      const result = await copyOrPasteEntry(entry.id, {
        paste,
        targetWindow: Bun.env.DITOX_TARGET_WINDOW,
        labels: config.labels,
      });
      const status = result === "pasted" ? config.labels.statusPasted : config.labels.statusCopied;
      const shouldExit = result === "pasted" ? config.behavior.exitAfterPaste : paste ? config.behavior.exitAfterPaste : config.behavior.exitAfterCopy;
      if (shouldExit) {
        setState((previous) => ({ ...previous, status }));
        exitAfter(true, renderer);
        return;
      }
      // The backend bumps last_used on copy/paste and re-sorts the entry to the
      // top. Refresh so the reorder is visible immediately, then keep the cursor
      // on the entry we just used wherever it landed.
      await refresh({ status });
      setState((previous) => {
        const index = previous.entries.findIndex((candidate) => candidate.id === entry.id);
        return index >= 0 ? clampSelection({ ...previous, selectedIndex: index, previewOffset: 0 }) : previous;
      });
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function bulkCopySelected() {
    const ids = selectedIdsOrCurrent(state());
    if (ids.length === 0) return;
    try {
      const result = ids.length === 1 ? await copyEntry(ids[0]!, config.labels) : await bulkCopyEntries(ids, config.labels);
      setState((previous) => ({
        ...previous,
        status: result.copied ? formatCopiedCountStatus(ids.length, config.labels) : config.labels.statusNothingCopied,
      }));
      exitAfter(result.copied && config.behavior.exitAfterBulkCopy, renderer);
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function copySearchMatches() {
    const current = state();
    try {
      const result = await listEntries(current.query, current.pinnedOnly ? "favorites" : current.filter, config.layout.historyLimit, config.labels);
      const ids = result.entries.map((entry) => entry.id);
      if (ids.length === 0) {
        setState((previous) =>
          clampSelection({
            ...previous,
            entries: result.entries,
            selectedIds: new Set(),
            status: config.labels.statusNothingCopied,
          }),
        );
        return;
      }

      const copyResult = ids.length === 1 ? await copyEntry(ids[0]!, config.labels) : await bulkCopyEntries(ids, config.labels);
      setState((previous) =>
        clampSelection({
          ...previous,
          entries: result.entries,
          selectedIds: new Set(ids),
          status: copyResult.copied ? formatCopiedCountStatus(ids.length, config.labels) : config.labels.statusNothingCopied,
        }),
      );
      exitAfter(copyResult.copied && config.behavior.exitAfterSearchCopy, renderer);
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function outputSelected() {
    const ids = selectedIdsOrCurrent(state());
    if (ids.length === 0) return;
    try {
      const result = await outputEntries(ids, config.labels);
      shutdownTui(renderer, {
        afterDestroy: () => {
          process.stdout.write(result.content);
          if (result.content.length > 0 && !result.content.endsWith("\n")) process.stdout.write("\n");
        },
      });
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function toggleFavorite() {
    const entry = selectedEntry(state());
    if (!entry) return;
    const nextFavorite = !entry.favorite;
    try {
      await favoriteEntry(entry.id, nextFavorite, config.labels);
      await refresh({ status: formatPinStatus(entry, nextFavorite, config.labels) });
    } catch (error) {
      setState((previous) => ({ ...previous, status: error instanceof Error ? error.message : String(error) }));
    }
  }

  async function confirmDelete() {
    const ids = selectedIdsOrCurrent(state());
    if (ids.length === 0) return;
    let deleted = 0;
    for (const id of ids) {
      if ((await deleteEntry(id, config.labels)).deleted) deleted += 1;
    }
    await refresh({ mode: "browse", selectedIds: new Set(), status: formatDeletedStatus(deleted, config.labels) });
  }

  async function confirmClear() {
    const current = state();
    const result = await clearEntries(current.clearKind, current.clearPreserveFavorites, config.labels);
    await refresh({
      mode: "browse",
      selectedIds: new Set(),
      status: formatClearedStatus(result.deleted, current.clearPreserveFavorites, config.labels),
    });
  }

  async function togglePinnedView() {
    const next = togglePinnedOnly(state());
    await refresh({
      pinnedOnly: next.pinnedOnly,
      selectedIndex: next.selectedIndex,
      selectedIds: next.selectedIds,
      previewOffset: next.previewOffset,
      status: formatViewStatus(next.pinnedOnly, config.labels),
    });
  }

  function openClear(kind: UiState["clearKind"], preserveFavorites = true) {
    setState((previous) => ({ ...previous, mode: "confirm-clear", clearKind: kind, clearPreserveFavorites: preserveFavorites }));
  }

  useDitoxKeymap({
    state,
    setState,
    refresh,
    copySelected,
    bulkCopySelected,
    copySearchMatches,
    outputSelected,
    toggleFavorite,
    confirmDelete,
    confirmClear,
    togglePinnedView,
    openClear,
    config,
    browsePageRows: visibleListEntries,
    previewRows: listRows,
    previewWrapWidth: fullPreviewTextWidth,
  });

  createEffect(() => {
    const current = state();
    if (!config.behavior.liveSearch || current.mode !== "search") {
      lastLiveSearchKey = "";
      return;
    }
    const key = liveSearchKey(current);
    if (key === lastLiveSearchKey) return;
    lastLiveSearchKey = key;
    const timer = setTimeout(() => {
      void refresh({ status: state().status });
    }, config.behavior.liveSearchDebounceMs);
    onCleanup(() => clearTimeout(timer));
  });

  onMount(() => {
    if (config.terminal.title !== null) renderer.setTerminalTitle(config.terminal.title);
    const cursor = terminalCursorStyleOptions(config);
    if (cursor) renderer.setCursorStyle(cursor);
    const updateCapabilities = (capabilities: TerminalCapabilities) => setTerminalCapabilities(capabilities);
    renderer.on(CliRenderEvents.CAPABILITIES, updateCapabilities);
    onCleanup(() => renderer.off(CliRenderEvents.CAPABILITIES, updateCapabilities));

    // Pull renderer.resolution into the reactive signal once the terminal
    // replies. The reply arrives shortly after setup (and again after a
    // resize), so poll briefly and re-arm on resize; syncResolution only
    // writes the signal when the value actually changes.
    const syncResolution = (): boolean => {
      const current = renderer.resolution;
      if (!current || current.width <= 0 || current.height <= 0) return false;
      setResolution((previous) => (previous && previous.width === current.width && previous.height === current.height ? previous : current));
      return true;
    };
    let resolutionPoll: ReturnType<typeof setInterval> | null = null;
    const stopResolutionPoll = () => {
      if (resolutionPoll) {
        clearInterval(resolutionPoll);
        resolutionPoll = null;
      }
    };
    const armResolutionSync = (durationMs = 3000, intervalMs = 100) => {
      stopResolutionPoll();
      syncResolution();
      const deadline = Date.now() + durationMs;
      resolutionPoll = setInterval(() => {
        syncResolution();
        if (Date.now() >= deadline) stopResolutionPoll();
      }, intervalMs);
    };
    armResolutionSync();
    const onResize = () => armResolutionSync();
    renderer.on(CliRenderEvents.RESIZE, onResize);
    onCleanup(() => {
      renderer.off(CliRenderEvents.RESIZE, onResize);
      stopResolutionPoll();
    });

    void refresh();
    if (config.layout.refreshIntervalMs <= 0) return;
    let polling = false;
    const timer = setInterval(() => {
      if (polling || state().mode !== "browse") return;
      polling = true;
      void refresh({ status: state().status }).finally(() => {
        polling = false;
      });
    }, config.layout.refreshIntervalMs);
    onCleanup(() => clearInterval(timer));
  });
  onCleanup(() => imageManager.destroy());

  return (
    <AppFrame
      config={config}
      header={config.layout.showHeader ? <HeaderBar config={config} state={state()} selectedCount={state().selectedIds.size} width={contentWidth()} /> : null}
      content={
        state().mode === "preview" ? (
          <FullPreview
            config={config}
            entry={selectedEntry(state())}
            rows={listRows()}
            width={fullPreviewWidth()}
            offset={state().previewOffset}
            imageCapabilities={imageProtocolCapabilities(terminalCapabilities(), resolution())}
            imageTerminal={imageTerminal()}
            imageManager={imageManager}
            onScroll={(direction) =>
              setState((previous) =>
                movePreview(
                  previous,
                  direction * config.layout.mouseScrollRows,
                  previewModel(selectedEntry(previous), config.layout.maxFullPreviewLines, config.labels, config.layout, fullPreviewTextWidth()).length,
                  previewVisibleRows(),
                ),
              )
            }
          />
        ) : (
          <box flexDirection="row" flexGrow={1} backgroundColor={surface(config, "shell").bg}>
            <EntryList
              config={config}
              entries={state().entries}
              selectedIndex={state().selectedIndex}
              selectedIds={state().selectedIds}
              rows={listRows()}
              width={rowWidth()}
              widthPercent={listPaneWidthPercent()}
              query={state().query}
              onSelectEntry={(index, options) =>
                setState((previous) => {
                  if (options.extend) return selectThroughIndex(previous, index);
                  const next = clampSelection({ ...previous, selectedIndex: index });
                  return options.toggle ? toggleSelectedId(next) : next;
                })
              }
              onScroll={(direction) => setState((previous) => moveSelection(previous, direction * config.layout.mouseScrollRows))}
            />
            {config.layout.showPreviewPane && splitPaneGap() > 0 ? <box width={splitPaneGap()} flexShrink={0} backgroundColor={surface(config, "splitPaneGap").bg} /> : null}
            {config.layout.showPreviewPane ? (
              <PreviewPane
                config={config}
                entry={selectedEntry(state())}
                rows={listRows()}
                width={previewWidth()}
                widthPercent={previewPaneWidthPercent()}
                imageCapabilities={imageProtocolCapabilities(terminalCapabilities(), resolution())}
                imageTerminal={imageTerminal()}
                imageManager={imageManager}
              />
            ) : null}
          </box>
        )
      }
      status={config.layout.showStatusLine ? <StatusLine config={config} status={state().status} watcher={state().watcher} width={contentWidth()} mode={statusHintMode(state().mode)} /> : null}
      overlay={<ModeOverlay config={config} state={state()} width={contentWidth()} />}
    />
  );
}

export function AppFrame(props: {
  config: ResolvedTuiConfig;
  header?: JSX.Element;
  content: JSX.Element;
  status?: JSX.Element;
  overlay?: JSX.Element;
}) {
  const topOverlay = () => (props.config.layout.overlayPlacement === "top" ? props.overlay : null);
  const bottomOverlay = () => (props.config.layout.overlayPlacement === "bottom" ? props.overlay : null);
  return (
    <Shell config={props.config}>
      {props.header}
      {topOverlay()}
      {props.content}
      {props.status}
      {bottomOverlay()}
    </Shell>
  );
}

export function imageProtocolCapabilities(capabilities: TerminalCapabilities | null, resolution?: PixelResolution | null): ImageProtocolCapabilities {
  const kittyGraphics = capabilities?.kitty_graphics ?? null;
  const sixel = capabilities?.sixel ?? null;
  return {
    kittyGraphics,
    sixel,
    nativeRenderer: Boolean(resolution && (kittyGraphics === true || sixel === true)),
  };
}

export function terminalImageState(
  columns: number,
  rows: number,
  resolution: PixelResolution | null,
  capabilities: TerminalCapabilities | null,
): TerminalImageState {
  return {
    columns: Math.max(1, Math.floor(columns)),
    rows: Math.max(1, Math.floor(rows)),
    resolution,
    capabilities: imageProtocolCapabilities(capabilities, resolution),
  };
}

function statusHintMode(mode: UiState["mode"]): TuiStatusHintMode {
  if (mode === "search") return "search";
  if (mode === "preview") return "preview";
  if (mode === "confirm-delete" || mode === "confirm-clear") return "confirm";
  return "browse";
}

type DitoxKeymapActions = {
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

function useDitoxKeymap(actions: DitoxKeymapActions) {
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
  return visibleFullPreviewLineCapacity(
    actions.previewRows(),
    actions.config.layout.fullPreviewLineSpacing,
    actions.config.layout.fullPreviewScrollInsetRows,
    fullPreviewReservedRows(selectedEntry(actions.state()), actions.config, actions.previewRows()),
  );
}

export function fullPreviewReservedRows(entry: Entry | undefined, config: ResolvedTuiConfig, rows: number): number {
  const metadataRows = config.layout.showFullPreviewMetadata && entry ? config.layout.fullPreviewMetaHeight : 0;
  return metadataRows + estimatedImagePreviewRows(entry, config, rows);
}

export function estimatedImagePreviewRows(entry: Entry | undefined, config: ResolvedTuiConfig, rows: number): number {
  if (entry?.kind !== "image" || config.layout.fullPreviewImageMode === "metadata") return 0;
  const canRenderBlocks = entry.blob_path !== null && isBlockPreviewMime(entry.mime);
  if (!canRenderBlocks) return 1;
  const renderedRows = Math.min(Math.max(2, rows - config.layout.fullPreviewImageRowInset), config.layout.fullPreviewImageMaxRows);
  const noticeRows =
    config.layout.fullPreviewImageNoticeVisibility === "always" ||
    (config.layout.fullPreviewImageNoticeVisibility === "protocol" && (config.layout.fullPreviewImageMode === "kitty" || config.layout.fullPreviewImageMode === "sixel"))
      ? 1
      : 0;
  return renderedRows + noticeRows + (noticeRows > 0 ? config.layout.fullPreviewImageNoticeSpacing : 0);
}

function liveSearchKey(state: UiState): string {
  return [state.query, state.pinnedOnly ? "pinned" : state.filter].join("\u0000");
}

function exitAfter(enabled: boolean, renderer: CliRenderer): void {
  if (!enabled) return;
  setTimeout(() => shutdownTui(renderer), 0);
}

export async function copyOrPasteEntry(
  entryId: number,
  options: {
    paste: boolean;
    targetWindow?: string;
    labels: ResolvedTuiConfig["labels"];
    rpc?: CopyPasteRpc;
  },
): Promise<"copied" | "pasted"> {
  const rpc = options.rpc ?? { copyEntry, pasteEntry };
  if (!options.paste || !options.targetWindow) {
    await rpc.copyEntry(entryId, options.labels);
    return "copied";
  }

  try {
    await rpc.pasteEntry(entryId, options.targetWindow, options.labels);
    return "pasted";
  } catch (error) {
    if (!isPasteBackFailure(error, options.labels)) throw error;
    await rpc.copyEntry(entryId, options.labels);
    return "copied";
  }
}

function isPasteBackFailure(error: unknown, labels: ResolvedTuiConfig["labels"]): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message === labels.errorPasteBackFailed || message.includes("PasteBackFailed");
}

type TerminalWriter = {
  isTTY?: boolean;
  write: (chunk: string) => unknown;
};

type ShutdownTuiOptions = {
  code?: number;
  stdout?: TerminalWriter;
  exit?: (code: number) => void;
  schedule?: (callback: () => void) => unknown;
  afterDestroy?: () => void;
};

export const terminalExitResetSequence = "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1015l\x1b[?1004l\x1b[?2004l\x1b[?25h\x1b[0m";

let terminalExitResetInstalled = false;
let terminalExitResetWritten = false;
let tuiShutdownStarted = false;

export function writeTerminalExitReset(stdout: TerminalWriter = process.stdout): void {
  if (terminalExitResetWritten) return;
  if (!stdout.isTTY) return;
  stdout.write(terminalExitResetSequence);
  terminalExitResetWritten = true;
}

export function installTerminalExitReset(stdout: TerminalWriter = process.stdout): void {
  if (terminalExitResetInstalled) return;
  terminalExitResetInstalled = true;
  process.once("exit", () => writeTerminalExitReset(stdout));
}

export function shutdownTui(renderer: Pick<CliRenderer, "destroy">, options: ShutdownTuiOptions = {}): void {
  if (tuiShutdownStarted) return;
  tuiShutdownStarted = true;
  const stdout = options.stdout ?? process.stdout;
  try {
    renderer.destroy();
  } finally {
    writeTerminalExitReset(stdout);
    options.afterDestroy?.();
    const exit = options.exit ?? process.exit;
    const schedule = options.schedule ?? ((callback: () => void) => setTimeout(callback, 0));
    schedule(() => exit(options.code ?? 0));
  }
}

function activeOverlayHeight(state: UiState, config: ResolvedTuiConfig): number {
  if (state.mode === "help") return config.layout.helpOverlayHeight;
  if (state.mode === "confirm-delete") {
    return config.layout.confirmOverlayHeight + (selectedSetIncludesPinned(state) ? config.layout.confirmPinnedExtraRows : 0);
  }
  if (state.mode === "confirm-clear") return config.layout.clearOverlayHeight;
  if (state.mode === "search") return config.layout.searchOverlayHeight;
  return 0;
}

export async function run() {
  const config = currentTuiConfig();
  installTerminalExitReset();
  await render(() => <AppRoot config={config} />, tuiRenderOptions(config));
}

export function tuiRenderOptions(config: ResolvedTuiConfig) {
  const options: CliRendererConfig = {
    useMouse: config.layout.mouseEnabled,
    enableMouseMovement: config.layout.mouseEnabled,
    screenMode: config.terminal.screenMode,
    footerHeight: config.terminal.footerHeight,
    clearOnShutdown: config.terminal.clearOnShutdown,
    backgroundColor: terminalBackgroundColor(config),
  };
  const kittyKeyboard = terminalKittyKeyboardOptions(config);
  if (kittyKeyboard) options.useKittyKeyboard = kittyKeyboard;
  if (config.terminal.targetFps !== null) options.targetFps = config.terminal.targetFps;
  if (config.terminal.maxFps !== null) options.maxFps = config.terminal.maxFps;
  if (config.terminal.debounceDelay !== null) options.debounceDelay = config.terminal.debounceDelay;
  if (config.terminal.stdinParserMaxBufferBytes !== null) options.stdinParserMaxBufferBytes = config.terminal.stdinParserMaxBufferBytes;
  return options;
}

export function terminalBackgroundColor(config: ResolvedTuiConfig): string {
  return config.terminal.backgroundColor === "auto" ? surface(config, "shell").bg : config.terminal.backgroundColor;
}

export function terminalKittyKeyboardOptions(config: ResolvedTuiConfig): KittyKeyboardOptions | null {
  const keyboard = config.terminal.kittyKeyboard;
  if (!keyboard.enabled) return null;
  return {
    disambiguate: keyboard.disambiguate,
    alternateKeys: keyboard.alternateKeys,
    events: keyboard.events,
    allKeysAsEscapes: keyboard.allKeysAsEscapes,
    reportText: keyboard.reportText,
  };
}

export function terminalCursorStyleOptions(config: ResolvedTuiConfig): CursorStyleOptions | null {
  const cursor = config.terminal.cursor;
  if (cursor.style === null && cursor.blinking === null && cursor.color === null) return null;
  return {
    ...(cursor.style !== null ? { style: cursor.style } : {}),
    ...(cursor.blinking !== null ? { blinking: cursor.blinking } : {}),
    ...(cursor.color !== null ? { color: parseColor(cursor.color) } : {}),
  };
}
