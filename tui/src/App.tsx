import { createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { render, useRenderer, useTerminalDimensions } from "@opentui/solid";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";
import { KeymapProvider } from "@opentui/keymap/solid";
import {
  CliRenderEvents,
  parseColor,
  type CliRendererConfig,
  type CursorStyleOptions,
  type KittyKeyboardOptions,
  type PixelResolution,
  type TerminalCapabilities,
} from "@opentui/core";
import {
  clampSelection,
  initialState,
  movePreview,
  moveSelection,
  selectThroughIndex,
  selectedIdsOrCurrent,
  selectedEntry,
  selectedSetIncludesPinned,
  togglePinnedOnly,
  toggleSelectedId,
  visibleEntryCapacity,
  type UiState,
} from "./state";
import { bulkCopyEntries, clearEntries, copyEntry, deleteEntry, favoriteEntry, getImageBytes, getWatcherStatus, listEntries, outputEntries, pasteEntry } from "./rpc";
import { currentTuiConfig, surface, type ResolvedTuiConfig, type TuiStatusHintMode } from "./tui-config";
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
import { setImageByteSource, type ImageProtocolCapabilities } from "./image-preview";
import { useDitoxKeymap } from "./keymap";
import { fullPreviewTextWidth as layoutFullPreviewTextWidth, fullPreviewVisibleRows, fullPreviewWidth as layoutFullPreviewWidth } from "./layout";
import { exitAfter, installTerminalExitReset, shutdownTui } from "./terminal-lifecycle";
import { TerminalImageManager, type TerminalImageState } from "./terminal-image";

// Re-exported for backwards compatibility (tests and external callers import
// these from "./App").
export { runtimeKeysForBinding, runtimeSequenceKey } from "./keymap";
export { estimatedImagePreviewRows, fullPreviewReservedRows } from "./layout";
export { installTerminalExitReset, shutdownTui, terminalExitResetSequence, writeTerminalExitReset } from "./terminal-lifecycle";

type CopyPasteRpc = {
  copyEntry: typeof copyEntry;
  pasteEntry: typeof pasteEntry;
};

function AppRoot(props: { config: ResolvedTuiConfig; initialUiState?: UiState }) {
  const renderer = useRenderer();
  const keymap = createDefaultOpenTuiKeymap(renderer);
  return (
    <KeymapProvider keymap={keymap}>
      <App config={props.config} initialUiState={props.initialUiState} />
    </KeymapProvider>
  );
}

export function App(props: { config?: ResolvedTuiConfig; initialUiState?: UiState } = {}) {
  const config = props.config ?? currentTuiConfig();
  const renderer = useRenderer();
  const imageManager = new TerminalImageManager();
  const [state, setState] = createSignal<UiState>(props.initialUiState ?? initialState(config.startup));
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
  const fullPreviewWidth = () => layoutFullPreviewWidth(config, contentWidth());
  const fullPreviewTextWidth = () => layoutFullPreviewTextWidth(config, contentWidth());
  const previewVisibleRows = () => fullPreviewVisibleRows(selectedEntry(state()), config, listRows());
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

    // run() preloads the first screenful before rendering; only fetch here
    // when the App was mounted without preloaded state (e.g. in tests).
    if (!props.initialUiState) void refresh();
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

function liveSearchKey(state: UiState): string {
  return [state.query, state.pinnedOnly ? "pinned" : state.filter].join("\u0000");
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
  // Production image previews fetch bytes over RPC; the TUI process never
  // reads blob files from the daemon's filesystem (AGENTS.md layering rule).
  setImageByteSource({
    readSync: null,
    read: async (entry) => (await getImageBytes(entry.id, config.labels)).data,
  });
  const initialUiState = await initialAppState(config);
  await render(() => <AppRoot config={config} initialUiState={initialUiState} />, tuiRenderOptions(config));
}

// Loads the first screenful of data before the renderer starts, so the first
// painted frame already shows history instead of flashing an empty list and
// immediately repainting. RPC failures degrade to an empty list with the error
// in the status line.
export async function initialAppState(config: ResolvedTuiConfig): Promise<UiState> {
  const state = initialState(config.startup);
  try {
    const [result, watcher] = await Promise.all([
      listEntries(state.query, state.pinnedOnly ? "favorites" : state.filter, config.layout.historyLimit, config.labels),
      getWatcherStatus(config.labels),
    ]);
    return clampSelection({
      ...state,
      entries: result.entries,
      watcher,
      status: formatEntriesStatus(result.entries.length, config.labels),
    });
  } catch (error) {
    return { ...state, status: error instanceof Error ? error.message : String(error) };
  }
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
