import { describe, expect, test } from "bun:test";
import { helpRows, loadTuiConfig, normalizeKey, resolveTuiConfig, statusHint, surface, templateSegments } from "./tui-config";
import { themeNames, themes } from "./theme";
import { terminalCursorStyleOptions, tuiRenderOptions } from "./App";

const defaultTerminalRendererOptions = {
  title: null,
  cursor: {
    style: null,
    blinking: null,
    color: null,
  },
  kittyKeyboard: {
    enabled: false,
    disambiguate: true,
    alternateKeys: true,
    events: false,
    allKeysAsEscapes: false,
    reportText: false,
  },
  targetFps: null,
  maxFps: null,
  debounceDelay: null,
  stdinParserMaxBufferBytes: null,
};

describe("tui config", () => {
  test("normalizes OpenTUI return key events to logical Enter", () => {
    expect(normalizeKey("return")).toBe("enter");
    expect(normalizeKey("Return")).toBe("enter");
    expect(normalizeKey("ctrl+return")).toBe("ctrl+enter");
  });

  test("uses safe defaults without a config file", () => {
    const config = resolveTuiConfig({}, { HOME: "/tmp/home" }, null);
    expect(config.theme.name).toBe("ditoxDark");
    expect(config.terminal).toEqual({
      altScreen: "auto",
      screenMode: "alternate-screen",
      backgroundColor: "auto",
      footerHeight: 12,
      clearOnShutdown: true,
      ...defaultTerminalRendererOptions,
    });
    expect(tuiRenderOptions(config).backgroundColor).toBe(surface(config, "shell").bg);
    expect(config.layout.listWidthPercent).toBe(46);
    expect(config.layout.previewWidthPercent).toBe(54);
    expect(config.layout.maxFullPreviewLines).toBe(2000);
    expect(config.layout.historyLimit).toBe(100);
    expect(config.layout.imagePreviewMode).toBe("blocks");
    expect(config.layout.fullPreviewImageMode).toBe("blocks");
    expect(config.layout.imagePreviewMaxWidth).toBe(80);
    expect(config.layout.imagePreviewMaxRows).toBe(24);
    expect(config.layout.fullPreviewImageMaxWidth).toBe(120);
    expect(config.layout.fullPreviewImageMaxRows).toBe(40);
    expect(config.layout.imagePreviewRenderer).toBe("auto");
    expect(config.layout.fullPreviewImageRenderer).toBe("auto");
    expect(config.layout.imagePreviewAlign).toBe("left");
    expect(config.layout.fullPreviewImageAlign).toBe("left");
    expect(config.layout.imagePreviewBlockGlyph).toBe("▀");
    expect(config.layout.fullPreviewImageBlockGlyph).toBe("▀");
    expect(config.layout.imagePreviewBackground).toBe("auto");
    expect(config.layout.fullPreviewImageBackground).toBe("auto");
    expect(config.layout.headerHeight).toBe(3);
    expect(config.layout.statusHeight).toBe(1);
    expect(config.layout.showHeader).toBe(true);
    expect(config.layout.showStatusLine).toBe(true);
    expect(config.layout.searchOverlayHeight).toBe(3);
    expect(config.layout.confirmOverlayHeight).toBe(3);
    expect(config.layout.confirmPinnedExtraRows).toBe(1);
    expect(config.layout.clearOverlayHeight).toBe(4);
    expect(config.layout.helpOverlayHeight).toBe(16);
    expect(config.layout.minPaneWidth).toBe(24);
    expect(config.layout.splitPaneGap).toBe(1);
    expect(config.layout.splitPaneWidthInset).toBe(6);
    expect(config.layout.fullPreviewWidthInset).toBe(4);
    expect(config.layout.previewTextWidthInset).toBe(8);
    expect(config.layout.fullPreviewTextWidthInset).toBe(8);
    expect(config.layout.previewContentAlign).toBe("left");
    expect(config.layout.fullPreviewContentAlign).toBe("left");
    expect(config.layout.previewBodyVerticalAlign).toBe("top");
    expect(config.layout.fullPreviewBodyVerticalAlign).toBe("top");
    expect(config.layout.imagePreviewRowInset).toBe(3);
    expect(config.layout.fullPreviewImageRowInset).toBe(3);
    expect(config.layout.fullPreviewScrollInsetRows).toBe(2);
    expect(config.layout.previewLineNumberWidth).toBe(3);
    expect(config.layout.previewGutterWidth).toBe(4);
    expect(config.layout.fullPreviewGutterWidth).toBe(4);
    expect(config.layout.previewGutterAlign).toBe("right");
    expect(config.layout.fullPreviewGutterAlign).toBe("right");
    expect(config.layout.previewLineSpacing).toBe(0);
    expect(config.layout.fullPreviewLineSpacing).toBe(0);
    expect(config.layout.previewMetaHeight).toBe(3);
    expect(config.layout.fullPreviewMetaHeight).toBe(1);
    expect(config.layout.previewMetaLineSpacing).toBe(0);
    expect(config.layout.fullPreviewMetaLineSpacing).toBe(0);
    expect(config.layout.previewMetaHashLength).toBe(12);
    expect(config.layout.fullPreviewMetaHashLength).toBe(12);
    expect(config.layout.previewImageFields).toEqual(["type", "mime", "size", "dimensions", "hash", "blob"]);
    expect(config.layout.headerBrandMaxWidth).toBe(0);
    expect(config.layout.headerFilterMaxWidth).toBe(0);
    expect(config.layout.headerQueryMaxWidth).toBe(0);
    expect(config.layout.headerModeMaxWidth).toBe(0);
    expect(config.layout.statusSeparatorPadding).toBe(2);
    expect(config.layout.statusSeparatorPaddingLeft).toBe(2);
    expect(config.layout.statusSeparatorPaddingRight).toBe(2);
    expect(config.layout.statusOperationMaxWidth).toBe(0);
    expect(config.layout.statusWatcherMaxWidth).toBe(0);
    expect(config.layout.statusHintMaxWidth).toBe(0);
    expect(config.layout.searchOverlayPromptMaxWidth).toBe(0);
    expect(config.layout.searchOverlayQueryMaxWidth).toBe(0);
    expect(config.layout.searchOverlayCursorMaxWidth).toBe(0);
    expect(config.layout.dangerOverlayPromptMaxWidth).toBe(0);
    expect(config.layout.dangerOverlayHintMaxWidth).toBe(0);
    expect(config.layout.helpOverlayActionMaxWidth).toBe(0);
    expect(config.layout.frameTitlePadding).toBe(1);
    expect(config.layout.frameTitlePaddingLeft).toBe(1);
    expect(config.layout.frameTitlePaddingRight).toBe(1);
    expect(config.layout.shellPaddingX).toBe(0);
    expect(config.layout.shellPaddingY).toBe(0);
    expect(config.layout.headerPaddingX).toBe(1);
    expect(config.layout.headerPaddingY).toBe(0);
    expect(config.layout.statusPaddingX).toBe(1);
    expect(config.layout.statusPaddingY).toBe(0);
    expect(config.layout.headerContentAlign).toBe("left");
    expect(config.layout.statusContentAlign).toBe("left");
    expect(config.layout.headerVerticalAlign).toBe("top");
    expect(config.layout.statusVerticalAlign).toBe("top");
    expect(config.layout.overlayPaddingX).toBe(1);
    expect(config.layout.overlayPaddingY).toBe(0);
    expect(config.layout.overlayContentAlign).toBe("left");
    expect(config.layout.overlayVerticalAlign).toBe("top");
    expect(config.layout.overlayLineSpacing).toBe(0);
    expect(config.layout.searchOverlayPaddingX).toBe(1);
    expect(config.layout.searchOverlayPaddingY).toBe(0);
    expect(config.layout.searchOverlayContentAlign).toBe("left");
    expect(config.layout.searchOverlayVerticalAlign).toBe("top");
    expect(config.layout.searchOverlayLineSpacing).toBe(0);
    expect(config.layout.dangerOverlayPaddingX).toBe(1);
    expect(config.layout.dangerOverlayPaddingY).toBe(0);
    expect(config.layout.dangerOverlayContentAlign).toBe("left");
    expect(config.layout.dangerOverlayVerticalAlign).toBe("top");
    expect(config.layout.dangerOverlayLineSpacing).toBe(0);
    expect(config.layout.helpOverlayPaddingX).toBe(1);
    expect(config.layout.helpOverlayPaddingY).toBe(0);
    expect(config.layout.helpOverlayContentAlign).toBe("left");
    expect(config.layout.helpOverlayVerticalAlign).toBe("top");
    expect(config.layout.helpOverlayLineSpacing).toBe(0);
    expect(config.layout.listPaddingX).toBe(1);
    expect(config.layout.listPaddingY).toBe(0);
    expect(config.layout.previewPaddingX).toBe(1);
    expect(config.layout.previewPaddingY).toBe(0);
    expect(config.layout.fullPreviewPaddingX).toBe(1);
    expect(config.layout.fullPreviewPaddingY).toBe(0);
    expect(config.layout.previewMetaPaddingX).toBe(1);
    expect(config.layout.previewMetaPaddingY).toBe(0);
    expect(config.layout.fullPreviewMetaPaddingX).toBe(0);
    expect(config.layout.fullPreviewMetaPaddingY).toBe(0);
    expect(config.layout.previewMetaContentAlign).toBe("left");
    expect(config.layout.fullPreviewMetaContentAlign).toBe("left");
    expect(config.layout.previewMetaVerticalAlign).toBe("top");
    expect(config.layout.fullPreviewMetaVerticalAlign).toBe("top");
    expect(config.layout.emptyStatePaddingX).toBe(1);
    expect(config.layout.emptyStatePaddingY).toBe(1);
    expect(config.layout.emptyStateContentAlign).toBe("left");
    expect(config.layout.emptyStateTitleAlign).toBe("left");
    expect(config.layout.emptyStateHelpAlign).toBe("left");
    expect(config.layout.emptyStateVerticalAlign).toBe("top");
    expect(config.layout.emptyStateLineSpacing).toBe(0);
    expect(config.layout.helpKeyWidth).toBe(24);
    expect(config.layout.helpKeyAlign).toBe("left");
    expect(config.layout.confirmHintIndent).toBe(2);
    expect(config.layout.rowContentAlign).toBe("left");
    expect(config.layout.rowMetadataAlign).toBe("left");
    expect(config.layout.rowPreviewAlign).toBe("left");
    expect(config.layout.rowAgeWidth).toBe(2);
    expect(config.layout.rowAgeAlign).toBe("right");
    expect(config.layout.rowSizeWidth).toBe(6);
    expect(config.layout.rowSizeAlign).toBe("right");
    expect(config.layout.rowPinnedWidth).toBe(3);
    expect(config.layout.rowPinnedAlign).toBe("left");
    expect(config.layout.rowMetaHashLength).toBe(8);
    expect(config.layout.rowMarkerWidth).toBe(0);
    expect(config.layout.rowMarkerAlign).toBe("left");
    expect(config.layout.rowMarkerGap).toBe(1);
    expect(config.layout.rowMetaPreviewGap).toBe(2);
    expect(config.layout.rowPreviewReservedWidth).toBe(28);
    expect(config.layout.rowPreviewMaxWidth).toBe(0);
    expect(config.layout.rowSpacing).toBe(0);
    expect(config.layout.alternateRows).toBe(true);
    expect(config.layout.refreshIntervalMs).toBe(1000);
    expect(config.layout.mouseEnabled).toBe(true);
    expect(config.layout.mouseScrollRows).toBe(3);
    expect(config.layout.showScrollbar).toBe(true);
    expect(config.layout.scrollbarWidth).toBe(1);
    expect(config.layout.scrollbarPlacement).toBe("right");
    expect(config.layout.scrollbarAlign).toBe("left");
    expect(config.layout.showRowMetadata).toBe(true);
    expect(config.layout.showPreviewPane).toBe(true);
    expect(config.layout.showFullPreviewMetadata).toBe(true);
    expect(config.layout.showPreviewGutter).toBe(true);
    expect(config.layout.showFullPreviewGutter).toBe(true);
    expect(config.layout.highlightSearchMatches).toBe(true);
    expect(config.layout.showEmptyStateHelp).toBe(true);
    expect(config.layout.imagePreviewNoticeVisibility).toBe("protocol");
    expect(config.layout.fullPreviewImageNoticeVisibility).toBe("protocol");
    expect(config.layout.imagePreviewNoticeSpacing).toBe(0);
    expect(config.layout.fullPreviewImageNoticeSpacing).toBe(0);
    expect(config.layout.overlayPlacement).toBe("bottom");
    expect(config.chrome.panelBorderStyle).toBe("rounded");
    expect(config.chrome.panelBorder).toBe(true);
    expect(config.chrome.overlayBorder).toBe(true);
    expect(config.chrome.headerBorder).toBe(true);
    expect(config.chrome.listBorder).toBe(true);
    expect(config.chrome.previewBorder).toBe(true);
    expect(config.chrome.fullPreviewBorder).toBe(true);
    expect(config.chrome.statusBorder).toBe(false);
    expect(config.chrome.searchOverlayBorder).toBe(true);
    expect(config.chrome.dangerOverlayBorder).toBe(true);
    expect(config.chrome.helpOverlayBorder).toBe(true);
    expect(config.chrome.showPanelTitles).toBe(true);
    expect(config.chrome.showOverlayTitles).toBe(true);
    expect(config.chrome.showHeaderTitle).toBe(true);
    expect(config.chrome.showListTitle).toBe(true);
    expect(config.chrome.showPreviewTitle).toBe(true);
    expect(config.chrome.showFullPreviewTitle).toBe(true);
    expect(config.chrome.showStatusTitle).toBe(false);
    expect(config.chrome.showSearchOverlayTitle).toBe(true);
    expect(config.chrome.showDangerOverlayTitle).toBe(true);
    expect(config.chrome.showHelpOverlayTitle).toBe(true);
    expect(config.chrome.showListPositionTitle).toBe(true);
    expect(config.chrome.showPreviewEntryTitle).toBe(true);
    expect(config.chrome.showFullPreviewBottomTitle).toBe(true);
    expect(config.chrome.headerBorderStyle).toBe("rounded");
    expect(config.chrome.listBorderStyle).toBe("rounded");
    expect(config.chrome.previewBorderStyle).toBe("rounded");
    expect(config.chrome.fullPreviewBorderStyle).toBe("rounded");
    expect(config.chrome.statusBorderStyle).toBe("rounded");
    expect(config.chrome.searchOverlayBorderStyle).toBe("rounded");
    expect(config.chrome.dangerOverlayBorderStyle).toBe("rounded");
    expect(config.chrome.helpOverlayBorderStyle).toBe("rounded");
    expect(config.chrome.panelTitleAlignment).toBe("left");
    expect(config.chrome.panelBottomTitleAlignment).toBe("left");
    expect(config.chrome.overlayTitleAlignment).toBe("left");
    expect(config.chrome.headerTitleAlignment).toBe("left");
    expect(config.chrome.listTitleAlignment).toBe("left");
    expect(config.chrome.previewTitleAlignment).toBe("left");
    expect(config.chrome.fullPreviewTitleAlignment).toBe("left");
    expect(config.chrome.statusTitleAlignment).toBe("left");
    expect(config.chrome.listBottomTitleAlignment).toBe("left");
    expect(config.chrome.previewBottomTitleAlignment).toBe("left");
    expect(config.chrome.fullPreviewBottomTitleAlignment).toBe("left");
    expect(config.chrome.searchOverlayTitleAlignment).toBe("left");
    expect(config.chrome.dangerOverlayTitleAlignment).toBe("left");
    expect(config.chrome.helpOverlayTitleAlignment).toBe("left");
    expect(config.chrome.selectedMarker).toBe(">");
    expect(config.chrome.selectedMarkedMarker).toBe("*");
    expect(config.chrome.markedMarker).toBe("+");
    expect(surface(config, "header").bg).toBe(config.theme.bgPanel);
    expect(surface(config, "header").search).toBe(config.theme.accentSearch);
    expect(surface(config, "header").favorite).toBe(config.theme.accentFavorite);
    expect(surface(config, "header").secondary).toBe(config.theme.textSecondary);
    expect(surface(config, "header").bold).toBe(false);
    expect(surface(config, "header").underline).toBe(false);
    expect(surface(config, "list").image).toBe(config.theme.accentImage);
    expect(surface(config, "rowSpacer").bg).toBe(surface(config, "list").bg);
    expect(surface(config, "alternateRow").bg).toBe(config.theme.bgSubtle);
    expect(surface(config, "alternateRow").fg).toBe(config.theme.textPrimary);
    expect(surface(config, "selectedMarkedRow").bg).toBe(config.theme.bgSelected);
    expect(surface(config, "selectedMarkedRow").accent).toBe(config.theme.accentFavorite);
    expect(surface(config, "emptyState").bg).toBe(surface(config, "list").bg);
    expect(surface(config, "fullPreview").border).toBe(config.theme.borderFocused);
    expect(surface(config, "fullPreview").success).toBe(config.theme.accentSuccess);
    expect(surface(config, "previewGutter").muted).toBe(surface(config, "preview").muted);
    expect(surface(config, "previewSpacer").bg).toBe(surface(config, "preview").bg);
    expect(surface(config, "fullPreviewGutter").border).toBe(surface(config, "fullPreview").border);
    expect(surface(config, "fullPreviewMeta").bg).toBe(config.theme.bgSubtle);
    expect(surface(config, "fullPreviewMeta").accent).toBe(config.theme.accentText);
    expect(surface(config, "fullPreviewSpacer").bg).toBe(surface(config, "fullPreview").bg);
    expect(surface(config, "overlay").error).toBe(config.theme.accentError);
    expect(surface(config, "overlay").warning).toBe(config.theme.accentWarning);
    expect(surface(config, "searchOverlay").bg).toBe(surface(config, "overlay").bg);
    expect(surface(config, "dangerOverlay").error).toBe(surface(config, "overlay").error);
    expect(surface(config, "helpOverlay").accent).toBe(surface(config, "overlay").accent);
    expect(surface(config, "status").success).toBe(config.theme.accentSuccess);
    expect(surface(config, "splitPaneGap").bg).toBe(surface(config, "shell").bg);
    expect(config.keyBindings.copyPaste).toEqual(["enter"]);
    expect(config.keyBindings.preview).toEqual(["space"]);
    expect(config.keyBindings.searchCopyMatches).toEqual(["ctrl+s"]);
    expect(config.keyBindings.clearSelection).toEqual(["shift+s"]);
    expect(config.keyLabels.space).toBe("space");
    expect(config.keyLabels.escape).toBe("esc");
    expect(config.keyLabels.enter).toBe("enter");
    expect(config.statusTones.success).toEqual(["copied", "pasted", "cleared", "pinned", "unpinned"]);
    expect(config.statusTones.warning).toEqual(["paused"]);
    expect(config.statusTones.error).toContain("failed");
    expect(config.headerLineTones).toEqual({
      brand: "accent",
      filter: "auto",
      query: "search",
      mode: "secondary",
      filterLabel: "muted",
      queryLabel: "muted",
      modeLabel: "muted",
      sectionSeparator: "muted",
      labelSeparator: "muted",
    });
    expect(config.statusLineTones).toEqual({
      operation: "auto",
      watcher: "auto",
      hint: "muted",
      separator: "muted",
    });
    expect(config.overlayBorderTones).toEqual({
      search: "search",
      danger: "error",
      command: "border",
    });
    expect(config.overlayContentTones).toEqual({
      searchInput: "search",
      searchPrompt: "search",
      searchQuery: "search",
      searchCursor: "accent",
      deletePrompt: "error",
      deleteWarning: "warning",
      confirmHint: "muted",
      clearPrompt: "error",
      clearSafeHint: "success",
      clearUnsafeHint: "warning",
      helpKey: "accent",
      helpAction: "fg",
    });
    expect(config.listContentTones).toEqual({
      marker: "accent",
      markerGap: "muted",
      metadata: "accent",
      metadataGap: "muted",
      preview: "fg",
      searchMatch: "search",
      emptyTitle: "fg",
      emptyHelp: "muted",
      scrollbarThumb: "accent",
      scrollbarTrack: "muted",
    });
    expect(config.previewContentTones).toEqual({
      splitBorder: "auto",
      splitEmptyTitle: "muted",
      splitEmptyHelp: "secondary",
      splitImageFallbackPrefix: "muted",
      splitImageFallbackSeparator: "muted",
      splitImageFallbackReason: "muted",
      splitImageNotice: "muted",
      splitGutter: "muted",
      splitGutterSeparator: "muted",
      splitPrimary: "fg",
      splitSecondary: "secondary",
      splitMuted: "muted",
      splitAccent: "accent",
      splitError: "error",
      splitSuccess: "success",
      splitMetaHeader: "accent",
      splitMetaDetails: "fg",
      fullBorder: "auto",
      fullMeta: "auto",
      fullMetaHeader: "auto",
      fullMetaDetails: "fg",
      fullEmptyTitle: "muted",
      fullEmptyHelp: "secondary",
      fullImageFallbackPrefix: "muted",
      fullImageFallbackSeparator: "muted",
      fullImageFallbackReason: "muted",
      fullImageNotice: "muted",
      fullGutter: "muted",
      fullGutterSeparator: "muted",
      fullPrimary: "fg",
      fullSecondary: "secondary",
      fullMuted: "muted",
      fullAccent: "accent",
      fullError: "error",
      fullSuccess: "success",
    });
    expect(config.keyBindings.selectToggle).toEqual(["x"]);
    expect(config.keyBindings.selectSingle).toEqual(["s"]);
    expect(config.keyBindings.togglePinnedView).toEqual(["shift+tab"]);
    expect(config.keyBindings.clearAllIncludingPinned).toEqual(["c x"]);
    expect(config.filterOrder).toEqual(["all", "text", "images", "favorites", "today"]);
    expect(config.helpOrder).toEqual([
      "moveSelection",
      "pageSelection",
      "firstLastEntry",
      "preview",
      "pinnedView",
      "paste",
      "copySet",
      "output",
      "markSingle",
      "rangeSelect",
      "searchFilter",
      "searchCopyMatches",
      "pinDelete",
      "clearHistory",
      "clearAllIncludingPinned",
      "quit",
    ]);
    expect(config.startup).toEqual({ filter: "all", pinnedOnly: false, query: "" });
    expect(config.behavior).toEqual({
      liveSearch: true,
      liveSearchDebounceMs: 120,
      clearQueryOnSearchOpen: true,
      restoreQueryOnSearchCancel: true,
      exitAfterPaste: true,
      exitAfterCopy: false,
      exitAfterBulkCopy: false,
      exitAfterSearchCopy: false,
    });
    expect(config.labels.selectedCountTemplate).toBe("{prefix} {count}");
    expect(config.labels.kindText).toBe("TXT");
    expect(config.labels.statusTitle).toBe("status");
    expect(config.labels.rowPinnedLabel).toBe("PIN");
    expect(config.labels.rowMetaTemplate).toBe("{kind} {age} {size}{pinnedSlot}");
    expect(config.labels.rowPinnedSlotTemplate).toBe(" {pinned}");
    expect(config.labels.rowUnpinnedSlotTemplate).toBe("    ");
    expect(config.labels.entryIdPrefix).toBe("#");
    expect(config.labels.listPositionTemplate).toBe("{index}/{total}");
    expect(config.labels.headerSectionSeparator).toBe("  ");
    expect(config.labels.headerLabelSeparator).toBe(" ");
    expect(config.labels.headerLineTemplate).toBe(
      "{brand}{sectionSeparator}{filterLabel}{labelSeparator}{filter}{sectionSeparator}{queryLabel}{labelSeparator}{query}{sectionSeparator}{modeLabel}{labelSeparator}{mode}",
    );
    expect(config.labels.filterImages).toBe("IMAGES");
    expect(config.labels.clearKindImages).toBe("images");
    expect(config.labels.queryEmpty).toBe("-");
    expect(config.labels.searchPrompt).toBe("/");
    expect(config.labels.searchCursor).toBe("|");
    expect(config.labels.searchInputTemplate).toBe("{prompt}{query}{cursor}");
    expect(config.labels.confirmHintTemplate).toBe("{indent}{hint}");
    expect(config.labels.deleteOneTemplate).toBe("{message}");
    expect(config.labels.deleteManyTemplate).toBe("{message} ({count})");
    expect(config.labels.clearPromptTemplate).toBe("{prefix} {kind}?");
    expect(config.labels.errorPasteBackFailed).toBe("failed to paste through Hyprland");
    expect(config.labels.statusPinnedView).toBe("pinned");
    expect(config.labels.imagePreviewFallbackPrefix).toBe("img");
    expect(config.labels.imagePreviewFallbackSeparator).toBe("  ");
    expect(config.labels.previewMetaSeparator).toBe("  ");
    expect(config.labels.previewMetaLabelSeparator).toBe(" ");
    expect(config.labels.previewGutterSeparator).toBe("  ");
    expect(config.labels.fullPreviewGutterSeparator).toBe("  ");
    expect(config.labels.previewPinnedSuffixTemplate).toBe("{separator}{pinned}");
    expect(config.labels.previewEntryTitleTemplate).toBe("{entryIdPrefix}{id}");
    expect(config.labels.previewMetaHeaderTemplate).toBe("{kind} {entryIdPrefix}{id}{separator}{hashLabel}{hashLabelSeparator}{hash}");
    expect(config.labels.previewMetaDetailsTemplate).toBe("{mime}{separator}{size}{pinnedSuffix}");
    expect(config.labels.fullPreviewMetaTemplate).toBe("{kind} {entryIdPrefix}{id}{separator}{mime}{pinnedSuffix}");
    expect(config.labels.fullPreviewMetaHeaderTemplate).toBe("{kind} {entryIdPrefix}{id}{separator}{mime}{pinnedSuffix}");
    expect(config.labels.fullPreviewMetaDetailsTemplate).toBe("{size}{separator}{hashLabel}{hashLabelSeparator}{hash}");
    expect(config.labels.fullPreviewBottomTitleTemplate).toBe("{entryIdPrefix}{id} {start}-{end}/{total}{separator}{back}");
    expect(config.labels.previewTextGutterTemplate).toBe("{linePadded}");
    expect(config.labels.imagePreviewDecodePending).toBe("decoding image preview");
    expect(config.labels.imagePreviewUnsupportedMime).toBe("{mime} block preview is not supported yet");
    expect(config.labels.imagePreviewBlocksSource).toBe("image blocks");
    expect(config.labels.imagePreviewKittyFallbackSource).toBe("kitty fallback blocks");
    expect(config.labels.imagePreviewSixelFallbackSource).toBe("sixel fallback blocks");
    expect(config.labels.imagePreviewKittyProtocolName).toBe("Kitty");
    expect(config.labels.imagePreviewSixelProtocolName).toBe("Sixel");
    expect(config.labels.watcherRunning).toBe("watcher live");
    expect(config.labels.watcherErrorSeparator).toBe(": ");
    expect(config.labels.watcherRunningTemplate).toBe("{status} {age}");
    expect(config.labels.watcherPausedTemplate).toBe("{status}");
    expect(config.labels.watcherStaleTemplate).toBe("{status} {age}");
    expect(config.labels.watcherStoppedTemplate).toBe("{status}");
    expect(config.labels.watcherErrorTemplate).toBe("{status}{separator}{error}");
    expect(config.labels.errorUnknownStatus).toBe("unknown status");
    expect(config.labels.errorProcessTemplate).toBe("{message}");
    expect(config.labels.errorRpcTemplate).toBe("{message}");
    expect(config.labels.keyAlternativeSeparator).toBe(" / ");
    expect(config.labels.keyGroupSeparator).toBe("  ");
    expect(config.labels.statusHintSeparator).toBe("  ");
    expect(config.labels.statusHintTemplate).toBe(
      "{pasteKeys} {paste}{separator}{copyKeys} {copy}{separator}{previewKeys} {preview}{separator}{searchKeys} {search}{separator}{helpKeys} {help}",
    );
    expect(config.labels.statusSearchModeHintTemplate).toBe(
      "{applyKeys} {apply}{separator}{backspaceKeys} {backspace}{separator}{searchCopyKeys} {searchCopy}{separator}{cancelKeys} {cancel}",
    );
    expect(config.labels.statusPreviewModeHintTemplate).toBe(
      "{previewBackKeys} {previewBack}{separator}{previewScrollKeys} {previewScroll}{separator}{pasteKeys} {paste}{separator}{copyKeys} {copy}",
    );
    expect(config.labels.statusConfirmModeHintTemplate).toBe("{confirmYesKeys} {confirmYes}{separator}{confirmNoKeys} {confirmNo}");
    expect(config.labels.statusLineTemplate).toBe("{hint}{separator}{watcher}{separator}{operation}");
    expect(config.labels.statusPasteHint).toBe("paste");
    expect(config.labels.statusCopyHint).toBe("copy");
    expect(config.labels.statusPreviewHint).toBe("preview");
    expect(config.labels.statusSearchHint).toBe("search");
    expect(config.labels.statusFilterHint).toBe("filter");
    expect(config.labels.statusPinnedHint).toBe("pinned");
    expect(config.labels.statusDeleteHint).toBe("delete");
    expect(config.labels.statusOutputHint).toBe("output");
    expect(config.labels.statusHelpHint).toBe("help");
    expect(config.labels.statusQuitHint).toBe("quit");
    expect(config.labels.statusApplyHint).toBe("apply");
    expect(config.labels.statusCancelHint).toBe("cancel");
    expect(config.labels.statusBackspaceHint).toBe("delete char");
    expect(config.labels.statusSearchCopyHint).toBe("copy matches");
    expect(config.labels.statusPreviewBackHint).toBe("back");
    expect(config.labels.statusPreviewScrollHint).toBe("scroll");
    expect(config.labels.statusConfirmYesHint).toBe("confirm");
    expect(config.labels.statusConfirmNoHint).toBe("cancel");
    expect(config.labels.statusPinnedPrefix).toBe("pinned");
    expect(config.labels.statusUnpinnedPrefix).toBe("unpinned");
    expect(config.labels.statusPinTemplate).toBe("{prefix} {entryIdPrefix}{id}");
    expect(config.labels.statusClearedTemplate).toBe("{prefix} {count}; {pinned}");
    expect(config.labels.statusEntriesTemplate).toBe("{count} {entries}");
    expect(config.labels.helpSearchCopyMatches).toBe("copy matched search results");
    expect(config.labels.helpMarkSingle).toBe("mark / isolate / clear");
    expect(config.labels.helpOutput).toBe("print selected");
    expect(config.labels.helpPreviewNavigation).toBe("scroll preview");
    expect(config.labels.helpConfirmChoice).toBe("confirm / cancel");
    expect(config.labels.sizeKibUnit).toBe("KiB");
    expect(config.labels.ageMinutesUnit).toBe("m");
    expect(config.labels.textTruncationMarker).toBe("...");
    expect(config.labels.textWhitespaceReplacement).toBe(" ");
  });

  test("accepts every built-in theme preset from file config and env", () => {
    for (const name of themeNames) {
      const fileConfig = resolveTuiConfig({ theme: name }, { HOME: "/tmp/home" }, null);
      expect(fileConfig.theme.name).toBe(name);
      expect(surface(fileConfig, "shell").bg).toBe(themes[name].bgBase);

      const envConfig = resolveTuiConfig({ theme: "ditoxDark" }, { HOME: "/tmp/home", DITOX_TUI_THEME: name }, null);
      expect(envConfig.theme.name).toBe(name);
      expect(surface(envConfig, "status").success).toBe(themes[name].accentSuccess);
    }
  });

  test("compact mode applies dense layout defaults without overriding explicit values", () => {
    const compact = resolveTuiConfig({ layout: { compactMode: true } }, { HOME: "/tmp/home" }, null);
    expect(compact.layout.compactMode).toBe(true);
    expect(compact.layout.maxPreviewLines).toBe(18);
    expect(compact.layout.helpOverlayHeight).toBe(10);
    expect(compact.layout.minPaneWidth).toBe(18);
    expect(compact.layout.splitPaneGap).toBe(0);
    expect(compact.layout.splitPaneWidthInset).toBe(2);
    expect(compact.layout.fullPreviewWidthInset).toBe(2);
    expect(compact.layout.previewTextWidthInset).toBe(4);
    expect(compact.layout.fullPreviewTextWidthInset).toBe(4);
    expect(compact.layout.imagePreviewRowInset).toBe(4);
    expect(compact.layout.fullPreviewImageRowInset).toBe(4);
    expect(compact.layout.fullPreviewScrollInsetRows).toBe(1);
    expect(compact.layout.previewGutterWidth).toBe(3);
    expect(compact.layout.fullPreviewGutterWidth).toBe(3);
    expect(compact.layout.previewMetaHeight).toBe(2);
    expect(compact.layout.fullPreviewMetaHeight).toBe(1);
    expect(compact.layout.statusSeparatorPadding).toBe(1);
    expect(compact.layout.statusSeparatorPaddingLeft).toBe(1);
    expect(compact.layout.statusSeparatorPaddingRight).toBe(1);
    expect(compact.layout.frameTitlePadding).toBe(0);
    expect(compact.layout.frameTitlePaddingLeft).toBe(0);
    expect(compact.layout.frameTitlePaddingRight).toBe(0);
    expect(compact.layout.overlayPaddingX).toBe(0);
    expect(compact.layout.searchOverlayPaddingX).toBe(0);
    expect(compact.layout.dangerOverlayPaddingX).toBe(0);
    expect(compact.layout.helpOverlayPaddingX).toBe(0);
    expect(compact.layout.listPaddingX).toBe(0);
    expect(compact.layout.previewPaddingX).toBe(0);
    expect(compact.layout.fullPreviewPaddingX).toBe(0);
    expect(compact.layout.previewMetaPaddingX).toBe(0);
    expect(compact.layout.previewMetaPaddingY).toBe(0);
    expect(compact.layout.fullPreviewMetaPaddingX).toBe(0);
    expect(compact.layout.fullPreviewMetaPaddingY).toBe(0);
    expect(compact.layout.emptyStatePaddingX).toBe(0);
    expect(compact.layout.emptyStatePaddingY).toBe(0);
    expect(compact.layout.helpKeyWidth).toBe(18);
    expect(compact.layout.rowSizeWidth).toBe(5);
    expect(compact.layout.rowMetaPreviewGap).toBe(1);
    expect(compact.layout.rowPreviewReservedWidth).toBe(20);
    expect(compact.layout.showEmptyStateHelp).toBe(false);
    expect(compact.layout.panelPaddingX).toBe(0);

    const explicit = resolveTuiConfig({
      layout: {
        compactMode: true,
        maxPreviewLines: 30,
        previewMetaHeight: 4,
        fullPreviewMetaHeight: 3,
        fullPreviewMetaPaddingX: 3,
        fullPreviewTextWidthInset: 6,
        fullPreviewImageRowInset: 3,
        fullPreviewGutterWidth: 5,
        splitPaneGap: 4,
        rowPreviewReservedWidth: 34,
        showEmptyStateHelp: true,
        panelPaddingX: 2,
        listPaddingX: 1,
        previewPaddingX: 3,
        fullPreviewPaddingX: 4,
        overlayPaddingX: 3,
        overlayContentAlign: "center",
        overlayVerticalAlign: "center",
        overlayLineSpacing: 1,
        searchOverlayPaddingX: 0,
        searchOverlayContentAlign: "right",
        searchOverlayVerticalAlign: "bottom",
        searchOverlayLineSpacing: 2,
        dangerOverlayPaddingX: 2,
        helpOverlayPaddingX: 1,
        helpOverlayContentAlign: "left",
      },
    });
    expect(explicit.layout.maxPreviewLines).toBe(30);
    expect(explicit.layout.previewMetaHeight).toBe(4);
    expect(explicit.layout.fullPreviewMetaHeight).toBe(3);
    expect(explicit.layout.fullPreviewMetaPaddingX).toBe(3);
    expect(explicit.layout.fullPreviewTextWidthInset).toBe(6);
    expect(explicit.layout.fullPreviewImageRowInset).toBe(3);
    expect(explicit.layout.fullPreviewGutterWidth).toBe(5);
    expect(explicit.layout.splitPaneGap).toBe(4);
    expect(explicit.layout.rowPreviewReservedWidth).toBe(34);
    expect(explicit.layout.showEmptyStateHelp).toBe(true);
    expect(explicit.layout.panelPaddingX).toBe(2);
    expect(explicit.layout.listPaddingX).toBe(1);
    expect(explicit.layout.previewPaddingX).toBe(3);
    expect(explicit.layout.fullPreviewPaddingX).toBe(4);
    expect(explicit.layout.overlayPaddingX).toBe(3);
    expect(explicit.layout.overlayContentAlign).toBe("center");
    expect(explicit.layout.overlayVerticalAlign).toBe("center");
    expect(explicit.layout.overlayLineSpacing).toBe(1);
    expect(explicit.layout.searchOverlayPaddingX).toBe(0);
    expect(explicit.layout.searchOverlayContentAlign).toBe("right");
    expect(explicit.layout.searchOverlayVerticalAlign).toBe("bottom");
    expect(explicit.layout.searchOverlayLineSpacing).toBe(2);
    expect(explicit.layout.dangerOverlayPaddingX).toBe(2);
    expect(explicit.layout.dangerOverlayContentAlign).toBe("center");
    expect(explicit.layout.dangerOverlayVerticalAlign).toBe("center");
    expect(explicit.layout.dangerOverlayLineSpacing).toBe(1);
    expect(explicit.layout.helpOverlayPaddingX).toBe(1);
    expect(explicit.layout.helpOverlayContentAlign).toBe("left");
    expect(explicit.layout.helpOverlayVerticalAlign).toBe("center");
    expect(explicit.layout.helpOverlayLineSpacing).toBe(1);

    const envCompact = resolveTuiConfig(
      { layout: { compactMode: false } },
      { HOME: "/tmp/home", DITOX_TUI_COMPACT: "true", DITOX_TUI_MAX_PREVIEW_LINES: "36" },
      null,
    );
    expect(envCompact.layout.compactMode).toBe(true);
    expect(envCompact.layout.maxPreviewLines).toBe(36);
    expect(envCompact.layout.previewMetaHeight).toBe(2);
  });

  test("uses legacy global padding as per-area padding fallback", () => {
    const legacy = resolveTuiConfig({
      layout: {
        panelPaddingX: 2,
        panelPaddingY: 1,
        overlayPaddingX: 3,
        overlayPaddingY: 2,
        overlayContentAlign: "center",
        overlayVerticalAlign: "bottom",
        overlayLineSpacing: 2,
      },
    });
    expect(legacy.layout.listPaddingX).toBe(2);
    expect(legacy.layout.listPaddingY).toBe(1);
    expect(legacy.layout.previewPaddingX).toBe(2);
    expect(legacy.layout.previewPaddingY).toBe(1);
    expect(legacy.layout.fullPreviewPaddingX).toBe(2);
    expect(legacy.layout.fullPreviewPaddingY).toBe(1);
    expect(legacy.layout.searchOverlayPaddingX).toBe(3);
    expect(legacy.layout.searchOverlayPaddingY).toBe(2);
    expect(legacy.layout.searchOverlayContentAlign).toBe("center");
    expect(legacy.layout.searchOverlayVerticalAlign).toBe("bottom");
    expect(legacy.layout.searchOverlayLineSpacing).toBe(2);
    expect(legacy.layout.dangerOverlayPaddingX).toBe(3);
    expect(legacy.layout.dangerOverlayPaddingY).toBe(2);
    expect(legacy.layout.dangerOverlayContentAlign).toBe("center");
    expect(legacy.layout.dangerOverlayVerticalAlign).toBe("bottom");
    expect(legacy.layout.dangerOverlayLineSpacing).toBe(2);
    expect(legacy.layout.helpOverlayPaddingX).toBe(3);
    expect(legacy.layout.helpOverlayPaddingY).toBe(2);
    expect(legacy.layout.helpOverlayContentAlign).toBe("center");
    expect(legacy.layout.helpOverlayVerticalAlign).toBe("bottom");
    expect(legacy.layout.helpOverlayLineSpacing).toBe(2);

    const explicit = resolveTuiConfig({
      layout: {
        panelPaddingX: 2,
        panelPaddingY: 1,
        listPaddingX: 0,
        previewPaddingY: 0,
        fullPreviewPaddingX: 4,
        overlayPaddingX: 3,
        overlayPaddingY: 2,
        overlayContentAlign: "center",
        overlayVerticalAlign: "bottom",
        overlayLineSpacing: 2,
        searchOverlayPaddingX: 0,
        searchOverlayContentAlign: "right",
        searchOverlayVerticalAlign: "center",
        searchOverlayLineSpacing: 1,
        dangerOverlayPaddingY: 1,
        dangerOverlayVerticalAlign: "top",
        dangerOverlayLineSpacing: 0,
        helpOverlayPaddingX: 4,
        helpOverlayContentAlign: "left",
      },
    });
    expect(explicit.layout.listPaddingX).toBe(0);
    expect(explicit.layout.listPaddingY).toBe(1);
    expect(explicit.layout.previewPaddingX).toBe(2);
    expect(explicit.layout.previewPaddingY).toBe(0);
    expect(explicit.layout.fullPreviewPaddingX).toBe(4);
    expect(explicit.layout.fullPreviewPaddingY).toBe(1);
    expect(explicit.layout.searchOverlayPaddingX).toBe(0);
    expect(explicit.layout.searchOverlayPaddingY).toBe(2);
    expect(explicit.layout.searchOverlayContentAlign).toBe("right");
    expect(explicit.layout.searchOverlayVerticalAlign).toBe("center");
    expect(explicit.layout.searchOverlayLineSpacing).toBe(1);
    expect(explicit.layout.dangerOverlayPaddingX).toBe(3);
    expect(explicit.layout.dangerOverlayPaddingY).toBe(1);
    expect(explicit.layout.dangerOverlayContentAlign).toBe("center");
    expect(explicit.layout.dangerOverlayVerticalAlign).toBe("top");
    expect(explicit.layout.dangerOverlayLineSpacing).toBe(0);
    expect(explicit.layout.helpOverlayPaddingX).toBe(4);
    expect(explicit.layout.helpOverlayPaddingY).toBe(2);
    expect(explicit.layout.helpOverlayContentAlign).toBe("left");
    expect(explicit.layout.helpOverlayVerticalAlign).toBe("bottom");
    expect(explicit.layout.helpOverlayLineSpacing).toBe(2);
  });

  test("uses legacy global chrome as per-panel and per-overlay fallback", () => {
    const legacy = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        overlayBorder: false,
        showPanelTitles: false,
        showOverlayTitles: false,
        panelBorderStyle: "heavy",
        overlayBorderStyle: "double",
        panelTitleAlignment: "center",
        panelBottomTitleAlignment: "right",
        overlayTitleAlignment: "right",
      },
    });
    expect(legacy.chrome.headerBorder).toBe(false);
    expect(legacy.chrome.listBorder).toBe(false);
    expect(legacy.chrome.previewBorder).toBe(false);
    expect(legacy.chrome.fullPreviewBorder).toBe(false);
    expect(legacy.chrome.statusBorder).toBe(false);
    expect(legacy.chrome.searchOverlayBorder).toBe(false);
    expect(legacy.chrome.dangerOverlayBorder).toBe(false);
    expect(legacy.chrome.helpOverlayBorder).toBe(false);
    expect(legacy.chrome.showHeaderTitle).toBe(false);
    expect(legacy.chrome.showListTitle).toBe(false);
    expect(legacy.chrome.showPreviewTitle).toBe(false);
    expect(legacy.chrome.showFullPreviewTitle).toBe(false);
    expect(legacy.chrome.showStatusTitle).toBe(false);
    expect(legacy.chrome.showSearchOverlayTitle).toBe(false);
    expect(legacy.chrome.showDangerOverlayTitle).toBe(false);
    expect(legacy.chrome.showHelpOverlayTitle).toBe(false);
    expect(legacy.chrome.headerBorderStyle).toBe("heavy");
    expect(legacy.chrome.listBorderStyle).toBe("heavy");
    expect(legacy.chrome.previewBorderStyle).toBe("heavy");
    expect(legacy.chrome.fullPreviewBorderStyle).toBe("heavy");
    expect(legacy.chrome.statusBorderStyle).toBe("rounded");
    expect(legacy.chrome.searchOverlayBorderStyle).toBe("double");
    expect(legacy.chrome.dangerOverlayBorderStyle).toBe("double");
    expect(legacy.chrome.helpOverlayBorderStyle).toBe("double");
    expect(legacy.chrome.headerTitleAlignment).toBe("center");
    expect(legacy.chrome.listTitleAlignment).toBe("center");
    expect(legacy.chrome.previewTitleAlignment).toBe("center");
    expect(legacy.chrome.fullPreviewTitleAlignment).toBe("center");
    expect(legacy.chrome.statusTitleAlignment).toBe("left");
    expect(legacy.chrome.listBottomTitleAlignment).toBe("right");
    expect(legacy.chrome.previewBottomTitleAlignment).toBe("right");
    expect(legacy.chrome.fullPreviewBottomTitleAlignment).toBe("right");
    expect(legacy.chrome.searchOverlayTitleAlignment).toBe("right");
    expect(legacy.chrome.dangerOverlayTitleAlignment).toBe("right");
    expect(legacy.chrome.helpOverlayTitleAlignment).toBe("right");

    const explicit = resolveTuiConfig({
      chrome: {
        panelBorder: false,
        overlayBorder: false,
        headerBorder: true,
        previewBorder: true,
        statusBorder: true,
        searchOverlayBorder: true,
        helpOverlayBorder: true,
        showPanelTitles: false,
        showOverlayTitles: false,
        showHeaderTitle: true,
        showPreviewTitle: true,
        showStatusTitle: true,
        showSearchOverlayTitle: true,
        showHelpOverlayTitle: true,
        panelBorderStyle: "heavy",
        headerBorderStyle: "single",
        previewBorderStyle: "rounded",
        statusBorderStyle: "double",
        overlayBorderStyle: "double",
        searchOverlayBorderStyle: "single",
        helpOverlayBorderStyle: "heavy",
        panelTitleAlignment: "center",
        panelBottomTitleAlignment: "right",
        overlayTitleAlignment: "right",
        headerTitleAlignment: "left",
        previewTitleAlignment: "right",
        statusTitleAlignment: "right",
        listBottomTitleAlignment: "left",
        searchOverlayTitleAlignment: "center",
        helpOverlayTitleAlignment: "left",
      },
    });
    expect(explicit.chrome.headerBorder).toBe(true);
    expect(explicit.chrome.listBorder).toBe(false);
    expect(explicit.chrome.previewBorder).toBe(true);
    expect(explicit.chrome.fullPreviewBorder).toBe(false);
    expect(explicit.chrome.statusBorder).toBe(true);
    expect(explicit.chrome.searchOverlayBorder).toBe(true);
    expect(explicit.chrome.dangerOverlayBorder).toBe(false);
    expect(explicit.chrome.helpOverlayBorder).toBe(true);
    expect(explicit.chrome.showHeaderTitle).toBe(true);
    expect(explicit.chrome.showListTitle).toBe(false);
    expect(explicit.chrome.showPreviewTitle).toBe(true);
    expect(explicit.chrome.showFullPreviewTitle).toBe(false);
    expect(explicit.chrome.showStatusTitle).toBe(true);
    expect(explicit.chrome.showSearchOverlayTitle).toBe(true);
    expect(explicit.chrome.showDangerOverlayTitle).toBe(false);
    expect(explicit.chrome.showHelpOverlayTitle).toBe(true);
    expect(explicit.chrome.headerBorderStyle).toBe("single");
    expect(explicit.chrome.listBorderStyle).toBe("heavy");
    expect(explicit.chrome.previewBorderStyle).toBe("rounded");
    expect(explicit.chrome.fullPreviewBorderStyle).toBe("heavy");
    expect(explicit.chrome.statusBorderStyle).toBe("double");
    expect(explicit.chrome.searchOverlayBorderStyle).toBe("single");
    expect(explicit.chrome.dangerOverlayBorderStyle).toBe("double");
    expect(explicit.chrome.helpOverlayBorderStyle).toBe("heavy");
    expect(explicit.chrome.headerTitleAlignment).toBe("left");
    expect(explicit.chrome.listTitleAlignment).toBe("center");
    expect(explicit.chrome.previewTitleAlignment).toBe("right");
    expect(explicit.chrome.fullPreviewTitleAlignment).toBe("center");
    expect(explicit.chrome.statusTitleAlignment).toBe("right");
    expect(explicit.chrome.listBottomTitleAlignment).toBe("left");
    expect(explicit.chrome.previewBottomTitleAlignment).toBe("right");
    expect(explicit.chrome.fullPreviewBottomTitleAlignment).toBe("right");
    expect(explicit.chrome.searchOverlayTitleAlignment).toBe("center");
    expect(explicit.chrome.dangerOverlayTitleAlignment).toBe("right");
    expect(explicit.chrome.helpOverlayTitleAlignment).toBe("left");
  });

  test("keeps legacy split-preview layout config as the full-preview fallback", () => {
    const legacy = resolveTuiConfig({
      labels: { previewGutterSeparator: "::" },
      layout: {
        previewGutterWidth: 2,
        previewTextWidthInset: 10,
        imagePreviewMode: "kitty",
        imagePreviewMaxWidth: 24,
        imagePreviewMaxRows: 8,
        imagePreviewRenderer: "text",
        imagePreviewAlign: "center",
        imagePreviewBlockGlyph: "█",
        imagePreviewBackground: "#112233",
        imagePreviewNoticeVisibility: "always",
        imagePreviewNoticeSpacing: 2,
        imagePreviewRowInset: 7,
      },
    });
    expect(legacy.labels.previewGutterSeparator).toBe("::");
    expect(legacy.labels.fullPreviewGutterSeparator).toBe("::");
    expect(legacy.layout.previewGutterWidth).toBe(2);
    expect(legacy.layout.fullPreviewGutterWidth).toBe(2);
    expect(legacy.layout.previewTextWidthInset).toBe(10);
    expect(legacy.layout.fullPreviewTextWidthInset).toBe(10);
    expect(legacy.layout.imagePreviewMode).toBe("kitty");
    expect(legacy.layout.fullPreviewImageMode).toBe("kitty");
    expect(legacy.layout.imagePreviewMaxWidth).toBe(24);
    expect(legacy.layout.fullPreviewImageMaxWidth).toBe(24);
    expect(legacy.layout.imagePreviewMaxRows).toBe(8);
    expect(legacy.layout.fullPreviewImageMaxRows).toBe(8);
    expect(legacy.layout.imagePreviewRenderer).toBe("text");
    expect(legacy.layout.fullPreviewImageRenderer).toBe("text");
    expect(legacy.layout.imagePreviewAlign).toBe("center");
    expect(legacy.layout.fullPreviewImageAlign).toBe("center");
    expect(legacy.layout.imagePreviewBlockGlyph).toBe("█");
    expect(legacy.layout.fullPreviewImageBlockGlyph).toBe("█");
    expect(legacy.layout.imagePreviewBackground).toBe("#112233");
    expect(legacy.layout.fullPreviewImageBackground).toBe("#112233");
    expect(legacy.layout.imagePreviewNoticeVisibility).toBe("always");
    expect(legacy.layout.fullPreviewImageNoticeVisibility).toBe("always");
    expect(legacy.layout.imagePreviewNoticeSpacing).toBe(2);
    expect(legacy.layout.fullPreviewImageNoticeSpacing).toBe(2);
    expect(legacy.layout.imagePreviewRowInset).toBe(7);
    expect(legacy.layout.fullPreviewImageRowInset).toBe(7);

    const explicit = resolveTuiConfig({
      labels: { previewGutterSeparator: "::", fullPreviewGutterSeparator: ">>" },
      layout: {
        previewGutterWidth: 2,
        fullPreviewGutterWidth: 6,
        previewTextWidthInset: 10,
        fullPreviewTextWidthInset: 5,
        imagePreviewMode: "kitty",
        fullPreviewImageMode: "sixel",
        imagePreviewMaxWidth: 24,
        fullPreviewImageMaxWidth: 32,
        imagePreviewMaxRows: 8,
        fullPreviewImageMaxRows: 10,
        imagePreviewRenderer: "text",
        fullPreviewImageRenderer: "opentui",
        imagePreviewAlign: "center",
        fullPreviewImageAlign: "right",
        imagePreviewBlockGlyph: "█",
        fullPreviewImageBlockGlyph: "░",
        imagePreviewBackground: "#112233",
        fullPreviewImageBackground: "#445566",
        imagePreviewNoticeVisibility: "always",
        fullPreviewImageNoticeVisibility: "never",
        imagePreviewNoticeSpacing: 2,
        fullPreviewImageNoticeSpacing: 3,
        imagePreviewRowInset: 7,
        fullPreviewImageRowInset: 2,
      },
    });
    expect(explicit.labels.previewGutterSeparator).toBe("::");
    expect(explicit.labels.fullPreviewGutterSeparator).toBe(">>");
    expect(explicit.layout.previewGutterWidth).toBe(2);
    expect(explicit.layout.fullPreviewGutterWidth).toBe(6);
    expect(explicit.layout.previewTextWidthInset).toBe(10);
    expect(explicit.layout.fullPreviewTextWidthInset).toBe(5);
    expect(explicit.layout.imagePreviewMode).toBe("kitty");
    expect(explicit.layout.fullPreviewImageMode).toBe("sixel");
    expect(explicit.layout.imagePreviewMaxWidth).toBe(24);
    expect(explicit.layout.fullPreviewImageMaxWidth).toBe(32);
    expect(explicit.layout.imagePreviewMaxRows).toBe(8);
    expect(explicit.layout.fullPreviewImageMaxRows).toBe(10);
    expect(explicit.layout.imagePreviewRenderer).toBe("text");
    expect(explicit.layout.fullPreviewImageRenderer).toBe("opentui");
    expect(explicit.layout.imagePreviewAlign).toBe("center");
    expect(explicit.layout.fullPreviewImageAlign).toBe("right");
    expect(explicit.layout.imagePreviewBlockGlyph).toBe("█");
    expect(explicit.layout.fullPreviewImageBlockGlyph).toBe("░");
    expect(explicit.layout.imagePreviewBackground).toBe("#112233");
    expect(explicit.layout.fullPreviewImageBackground).toBe("#445566");
    expect(explicit.layout.imagePreviewNoticeVisibility).toBe("always");
    expect(explicit.layout.fullPreviewImageNoticeVisibility).toBe("never");
    expect(explicit.layout.imagePreviewNoticeSpacing).toBe(2);
    expect(explicit.layout.fullPreviewImageNoticeSpacing).toBe(3);
    expect(explicit.layout.imagePreviewRowInset).toBe(7);
    expect(explicit.layout.fullPreviewImageRowInset).toBe(2);
  });

  test("keeps legacy image preview copy as split and full fallback labels", () => {
    const legacy = resolveTuiConfig({
      labels: {
        imagePreviewFallbackPrefix: "legacy-img",
        imagePreviewFallbackSeparator: " :: ",
        imagePreviewSourceTemplate: "legacy {source}",
      },
    });

    expect(legacy.labels.splitImagePreviewFallbackPrefix).toBe("legacy-img");
    expect(legacy.labels.splitImagePreviewFallbackSeparator).toBe(" :: ");
    expect(legacy.labels.fullImagePreviewFallbackPrefix).toBe("legacy-img");
    expect(legacy.labels.fullImagePreviewFallbackSeparator).toBe(" :: ");
    expect(legacy.labels.splitImagePreviewSourceTemplate).toBe("legacy {source}");
    expect(legacy.labels.fullImagePreviewSourceTemplate).toBe("legacy {source}");

    const explicit = resolveTuiConfig({
      labels: {
        imagePreviewFallbackPrefix: "legacy-img",
        imagePreviewFallbackSeparator: " :: ",
        imagePreviewSourceTemplate: "legacy {source}",
        splitImagePreviewFallbackPrefix: "split-img",
        splitImagePreviewFallbackSeparator: " -- ",
        fullImagePreviewFallbackPrefix: "full-img",
        fullImagePreviewFallbackSeparator: " => ",
        splitImagePreviewSourceTemplate: "split {source}",
        fullImagePreviewSourceTemplate: "full {source}",
      },
    });

    expect(explicit.labels.splitImagePreviewFallbackPrefix).toBe("split-img");
    expect(explicit.labels.splitImagePreviewFallbackSeparator).toBe(" -- ");
    expect(explicit.labels.fullImagePreviewFallbackPrefix).toBe("full-img");
    expect(explicit.labels.fullImagePreviewFallbackSeparator).toBe(" => ");
    expect(explicit.labels.splitImagePreviewSourceTemplate).toBe("split {source}");
    expect(explicit.labels.fullImagePreviewSourceTemplate).toBe("full {source}");
  });

  test("configures terminal alternate-screen behavior from file and env", () => {
    const defaultConfig = resolveTuiConfig();
    expect(tuiRenderOptions(defaultConfig).screenMode).toBe("alternate-screen");
    expect(tuiRenderOptions(defaultConfig).backgroundColor).toBe(surface(defaultConfig, "shell").bg);

    const mainScreen = resolveTuiConfig({ terminal: { altScreen: "never" } });
    expect(mainScreen.terminal).toEqual({
      altScreen: "never",
      screenMode: "main-screen",
      backgroundColor: "auto",
      footerHeight: 12,
      clearOnShutdown: true,
      ...defaultTerminalRendererOptions,
    });
    expect(tuiRenderOptions(mainScreen).screenMode).toBe("main-screen");

    const grokStyleAlias = resolveTuiConfig({ terminal: { alt_screen: "always" } });
    expect(grokStyleAlias.terminal).toEqual({
      altScreen: "always",
      screenMode: "alternate-screen",
      backgroundColor: "auto",
      footerHeight: 12,
      clearOnShutdown: true,
      ...defaultTerminalRendererOptions,
    });

    const directMode = resolveTuiConfig({ terminal: { altScreen: "never", screenMode: "alternate-screen" } });
    expect(directMode.terminal).toEqual({
      altScreen: "always",
      screenMode: "alternate-screen",
      backgroundColor: "auto",
      footerHeight: 12,
      clearOnShutdown: true,
      ...defaultTerminalRendererOptions,
    });

    const envAltScreen = resolveTuiConfig({ terminal: { altScreen: "always" } }, { DITOX_TUI_ALT_SCREEN: "never" }, null);
    expect(envAltScreen.terminal).toEqual({
      altScreen: "never",
      screenMode: "main-screen",
      backgroundColor: "auto",
      footerHeight: 12,
      clearOnShutdown: true,
      ...defaultTerminalRendererOptions,
    });

    const envDirectMode = resolveTuiConfig(
      { terminal: { altScreen: "never" } },
      { DITOX_TUI_ALT_SCREEN: "never", DITOX_TUI_SCREEN_MODE: "alternate" },
      null,
    );
    expect(envDirectMode.terminal).toEqual({
      altScreen: "always",
      screenMode: "alternate-screen",
      backgroundColor: "auto",
      footerHeight: 12,
      clearOnShutdown: true,
      ...defaultTerminalRendererOptions,
    });

    const splitFooter = resolveTuiConfig({ terminal: { screenMode: "split-footer", footerHeight: 18, clearOnShutdown: false } });
    expect(splitFooter.terminal).toEqual({
      altScreen: "never",
      screenMode: "split-footer",
      backgroundColor: "auto",
      footerHeight: 18,
      clearOnShutdown: false,
      ...defaultTerminalRendererOptions,
    });
    expect(tuiRenderOptions(splitFooter).screenMode).toBe("split-footer");
    expect(tuiRenderOptions(splitFooter).footerHeight).toBe(18);
    expect(tuiRenderOptions(splitFooter).clearOnShutdown).toBe(false);

    const envSplitFooter = resolveTuiConfig(
      { terminal: { screenMode: "main-screen", footerHeight: 99, clearOnShutdown: true } },
      { DITOX_TUI_SCREEN_MODE: "split", DITOX_TUI_FOOTER_HEIGHT: "4", DITOX_TUI_CLEAR_ON_SHUTDOWN: "false" },
      null,
    );
    expect(envSplitFooter.terminal.screenMode).toBe("split-footer");
    expect(envSplitFooter.terminal.footerHeight).toBe(4);
    expect(envSplitFooter.terminal.clearOnShutdown).toBe(false);

    const customBackground = resolveTuiConfig({ terminal: { backgroundColor: "#123abc" } });
    expect(customBackground.terminal.backgroundColor).toBe("#123abc");
    expect(tuiRenderOptions(customBackground).backgroundColor).toBe("#123abc");

    const transparentBackground = resolveTuiConfig({ terminal: { backgroundColor: "transparent" } });
    expect(transparentBackground.terminal.backgroundColor).toBe("transparent");
    expect(tuiRenderOptions(transparentBackground).backgroundColor).toBe("transparent");

    const envBackground = resolveTuiConfig({ terminal: { backgroundColor: "#000000" } }, { DITOX_TUI_BACKGROUND: "#abcdef" }, null);
    expect(envBackground.terminal.backgroundColor).toBe("#abcdef");
    expect(tuiRenderOptions(envBackground).backgroundColor).toBe("#abcdef");

    const envBackgroundAlias = resolveTuiConfig({ terminal: { backgroundColor: "#000000" } }, { DITOX_TUI_BACKGROUND_COLOR: "transparent" }, null);
    expect(envBackgroundAlias.terminal.backgroundColor).toBe("transparent");

    const invalidBackground = resolveTuiConfig({ terminal: { backgroundColor: "not-a-color" } });
    expect(invalidBackground.terminal.backgroundColor).toBe("auto");
    expect(tuiRenderOptions(invalidBackground).backgroundColor).toBe(surface(invalidBackground, "shell").bg);

    const kittyKeyboard = resolveTuiConfig({
      terminal: {
        title: "ditox picker",
        cursor: {
          style: "line",
          blinking: false,
          color: "#abcdef",
        },
        kittyKeyboard: {
          enabled: true,
          events: true,
          allKeysAsEscapes: true,
          reportText: true,
          alternateKeys: false,
        },
        targetFps: 90,
        maxFps: 120,
        debounceDelay: 8,
        stdinParserMaxBufferBytes: 65536,
      },
    });
    expect(kittyKeyboard.terminal.title).toBe("ditox picker");
    const cursorOptions = terminalCursorStyleOptions(kittyKeyboard);
    expect(cursorOptions?.style).toBe("line");
    expect(cursorOptions?.blinking).toBe(false);
    expect(cursorOptions?.color?.toInts()).toEqual([171, 205, 239, 255]);
    expect(kittyKeyboard.terminal.kittyKeyboard).toEqual({
      enabled: true,
      disambiguate: true,
      alternateKeys: false,
      events: true,
      allKeysAsEscapes: true,
      reportText: true,
    });
    expect(tuiRenderOptions(kittyKeyboard).useKittyKeyboard).toEqual({
      disambiguate: true,
      alternateKeys: false,
      events: true,
      allKeysAsEscapes: true,
      reportText: true,
    });
    expect(tuiRenderOptions(kittyKeyboard).targetFps).toBe(90);
    expect(tuiRenderOptions(kittyKeyboard).maxFps).toBe(120);
    expect(tuiRenderOptions(kittyKeyboard).debounceDelay).toBe(8);
    expect(tuiRenderOptions(kittyKeyboard).stdinParserMaxBufferBytes).toBe(65536);

    const envRenderer = resolveTuiConfig(
      {
        terminal: {
          kittyKeyboard: false,
          cursor: { style: "block", blinking: true, color: "#111111" },
          title: "file title",
          targetFps: 500,
          maxFps: 0,
          debounceDelay: 999,
          stdinParserMaxBufferBytes: 1,
        },
      },
      {
        DITOX_TUI_KITTY_KEYBOARD: "all",
        DITOX_TUI_KITTY_KEYBOARD_ALTERNATE_KEYS: "false",
        DITOX_TUI_TARGET_FPS: "30",
        DITOX_TUI_MAX_FPS: "300",
        DITOX_TUI_RENDER_DEBOUNCE_MS: "4",
        DITOX_TUI_STDIN_BUFFER_BYTES: "2048",
        DITOX_TUI_TITLE: "env title",
        DITOX_TUI_CURSOR_STYLE: "underline",
        DITOX_TUI_CURSOR_BLINKING: "false",
        DITOX_TUI_CURSOR_COLOR: "#010203",
      },
      null,
    );
    expect(envRenderer.terminal.title).toBe("env title");
    expect(envRenderer.terminal.cursor).toEqual({ style: "underline", blinking: false, color: "#010203" });
    expect(envRenderer.terminal.kittyKeyboard).toEqual({
      enabled: true,
      disambiguate: true,
      alternateKeys: false,
      events: true,
      allKeysAsEscapes: true,
      reportText: true,
    });
    expect(envRenderer.terminal.targetFps).toBe(30);
    expect(envRenderer.terminal.maxFps).toBe(240);
    expect(envRenderer.terminal.debounceDelay).toBe(4);
    expect(envRenderer.terminal.stdinParserMaxBufferBytes).toBe(4096);
  });

  test("merges file overrides and clamps layout values", () => {
    const config = resolveTuiConfig(
      {
        theme: { preset: "ditoxLight", colors: { accentText: "#123456", accentError: "not a color value" } },
        layout: {
          listWidthPercent: 90,
          compactMode: true,
          maxPreviewLines: 200,
          maxFullPreviewLines: 12,
          historyLimit: 999,
          imagePreviewMode: "metadata",
          fullPreviewImageMode: "sixel",
          imagePreviewMaxWidth: 500,
          imagePreviewMaxRows: 1,
          fullPreviewImageMaxWidth: 500,
          fullPreviewImageMaxRows: 1,
          imagePreviewRenderer: "opentui",
          fullPreviewImageRenderer: "text",
          imagePreviewAlign: "center",
          fullPreviewImageAlign: "right",
          imagePreviewBlockGlyph: "▓▓",
          fullPreviewImageBlockGlyph: "▒▒",
          imagePreviewBackground: "#123abc",
          fullPreviewImageBackground: "#445566",
          fullPreviewImageNoticeVisibility: "never",
          fullPreviewImageNoticeSpacing: 1,
          headerHeight: 99,
          statusHeight: 0,
          showHeader: false,
          showStatusLine: false,
          searchOverlayHeight: 0,
          confirmOverlayHeight: 99,
          confirmPinnedExtraRows: 99,
          clearOverlayHeight: 99,
          helpOverlayHeight: 1,
          overlayPlacement: "top",
          minPaneWidth: 500,
          splitPaneGap: 99,
          splitPaneWidthInset: 99,
          fullPreviewWidthInset: 99,
          previewTextWidthInset: 99,
          fullPreviewTextWidthInset: 99,
          previewContentAlign: "center",
          fullPreviewContentAlign: "right",
          previewBodyVerticalAlign: "center",
          fullPreviewBodyVerticalAlign: "bottom",
          imagePreviewRowInset: 99,
          fullPreviewImageRowInset: 99,
          fullPreviewScrollInsetRows: 99,
          previewLineNumberWidth: 0,
          previewGutterWidth: 99,
          fullPreviewGutterWidth: 99,
          previewGutterAlign: "center",
          fullPreviewGutterAlign: "left",
          previewLineSpacing: 99,
          fullPreviewLineSpacing: 99,
          previewMetaHeight: 99,
          fullPreviewMetaHeight: 99,
          previewMetaLineSpacing: 99,
          fullPreviewMetaLineSpacing: 1,
          previewMetaHashLength: 2,
          fullPreviewMetaHashLength: 99,
          previewImageFields: ["mime", "hash", "mime", "bad" as never],
          headerBrandMaxWidth: 201,
          headerFilterMaxWidth: 12,
          headerQueryMaxWidth: 13,
          headerModeMaxWidth: 14,
          statusSeparatorPadding: 99,
          statusSeparatorPaddingLeft: 3,
          statusSeparatorPaddingRight: 4,
          statusOperationMaxWidth: 201,
          statusWatcherMaxWidth: 17,
          statusHintMaxWidth: 18,
          searchOverlayPromptMaxWidth: 19,
          searchOverlayQueryMaxWidth: 20,
          searchOverlayCursorMaxWidth: 21,
          dangerOverlayPromptMaxWidth: 22,
          dangerOverlayHintMaxWidth: 23,
          helpOverlayActionMaxWidth: 201,
          frameTitlePadding: 99,
          frameTitlePaddingLeft: 2,
          frameTitlePaddingRight: 3,
          shellPaddingX: 99,
          shellPaddingY: 99,
          headerPaddingX: 99,
          headerPaddingY: 99,
          statusPaddingX: 99,
          statusPaddingY: 99,
          headerContentAlign: "center",
          statusContentAlign: "right",
          headerVerticalAlign: "bottom",
          statusVerticalAlign: "center",
          overlayPaddingX: 99,
          overlayPaddingY: 99,
          overlayContentAlign: "center",
          overlayVerticalAlign: "center",
          overlayLineSpacing: 99,
          searchOverlayPaddingX: 99,
          searchOverlayPaddingY: 99,
          searchOverlayContentAlign: "right",
          searchOverlayVerticalAlign: "bottom",
          searchOverlayLineSpacing: 99,
          dangerOverlayPaddingX: 99,
          dangerOverlayPaddingY: 99,
          dangerOverlayContentAlign: "left",
          dangerOverlayVerticalAlign: "top",
          dangerOverlayLineSpacing: 99,
          helpOverlayPaddingX: 99,
          helpOverlayPaddingY: 99,
          helpOverlayContentAlign: "center",
          helpOverlayVerticalAlign: "bottom",
          helpOverlayLineSpacing: 99,
          listPaddingX: 99,
          listPaddingY: 99,
          previewPaddingX: 99,
          previewPaddingY: 99,
          fullPreviewPaddingX: 99,
          fullPreviewPaddingY: 99,
          previewMetaPaddingX: 99,
          previewMetaPaddingY: 99,
          fullPreviewMetaPaddingX: 99,
          fullPreviewMetaPaddingY: 99,
          previewMetaContentAlign: "right",
          fullPreviewMetaContentAlign: "center",
          previewMetaVerticalAlign: "center",
          fullPreviewMetaVerticalAlign: "bottom",
          emptyStatePaddingX: 99,
          emptyStatePaddingY: 99,
          emptyStateContentAlign: "center",
          emptyStateTitleAlign: "right",
          emptyStateHelpAlign: "left",
          emptyStateVerticalAlign: "bottom",
          emptyStateLineSpacing: 99,
          helpKeyWidth: 2,
          helpKeyAlign: "center",
          confirmHintIndent: 99,
          rowContentAlign: "center",
          rowMetadataAlign: "right",
          rowPreviewAlign: "center",
          rowAgeWidth: 99,
          rowAgeAlign: "left",
          rowSizeWidth: 99,
          rowSizeAlign: "center",
          rowPinnedWidth: 99,
          rowPinnedAlign: "right",
          rowMetaHashLength: 2,
          rowMarkerWidth: 99,
          rowMarkerAlign: "center",
          rowMarkerGap: 99,
          rowMetaPreviewGap: 99,
          rowPreviewReservedWidth: 2,
          rowPreviewMaxWidth: 999,
          rowSpacing: 99,
          alternateRows: false,
          refreshIntervalMs: 100000,
          mouseEnabled: false,
          mouseScrollRows: 100,
          scrollbarWidth: 99,
          scrollbarPlacement: "left",
          scrollbarAlign: "center",
          showMetadata: false,
          showRowMetadata: true,
          showPreviewPane: false,
          showFullPreviewMetadata: true,
          showPreviewGutter: false,
          showFullPreviewGutter: false,
          highlightSearchMatches: false,
          showEmptyStateHelp: false,
          imagePreviewNoticeVisibility: "always",
          imagePreviewNoticeSpacing: 99,
        },
        chrome: {
          panelBorder: false,
          overlayBorder: false,
          headerBorder: true,
          listBorder: false,
          previewBorder: true,
          fullPreviewBorder: false,
          statusBorder: true,
          searchOverlayBorder: true,
          dangerOverlayBorder: false,
          helpOverlayBorder: true,
          showPanelTitles: false,
          showOverlayTitles: false,
          showHeaderTitle: true,
          showListTitle: false,
          showPreviewTitle: true,
          showFullPreviewTitle: false,
          showStatusTitle: true,
          showSearchOverlayTitle: true,
          showDangerOverlayTitle: false,
          showHelpOverlayTitle: true,
          showListPositionTitle: false,
          showPreviewEntryTitle: false,
          showFullPreviewBottomTitle: false,
          panelBorderStyle: "double",
          headerBorderStyle: "single",
          listBorderStyle: "bad" as never,
          previewBorderStyle: "heavy",
          fullPreviewBorderStyle: "rounded",
          statusBorderStyle: "single",
          searchOverlayBorderStyle: "single",
          dangerOverlayBorderStyle: "heavy",
          helpOverlayBorderStyle: "double",
          panelTitleAlignment: "center",
          panelBottomTitleAlignment: "right",
          overlayTitleAlignment: "right",
          statusTitleAlignment: "center",
          selectedMarker: ">>",
          selectedMarkedMarker: "**",
          scrollbarThumb: "█",
        },
        styles: {
          shell: { bg: "#000102" },
          splitPaneGap: { bg: "#020304" },
          header: { bg: "#010203", fg: "#abcdef", search: "#222222", favorite: "#333333", secondary: "#444444", bold: true },
          list: { image: "#112255", favorite: "#775511" },
          rowSpacer: { bg: "#0f1011" },
          alternateRow: { bg: "#101820", fg: "#f0f4f8" },
          selectedMarkedRow: { bg: "#202c34", accent: "#f0c050", bold: true },
          emptyState: { fg: "#445566" },
          preview: { muted: "#223344" },
          previewGutter: { muted: "#334455" },
          previewSpacer: { bg: "#141516" },
          fullPreview: { muted: "#445533" },
          fullPreviewGutter: { muted: "#556644" },
          fullPreviewMeta: { bg: "#121314", accent: "#234567", dim: true },
          fullPreviewSpacer: { bg: "#171819" },
          overlay: { error: "#551111", search: "#552255", inverse: true },
          searchOverlay: { search: "#335577" },
          dangerOverlay: { error: "#773333" },
          helpOverlay: { accent: "#337755" },
          status: { bg: "not a color", accent: "#111111", success: "#225522", warning: "#665522", error: "#552222", dim: true },
        },
        labels: {
          brand: "DX",
          statusTitle: "state",
          ready: "idle",
          selectedCountTemplate: "{count} {prefix}",
          watcherRunning: "daemon live",
          watcherErrorSeparator: " -> ",
          watcherRunningTemplate: "{age} / {status}",
          watcherErrorTemplate: "{error} / {status}",
          kindText: "TEXT",
          rowPinnedLabel: "SV",
          rowMetaTemplate: "{kind}|{age}|{size}|{pinnedSlot}",
          rowPinnedSlotTemplate: "[{pinned}]",
          rowUnpinnedSlotTemplate: "[ ]",
          entryIdPrefix: "id:",
          listPositionTemplate: "{index} of {total}",
          filterImages: "PICS",
          clearKindImages: "pictures",
          queryEmpty: "none",
          searchPrompt: "find> ",
          searchCursor: "_",
          searchInputTemplate: "{prompt}[{query}]{cursor}",
          confirmHintTemplate: "confirm => {hint}",
          deleteOneTemplate: "one: {message}",
          deleteManyTemplate: "{count}x {message}",
          clearPromptTemplate: "{kind} <- {prefix}",
          headerSectionSeparator: " / ",
          headerLabelSeparator: "=",
          headerLineTemplate: "{query} <- {brand} [{mode}]",
          previewGutterSeparator: " | ",
          fullPreviewGutterSeparator: " >> ",
          previewMetaSeparator: " :: ",
          previewMetaLabelSeparator: "=",
          previewPinnedSuffixTemplate: "{separator}saved:{pinned}",
          previewEntryTitleTemplate: "clip {id}",
          previewMetaHeaderTemplate: "{kind}/{id}/{hash}",
          previewMetaDetailsTemplate: "{size}/{mime}{pinnedSuffix}",
          fullPreviewMetaTemplate: "{kind}/{id}/{mime}{pinnedSuffix}",
          fullPreviewMetaHeaderTemplate: "full:{kind}/{id}/{mime}",
          fullPreviewMetaDetailsTemplate: "{size}/{hashShort}{pinnedSuffix}",
          fullPreviewBottomTitleTemplate: "{id}:{start}-{end}/{total}{separator}{back}",
          previewTextGutterTemplate: "L{line}",
          imagePreviewFallbackSeparator: " -> ",
          splitImagePreviewFallbackPrefix: "split-img",
          splitImagePreviewFallbackSeparator: " <- ",
          fullImagePreviewFallbackPrefix: "full-img",
          fullImagePreviewFallbackSeparator: " => ",
          imagePreviewDecodePending: "loading pixels",
          imagePreviewSourceTemplate: "SRC {source}",
          splitImagePreviewSourceTemplate: "SPLIT {source}",
          fullImagePreviewSourceTemplate: "FULL {source}",
          imagePreviewBlocksSource: "ansi blocks",
          imagePreviewKittyFallbackSource: "kitty ansi fallback",
          imagePreviewSixelFallbackSource: "sixel ansi fallback",
          imagePreviewKittyProtocolName: "KITTY",
          imagePreviewSixelProtocolName: "SIXEL",
          errorPasteBackFailed: "paste transport failed",
          errorUnknownStatus: "unknown backend exit",
          errorProcessTemplate: "process {method}: {message}",
          errorRpcTemplate: "rpc {method}: {message}",
          statusPinnedView: "SAVED",
          statusPinnedPrefix: "saved",
          statusUnpinnedPrefix: "unsaved",
          statusPinTemplate: "{prefix}:{entryIdPrefix}{id}",
          statusClearedTemplate: "{prefix}: {count} ({pinned})",
          statusEntriesTemplate: "{entries}: {count}",
          imagePreviewUnsupportedMime: "cannot draw {mime}",
          keyAlternativeSeparator: " | ",
          keyGroupSeparator: " + ",
          statusHintSeparator: " :: ",
          statusHintTemplate: "{searchKeys}:{search}{separator}{pasteKeys}:{paste}{separator}{helpKeys}:{help}",
          statusLineTemplate: "{operation}{separator}{watcher}{separator}{hint}",
          statusPasteHint: "send",
          helpSearchCopyMatches: "yank visible matches",
          sizeBytesUnit: "bytes",
          sizeKibUnit: "KB",
          sizeMibUnit: "MB",
          ageSecondsUnit: "sec",
          ageMinutesUnit: "min",
          ageHoursUnit: "hr",
          ageDaysUnit: "day",
          textTruncationMarker: "~",
          textWhitespaceReplacement: "_",
        },
        filterOrder: ["images", "all", "images", "invalid", "favorites"],
        helpOrder: ["paste", "searchCopyMatches", "paste", "invalid", "preview"],
        startup: {
          filter: "today",
          pinnedOnly: true,
          query: "needle",
        },
        behavior: {
          liveSearch: false,
          liveSearchDebounceMs: 9999,
          clearQueryOnSearchOpen: false,
          restoreQueryOnSearchCancel: false,
          exitAfterPaste: false,
          exitAfterCopy: true,
          exitAfterBulkCopy: true,
          exitAfterSearchCopy: true,
        },
        keyBindings: { quit: "q,ctrl+q", copyPaste: ["enter", "ctrl+p"], searchCopyMatches: "ctrl+g" },
        keyLabels: { space: "SPC", escape: "ESC", enter: "RET", PageDown: "PGDN" },
        statusTones: { success: ["stored"], warning: ["waiting"], error: ["boom"] },
        headerLineTones: {
          brand: "success",
          filter: "warning",
          query: "search",
          mode: "secondary",
          filterLabel: "accent",
          queryLabel: "image",
          modeLabel: "favorite",
          sectionSeparator: "fg",
          labelSeparator: "muted",
        },
        statusLineTones: { operation: "success", watcher: "warning", hint: "secondary", separator: "accent" },
        overlayBorderTones: { search: "accent", danger: "warning", command: "success" },
        overlayContentTones: {
          searchInput: "accent",
          searchPrompt: "muted",
          searchQuery: "search",
          searchCursor: "warning",
          deletePrompt: "warning",
          deleteWarning: "secondary",
          confirmHint: "fg",
          clearPrompt: "image",
          clearSafeHint: "favorite",
          clearUnsafeHint: "error",
          helpKey: "success",
          helpAction: "muted",
        },
        listContentTones: {
          marker: "success",
          markerGap: "muted",
          metadata: "warning",
          metadataGap: "secondary",
          preview: "fg",
          searchMatch: "favorite",
          emptyTitle: "image",
          emptyHelp: "accent",
          scrollbarThumb: "error",
          scrollbarTrack: "border",
        },
        previewContentTones: {
          splitBorder: "success",
          splitEmptyTitle: "warning",
          splitEmptyHelp: "secondary",
          splitImageFallbackPrefix: "accent",
          splitImageFallbackSeparator: "muted",
          splitImageFallbackReason: "error",
          splitGutter: "image",
          splitGutterSeparator: "favorite",
          splitPrimary: "fg",
          splitSecondary: "secondary",
          splitMuted: "muted",
          splitAccent: "accent",
          splitError: "error",
          splitSuccess: "success",
          splitMetaHeader: "warning",
          splitMetaDetails: "border",
          fullBorder: "image",
          fullMeta: "favorite",
          fullMetaHeader: "accent",
          fullMetaDetails: "secondary",
          fullEmptyTitle: "warning",
          fullEmptyHelp: "secondary",
          fullImageFallbackPrefix: "accent",
          fullImageFallbackSeparator: "muted",
          fullImageFallbackReason: "error",
          fullGutter: "image",
          fullGutterSeparator: "favorite",
          fullPrimary: "fg",
          fullSecondary: "secondary",
          fullMuted: "muted",
          fullAccent: "accent",
          fullError: "error",
          fullSuccess: "success",
        },
      },
      {},
      "/tmp/tui.json",
    );

    expect(config.sourcePath).toBe("/tmp/tui.json");
    expect(config.theme.name).toBe("ditoxLight");
    expect(config.theme.accentText).toBe("#123456");
    expect(config.theme.accentError).toBe("#b53330");
    expect(config.layout.listWidthPercent).toBe(68);
    expect(config.layout.previewWidthPercent).toBe(32);
    expect(config.layout.maxPreviewLines).toBe(120);
    expect(config.layout.maxFullPreviewLines).toBe(24);
    expect(config.layout.historyLimit).toBe(500);
    expect(config.layout.imagePreviewMode).toBe("metadata");
    expect(config.layout.fullPreviewImageMode).toBe("sixel");
    expect(config.layout.imagePreviewMaxWidth).toBe(120);
    expect(config.layout.imagePreviewMaxRows).toBe(2);
    expect(config.layout.fullPreviewImageMaxWidth).toBe(120);
    expect(config.layout.fullPreviewImageMaxRows).toBe(2);
    expect(config.layout.imagePreviewRenderer).toBe("opentui");
    expect(config.layout.fullPreviewImageRenderer).toBe("text");
    expect(config.layout.imagePreviewAlign).toBe("center");
    expect(config.layout.fullPreviewImageAlign).toBe("right");
    expect(config.layout.imagePreviewBlockGlyph).toBe("▓");
    expect(config.layout.fullPreviewImageBlockGlyph).toBe("▒");
    expect(config.layout.imagePreviewBackground).toBe("#123abc");
    expect(config.layout.fullPreviewImageBackground).toBe("#445566");
    expect(config.layout.fullPreviewImageNoticeVisibility).toBe("never");
    expect(config.layout.fullPreviewImageNoticeSpacing).toBe(1);
    expect(config.layout.headerHeight).toBe(6);
    expect(config.layout.statusHeight).toBe(1);
    expect(config.layout.showHeader).toBe(false);
    expect(config.layout.showStatusLine).toBe(false);
    expect(config.layout.searchOverlayHeight).toBe(1);
    expect(config.layout.confirmOverlayHeight).toBe(8);
    expect(config.layout.confirmPinnedExtraRows).toBe(4);
    expect(config.layout.clearOverlayHeight).toBe(8);
    expect(config.layout.helpOverlayHeight).toBe(8);
    expect(config.layout.minPaneWidth).toBe(80);
    expect(config.layout.splitPaneGap).toBe(8);
    expect(config.layout.splitPaneWidthInset).toBe(20);
    expect(config.layout.fullPreviewWidthInset).toBe(20);
    expect(config.layout.previewTextWidthInset).toBe(24);
    expect(config.layout.fullPreviewTextWidthInset).toBe(24);
    expect(config.layout.previewContentAlign).toBe("center");
    expect(config.layout.fullPreviewContentAlign).toBe("right");
    expect(config.layout.previewBodyVerticalAlign).toBe("center");
    expect(config.layout.fullPreviewBodyVerticalAlign).toBe("bottom");
    expect(config.layout.imagePreviewRowInset).toBe(20);
    expect(config.layout.fullPreviewImageRowInset).toBe(20);
    expect(config.layout.fullPreviewScrollInsetRows).toBe(12);
    expect(config.layout.previewLineNumberWidth).toBe(1);
    expect(config.layout.previewGutterWidth).toBe(12);
    expect(config.layout.fullPreviewGutterWidth).toBe(12);
    expect(config.layout.previewGutterAlign).toBe("center");
    expect(config.layout.fullPreviewGutterAlign).toBe("left");
    expect(config.layout.previewLineSpacing).toBe(3);
    expect(config.layout.fullPreviewLineSpacing).toBe(3);
    expect(config.layout.previewMetaHeight).toBe(4);
    expect(config.layout.fullPreviewMetaHeight).toBe(4);
    expect(config.layout.previewMetaLineSpacing).toBe(2);
    expect(config.layout.fullPreviewMetaLineSpacing).toBe(1);
    expect(config.layout.previewMetaHashLength).toBe(6);
    expect(config.layout.fullPreviewMetaHashLength).toBe(64);
    expect(config.layout.previewImageFields).toEqual(["mime", "hash"]);
    expect(config.layout.headerBrandMaxWidth).toBe(200);
    expect(config.layout.headerFilterMaxWidth).toBe(12);
    expect(config.layout.headerQueryMaxWidth).toBe(13);
    expect(config.layout.headerModeMaxWidth).toBe(14);
    expect(config.layout.statusSeparatorPadding).toBe(6);
    expect(config.layout.statusSeparatorPaddingLeft).toBe(3);
    expect(config.layout.statusSeparatorPaddingRight).toBe(4);
    expect(config.layout.statusOperationMaxWidth).toBe(200);
    expect(config.layout.statusWatcherMaxWidth).toBe(17);
    expect(config.layout.statusHintMaxWidth).toBe(18);
    expect(config.layout.searchOverlayPromptMaxWidth).toBe(19);
    expect(config.layout.searchOverlayQueryMaxWidth).toBe(20);
    expect(config.layout.searchOverlayCursorMaxWidth).toBe(21);
    expect(config.layout.dangerOverlayPromptMaxWidth).toBe(22);
    expect(config.layout.dangerOverlayHintMaxWidth).toBe(23);
    expect(config.layout.helpOverlayActionMaxWidth).toBe(200);
    expect(config.layout.frameTitlePadding).toBe(4);
    expect(config.layout.frameTitlePaddingLeft).toBe(2);
    expect(config.layout.frameTitlePaddingRight).toBe(3);
    expect(config.layout.shellPaddingX).toBe(6);
    expect(config.layout.shellPaddingY).toBe(4);
    expect(config.layout.headerPaddingX).toBe(4);
    expect(config.layout.headerPaddingY).toBe(3);
    expect(config.layout.statusPaddingX).toBe(4);
    expect(config.layout.statusPaddingY).toBe(3);
    expect(config.layout.headerContentAlign).toBe("center");
    expect(config.layout.statusContentAlign).toBe("right");
    expect(config.layout.headerVerticalAlign).toBe("bottom");
    expect(config.layout.statusVerticalAlign).toBe("center");
    expect(config.layout.overlayPaddingX).toBe(4);
    expect(config.layout.overlayPaddingY).toBe(3);
    expect(config.layout.overlayContentAlign).toBe("center");
    expect(config.layout.overlayVerticalAlign).toBe("center");
    expect(config.layout.overlayLineSpacing).toBe(3);
    expect(config.layout.searchOverlayPaddingX).toBe(4);
    expect(config.layout.searchOverlayPaddingY).toBe(3);
    expect(config.layout.searchOverlayContentAlign).toBe("right");
    expect(config.layout.searchOverlayVerticalAlign).toBe("bottom");
    expect(config.layout.searchOverlayLineSpacing).toBe(3);
    expect(config.layout.dangerOverlayPaddingX).toBe(4);
    expect(config.layout.dangerOverlayPaddingY).toBe(3);
    expect(config.layout.dangerOverlayContentAlign).toBe("left");
    expect(config.layout.dangerOverlayVerticalAlign).toBe("top");
    expect(config.layout.dangerOverlayLineSpacing).toBe(3);
    expect(config.layout.helpOverlayPaddingX).toBe(4);
    expect(config.layout.helpOverlayPaddingY).toBe(3);
    expect(config.layout.helpOverlayContentAlign).toBe("center");
    expect(config.layout.helpOverlayVerticalAlign).toBe("bottom");
    expect(config.layout.helpOverlayLineSpacing).toBe(3);
    expect(config.layout.listPaddingX).toBe(4);
    expect(config.layout.listPaddingY).toBe(2);
    expect(config.layout.previewPaddingX).toBe(4);
    expect(config.layout.previewPaddingY).toBe(2);
    expect(config.layout.fullPreviewPaddingX).toBe(4);
    expect(config.layout.fullPreviewPaddingY).toBe(2);
    expect(config.layout.previewMetaPaddingX).toBe(4);
    expect(config.layout.previewMetaPaddingY).toBe(2);
    expect(config.layout.fullPreviewMetaPaddingX).toBe(4);
    expect(config.layout.fullPreviewMetaPaddingY).toBe(2);
    expect(config.layout.previewMetaContentAlign).toBe("right");
    expect(config.layout.fullPreviewMetaContentAlign).toBe("center");
    expect(config.layout.previewMetaVerticalAlign).toBe("center");
    expect(config.layout.fullPreviewMetaVerticalAlign).toBe("bottom");
    expect(config.layout.emptyStatePaddingX).toBe(4);
    expect(config.layout.emptyStatePaddingY).toBe(2);
    expect(config.layout.emptyStateContentAlign).toBe("center");
    expect(config.layout.emptyStateTitleAlign).toBe("right");
    expect(config.layout.emptyStateHelpAlign).toBe("left");
    expect(config.layout.emptyStateVerticalAlign).toBe("bottom");
    expect(config.layout.emptyStateLineSpacing).toBe(4);
    expect(config.layout.helpKeyWidth).toBe(8);
    expect(config.layout.helpKeyAlign).toBe("center");
    expect(config.layout.confirmHintIndent).toBe(8);
    expect(config.layout.rowContentAlign).toBe("center");
    expect(config.layout.rowMetadataAlign).toBe("right");
    expect(config.layout.rowPreviewAlign).toBe("center");
    expect(config.layout.rowAgeWidth).toBe(12);
    expect(config.layout.rowAgeAlign).toBe("left");
    expect(config.layout.rowSizeWidth).toBe(16);
    expect(config.layout.rowSizeAlign).toBe("center");
    expect(config.layout.rowPinnedWidth).toBe(12);
    expect(config.layout.rowPinnedAlign).toBe("right");
    expect(config.layout.rowMetaHashLength).toBe(4);
    expect(config.layout.rowMarkerWidth).toBe(12);
    expect(config.layout.rowMarkerAlign).toBe("center");
    expect(config.layout.rowMarkerGap).toBe(4);
    expect(config.layout.rowMetaPreviewGap).toBe(8);
    expect(config.layout.rowPreviewReservedWidth).toBe(8);
    expect(config.layout.rowPreviewMaxWidth).toBe(240);
    expect(config.layout.rowSpacing).toBe(3);
    expect(config.layout.alternateRows).toBe(false);
    expect(config.layout.refreshIntervalMs).toBe(60000);
    expect(config.layout.mouseEnabled).toBe(false);
    expect(config.layout.mouseScrollRows).toBe(20);
    expect(config.layout.scrollbarWidth).toBe(4);
    expect(config.layout.scrollbarPlacement).toBe("left");
    expect(config.layout.scrollbarAlign).toBe("center");
    expect(config.layout.showMetadata).toBe(false);
    expect(config.layout.showRowMetadata).toBe(true);
    expect(config.layout.showPreviewPane).toBe(false);
    expect(config.layout.showFullPreviewMetadata).toBe(true);
    expect(config.layout.showPreviewGutter).toBe(false);
    expect(config.layout.showFullPreviewGutter).toBe(false);
    expect(config.layout.highlightSearchMatches).toBe(false);
    expect(config.layout.showEmptyStateHelp).toBe(false);
    expect(config.layout.imagePreviewNoticeVisibility).toBe("always");
    expect(config.layout.imagePreviewNoticeSpacing).toBe(4);
    expect(config.layout.overlayPlacement).toBe("top");
    expect(config.chrome.panelBorder).toBe(false);
    expect(config.chrome.overlayBorder).toBe(false);
    expect(config.chrome.headerBorder).toBe(true);
    expect(config.chrome.listBorder).toBe(false);
    expect(config.chrome.previewBorder).toBe(true);
    expect(config.chrome.fullPreviewBorder).toBe(false);
    expect(config.chrome.statusBorder).toBe(true);
    expect(config.chrome.searchOverlayBorder).toBe(true);
    expect(config.chrome.dangerOverlayBorder).toBe(false);
    expect(config.chrome.helpOverlayBorder).toBe(true);
    expect(config.chrome.showPanelTitles).toBe(false);
    expect(config.chrome.showOverlayTitles).toBe(false);
    expect(config.chrome.showHeaderTitle).toBe(true);
    expect(config.chrome.showListTitle).toBe(false);
    expect(config.chrome.showPreviewTitle).toBe(true);
    expect(config.chrome.showFullPreviewTitle).toBe(false);
    expect(config.chrome.showStatusTitle).toBe(true);
    expect(config.chrome.showSearchOverlayTitle).toBe(true);
    expect(config.chrome.showDangerOverlayTitle).toBe(false);
    expect(config.chrome.showHelpOverlayTitle).toBe(true);
    expect(config.chrome.showListPositionTitle).toBe(false);
    expect(config.chrome.showPreviewEntryTitle).toBe(false);
    expect(config.chrome.showFullPreviewBottomTitle).toBe(false);
    expect(config.chrome.panelBorderStyle).toBe("double");
    expect(config.chrome.headerBorderStyle).toBe("single");
    expect(config.chrome.listBorderStyle).toBe("double");
    expect(config.chrome.previewBorderStyle).toBe("heavy");
    expect(config.chrome.fullPreviewBorderStyle).toBe("rounded");
    expect(config.chrome.statusBorderStyle).toBe("single");
    expect(config.chrome.searchOverlayBorderStyle).toBe("single");
    expect(config.chrome.dangerOverlayBorderStyle).toBe("heavy");
    expect(config.chrome.helpOverlayBorderStyle).toBe("double");
    expect(config.chrome.panelTitleAlignment).toBe("center");
    expect(config.chrome.panelBottomTitleAlignment).toBe("right");
    expect(config.chrome.overlayTitleAlignment).toBe("right");
    expect(config.chrome.statusTitleAlignment).toBe("center");
    expect(config.chrome.selectedMarker).toBe(">>");
    expect(config.chrome.selectedMarkedMarker).toBe("**");
    expect(config.chrome.scrollbarThumb).toBe("█");
    expect(surface(config, "shell").bg).toBe("#000102");
    expect(surface(config, "splitPaneGap").bg).toBe("#020304");
    expect(surface(config, "header").bg).toBe("#010203");
    expect(surface(config, "header").fg).toBe("#abcdef");
    expect(surface(config, "header").search).toBe("#222222");
    expect(surface(config, "header").favorite).toBe("#333333");
    expect(surface(config, "header").secondary).toBe("#444444");
    expect(surface(config, "header").bold).toBe(true);
    expect(surface(config, "header").underline).toBe(false);
    expect(surface(config, "list").image).toBe("#112255");
    expect(surface(config, "list").favorite).toBe("#775511");
    expect(surface(config, "rowSpacer").bg).toBe("#0f1011");
    expect(surface(config, "alternateRow").bg).toBe("#101820");
    expect(surface(config, "alternateRow").fg).toBe("#f0f4f8");
    expect(surface(config, "alternateRow").image).toBe(config.theme.accentImage);
    expect(surface(config, "selectedMarkedRow").bg).toBe("#202c34");
    expect(surface(config, "selectedMarkedRow").accent).toBe("#f0c050");
    expect(surface(config, "selectedMarkedRow").bold).toBe(true);
    expect(surface(config, "emptyState").fg).toBe("#445566");
    expect(surface(config, "emptyState").image).toBe("#112255");
    expect(surface(config, "preview").muted).toBe("#223344");
    expect(surface(config, "previewGutter").muted).toBe("#334455");
    expect(surface(config, "previewGutter").fg).toBe(surface(config, "preview").fg);
    expect(surface(config, "previewSpacer").bg).toBe("#141516");
    expect(surface(config, "fullPreview").muted).toBe("#445533");
    expect(surface(config, "fullPreviewGutter").muted).toBe("#556644");
    expect(surface(config, "fullPreviewGutter").border).toBe(surface(config, "fullPreview").border);
    expect(surface(config, "fullPreviewMeta").bg).toBe("#121314");
    expect(surface(config, "fullPreviewMeta").accent).toBe("#234567");
    expect(surface(config, "fullPreviewMeta").dim).toBe(true);
    expect(surface(config, "fullPreviewSpacer").bg).toBe("#171819");
    expect(surface(config, "overlay").error).toBe("#551111");
    expect(surface(config, "overlay").search).toBe("#552255");
    expect(surface(config, "overlay").inverse).toBe(true);
    expect(surface(config, "searchOverlay").search).toBe("#335577");
    expect(surface(config, "searchOverlay").inverse).toBe(true);
    expect(surface(config, "dangerOverlay").error).toBe("#773333");
    expect(surface(config, "dangerOverlay").search).toBe("#552255");
    expect(surface(config, "helpOverlay").accent).toBe("#337755");
    expect(surface(config, "status").bg).toBe("#f6f7f4");
    expect(surface(config, "status").accent).toBe("#111111");
    expect(surface(config, "status").success).toBe("#225522");
    expect(surface(config, "status").warning).toBe("#665522");
    expect(surface(config, "status").error).toBe("#552222");
    expect(surface(config, "status").dim).toBe(true);
    expect(config.labels.brand).toBe("DX");
    expect(config.labels.statusTitle).toBe("state");
    expect(config.labels.selectedCountTemplate).toBe("{count} {prefix}");
    expect(config.labels.watcherRunning).toBe("daemon live");
    expect(config.labels.watcherErrorSeparator).toBe(" -> ");
    expect(config.labels.watcherRunningTemplate).toBe("{age} / {status}");
    expect(config.labels.watcherErrorTemplate).toBe("{error} / {status}");
    expect(config.labels.kindText).toBe("TEXT");
    expect(config.labels.rowPinnedLabel).toBe("SV");
    expect(config.labels.rowMetaTemplate).toBe("{kind}|{age}|{size}|{pinnedSlot}");
    expect(config.labels.rowPinnedSlotTemplate).toBe("[{pinned}]");
    expect(config.labels.rowUnpinnedSlotTemplate).toBe("[ ]");
    expect(config.labels.entryIdPrefix).toBe("id:");
    expect(config.labels.listPositionTemplate).toBe("{index} of {total}");
    expect(config.labels.filterImages).toBe("PICS");
    expect(config.labels.clearKindImages).toBe("pictures");
    expect(config.labels.queryEmpty).toBe("none");
    expect(config.labels.searchPrompt).toBe("find> ");
    expect(config.labels.searchCursor).toBe("_");
    expect(config.labels.searchInputTemplate).toBe("{prompt}[{query}]{cursor}");
    expect(config.labels.confirmHintTemplate).toBe("confirm => {hint}");
    expect(config.labels.deleteOneTemplate).toBe("one: {message}");
    expect(config.labels.deleteManyTemplate).toBe("{count}x {message}");
    expect(config.labels.clearPromptTemplate).toBe("{kind} <- {prefix}");
    expect(config.labels.headerSectionSeparator).toBe(" / ");
    expect(config.labels.headerLabelSeparator).toBe("=");
    expect(config.labels.headerLineTemplate).toBe("{query} <- {brand} [{mode}]");
    expect(config.labels.previewGutterSeparator).toBe(" | ");
    expect(config.labels.fullPreviewGutterSeparator).toBe(" >> ");
    expect(config.labels.previewMetaSeparator).toBe(" :: ");
    expect(config.labels.previewMetaLabelSeparator).toBe("=");
    expect(config.labels.previewPinnedSuffixTemplate).toBe("{separator}saved:{pinned}");
    expect(config.labels.previewEntryTitleTemplate).toBe("clip {id}");
    expect(config.labels.previewMetaHeaderTemplate).toBe("{kind}/{id}/{hash}");
    expect(config.labels.previewMetaDetailsTemplate).toBe("{size}/{mime}{pinnedSuffix}");
    expect(config.labels.fullPreviewMetaTemplate).toBe("{kind}/{id}/{mime}{pinnedSuffix}");
    expect(config.labels.fullPreviewMetaHeaderTemplate).toBe("full:{kind}/{id}/{mime}");
    expect(config.labels.fullPreviewMetaDetailsTemplate).toBe("{size}/{hashShort}{pinnedSuffix}");
    expect(config.labels.fullPreviewBottomTitleTemplate).toBe("{id}:{start}-{end}/{total}{separator}{back}");
    expect(config.labels.previewTextGutterTemplate).toBe("L{line}");
    expect(config.labels.imagePreviewFallbackSeparator).toBe(" -> ");
    expect(config.labels.splitImagePreviewFallbackPrefix).toBe("split-img");
    expect(config.labels.splitImagePreviewFallbackSeparator).toBe(" <- ");
    expect(config.labels.fullImagePreviewFallbackPrefix).toBe("full-img");
    expect(config.labels.fullImagePreviewFallbackSeparator).toBe(" => ");
    expect(config.labels.imagePreviewDecodePending).toBe("loading pixels");
    expect(config.labels.imagePreviewSourceTemplate).toBe("SRC {source}");
    expect(config.labels.splitImagePreviewSourceTemplate).toBe("SPLIT {source}");
    expect(config.labels.fullImagePreviewSourceTemplate).toBe("FULL {source}");
    expect(config.labels.imagePreviewBlocksSource).toBe("ansi blocks");
    expect(config.labels.imagePreviewKittyFallbackSource).toBe("kitty ansi fallback");
    expect(config.labels.imagePreviewSixelFallbackSource).toBe("sixel ansi fallback");
    expect(config.labels.imagePreviewKittyProtocolName).toBe("KITTY");
    expect(config.labels.imagePreviewSixelProtocolName).toBe("SIXEL");
    expect(config.labels.errorPasteBackFailed).toBe("paste transport failed");
    expect(config.labels.errorUnknownStatus).toBe("unknown backend exit");
    expect(config.labels.errorProcessTemplate).toBe("process {method}: {message}");
    expect(config.labels.errorRpcTemplate).toBe("rpc {method}: {message}");
    expect(config.labels.statusPinnedView).toBe("SAVED");
    expect(config.labels.statusPinnedPrefix).toBe("saved");
    expect(config.labels.statusUnpinnedPrefix).toBe("unsaved");
    expect(config.labels.statusPinTemplate).toBe("{prefix}:{entryIdPrefix}{id}");
    expect(config.labels.statusClearedTemplate).toBe("{prefix}: {count} ({pinned})");
    expect(config.labels.statusEntriesTemplate).toBe("{entries}: {count}");
    expect(config.labels.imagePreviewUnsupportedMime).toBe("cannot draw {mime}");
    expect(config.labels.keyAlternativeSeparator).toBe(" | ");
    expect(config.labels.keyGroupSeparator).toBe(" + ");
    expect(config.labels.statusHintSeparator).toBe(" :: ");
    expect(config.labels.statusHintTemplate).toBe("{searchKeys}:{search}{separator}{pasteKeys}:{paste}{separator}{helpKeys}:{help}");
    expect(config.labels.statusLineTemplate).toBe("{operation}{separator}{watcher}{separator}{hint}");
    expect(config.labels.statusPasteHint).toBe("send");
    expect(config.labels.helpSearchCopyMatches).toBe("yank visible matches");
    expect(config.labels.sizeBytesUnit).toBe("bytes");
    expect(config.labels.sizeKibUnit).toBe("KB");
    expect(config.labels.sizeMibUnit).toBe("MB");
    expect(config.labels.ageSecondsUnit).toBe("sec");
    expect(config.labels.ageMinutesUnit).toBe("min");
    expect(config.labels.ageHoursUnit).toBe("hr");
    expect(config.labels.ageDaysUnit).toBe("day");
    expect(config.labels.textTruncationMarker).toBe("~");
    expect(config.labels.textWhitespaceReplacement).toBe("_");
    expect(config.keyBindings.quit).toEqual(["q", "ctrl+q"]);
    expect(config.keyBindings.copyPaste).toEqual(["enter", "ctrl+p"]);
    expect(config.keyBindings.searchCopyMatches).toEqual(["ctrl+g"]);
    expect(config.keyLabels.space).toBe("SPC");
    expect(config.keyLabels.escape).toBe("ESC");
    expect(config.keyLabels.enter).toBe("RET");
    expect(config.keyLabels.pagedown).toBe("PGDN");
    expect(config.statusTones.success).toEqual(["stored"]);
    expect(config.statusTones.warning).toEqual(["waiting"]);
    expect(config.statusTones.error).toEqual(["boom"]);
    expect(config.headerLineTones).toEqual({
      brand: "success",
      filter: "warning",
      query: "search",
      mode: "secondary",
      filterLabel: "accent",
      queryLabel: "image",
      modeLabel: "favorite",
      sectionSeparator: "fg",
      labelSeparator: "muted",
    });
    expect(config.statusLineTones).toEqual({
      operation: "success",
      watcher: "warning",
      hint: "secondary",
      separator: "accent",
    });
    expect(config.overlayBorderTones).toEqual({
      search: "accent",
      danger: "warning",
      command: "success",
    });
    expect(config.overlayContentTones).toEqual({
      searchInput: "accent",
      searchPrompt: "muted",
      searchQuery: "search",
      searchCursor: "warning",
      deletePrompt: "warning",
      deleteWarning: "secondary",
      confirmHint: "fg",
      clearPrompt: "image",
      clearSafeHint: "favorite",
      clearUnsafeHint: "error",
      helpKey: "success",
      helpAction: "muted",
    });
    expect(config.listContentTones).toEqual({
      marker: "success",
      markerGap: "muted",
      metadata: "warning",
      metadataGap: "secondary",
      preview: "fg",
      searchMatch: "favorite",
      emptyTitle: "image",
      emptyHelp: "accent",
      scrollbarThumb: "error",
      scrollbarTrack: "border",
    });
    expect(config.previewContentTones).toEqual({
      splitBorder: "success",
      splitEmptyTitle: "warning",
      splitEmptyHelp: "secondary",
      splitImageFallbackPrefix: "accent",
      splitImageFallbackSeparator: "muted",
      splitImageFallbackReason: "error",
      splitImageNotice: "muted",
      splitGutter: "image",
      splitGutterSeparator: "favorite",
      splitPrimary: "fg",
      splitSecondary: "secondary",
      splitMuted: "muted",
      splitAccent: "accent",
      splitError: "error",
      splitSuccess: "success",
      splitMetaHeader: "warning",
      splitMetaDetails: "border",
      fullBorder: "image",
      fullMeta: "favorite",
      fullMetaHeader: "accent",
      fullMetaDetails: "secondary",
      fullEmptyTitle: "warning",
      fullEmptyHelp: "secondary",
      fullImageFallbackPrefix: "accent",
      fullImageFallbackSeparator: "muted",
      fullImageFallbackReason: "error",
      fullImageNotice: "muted",
      fullGutter: "image",
      fullGutterSeparator: "favorite",
      fullPrimary: "fg",
      fullSecondary: "secondary",
      fullMuted: "muted",
      fullAccent: "accent",
      fullError: "error",
      fullSuccess: "success",
    });
    expect(config.filterOrder).toEqual(["images", "all", "favorites"]);
    expect(config.helpOrder).toEqual(["paste", "searchCopyMatches", "preview"]);
    expect(config.startup).toEqual({ filter: "today", pinnedOnly: true, query: "needle" });
    expect(config.behavior).toEqual({
      liveSearch: false,
      liveSearchDebounceMs: 5000,
      clearQueryOnSearchOpen: false,
      restoreQueryOnSearchCancel: false,
      exitAfterPaste: false,
      exitAfterCopy: true,
      exitAfterBulkCopy: true,
      exitAfterSearchCopy: true,
    });
  });

  test("falls back to default filter order when configured filters are unusable", () => {
    expect(resolveTuiConfig({ filterOrder: ["bad", "worse"] }).filterOrder).toEqual(["all", "text", "images", "favorites", "today"]);
  });

  test("keeps legacy full-preview metadata template and tone overrides as fallbacks", () => {
    const config = resolveTuiConfig({
      labels: {
        fullPreviewMetaTemplate: "legacy-meta-{id}",
      },
      previewContentTones: {
        fullMeta: "favorite",
      },
    });

    expect(config.labels.fullPreviewMetaHeaderTemplate).toBe("legacy-meta-{id}");
    expect(config.previewContentTones.fullMetaHeader).toBe("favorite");
    expect(config.previewContentTones.fullMetaDetails).toBe("favorite");
  });

  test("falls back to default help order when configured help actions are unusable", () => {
    const config = resolveTuiConfig({ helpOrder: ["bad", "worse"] });

    expect(config.helpOrder).toEqual(resolveTuiConfig().helpOrder);
  });

  test("falls back to default image preview fields when configured fields are unusable", () => {
    const config = resolveTuiConfig({ layout: { previewImageFields: ["bad", "worse"] as never } });

    expect(config.layout.previewImageFields).toEqual(resolveTuiConfig().layout.previewImageFields);
  });

  test("falls back to auto image preview background when configured color cannot be rendered", () => {
    const config = resolveTuiConfig({ layout: { imagePreviewBackground: "blue" } });

    expect(config.layout.imagePreviewBackground).toBe("auto");
  });

  test("falls back to inherited image preview alignment when configured values are unusable", () => {
    const inherited = resolveTuiConfig({ layout: { emptyStateContentAlign: "center" } });
    expect(inherited.layout.emptyStateTitleAlign).toBe("center");
    expect(inherited.layout.emptyStateHelpAlign).toBe("center");

    const previewInherited = resolveTuiConfig({ layout: { previewContentAlign: "center", fullPreviewContentAlign: "right" } });
    expect(previewInherited.layout.fullPreviewContentAlign).toBe("right");
    expect(previewInherited.layout.previewMetaContentAlign).toBe("center");
    expect(previewInherited.layout.fullPreviewMetaContentAlign).toBe("right");

    const previewMetaInherited = resolveTuiConfig({ layout: { previewMetaVerticalAlign: "center" } });
    expect(previewMetaInherited.layout.previewMetaVerticalAlign).toBe("center");
    expect(previewMetaInherited.layout.fullPreviewMetaVerticalAlign).toBe("center");

    const previewBodyInherited = resolveTuiConfig({ layout: { previewBodyVerticalAlign: "bottom" } });
    expect(previewBodyInherited.layout.previewBodyVerticalAlign).toBe("bottom");
    expect(previewBodyInherited.layout.fullPreviewBodyVerticalAlign).toBe("bottom");

    const previewGutterInherited = resolveTuiConfig({ layout: { previewGutterAlign: "center" } });
    expect(previewGutterInherited.layout.previewGutterAlign).toBe("center");
    expect(previewGutterInherited.layout.fullPreviewGutterAlign).toBe("center");

    const metaHashInherited = resolveTuiConfig({ layout: { previewMetaHashLength: 9 } });
    expect(metaHashInherited.layout.previewMetaHashLength).toBe(9);
    expect(metaHashInherited.layout.fullPreviewMetaHashLength).toBe(9);

    const config = resolveTuiConfig({
      layout: {
        imagePreviewAlign: "middle" as never,
        fullPreviewImageAlign: "bottom" as never,
        previewGutterAlign: "middle" as never,
        fullPreviewGutterAlign: "bottom" as never,
        previewContentAlign: "middle" as never,
        fullPreviewContentAlign: "bottom" as never,
        previewBodyVerticalAlign: "left" as never,
        fullPreviewBodyVerticalAlign: "right" as never,
        previewMetaContentAlign: "start" as never,
        fullPreviewMetaContentAlign: "end" as never,
        previewMetaVerticalAlign: "left" as never,
        fullPreviewMetaVerticalAlign: "right" as never,
        scrollbarPlacement: "middle" as never,
        scrollbarAlign: "middle" as never,
        rowContentAlign: "middle" as never,
        rowMetadataAlign: "start" as never,
        rowPreviewAlign: "end" as never,
        rowAgeAlign: "start" as never,
        rowSizeAlign: "middle" as never,
        rowPinnedAlign: "end" as never,
        rowMarkerAlign: "middle" as never,
        headerContentAlign: "start" as never,
        statusContentAlign: "end" as never,
        headerVerticalAlign: "left" as never,
        statusVerticalAlign: "right" as never,
        overlayContentAlign: "middle" as never,
        overlayVerticalAlign: "middle" as never,
        searchOverlayContentAlign: "start" as never,
        searchOverlayVerticalAlign: "under" as never,
        dangerOverlayContentAlign: "end" as never,
        dangerOverlayVerticalAlign: "over" as never,
        helpOverlayContentAlign: "bottom" as never,
        helpOverlayVerticalAlign: "left" as never,
        emptyStateContentAlign: "middle" as never,
        emptyStateTitleAlign: "end" as never,
        emptyStateHelpAlign: "start" as never,
        emptyStateVerticalAlign: "middle" as never,
        helpKeyAlign: "middle" as never,
      },
    });

    expect(config.layout.imagePreviewAlign).toBe("left");
    expect(config.layout.fullPreviewImageAlign).toBe("left");
    expect(config.layout.previewGutterAlign).toBe("right");
    expect(config.layout.fullPreviewGutterAlign).toBe("right");
    expect(config.layout.previewContentAlign).toBe("left");
    expect(config.layout.fullPreviewContentAlign).toBe("left");
    expect(config.layout.previewBodyVerticalAlign).toBe("top");
    expect(config.layout.fullPreviewBodyVerticalAlign).toBe("top");
    expect(config.layout.previewMetaContentAlign).toBe("left");
    expect(config.layout.fullPreviewMetaContentAlign).toBe("left");
    expect(config.layout.previewMetaVerticalAlign).toBe("top");
    expect(config.layout.fullPreviewMetaVerticalAlign).toBe("top");
    expect(config.layout.scrollbarPlacement).toBe("right");
    expect(config.layout.scrollbarAlign).toBe("left");
    expect(config.layout.rowContentAlign).toBe("left");
    expect(config.layout.rowMetadataAlign).toBe("left");
    expect(config.layout.rowPreviewAlign).toBe("left");
    expect(config.layout.rowAgeAlign).toBe("right");
    expect(config.layout.rowSizeAlign).toBe("right");
    expect(config.layout.rowPinnedAlign).toBe("left");
    expect(config.layout.rowMarkerAlign).toBe("left");
    expect(config.layout.headerContentAlign).toBe("left");
    expect(config.layout.statusContentAlign).toBe("left");
    expect(config.layout.headerVerticalAlign).toBe("top");
    expect(config.layout.statusVerticalAlign).toBe("top");
    expect(config.layout.overlayContentAlign).toBe("left");
    expect(config.layout.overlayVerticalAlign).toBe("top");
    expect(config.layout.searchOverlayContentAlign).toBe("left");
    expect(config.layout.searchOverlayVerticalAlign).toBe("top");
    expect(config.layout.dangerOverlayContentAlign).toBe("left");
    expect(config.layout.dangerOverlayVerticalAlign).toBe("top");
    expect(config.layout.helpOverlayContentAlign).toBe("left");
    expect(config.layout.helpOverlayVerticalAlign).toBe("top");
    expect(config.layout.emptyStateContentAlign).toBe("left");
    expect(config.layout.emptyStateTitleAlign).toBe("left");
    expect(config.layout.emptyStateHelpAlign).toBe("left");
    expect(config.layout.emptyStateVerticalAlign).toBe("top");
    expect(config.layout.helpKeyAlign).toBe("left");
  });

  test("derives help rows from configured help order", () => {
    const config = resolveTuiConfig({
      helpOrder: ["searchCopyMatches", "paste"],
      labels: {
        helpPaste: "send",
        helpSearchCopyMatches: "copy matches",
      },
      keyBindings: {
        copyPaste: "enter",
        searchCopyMatches: "ctrl+g",
      },
    });

    expect(helpRows(config)).toEqual([
      { keys: "ctrl+g", action: "copy matches" },
      { keys: "enter", action: "send" },
    ]);
  });

  test("derives opt-in help and status hints for every non-default action group", () => {
    const config = resolveTuiConfig({
      helpOrder: ["quit", "previewNavigation", "previewBack", "output", "searchEdit", "confirmChoice"],
      labels: {
        statusHintTemplate:
          "{filterKeys}:{filter}|{pinnedKeys}:{pinned}|{deleteKeys}:{delete}|{outputKeys}:{output}|{quitKeys}:{quit}",
        statusSearchModeHintTemplate: "{applyKeys}:{apply}|{backspaceKeys}:{backspace}|{searchCopyKeys}:{searchCopy}|{cancelKeys}:{cancel}",
        statusPreviewModeHintTemplate: "{previewBackKeys}:{previewBack}|{previewScrollKeys}:{previewScroll}",
        statusConfirmModeHintTemplate: "{confirmYesKeys}:{confirmYes}|{confirmNoKeys}:{confirmNo}",
        statusFilterHint: "cycle",
        statusPinnedHint: "saved",
        statusDeleteHint: "remove",
        statusOutputHint: "print",
        statusQuitHint: "exit",
        statusApplyHint: "go",
        statusBackspaceHint: "erase",
        statusSearchCopyHint: "yank",
        statusCancelHint: "stop",
        statusPreviewBackHint: "return",
        statusPreviewScrollHint: "move",
        statusConfirmYesHint: "commit",
        statusConfirmNoHint: "abort",
        helpQuit: "leave picker",
        helpPreviewNavigation: "move preview",
        helpPreviewBack: "close preview",
        helpOutput: "print selection",
        helpSearchEdit: "edit search",
        helpConfirmChoice: "answer dialog",
      },
      keyBindings: {
        quit: "q",
        forceQuit: "ctrl+c",
        previewUp: "k",
        previewDown: "j",
        previewPageUp: "u",
        previewPageDown: "d",
        previewBack: "esc",
        output: "o",
        searchBackspace: "bs",
        searchApply: "ret",
        searchCancel: "esc",
        confirmYes: "yes",
        confirmNo: "no",
      },
    });

    expect(statusHint(config)).toBe("tab:cycle|shift+tab:saved|d / bksp:remove|o:print|q  ctrl+c:exit");
    expect(statusHint(config, "search")).toBe("ret:go|bs:erase|ctrl+s:yank|esc:stop");
    expect(statusHint(config, "preview")).toBe("esc:return|k  j  u  d:move");
    expect(statusHint(config, "confirm")).toBe("yes:commit|no:abort");
    expect(helpRows(config)).toEqual([
      { keys: "q  ctrl+c", action: "leave picker" },
      { keys: "k  j  u  d", action: "move preview" },
      { keys: "esc", action: "close preview" },
      { keys: "o", action: "print selection" },
      { keys: "bs  ret  esc", action: "edit search" },
      { keys: "yes  no", action: "answer dialog" },
    ]);
  });

  test("falls back to default startup filter when configured filter is unusable", () => {
    expect(resolveTuiConfig({ startup: { filter: "bad" as never, pinnedOnly: true, query: "kept" } }).startup).toEqual({
      filter: "all",
      pinnedOnly: true,
      query: "kept",
    });
  });

  test("accepts preview split width when list width is not configured", () => {
    const fileConfig = resolveTuiConfig({ layout: { previewWidthPercent: 62 } });
    expect(fileConfig.layout.listWidthPercent).toBe(38);
    expect(fileConfig.layout.previewWidthPercent).toBe(62);

    const envConfig = resolveTuiConfig({ layout: { listWidthPercent: 40 } }, { DITOX_TUI_PREVIEW_WIDTH: "55" });
    expect(envConfig.layout.listWidthPercent).toBe(45);
    expect(envConfig.layout.previewWidthPercent).toBe(55);

    const listEnvWins = resolveTuiConfig(
      { layout: { previewWidthPercent: 55 } },
      { DITOX_TUI_PREVIEW_WIDTH: "60", DITOX_TUI_LIST_WIDTH: "42" },
    );
    expect(listEnvWins.layout.listWidthPercent).toBe(42);
    expect(listEnvWins.layout.previewWidthPercent).toBe(58);
  });

  test("accepts empty chrome glyphs for markerless and separatorless layouts", () => {
    const config = resolveTuiConfig({
      chrome: {
        selectedMarker: "",
        selectedMarkedMarker: "",
        markedMarker: "",
        normalMarker: "",
        scrollbarThumb: "",
        scrollbarTrack: "",
        statusSeparator: "",
      },
    });

    expect(config.chrome.selectedMarker).toBe("");
    expect(config.chrome.selectedMarkedMarker).toBe("");
    expect(config.chrome.markedMarker).toBe("");
    expect(config.chrome.normalMarker).toBe("");
    expect(config.chrome.scrollbarThumb).toBe("");
    expect(config.chrome.scrollbarTrack).toBe("");
    expect(config.chrome.statusSeparator).toBe("");
  });

  test("keeps default browse-mode keybindings conflict free", () => {
    const config = resolveTuiConfig();
    const browseActions = [
      "forceQuit",
      "quit",
      "up",
      "down",
      "pageUp",
      "pageDown",
      "home",
      "end",
      "nextFilter",
      "search",
      "searchCopyMatches",
      "selectToggle",
      "selectSingle",
      "clearSelection",
      "selectUp",
      "selectDown",
      "toggleFavorite",
      "delete",
      "copyPaste",
      "copyOnly",
      "bulkCopy",
      "output",
      "help",
      "preview",
      "togglePinnedView",
      "clearAll",
      "clearText",
      "clearImages",
      "clearAllIncludingPinned",
    ] as const;
    const seen = new Map<string, string>();

    for (const action of browseActions) {
      for (const key of config.keyBindings[action]) {
        expect(seen.get(key), `${key} is shared by ${seen.get(key)} and ${action}`).toBeUndefined();
        seen.set(key, action);
      }
    }

    expect(seen.get("space")).toBe("preview");
    expect(seen.get("x")).toBe("selectToggle");
  });

  test("environment overrides file theme and layout", () => {
    const config = resolveTuiConfig(
      { theme: "ditoxLight", layout: { listWidthPercent: 40 } },
      {
        DITOX_TUI_THEME: "ditoxDark",
        DITOX_TUI_LIST_WIDTH: "60",
        DITOX_TUI_HISTORY_LIMIT: "42",
        DITOX_TUI_SCROLLBAR: "false",
        DITOX_TUI_REFRESH_MS: "0",
        DITOX_TUI_MOUSE: "false",
        DITOX_TUI_MOUSE_SCROLL_ROWS: "5",
        DITOX_TUI_SCROLLBAR_WIDTH: "2",
        DITOX_TUI_SCROLLBAR_PLACEMENT: "left",
        DITOX_TUI_SCROLLBAR_ALIGN: "right",
        DITOX_TUI_SHELL_PADDING_X: "2",
        DITOX_TUI_SHELL_PADDING_Y: "1",
        DITOX_TUI_IMAGE_PREVIEW: "metadata",
        DITOX_TUI_FULL_PREVIEW_IMAGE_MODE: "kitty",
        DITOX_TUI_IMAGE_PREVIEW_MAX_WIDTH: "24",
        DITOX_TUI_IMAGE_PREVIEW_MAX_ROWS: "8",
        DITOX_TUI_FULL_PREVIEW_IMAGE_MAX_WIDTH: "32",
        DITOX_TUI_FULL_PREVIEW_IMAGE_MAX_ROWS: "6",
        DITOX_TUI_IMAGE_RENDERER: "text",
        DITOX_TUI_FULL_PREVIEW_IMAGE_RENDERER: "opentui",
        DITOX_TUI_IMAGE_ALIGN: "center",
        DITOX_TUI_FULL_PREVIEW_IMAGE_ALIGN: "right",
        DITOX_TUI_IMAGE_BLOCK_GLYPH: "█",
        DITOX_TUI_FULL_PREVIEW_IMAGE_BLOCK_GLYPH: "▓",
        DITOX_TUI_IMAGE_BACKGROUND: "#223344",
        DITOX_TUI_FULL_PREVIEW_IMAGE_BACKGROUND: "#445566",
        DITOX_TUI_FULL_PREVIEW_IMAGE_NOTICE: "always",
        DITOX_TUI_IMAGE_NOTICE_SPACING: "1",
        DITOX_TUI_FULL_PREVIEW_IMAGE_NOTICE_SPACING: "2",
        DITOX_TUI_SPLIT_PANE_GAP: "3",
        DITOX_TUI_FULL_PREVIEW_TEXT_WIDTH_INSET: "5",
        DITOX_TUI_PREVIEW_CONTENT_ALIGN: "center",
        DITOX_TUI_FULL_PREVIEW_CONTENT_ALIGN: "right",
        DITOX_TUI_PREVIEW_BODY_VERTICAL_ALIGN: "center",
        DITOX_TUI_FULL_PREVIEW_BODY_VERTICAL_ALIGN: "bottom",
        DITOX_TUI_FULL_PREVIEW_IMAGE_ROW_INSET: "4",
        DITOX_TUI_FULL_PREVIEW_GUTTER_WIDTH: "2",
        DITOX_TUI_PREVIEW_GUTTER_ALIGN: "center",
        DITOX_TUI_FULL_PREVIEW_GUTTER_ALIGN: "left",
        DITOX_TUI_PREVIEW_META_HEIGHT: "1",
        DITOX_TUI_FULL_PREVIEW_META_HEIGHT: "2",
        DITOX_TUI_PREVIEW_META_LINE_SPACING: "1",
        DITOX_TUI_FULL_PREVIEW_META_LINE_SPACING: "2",
        DITOX_TUI_PREVIEW_META_HASH_LENGTH: "9",
        DITOX_TUI_FULL_PREVIEW_META_HASH_LENGTH: "10",
        DITOX_TUI_FULL_PREVIEW_META_PADDING_X: "2",
        DITOX_TUI_PREVIEW_META_CONTENT_ALIGN: "right",
        DITOX_TUI_FULL_PREVIEW_META_CONTENT_ALIGN: "center",
        DITOX_TUI_PREVIEW_META_VERTICAL_ALIGN: "center",
        DITOX_TUI_FULL_PREVIEW_META_VERTICAL_ALIGN: "bottom",
        DITOX_TUI_HEADER_CONTENT_ALIGN: "center",
        DITOX_TUI_STATUS_CONTENT_ALIGN: "right",
        DITOX_TUI_HEADER_VERTICAL_ALIGN: "bottom",
        DITOX_TUI_STATUS_VERTICAL_ALIGN: "center",
        DITOX_TUI_STATUS_SEPARATOR_PADDING: "1",
        DITOX_TUI_STATUS_SEPARATOR_PADDING_LEFT: "2",
        DITOX_TUI_STATUS_SEPARATOR_PADDING_RIGHT: "3",
        DITOX_TUI_TITLE_PADDING: "1",
        DITOX_TUI_TITLE_PADDING_LEFT: "2",
        DITOX_TUI_TITLE_PADDING_RIGHT: "3",
        DITOX_TUI_OVERLAY_CONTENT_ALIGN: "center",
        DITOX_TUI_OVERLAY_VERTICAL_ALIGN: "center",
        DITOX_TUI_OVERLAY_LINE_SPACING: "1",
        DITOX_TUI_SEARCH_OVERLAY_CONTENT_ALIGN: "right",
        DITOX_TUI_SEARCH_OVERLAY_VERTICAL_ALIGN: "bottom",
        DITOX_TUI_SEARCH_OVERLAY_LINE_SPACING: "2",
        DITOX_TUI_DANGER_OVERLAY_CONTENT_ALIGN: "left",
        DITOX_TUI_DANGER_OVERLAY_VERTICAL_ALIGN: "top",
        DITOX_TUI_DANGER_OVERLAY_LINE_SPACING: "3",
        DITOX_TUI_HELP_OVERLAY_CONTENT_ALIGN: "center",
        DITOX_TUI_HELP_OVERLAY_VERTICAL_ALIGN: "bottom",
        DITOX_TUI_HELP_OVERLAY_LINE_SPACING: "2",
        DITOX_TUI_HELP_KEY_ALIGN: "right",
        DITOX_TUI_EMPTY_STATE_ALIGN: "center",
        DITOX_TUI_EMPTY_STATE_TITLE_ALIGN: "right",
        DITOX_TUI_EMPTY_STATE_HELP_ALIGN: "left",
        DITOX_TUI_EMPTY_STATE_VERTICAL_ALIGN: "bottom",
        DITOX_TUI_EMPTY_STATE_LINE_SPACING: "2",
        DITOX_TUI_ROW_CONTENT_ALIGN: "center",
        DITOX_TUI_ROW_METADATA_ALIGN: "right",
        DITOX_TUI_ROW_PREVIEW_ALIGN: "center",
        DITOX_TUI_ROW_AGE_ALIGN: "left",
        DITOX_TUI_ROW_SIZE_ALIGN: "center",
        DITOX_TUI_ROW_PINNED_ALIGN: "right",
        DITOX_TUI_ROW_MARKER_WIDTH: "4",
        DITOX_TUI_ROW_MARKER_ALIGN: "right",
        DITOX_TUI_HEADER: "false",
        DITOX_TUI_STATUS_LINE: "false",
        DITOX_TUI_ROW_METADATA: "false",
        DITOX_TUI_PREVIEW_PANE: "false",
        DITOX_TUI_FULL_PREVIEW_METADATA: "false",
        DITOX_TUI_PREVIEW_GUTTER: "false",
        DITOX_TUI_FULL_PREVIEW_GUTTER: "false",
        DITOX_TUI_SEARCH_HIGHLIGHT: "false",
        DITOX_TUI_EMPTY_HELP: "false",
        DITOX_TUI_ALTERNATE_ROWS: "false",
        DITOX_TUI_STARTUP_FILTER: "images",
        DITOX_TUI_STARTUP_PINNED: "true",
        DITOX_TUI_STARTUP_QUERY: "logo",
        DITOX_TUI_LIVE_SEARCH: "false",
        DITOX_TUI_LIVE_SEARCH_DEBOUNCE_MS: "25",
        DITOX_TUI_CLEAR_QUERY_ON_SEARCH_OPEN: "false",
        DITOX_TUI_RESTORE_QUERY_ON_SEARCH_CANCEL: "false",
        DITOX_TUI_EXIT_AFTER_PASTE: "false",
        DITOX_TUI_EXIT_AFTER_COPY: "true",
        DITOX_TUI_EXIT_AFTER_BULK_COPY: "true",
        DITOX_TUI_EXIT_AFTER_SEARCH_COPY: "true",
      },
      null,
    );
    expect(config.theme.name).toBe("ditoxDark");
    expect(config.layout.listWidthPercent).toBe(60);
    expect(config.layout.historyLimit).toBe(42);
    expect(config.layout.imagePreviewMode).toBe("metadata");
    expect(config.layout.fullPreviewImageMode).toBe("kitty");
    expect(config.layout.imagePreviewMaxWidth).toBe(24);
    expect(config.layout.imagePreviewMaxRows).toBe(8);
    expect(config.layout.fullPreviewImageMaxWidth).toBe(32);
    expect(config.layout.fullPreviewImageMaxRows).toBe(6);
    expect(config.layout.imagePreviewRenderer).toBe("text");
    expect(config.layout.fullPreviewImageRenderer).toBe("opentui");
    expect(config.layout.imagePreviewAlign).toBe("center");
    expect(config.layout.fullPreviewImageAlign).toBe("right");
    expect(config.layout.imagePreviewBlockGlyph).toBe("█");
    expect(config.layout.fullPreviewImageBlockGlyph).toBe("▓");
    expect(config.layout.imagePreviewBackground).toBe("#223344");
    expect(config.layout.fullPreviewImageBackground).toBe("#445566");
    expect(config.layout.fullPreviewImageNoticeVisibility).toBe("always");
    expect(config.layout.imagePreviewNoticeSpacing).toBe(1);
    expect(config.layout.fullPreviewImageNoticeSpacing).toBe(2);
    expect(config.layout.splitPaneGap).toBe(3);
    expect(config.layout.fullPreviewTextWidthInset).toBe(5);
    expect(config.layout.previewContentAlign).toBe("center");
    expect(config.layout.fullPreviewContentAlign).toBe("right");
    expect(config.layout.previewBodyVerticalAlign).toBe("center");
    expect(config.layout.fullPreviewBodyVerticalAlign).toBe("bottom");
    expect(config.layout.fullPreviewImageRowInset).toBe(4);
    expect(config.layout.fullPreviewGutterWidth).toBe(2);
    expect(config.layout.previewGutterAlign).toBe("center");
    expect(config.layout.fullPreviewGutterAlign).toBe("left");
    expect(config.layout.previewMetaHeight).toBe(1);
    expect(config.layout.fullPreviewMetaHeight).toBe(2);
    expect(config.layout.previewMetaLineSpacing).toBe(1);
    expect(config.layout.fullPreviewMetaLineSpacing).toBe(2);
    expect(config.layout.previewMetaHashLength).toBe(9);
    expect(config.layout.fullPreviewMetaHashLength).toBe(10);
    expect(config.layout.fullPreviewMetaPaddingX).toBe(2);
    expect(config.layout.previewMetaContentAlign).toBe("right");
    expect(config.layout.fullPreviewMetaContentAlign).toBe("center");
    expect(config.layout.previewMetaVerticalAlign).toBe("center");
    expect(config.layout.fullPreviewMetaVerticalAlign).toBe("bottom");
    expect(config.layout.headerContentAlign).toBe("center");
    expect(config.layout.statusContentAlign).toBe("right");
    expect(config.layout.headerVerticalAlign).toBe("bottom");
    expect(config.layout.statusVerticalAlign).toBe("center");
    expect(config.layout.statusSeparatorPadding).toBe(1);
    expect(config.layout.statusSeparatorPaddingLeft).toBe(2);
    expect(config.layout.statusSeparatorPaddingRight).toBe(3);
    expect(config.layout.frameTitlePadding).toBe(1);
    expect(config.layout.frameTitlePaddingLeft).toBe(2);
    expect(config.layout.frameTitlePaddingRight).toBe(3);
    expect(config.layout.overlayContentAlign).toBe("center");
    expect(config.layout.overlayVerticalAlign).toBe("center");
    expect(config.layout.overlayLineSpacing).toBe(1);
    expect(config.layout.searchOverlayContentAlign).toBe("right");
    expect(config.layout.searchOverlayVerticalAlign).toBe("bottom");
    expect(config.layout.searchOverlayLineSpacing).toBe(2);
    expect(config.layout.dangerOverlayContentAlign).toBe("left");
    expect(config.layout.dangerOverlayVerticalAlign).toBe("top");
    expect(config.layout.dangerOverlayLineSpacing).toBe(3);
    expect(config.layout.helpOverlayContentAlign).toBe("center");
    expect(config.layout.helpOverlayVerticalAlign).toBe("bottom");
    expect(config.layout.helpOverlayLineSpacing).toBe(2);
    expect(config.layout.helpKeyAlign).toBe("right");
    expect(config.layout.emptyStateContentAlign).toBe("center");
    expect(config.layout.emptyStateTitleAlign).toBe("right");
    expect(config.layout.emptyStateHelpAlign).toBe("left");
    expect(config.layout.emptyStateVerticalAlign).toBe("bottom");
    expect(config.layout.emptyStateLineSpacing).toBe(2);
    expect(config.layout.rowContentAlign).toBe("center");
    expect(config.layout.rowMetadataAlign).toBe("right");
    expect(config.layout.rowPreviewAlign).toBe("center");
    expect(config.layout.rowAgeAlign).toBe("left");
    expect(config.layout.rowSizeAlign).toBe("center");
    expect(config.layout.rowPinnedAlign).toBe("right");
    expect(config.layout.rowMarkerWidth).toBe(4);
    expect(config.layout.rowMarkerAlign).toBe("right");
    expect(config.layout.refreshIntervalMs).toBe(0);
    expect(config.layout.mouseEnabled).toBe(false);
    expect(config.layout.mouseScrollRows).toBe(5);
    expect(config.layout.scrollbarWidth).toBe(2);
    expect(config.layout.scrollbarPlacement).toBe("left");
    expect(config.layout.scrollbarAlign).toBe("right");
    expect(config.layout.shellPaddingX).toBe(2);
    expect(config.layout.shellPaddingY).toBe(1);
    expect(config.layout.showScrollbar).toBe(false);
    expect(config.layout.showHeader).toBe(false);
    expect(config.layout.showStatusLine).toBe(false);
    expect(config.layout.showRowMetadata).toBe(false);
    expect(config.layout.showPreviewPane).toBe(false);
    expect(config.layout.showFullPreviewMetadata).toBe(false);
    expect(config.layout.showPreviewGutter).toBe(false);
    expect(config.layout.showFullPreviewGutter).toBe(false);
    expect(config.layout.highlightSearchMatches).toBe(false);
    expect(config.layout.showEmptyStateHelp).toBe(false);
    expect(config.layout.alternateRows).toBe(false);
    expect(config.startup).toEqual({ filter: "images", pinnedOnly: true, query: "logo" });
    expect(config.behavior).toEqual({
      liveSearch: false,
      liveSearchDebounceMs: 25,
      clearQueryOnSearchOpen: false,
      restoreQueryOnSearchCancel: false,
      exitAfterPaste: false,
      exitAfterCopy: true,
      exitAfterBulkCopy: true,
      exitAfterSearchCopy: true,
    });
  });

  test("accepts top-level TUI config aliases", () => {
    const config = resolveTuiConfig({
      maxEntryLength: 65,
      pollInterval: 50,
      enableMouse: false,
      enableDescription: false,
      imageDisplay: { type: "basic", scaleX: 9, scaleY: 9, heightCut: 2 },
      keyBindings: {
        clearSelected: "S",
      },
    });

    expect(config.layout.rowPreviewReservedWidth).toBe(28);
    expect(config.layout.rowPreviewMaxWidth).toBe(65);
    expect(config.layout.refreshIntervalMs).toBe(50);
    expect(config.layout.mouseEnabled).toBe(false);
    expect(config.layout.showMetadata).toBe(false);
    expect(config.layout.showRowMetadata).toBe(true);
    expect(config.layout.showFullPreviewMetadata).toBe(false);
    expect(config.layout.imagePreviewMode).toBe("blocks");
    expect(config.layout.fullPreviewImageMode).toBe("blocks");
    expect(config.layout.imagePreviewMaxWidth).toBe(45);
    expect(config.layout.imagePreviewMaxRows).toBe(16);
    expect(config.layout.fullPreviewImageMaxWidth).toBe(45);
    expect(config.layout.fullPreviewImageMaxRows).toBe(16);
    expect(config.keyBindings.clearSelection).toEqual(["shift+s"]);
    expect(config.keyBindings.delete).toEqual(["d", "backspace"]);
  });

  test("preserves Kitty and Sixel image display modes as explicit fallbacks", () => {
    expect(resolveTuiConfig({ imageDisplay: { type: "kitty" } }).layout.imagePreviewMode).toBe("kitty");
    expect(resolveTuiConfig({ imageDisplay: { type: "sixel" } }).layout.imagePreviewMode).toBe("sixel");
    expect(resolveTuiConfig({ layout: { imagePreviewMode: "sixel" } }).layout.imagePreviewMode).toBe("sixel");
    expect(resolveTuiConfig({}, { DITOX_TUI_IMAGE_PREVIEW: "kitty" }).layout.imagePreviewMode).toBe("kitty");
  });

  test("Ditox layout and env settings take precedence over compatibility aliases", () => {
    const config = resolveTuiConfig(
      {
        maxEntryLength: 65,
        pollInterval: 50,
        enableMouse: false,
        enableDescription: false,
        imageDisplay: { type: "basic" },
        layout: {
          rowPreviewReservedWidth: 24,
          rowPreviewMaxWidth: 42,
          refreshIntervalMs: 250,
          mouseEnabled: true,
          showMetadata: true,
          imagePreviewMode: "metadata",
          imagePreviewMaxWidth: 24,
          imagePreviewMaxRows: 8,
        },
      },
      {
        DITOX_TUI_REFRESH_MS: "500",
        DITOX_TUI_MOUSE: "false",
        DITOX_TUI_METADATA: "false",
        DITOX_TUI_IMAGE_PREVIEW: "blocks",
      },
    );

    expect(config.layout.rowPreviewReservedWidth).toBe(24);
    expect(config.layout.rowPreviewMaxWidth).toBe(42);
    expect(config.layout.refreshIntervalMs).toBe(500);
    expect(config.layout.mouseEnabled).toBe(false);
    expect(config.layout.showMetadata).toBe(false);
    expect(config.layout.imagePreviewMode).toBe("blocks");
    expect(config.layout.fullPreviewImageMode).toBe("blocks");
    expect(config.layout.imagePreviewMaxWidth).toBe(24);
    expect(config.layout.imagePreviewMaxRows).toBe(8);
    expect(config.layout.fullPreviewImageMaxWidth).toBe(24);
    expect(config.layout.fullPreviewImageMaxRows).toBe(8);
  });

  test("loads from explicit file path", () => {
    const config = loadTuiConfig({ DITOX_TUI_CONFIG: "/tmp/ditox-tui.json" }, (path) => {
      expect(path).toBe("/tmp/ditox-tui.json");
      return JSON.stringify({ labels: { brand: "FILE" }, keyBindings: { help: "f1,?", preview: "space,ctrl+o" } });
    });
    expect(config.labels.brand).toBe("FILE");
    expect(config.keyBindings.help).toEqual(["f1", "?"]);
    expect(config.keyBindings.preview).toEqual(["space", "ctrl+o"]);
  });

  test("loads external custom theme files relative to the TUI config file", () => {
    const config = loadTuiConfig({ DITOX_TUI_CONFIG: "/tmp/ditox/configuration.json" }, (path) => {
      if (path === "/tmp/ditox/configuration.json") {
        return JSON.stringify({
          themeFile: "custom_theme.json",
          theme: { preset: "ditoxLight", colors: { accentSuccess: "#010203" } },
          styles: { header: { bg: "#111111" } },
        });
      }
      expect(path).toBe("/tmp/ditox/custom_theme.json");
      return JSON.stringify({
        UseCustom: true,
        TitleFore: "#eeeeee",
        TitleBack: "#222222",
        TitleInfo: "#33aaff",
        NormalTitle: "#44dd77",
        NormalDesc: "#dddddd",
        DimmedDesc: "#777777",
        SelectedTitle: "#ff66cc",
        SelectedDesc: "#ffffff",
        StatusMsg: "#22cc66",
        PinIndicatorColor: "#ffd700",
        SelectedBorder: "#3498db",
        FilterPrompt: "#2ecc71",
        HelpKey: "#999999",
        HelpDesc: "#bbbbbb",
        PageActiveDot: "#abcdef",
        PageInactiveDot: "#123456",
        PreviewedText: "#fefefe",
        PreviewBorder: "#456789",
      });
    });

    expect(config.theme.name).toBe("ditoxLight");
    expect(config.theme.accentSuccess).toBe("#010203");
    expect(config.theme.accentFavorite).toBe("#ffd700");
    expect(surface(config, "header").bg).toBe("#111111");
    expect(surface(config, "header").fg).toBe("#eeeeee");
    expect(surface(config, "header").secondary).toBe("#33aaff");
    expect(surface(config, "list").fg).toBe("#dddddd");
    expect(surface(config, "alternateRow").fg).toBe("#dddddd");
    expect(surface(config, "selectedRow").accent).toBe("#ff66cc");
    expect(surface(config, "selectedMarkedRow").accent).toBe("#ffd700");
    expect(surface(config, "selectedMarkedRow").favorite).toBe("#ffd700");
    expect(surface(config, "overlay").search).toBe("#2ecc71");
    expect(surface(config, "status").success).toBe("#22cc66");
    expect(surface(config, "preview").fg).toBe("#fefefe");
    expect(surface(config, "preview").border).toBe("#456789");
    expect(surface(config, "scrollbar").accent).toBe("#abcdef");
    expect(surface(config, "scrollbar").muted).toBe("#123456");
  });

  test("ignores disabled external custom theme files", () => {
    const config = loadTuiConfig({ DITOX_TUI_CONFIG: "/tmp/ditox/configuration.json" }, (path) => {
      if (path === "/tmp/ditox/configuration.json") return JSON.stringify({ themeFile: "custom_theme.json" });
      return JSON.stringify({ UseCustom: false, TitleFore: "#eeeeee" });
    });

    expect(surface(config, "header").fg).toBe(config.theme.textPrimary);
  });

  test("accepts keybinding aliases without overriding explicit Ditox keys", () => {
    const config = resolveTuiConfig({
      keyBindings: {
        choose: "ctrl+p",
        clearSelected: "shift+delete",
        filter: "ctrl+f",
        more: "f1",
        nextPage: "ctrl+d",
        prevPage: "ctrl+u",
        remove: "delete",
        togglePin: "ctrl+t",
        togglePinned: "tab",
        yankFilter: "ctrl+g",
        searchCopyMatches: "ctrl+s",
      },
    });
    expect(config.keyBindings.copyPaste).toEqual(["ctrl+p"]);
    expect(config.keyBindings.clearSelection).toEqual(["shift+delete"]);
    expect(config.keyBindings.delete).toEqual(["delete"]);
    expect(config.keyBindings.search).toEqual(["ctrl+f"]);
    expect(config.keyBindings.help).toEqual(["f1"]);
    expect(config.keyBindings.pageDown).toEqual(["ctrl+d"]);
    expect(config.keyBindings.pageUp).toEqual(["ctrl+u"]);
    expect(config.keyBindings.toggleFavorite).toEqual(["ctrl+t"]);
    expect(config.keyBindings.togglePinnedView).toEqual(["tab"]);
    expect(config.keyBindings.searchCopyMatches).toEqual(["ctrl+s"]);
  });

  test("derives help and status text from customized keys and formatting", () => {
    const config = resolveTuiConfig({
      labels: {
        keyAlternativeSeparator: " | ",
        keyGroupSeparator: " + ",
        statusHintSeparator: " :: ",
        statusHintTemplate: "{searchKeys}:{search}{separator}{previewKeys}:{preview}{separator}{pasteKeys}:{paste}{separator}{helpKeys}:{help}",
        statusPasteHint: "send",
        statusCopyHint: "stash",
        statusPreviewHint: "inspect",
        statusSearchHint: "find",
        statusHelpHint: "keys",
      },
      keyBindings: {
        copyPaste: "ctrl+p",
        help: "f1",
        preview: "ctrl+o,space",
        togglePinnedView: "tab",
        searchCopyMatches: "ctrl+g",
      },
      keyLabels: {
        space: "SPC",
        up: "UP",
        down: "DN",
      },
    });
    expect(statusHint(config)).toBe("/:find :: ctrl+o | SPC:inspect :: ctrl+p:send :: f1:keys");
    expect(helpRows(config).some((row) => row.keys.includes("ctrl+p"))).toBe(true);
    expect(helpRows(config).some((row) => row.keys.includes("tab"))).toBe(true);
    expect(helpRows(config).some((row) => row.keys.includes("ctrl+g") && row.action.includes("matched"))).toBe(true);
    expect(helpRows(config).some((row) => row.keys === "UP | k + DN | j")).toBe(true);
    expect(helpRows(config).some((row) => row.action.includes("pinned"))).toBe(true);
  });

  test("splits configurable text templates into literal and colored placeholder segments", () => {
    expect(templateSegments("{query} <- {brand} [{missing}]", { query: "needle", brand: "DX" })).toEqual([
      { key: "query", text: "needle" },
      { key: null, text: " <- " },
      { key: "brand", text: "DX" },
      { key: null, text: " [" },
      { key: null, text: "{missing}" },
      { key: null, text: "]" },
    ]);
  });
});
