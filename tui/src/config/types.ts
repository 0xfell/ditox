import type { TuiTheme, ThemeName } from "../theme";
import type { EntryFilter } from "../types";
import type { PreviewImageField, UiConfig } from "../ui-config";

export type BorderStyle = "single" | "double" | "rounded" | "heavy";
export const titleAlignmentNames = ["left", "center", "right"] as const;
export type TitleAlignment = (typeof titleAlignmentNames)[number];
export const imagePreviewNoticeVisibilityNames = ["never", "protocol", "always"] as const;
export const imagePreviewRendererNames = ["auto", "native", "opentui", "text"] as const;
export const imagePreviewAlignNames = ["left", "center", "right"] as const;
export const verticalAlignNames = ["top", "center", "bottom"] as const;
export const scrollbarPlacementNames = ["left", "right"] as const;
export const overlayPlacementNames = ["bottom", "top"] as const;
export type ThemeColors = Omit<TuiTheme, "name">;

export const tuiSurfaceNames = [
  "shell",
  "header",
  "list",
  "alternateRow",
  "selectedRow",
  "selectedMarkedRow",
  "markedRow",
  "rowSpacer",
  "emptyState",
  "preview",
  "previewGutter",
  "previewMeta",
  "previewSpacer",
  "fullPreview",
  "fullPreviewGutter",
  "fullPreviewMeta",
  "fullPreviewSpacer",
  "overlay",
  "searchOverlay",
  "dangerOverlay",
  "helpOverlay",
  "status",
  "scrollbar",
  "splitPaneGap",
] as const;

export type TuiSurfaceName = (typeof tuiSurfaceNames)[number];

export type TuiSurfaceStyle = {
  bg: string;
  fg: string;
  border: string;
  accent: string;
  muted: string;
  secondary: string;
  success: string;
  warning: string;
  error: string;
  search: string;
  favorite: string;
  image: string;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
  blink: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
};

export type TuiTextStyle = {
  fg: string;
  bg: string;
  attributes: number;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
  blink: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
};

export type TuiChromeConfig = {
  panelBorder: boolean;
  overlayBorder: boolean;
  headerBorder: boolean;
  listBorder: boolean;
  previewBorder: boolean;
  fullPreviewBorder: boolean;
  statusBorder: boolean;
  searchOverlayBorder: boolean;
  dangerOverlayBorder: boolean;
  helpOverlayBorder: boolean;
  showPanelTitles: boolean;
  showOverlayTitles: boolean;
  showHeaderTitle: boolean;
  showListTitle: boolean;
  showPreviewTitle: boolean;
  showFullPreviewTitle: boolean;
  showStatusTitle: boolean;
  showSearchOverlayTitle: boolean;
  showDangerOverlayTitle: boolean;
  showHelpOverlayTitle: boolean;
  showListPositionTitle: boolean;
  showPreviewEntryTitle: boolean;
  showFullPreviewBottomTitle: boolean;
  panelBorderStyle: BorderStyle;
  overlayBorderStyle: BorderStyle;
  headerBorderStyle: BorderStyle;
  listBorderStyle: BorderStyle;
  previewBorderStyle: BorderStyle;
  fullPreviewBorderStyle: BorderStyle;
  statusBorderStyle: BorderStyle;
  searchOverlayBorderStyle: BorderStyle;
  dangerOverlayBorderStyle: BorderStyle;
  helpOverlayBorderStyle: BorderStyle;
  panelTitleAlignment: TitleAlignment;
  panelBottomTitleAlignment: TitleAlignment;
  overlayTitleAlignment: TitleAlignment;
  headerTitleAlignment: TitleAlignment;
  listTitleAlignment: TitleAlignment;
  previewTitleAlignment: TitleAlignment;
  fullPreviewTitleAlignment: TitleAlignment;
  statusTitleAlignment: TitleAlignment;
  listBottomTitleAlignment: TitleAlignment;
  previewBottomTitleAlignment: TitleAlignment;
  fullPreviewBottomTitleAlignment: TitleAlignment;
  searchOverlayTitleAlignment: TitleAlignment;
  dangerOverlayTitleAlignment: TitleAlignment;
  helpOverlayTitleAlignment: TitleAlignment;
  selectedMarker: string;
  selectedMarkedMarker: string;
  markedMarker: string;
  normalMarker: string;
  scrollbarThumb: string;
  scrollbarTrack: string;
  statusSeparator: string;
};

export const terminalAltScreenNames = ["auto", "always", "never"] as const;
export type TuiTerminalAltScreen = (typeof terminalAltScreenNames)[number];

export const terminalScreenModeNames = ["alternate-screen", "main-screen", "split-footer"] as const;
export type TuiTerminalScreenMode = (typeof terminalScreenModeNames)[number];

export const terminalCursorStyleNames = ["default", "block", "line", "underline"] as const;
export type TuiTerminalCursorStyle = (typeof terminalCursorStyleNames)[number];

export type TuiKittyKeyboardConfig = {
  enabled: boolean;
  disambiguate: boolean;
  alternateKeys: boolean;
  events: boolean;
  allKeysAsEscapes: boolean;
  reportText: boolean;
};

export type TuiTerminalCursorConfig = {
  style: TuiTerminalCursorStyle | null;
  blinking: boolean | null;
  color: string | null;
};

export type TuiTerminalConfig = {
  altScreen: TuiTerminalAltScreen;
  screenMode: TuiTerminalScreenMode;
  title: string | null;
  backgroundColor: string;
  footerHeight: number;
  clearOnShutdown: boolean;
  cursor: TuiTerminalCursorConfig;
  kittyKeyboard: TuiKittyKeyboardConfig;
  targetFps: number | null;
  maxFps: number | null;
  debounceDelay: number | null;
  stdinParserMaxBufferBytes: number | null;
};

export type TuiTerminalConfigFile = {
  altScreen?: string;
  alt_screen?: string;
  screenMode?: string;
  title?: string | null;
  backgroundColor?: string;
  footerHeight?: number;
  clearOnShutdown?: boolean;
  cursor?: Partial<TuiTerminalCursorConfig>;
  kittyKeyboard?: boolean | string | Partial<TuiKittyKeyboardConfig>;
  targetFps?: number | null;
  maxFps?: number | null;
  debounceDelay?: number | null;
  stdinParserMaxBufferBytes?: number | null;
};

export type KeyDisplayLabels = Record<string, string>;

export type TuiStatusToneMatchers = {
  error: string[];
  success: string[];
  warning: string[];
};

export const statusLinePartNames = ["operation", "watcher", "hint", "separator"] as const;
export type TuiStatusLinePartName = (typeof statusLinePartNames)[number];
export type TuiStatusHintMode = "browse" | "search" | "preview" | "confirm";

export const statusLineToneNames = [
  "auto",
  "fg",
  "border",
  "muted",
  "secondary",
  "accent",
  "success",
  "warning",
  "error",
  "search",
  "favorite",
  "image",
] as const;
export type TuiStatusLineToneName = (typeof statusLineToneNames)[number];
export type TuiStatusLineTones = Record<TuiStatusLinePartName, TuiStatusLineToneName>;

export const headerLinePartNames = [
  "brand",
  "filter",
  "query",
  "mode",
  "filterLabel",
  "queryLabel",
  "modeLabel",
  "sectionSeparator",
  "labelSeparator",
] as const;
export type TuiHeaderLinePartName = (typeof headerLinePartNames)[number];
export type TuiHeaderLineTones = Record<TuiHeaderLinePartName, TuiStatusLineToneName>;

export const overlayToneNames = ["search", "danger", "command"] as const;
export type TuiOverlayToneName = (typeof overlayToneNames)[number];
export type TuiOverlayBorderTones = Record<TuiOverlayToneName, TuiStatusLineToneName>;

export const overlayContentPartNames = [
  "searchInput",
  "searchPrompt",
  "searchQuery",
  "searchCursor",
  "deletePrompt",
  "deleteWarning",
  "confirmHint",
  "clearPrompt",
  "clearSafeHint",
  "clearUnsafeHint",
  "helpKey",
  "helpAction",
] as const;
export type TuiOverlayContentPartName = (typeof overlayContentPartNames)[number];
export type TuiOverlayContentTones = Record<TuiOverlayContentPartName, TuiStatusLineToneName>;

export const listContentPartNames = [
  "marker",
  "markerGap",
  "metadata",
  "metadataGap",
  "preview",
  "searchMatch",
  "emptyTitle",
  "emptyHelp",
  "scrollbarThumb",
  "scrollbarTrack",
] as const;
export type TuiListContentPartName = (typeof listContentPartNames)[number];
export type TuiListContentTones = Record<TuiListContentPartName, TuiStatusLineToneName>;

export const previewContentPartNames = [
  "splitBorder",
  "splitEmptyTitle",
  "splitEmptyHelp",
  "splitImageFallbackPrefix",
  "splitImageFallbackSeparator",
  "splitImageFallbackReason",
  "splitImageNotice",
  "splitGutter",
  "splitGutterSeparator",
  "splitPrimary",
  "splitSecondary",
  "splitMuted",
  "splitAccent",
  "splitError",
  "splitSuccess",
  "splitMetaHeader",
  "splitMetaDetails",
  "fullBorder",
  "fullMeta",
  "fullMetaHeader",
  "fullMetaDetails",
  "fullEmptyTitle",
  "fullEmptyHelp",
  "fullImageFallbackPrefix",
  "fullImageFallbackSeparator",
  "fullImageFallbackReason",
  "fullImageNotice",
  "fullGutter",
  "fullGutterSeparator",
  "fullPrimary",
  "fullSecondary",
  "fullMuted",
  "fullAccent",
  "fullError",
  "fullSuccess",
] as const;
export type TuiPreviewContentPartName = (typeof previewContentPartNames)[number];
export type TuiPreviewContentTones = Record<TuiPreviewContentPartName, TuiStatusLineToneName>;

export const helpActionNames = [
  "moveSelection",
  "pageSelection",
  "firstLastEntry",
  "quit",
  "preview",
  "previewNavigation",
  "previewBack",
  "pinnedView",
  "paste",
  "copySet",
  "output",
  "markSingle",
  "rangeSelect",
  "searchFilter",
  "searchEdit",
  "searchCopyMatches",
  "pinDelete",
  "clearHistory",
  "clearAllIncludingPinned",
  "confirmChoice",
] as const;

export type TuiHelpActionName = (typeof helpActionNames)[number];

export type TuiLabels = {
  appTitle: string;
  brand: string;
  historyTitle: string;
  previewTitle: string;
  statusTitle: string;
  headerSectionSeparator: string;
  headerLabelSeparator: string;
  headerLineTemplate: string;
  filterLabel: string;
  filterAll: string;
  filterText: string;
  filterImages: string;
  filterFavorites: string;
  filterToday: string;
  queryLabel: string;
  queryEmpty: string;
  modeLabel: string;
  singleSelection: string;
  selectedPrefix: string;
  selectedCountTemplate: string;
  kindText: string;
  kindImage: string;
  rowPinnedLabel: string;
  rowMetaTemplate: string;
  rowPinnedSlotTemplate: string;
  rowUnpinnedSlotTemplate: string;
  entryIdPrefix: string;
  listPositionTemplate: string;
  emptyEntryPreview: string;
  noHistoryTitle: string;
  noHistoryHelp: string;
  noMatchesTitle: string;
  noMatchesHelp: string;
  noEntryTitle: string;
  noEntryHelp: string;
  previewTypeGutter: string;
  previewMimeGutter: string;
  previewSizeGutter: string;
  previewDimensionsGutter: string;
  previewHashGutter: string;
  previewBlobGutter: string;
  previewImageEntry: string;
  previewUnknownDimensions: string;
  previewBlobMissing: string;
  previewMetaHashLabel: string;
  previewMetaSeparator: string;
  previewMetaLabelSeparator: string;
  previewPinnedSuffixTemplate: string;
  previewEntryTitleTemplate: string;
  previewMetaHeaderTemplate: string;
  previewMetaDetailsTemplate: string;
  fullPreviewMetaTemplate: string;
  fullPreviewMetaHeaderTemplate: string;
  fullPreviewMetaDetailsTemplate: string;
  fullPreviewBottomTitleTemplate: string;
  previewGutterSeparator: string;
  fullPreviewGutterSeparator: string;
  previewTextGutterTemplate: string;
  sizeBytesUnit: string;
  sizeKibUnit: string;
  sizeMibUnit: string;
  ageSecondsUnit: string;
  ageMinutesUnit: string;
  ageHoursUnit: string;
  ageDaysUnit: string;
  textTruncationMarker: string;
  textWhitespaceReplacement: string;
  imagePreviewFallbackPrefix: string;
  imagePreviewFallbackSeparator: string;
  splitImagePreviewFallbackPrefix: string;
  splitImagePreviewFallbackSeparator: string;
  fullImagePreviewFallbackPrefix: string;
  fullImagePreviewFallbackSeparator: string;
  imagePreviewNotImage: string;
  imagePreviewBlocksDisabled: string;
  imagePreviewBlobMissing: string;
  imagePreviewDecodePending: string;
  imagePreviewUnsupportedMime: string;
  imagePreviewUnsupportedBytes: string;
  imagePreviewDecodeFailed: string;
  imagePreviewSourceTemplate: string;
  splitImagePreviewSourceTemplate: string;
  fullImagePreviewSourceTemplate: string;
  imagePreviewBlocksSource: string;
  imagePreviewKittyFallbackSource: string;
  imagePreviewSixelFallbackSource: string;
  imagePreviewKittyProtocolName: string;
  imagePreviewSixelProtocolName: string;
  imagePreviewProtocolUnknown: string;
  imagePreviewProtocolUnsupported: string;
  imagePreviewProtocolRendererUnavailable: string;
  searchTitle: string;
  searchPrompt: string;
  searchCursor: string;
  searchInputTemplate: string;
  deleteTitle: string;
  clearTitle: string;
  helpTitle: string;
  confirmHint: string;
  confirmHintTemplate: string;
  deleteOne: string;
  deleteMany: string;
  deleteOneTemplate: string;
  deleteManyTemplate: string;
  deletePinnedWarning: string;
  clearPrefix: string;
  clearPromptTemplate: string;
  clearKindAll: string;
  clearKindText: string;
  clearKindImages: string;
  clearPinnedSafeHint: string;
  clearPinnedUnsafeHint: string;
  ready: string;
  pinned: string;
  watcherRunning: string;
  watcherPaused: string;
  watcherStale: string;
  watcherStopped: string;
  watcherErrorSeparator: string;
  watcherRunningTemplate: string;
  watcherPausedTemplate: string;
  watcherStaleTemplate: string;
  watcherStoppedTemplate: string;
  watcherErrorTemplate: string;
  keyAlternativeSeparator: string;
  keyGroupSeparator: string;
  statusHintSeparator: string;
  statusHintTemplate: string;
  statusSearchModeHintTemplate: string;
  statusPreviewModeHintTemplate: string;
  statusConfirmModeHintTemplate: string;
  statusLineTemplate: string;
  statusPasteHint: string;
  statusCopyHint: string;
  statusPreviewHint: string;
  statusSearchHint: string;
  statusFilterHint: string;
  statusPinnedHint: string;
  statusDeleteHint: string;
  statusOutputHint: string;
  statusHelpHint: string;
  statusQuitHint: string;
  statusApplyHint: string;
  statusCancelHint: string;
  statusBackspaceHint: string;
  statusSearchCopyHint: string;
  statusPreviewBackHint: string;
  statusPreviewScrollHint: string;
  statusConfirmYesHint: string;
  statusConfirmNoHint: string;
  statusCopied: string;
  statusPasted: string;
  statusNothingCopied: string;
  statusCopiedCountPrefix: string;
  statusCopiedCountTemplate: string;
  statusDeletedPrefix: string;
  statusDeletedTemplate: string;
  statusClearedPrefix: string;
  statusClearedTemplate: string;
  statusPinnedPrefix: string;
  statusUnpinnedPrefix: string;
  statusPinTemplate: string;
  statusEntries: string;
  statusEntriesTemplate: string;
  statusKeptPinned: string;
  statusIncludedPinned: string;
  statusViewSuffix: string;
  statusPinnedView: string;
  statusAllView: string;
  errorDitoxdMissing: string;
  errorClipboardToolMissing: string;
  errorPasteToolMissing: string;
  errorClipboardWriteFailed: string;
  errorPasteBackFailed: string;
  errorDitoxdExited: string;
  errorUnknownStatus: string;
  errorProcessTemplate: string;
  errorRpcTemplate: string;
  pinnedViewTitle: string;
  allViewTitle: string;
  previewModeTitle: string;
  previewBackHint: string;
  helpMoveSelection: string;
  helpPageSelection: string;
  helpFirstLastEntry: string;
  helpQuit: string;
  helpPreview: string;
  helpPreviewNavigation: string;
  helpPreviewBack: string;
  helpPinnedView: string;
  helpPaste: string;
  helpCopySet: string;
  helpOutput: string;
  helpMarkSingle: string;
  helpRangeSelect: string;
  helpSearchFilter: string;
  helpSearchEdit: string;
  helpSearchCopyMatches: string;
  helpPinDelete: string;
  helpClearHistory: string;
  helpClearAllIncludingPinned: string;
  helpConfirmChoice: string;
};

export type TuiKeyBindings = {
  quit: string[];
  forceQuit: string[];
  up: string[];
  down: string[];
  pageUp: string[];
  pageDown: string[];
  home: string[];
  end: string[];
  nextFilter: string[];
  search: string[];
  searchCancel: string[];
  searchBackspace: string[];
  searchApply: string[];
  searchCopyMatches: string[];
  selectToggle: string[];
  selectSingle: string[];
  clearSelection: string[];
  selectUp: string[];
  selectDown: string[];
  toggleFavorite: string[];
  delete: string[];
  copyPaste: string[];
  copyOnly: string[];
  bulkCopy: string[];
  output: string[];
  help: string[];
  preview: string[];
  previewBack: string[];
  previewUp: string[];
  previewDown: string[];
  previewPageUp: string[];
  previewPageDown: string[];
  togglePinnedView: string[];
  clearAll: string[];
  clearText: string[];
  clearImages: string[];
  clearAllIncludingPinned: string[];
  confirmYes: string[];
  confirmNo: string[];
};

export const compatKeyBindingAliasNames = [
  "choose",
  "clearSelected",
  "filter",
  "more",
  "nextPage",
  "prevPage",
  "remove",
  "togglePin",
  "togglePinned",
  "yankFilter",
] as const;

export type CompatKeyBindingAlias = (typeof compatKeyBindingAliasNames)[number];

export type CompatImageDisplayConfig = {
  type?: "basic" | "kitty" | "sixel" | string;
  scaleX?: number;
  scaleY?: number;
  heightCut?: number;
};

export type CompatThemeFile = {
  UseCustom?: boolean;
  TitleFore?: string;
  TitleBack?: string;
  TitleInfo?: string;
  NormalTitle?: string;
  DimmedTitle?: string;
  SelectedTitle?: string;
  NormalDesc?: string;
  DimmedDesc?: string;
  SelectedDesc?: string;
  StatusMsg?: string;
  PinIndicatorColor?: string;
  SelectedBorder?: string;
  SelectedDescBorder?: string;
  FilteredMatch?: string;
  FilterPrompt?: string;
  FilterInfo?: string;
  FilterText?: string;
  FilterCursor?: string;
  HelpKey?: string;
  HelpDesc?: string;
  PageActiveDot?: string;
  PageInactiveDot?: string;
  DividerDot?: string;
  PreviewedText?: string;
  PreviewBorder?: string;
};

export type ResolvedTuiConfig = {
  sourcePath: string | null;
  theme: TuiTheme;
  terminal: TuiTerminalConfig;
  layout: UiConfig;
  chrome: TuiChromeConfig;
  styles: Record<TuiSurfaceName, TuiSurfaceStyle>;
  labels: TuiLabels;
  keyBindings: TuiKeyBindings;
  keyLabels: KeyDisplayLabels;
  statusTones: TuiStatusToneMatchers;
  headerLineTones: TuiHeaderLineTones;
  statusLineTones: TuiStatusLineTones;
  overlayBorderTones: TuiOverlayBorderTones;
  overlayContentTones: TuiOverlayContentTones;
  listContentTones: TuiListContentTones;
  previewContentTones: TuiPreviewContentTones;
  filterOrder: EntryFilter[];
  helpOrder: TuiHelpActionName[];
  startup: TuiStartupConfig;
  behavior: TuiBehaviorConfig;
};

export type TuiStartupConfig = {
  filter: EntryFilter;
  pinnedOnly: boolean;
  query: string;
};

export type TuiBehaviorConfig = {
  liveSearch: boolean;
  liveSearchDebounceMs: number;
  clearQueryOnSearchOpen: boolean;
  restoreQueryOnSearchCancel: boolean;
  exitAfterPaste: boolean;
  exitAfterCopy: boolean;
  exitAfterBulkCopy: boolean;
  exitAfterSearchCopy: boolean;
};

export type TuiConfigFile = {
  theme?: ThemeName | { preset?: ThemeName; colors?: Partial<ThemeColors> };
  terminal?: Partial<TuiTerminalConfigFile>;
  layout?: Partial<UiConfig>;
  chrome?: Partial<TuiChromeConfig>;
  styles?: Partial<Record<TuiSurfaceName, Partial<TuiSurfaceStyle>>>;
  labels?: Partial<TuiLabels>;
  keyBindings?: Partial<Record<keyof TuiKeyBindings | CompatKeyBindingAlias, string | string[]>>;
  keyLabels?: Record<string, string>;
  statusTones?: Partial<Record<keyof TuiStatusToneMatchers, string[]>>;
  headerLineTones?: Partial<Record<TuiHeaderLinePartName, TuiStatusLineToneName>>;
  statusLineTones?: Partial<Record<TuiStatusLinePartName, TuiStatusLineToneName>>;
  overlayBorderTones?: Partial<Record<TuiOverlayToneName, TuiStatusLineToneName>>;
  overlayContentTones?: Partial<Record<TuiOverlayContentPartName, TuiStatusLineToneName>>;
  listContentTones?: Partial<Record<TuiListContentPartName, TuiStatusLineToneName>>;
  previewContentTones?: Partial<Record<TuiPreviewContentPartName, TuiStatusLineToneName>>;
  filterOrder?: string[];
  helpOrder?: string[];
  startup?: Partial<TuiStartupConfig>;
  behavior?: Partial<TuiBehaviorConfig>;
  themeFile?: string;
  maxEntryLength?: number;
  pollInterval?: number;
  enableMouse?: boolean;
  enableDescription?: boolean;
  imageDisplay?: CompatImageDisplayConfig;
};


export const defaultChrome: TuiChromeConfig = {
  panelBorder: true,
  overlayBorder: true,
  headerBorder: true,
  listBorder: true,
  previewBorder: true,
  fullPreviewBorder: true,
  statusBorder: false,
  searchOverlayBorder: true,
  dangerOverlayBorder: true,
  helpOverlayBorder: true,
  showPanelTitles: true,
  showOverlayTitles: true,
  showHeaderTitle: true,
  showListTitle: true,
  showPreviewTitle: true,
  showFullPreviewTitle: true,
  showStatusTitle: false,
  showSearchOverlayTitle: true,
  showDangerOverlayTitle: true,
  showHelpOverlayTitle: true,
  showListPositionTitle: true,
  showPreviewEntryTitle: true,
  showFullPreviewBottomTitle: true,
  panelBorderStyle: "rounded",
  overlayBorderStyle: "rounded",
  headerBorderStyle: "rounded",
  listBorderStyle: "rounded",
  previewBorderStyle: "rounded",
  fullPreviewBorderStyle: "rounded",
  statusBorderStyle: "rounded",
  searchOverlayBorderStyle: "rounded",
  dangerOverlayBorderStyle: "rounded",
  helpOverlayBorderStyle: "rounded",
  panelTitleAlignment: "left",
  panelBottomTitleAlignment: "left",
  overlayTitleAlignment: "left",
  headerTitleAlignment: "left",
  listTitleAlignment: "left",
  previewTitleAlignment: "left",
  fullPreviewTitleAlignment: "left",
  statusTitleAlignment: "left",
  listBottomTitleAlignment: "left",
  previewBottomTitleAlignment: "left",
  fullPreviewBottomTitleAlignment: "left",
  searchOverlayTitleAlignment: "left",
  dangerOverlayTitleAlignment: "left",
  helpOverlayTitleAlignment: "left",
  selectedMarker: "▌",
  selectedMarkedMarker: "█",
  markedMarker: "▎",
  normalMarker: " ",
  scrollbarThumb: "┃",
  scrollbarTrack: "│",
  statusSeparator: "·",
};

export const defaultTerminal: TuiTerminalConfig = {
  altScreen: "auto",
  screenMode: "alternate-screen",
  title: null,
  backgroundColor: "auto",
  footerHeight: 12,
  clearOnShutdown: true,
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

export const defaultKeyLabels: KeyDisplayLabels = {
  space: "space",
  escape: "esc",
  enter: "enter",
  backspace: "bksp",
  tab: "tab",
  delete: "delete",
  pageup: "pgup",
  pagedown: "pgdn",
  up: "up",
  down: "down",
  left: "left",
  right: "right",
};

export const defaultStatusTones: TuiStatusToneMatchers = {
  error: ["error", "failed", "not found", "exited", "unavailable"],
  success: ["copied", "pasted", "cleared", "pinned", "unpinned"],
  warning: ["paused"],
};

export const defaultStatusLineTones: TuiStatusLineTones = {
  operation: "auto",
  watcher: "auto",
  hint: "muted",
  separator: "muted",
};

export const defaultHeaderLineTones: TuiHeaderLineTones = {
  brand: "accent",
  filter: "auto",
  query: "search",
  mode: "secondary",
  filterLabel: "muted",
  queryLabel: "muted",
  modeLabel: "muted",
  sectionSeparator: "muted",
  labelSeparator: "muted",
};

export const defaultOverlayBorderTones: TuiOverlayBorderTones = {
  search: "search",
  danger: "error",
  command: "border",
};

export const defaultOverlayContentTones: TuiOverlayContentTones = {
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
};

export const defaultListContentTones: TuiListContentTones = {
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
};

export const defaultPreviewContentTones: TuiPreviewContentTones = {
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
};

export const defaultLabels: TuiLabels = {
  appTitle: "ditox",
  brand: "DITOX",
  historyTitle: "history",
  previewTitle: "preview",
  statusTitle: "status",
  headerSectionSeparator: "  ",
  headerLabelSeparator: " ",
  headerLineTemplate:
    "{brand}{sectionSeparator}{filterLabel}{labelSeparator}{filter}{sectionSeparator}{queryLabel}{labelSeparator}{query}{sectionSeparator}{modeLabel}{labelSeparator}{mode}",
  filterLabel: "filter",
  filterAll: "ALL",
  filterText: "TEXT",
  filterImages: "IMAGES",
  filterFavorites: "FAVORITES",
  filterToday: "TODAY",
  queryLabel: "query",
  queryEmpty: "-",
  modeLabel: "mode",
  singleSelection: "single",
  selectedPrefix: "selected",
  selectedCountTemplate: "{prefix} {count}",
  kindText: "TXT",
  kindImage: "IMG",
  rowPinnedLabel: "PIN",
  rowMetaTemplate: "{kind} {age} {size}{pinnedSlot}",
  rowPinnedSlotTemplate: " {pinned}",
  rowUnpinnedSlotTemplate: "    ",
  entryIdPrefix: "#",
  listPositionTemplate: "{index}/{total}",
  emptyEntryPreview: "(empty)",
  noHistoryTitle: "No clipboard history",
  noHistoryHelp: "Start the watcher or add an entry from the CLI.",
  noMatchesTitle: "No matches",
  noMatchesHelp: "Try a broader search.",
  noEntryTitle: "No entry selected",
  noEntryHelp: "Copy something or use `ditox add` to seed history.",
  previewTypeGutter: "type",
  previewMimeGutter: "mime",
  previewSizeGutter: "size",
  previewDimensionsGutter: "dims",
  previewHashGutter: "hash",
  previewBlobGutter: "blob",
  previewImageEntry: "Image entry",
  previewUnknownDimensions: "unknown",
  previewBlobMissing: "not stored",
  previewMetaHashLabel: "hash",
  previewMetaSeparator: "  ",
  previewMetaLabelSeparator: " ",
  previewPinnedSuffixTemplate: "{separator}{pinned}",
  previewEntryTitleTemplate: "{entryIdPrefix}{id}",
  previewMetaHeaderTemplate: "{kind} {entryIdPrefix}{id}{separator}{hashLabel}{hashLabelSeparator}{hash}",
  previewMetaDetailsTemplate: "{mime}{separator}{size}{pinnedSuffix}",
  fullPreviewMetaTemplate: "{kind} {entryIdPrefix}{id}{separator}{mime}{pinnedSuffix}",
  fullPreviewMetaHeaderTemplate: "{kind} {entryIdPrefix}{id}{separator}{mime}{pinnedSuffix}",
  fullPreviewMetaDetailsTemplate: "{size}{separator}{hashLabel}{hashLabelSeparator}{hash}",
  fullPreviewBottomTitleTemplate: "{entryIdPrefix}{id} {start}-{end}/{total}{separator}{back}",
  previewGutterSeparator: "  ",
  fullPreviewGutterSeparator: "  ",
  previewTextGutterTemplate: "{linePadded}",
  sizeBytesUnit: "B",
  sizeKibUnit: "KiB",
  sizeMibUnit: "MiB",
  ageSecondsUnit: "s",
  ageMinutesUnit: "m",
  ageHoursUnit: "h",
  ageDaysUnit: "d",
  textTruncationMarker: "...",
  textWhitespaceReplacement: " ",
  imagePreviewFallbackPrefix: "img",
  imagePreviewFallbackSeparator: "  ",
  splitImagePreviewFallbackPrefix: "img",
  splitImagePreviewFallbackSeparator: "  ",
  fullImagePreviewFallbackPrefix: "img",
  fullImagePreviewFallbackSeparator: "  ",
  imagePreviewNotImage: "not an image entry",
  imagePreviewBlocksDisabled: "image blocks disabled",
  imagePreviewBlobMissing: "image blob is not stored",
  imagePreviewDecodePending: "decoding image preview",
  imagePreviewUnsupportedMime: "{mime} block preview is not supported yet",
  imagePreviewUnsupportedBytes: "unsupported image bytes",
  imagePreviewDecodeFailed: "{error}",
  imagePreviewSourceTemplate: "{source}",
  splitImagePreviewSourceTemplate: "{source}",
  fullImagePreviewSourceTemplate: "{source}",
  imagePreviewBlocksSource: "image blocks",
  imagePreviewKittyFallbackSource: "kitty fallback blocks",
  imagePreviewSixelFallbackSource: "sixel fallback blocks",
  imagePreviewKittyProtocolName: "Kitty",
  imagePreviewSixelProtocolName: "Sixel",
  imagePreviewProtocolUnknown: "{protocol} support unknown; showing block fallback",
  imagePreviewProtocolUnsupported: "{protocol} not detected; showing block fallback",
  imagePreviewProtocolRendererUnavailable: "{protocol} detected; native renderer unavailable; showing block fallback",
  searchTitle: "search",
  searchPrompt: "/",
  searchCursor: "|",
  searchInputTemplate: "{prompt}{query}{cursor}",
  deleteTitle: "confirm delete",
  clearTitle: "confirm clear",
  helpTitle: "keymap",
  confirmHint: "y confirm  n cancel",
  confirmHintTemplate: "{indent}{hint}",
  deleteOne: "Delete selected entry?",
  deleteMany: "Delete selected entries?",
  deleteOneTemplate: "{message}",
  deleteManyTemplate: "{message} ({count})",
  deletePinnedWarning: "Pinned entries are included.",
  clearPrefix: "Clear",
  clearPromptTemplate: "{prefix} {kind}?",
  clearKindAll: "all",
  clearKindText: "text",
  clearKindImages: "images",
  clearPinnedSafeHint: "Pinned entries stay.",
  clearPinnedUnsafeHint: "Pinned entries will be deleted.",
  ready: "ready",
  pinned: "pinned",
  watcherRunning: "watcher live",
  watcherPaused: "watcher paused",
  watcherStale: "watcher stale",
  watcherStopped: "watcher stopped",
  watcherErrorSeparator: ": ",
  watcherRunningTemplate: "{status} {age}",
  watcherPausedTemplate: "{status}",
  watcherStaleTemplate: "{status} {age}",
  watcherStoppedTemplate: "{status}",
  watcherErrorTemplate: "{status}{separator}{error}",
  keyAlternativeSeparator: " / ",
  keyGroupSeparator: "  ",
  statusHintSeparator: "  ",
  statusHintTemplate:
    "{pasteKeys} {paste}{separator}{copyKeys} {copy}{separator}{previewKeys} {preview}{separator}{searchKeys} {search}{separator}{helpKeys} {help}",
  statusSearchModeHintTemplate:
    "{applyKeys} {apply}{separator}{backspaceKeys} {backspace}{separator}{searchCopyKeys} {searchCopy}{separator}{cancelKeys} {cancel}",
  statusPreviewModeHintTemplate:
    "{previewBackKeys} {previewBack}{separator}{previewScrollKeys} {previewScroll}{separator}{pasteKeys} {paste}{separator}{copyKeys} {copy}",
  statusConfirmModeHintTemplate: "{confirmYesKeys} {confirmYes}{separator}{confirmNoKeys} {confirmNo}",
  statusLineTemplate: "{hint}{separator}{watcher}{separator}{operation}",
  statusPasteHint: "paste",
  statusCopyHint: "copy",
  statusPreviewHint: "preview",
  statusSearchHint: "search",
  statusFilterHint: "filter",
  statusPinnedHint: "pinned",
  statusDeleteHint: "delete",
  statusOutputHint: "output",
  statusHelpHint: "help",
  statusQuitHint: "quit",
  statusApplyHint: "apply",
  statusCancelHint: "cancel",
  statusBackspaceHint: "delete char",
  statusSearchCopyHint: "copy matches",
  statusPreviewBackHint: "back",
  statusPreviewScrollHint: "scroll",
  statusConfirmYesHint: "confirm",
  statusConfirmNoHint: "cancel",
  statusCopied: "copied",
  statusPasted: "pasted",
  statusNothingCopied: "nothing copied",
  statusCopiedCountPrefix: "copied",
  statusCopiedCountTemplate: "{prefix} {count}",
  statusDeletedPrefix: "deleted",
  statusDeletedTemplate: "{prefix} {count}",
  statusClearedPrefix: "cleared",
  statusClearedTemplate: "{prefix} {count}; {pinned}",
  statusPinnedPrefix: "pinned",
  statusUnpinnedPrefix: "unpinned",
  statusPinTemplate: "{prefix} {entryIdPrefix}{id}",
  statusEntries: "entries",
  statusEntriesTemplate: "{count} {entries}",
  statusKeptPinned: "kept pinned",
  statusIncludedPinned: "included pinned",
  statusViewSuffix: "view",
  statusPinnedView: "pinned",
  statusAllView: "all",
  errorDitoxdMissing: "{binary} not found or not executable",
  errorClipboardToolMissing: "wl-copy was not found or could not be started",
  errorPasteToolMissing: "wl-copy or hyprctl was not found",
  errorClipboardWriteFailed: "failed to write the clipboard with wl-copy",
  errorPasteBackFailed: "failed to paste through Hyprland",
  errorDitoxdExited: "ditoxd exited with {status}",
  errorUnknownStatus: "unknown status",
  errorProcessTemplate: "{message}",
  errorRpcTemplate: "{message}",
  pinnedViewTitle: "PINNED",
  allViewTitle: "ALL",
  previewModeTitle: "full preview",
  previewBackHint: "preview / esc back",
  helpMoveSelection: "move selection",
  helpPageSelection: "page up / down",
  helpFirstLastEntry: "jump first / last",
  helpQuit: "quit",
  helpPreview: "open preview",
  helpPreviewNavigation: "scroll preview",
  helpPreviewBack: "leave preview",
  helpPinnedView: "show pinned only",
  helpPaste: "paste selected",
  helpCopySet: "copy selected",
  helpOutput: "print selected",
  helpMarkSingle: "mark / isolate / clear",
  helpRangeSelect: "extend selection",
  helpSearchFilter: "search / next filter",
  helpSearchEdit: "edit / apply / cancel search",
  helpSearchCopyMatches: "copy matched search results",
  helpPinDelete: "pin or delete",
  helpClearHistory: "clear all / text / images",
  helpClearAllIncludingPinned: "clear including pinned",
  helpConfirmChoice: "confirm / cancel",
};

export const defaultKeyBindings: TuiKeyBindings = {
  quit: ["escape", "q"],
  forceQuit: ["ctrl+c"],
  up: ["up", "k"],
  down: ["down", "j"],
  pageUp: ["pageup"],
  pageDown: ["pagedown"],
  home: ["home"],
  end: ["end"],
  nextFilter: ["tab"],
  search: ["/"],
  searchCancel: ["escape"],
  searchBackspace: ["backspace"],
  searchApply: ["enter"],
  searchCopyMatches: ["ctrl+s"],
  selectToggle: ["x"],
  selectSingle: ["s"],
  clearSelection: ["shift+s"],
  selectUp: ["shift+up"],
  selectDown: ["shift+down"],
  toggleFavorite: ["p"],
  delete: ["d", "backspace"],
  copyPaste: ["enter"],
  copyOnly: ["ctrl+y"],
  bulkCopy: ["y"],
  output: ["o"],
  help: ["?"],
  preview: ["space", "right"],
  previewBack: ["space", "escape", "left"],
  previewUp: ["up", "k"],
  previewDown: ["down", "j"],
  previewPageUp: ["pageup"],
  previewPageDown: ["pagedown"],
  togglePinnedView: ["shift+tab"],
  clearAll: ["c a"],
  clearText: ["c t"],
  clearImages: ["c i"],
  clearAllIncludingPinned: ["c x"],
  confirmYes: ["y"],
  confirmNo: ["n"],
};

export const defaultFilterOrder: EntryFilter[] = ["all", "text", "images", "favorites", "today"];
export const defaultHelpOrder: TuiHelpActionName[] = [
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
];
export const defaultPreviewImageFields: PreviewImageField[] = ["type", "mime", "size", "dimensions", "hash", "blob"];
export const defaultStartup: TuiStartupConfig = {
  filter: "all",
  pinnedOnly: false,
  query: "",
};
export const defaultBehavior: TuiBehaviorConfig = {
  liveSearch: true,
  liveSearchDebounceMs: 120,
  clearQueryOnSearchOpen: true,
  restoreQueryOnSearchCancel: true,
  exitAfterPaste: true,
  exitAfterCopy: false,
  exitAfterBulkCopy: false,
  exitAfterSearchCopy: false,
};

export const compatKeyBindingAliases: Record<CompatKeyBindingAlias, keyof TuiKeyBindings> = {
  choose: "copyPaste",
  clearSelected: "clearSelection",
  filter: "search",
  more: "help",
  nextPage: "pageDown",
  prevPage: "pageUp",
  remove: "delete",
  togglePin: "toggleFavorite",
  togglePinned: "togglePinnedView",
  yankFilter: "searchCopyMatches",
};

export const defaultLayout: UiConfig = {
  compactMode: false,
  listWidthPercent: 46,
  previewWidthPercent: 54,
  maxPreviewLines: 28,
  maxFullPreviewLines: 2000,
  historyLimit: 100,
  imagePreviewMode: "blocks",
  fullPreviewImageMode: "blocks",
  imagePreviewMaxWidth: 80,
  imagePreviewMaxRows: 24,
  fullPreviewImageMaxWidth: 120,
  fullPreviewImageMaxRows: 40,
  imagePreviewRenderer: "auto",
  fullPreviewImageRenderer: "auto",
  imagePreviewAlign: "left",
  fullPreviewImageAlign: "left",
  imagePreviewBlockGlyph: "▀",
  fullPreviewImageBlockGlyph: "▀",
  imagePreviewBackground: "auto",
  fullPreviewImageBackground: "auto",
  imagePreviewNoticeVisibility: "protocol",
  fullPreviewImageNoticeVisibility: "protocol",
  imagePreviewNoticeSpacing: 0,
  fullPreviewImageNoticeSpacing: 0,
  headerHeight: 3,
  statusHeight: 1,
  showHeader: true,
  showStatusLine: true,
  searchOverlayHeight: 3,
  confirmOverlayHeight: 3,
  confirmPinnedExtraRows: 1,
  clearOverlayHeight: 4,
  helpOverlayHeight: 16,
  overlayPlacement: "bottom",
  minPaneWidth: 24,
  splitPaneGap: 1,
  splitPaneWidthInset: 6,
  fullPreviewWidthInset: 4,
  previewTextWidthInset: 8,
  fullPreviewTextWidthInset: 8,
  previewContentAlign: "left",
  fullPreviewContentAlign: "left",
  previewBodyVerticalAlign: "top",
  fullPreviewBodyVerticalAlign: "top",
  imagePreviewRowInset: 3,
  fullPreviewImageRowInset: 3,
  fullPreviewScrollInsetRows: 2,
  previewLineNumberWidth: 3,
  previewGutterWidth: 4,
  fullPreviewGutterWidth: 4,
  previewGutterAlign: "right",
  fullPreviewGutterAlign: "right",
  previewLineSpacing: 0,
  fullPreviewLineSpacing: 0,
  previewMetaHeight: 3,
  fullPreviewMetaHeight: 1,
  previewMetaLineSpacing: 0,
  fullPreviewMetaLineSpacing: 0,
  previewMetaHashLength: 12,
  fullPreviewMetaHashLength: 12,
  previewImageFields: [...defaultPreviewImageFields],
  headerBrandMaxWidth: 0,
  headerFilterMaxWidth: 0,
  headerQueryMaxWidth: 0,
  headerModeMaxWidth: 0,
  statusSeparatorPadding: 2,
  statusSeparatorPaddingLeft: 2,
  statusSeparatorPaddingRight: 2,
  statusOperationMaxWidth: 0,
  statusWatcherMaxWidth: 0,
  statusHintMaxWidth: 0,
  searchOverlayPromptMaxWidth: 0,
  searchOverlayQueryMaxWidth: 0,
  searchOverlayCursorMaxWidth: 0,
  dangerOverlayPromptMaxWidth: 0,
  dangerOverlayHintMaxWidth: 0,
  helpOverlayActionMaxWidth: 0,
  frameTitlePadding: 1,
  frameTitlePaddingLeft: 1,
  frameTitlePaddingRight: 1,
  shellPaddingX: 0,
  shellPaddingY: 0,
  headerPaddingX: 1,
  headerPaddingY: 0,
  statusPaddingX: 1,
  statusPaddingY: 0,
  headerContentAlign: "left",
  statusContentAlign: "left",
  headerVerticalAlign: "top",
  statusVerticalAlign: "top",
  overlayPaddingX: 1,
  overlayPaddingY: 0,
  overlayContentAlign: "left",
  overlayVerticalAlign: "top",
  overlayLineSpacing: 0,
  searchOverlayPaddingX: 1,
  searchOverlayPaddingY: 0,
  searchOverlayContentAlign: "left",
  searchOverlayVerticalAlign: "top",
  searchOverlayLineSpacing: 0,
  dangerOverlayPaddingX: 1,
  dangerOverlayPaddingY: 0,
  dangerOverlayContentAlign: "left",
  dangerOverlayVerticalAlign: "top",
  dangerOverlayLineSpacing: 0,
  helpOverlayPaddingX: 1,
  helpOverlayPaddingY: 0,
  helpOverlayContentAlign: "left",
  helpOverlayVerticalAlign: "top",
  helpOverlayLineSpacing: 0,
  listPaddingX: 1,
  listPaddingY: 0,
  previewPaddingX: 1,
  previewPaddingY: 0,
  fullPreviewPaddingX: 1,
  fullPreviewPaddingY: 0,
  previewMetaPaddingX: 1,
  previewMetaPaddingY: 0,
  fullPreviewMetaPaddingX: 0,
  fullPreviewMetaPaddingY: 0,
  previewMetaContentAlign: "left",
  fullPreviewMetaContentAlign: "left",
  previewMetaVerticalAlign: "top",
  fullPreviewMetaVerticalAlign: "top",
  emptyStatePaddingX: 1,
  emptyStatePaddingY: 1,
  emptyStateContentAlign: "left",
  emptyStateTitleAlign: "left",
  emptyStateHelpAlign: "left",
  emptyStateVerticalAlign: "top",
  emptyStateLineSpacing: 0,
  helpKeyWidth: 24,
  helpKeyAlign: "left",
  confirmHintIndent: 2,
  rowContentAlign: "left",
  rowMetadataAlign: "left",
  rowPreviewAlign: "left",
  rowAgeWidth: 2,
  rowAgeAlign: "right",
  rowSizeWidth: 6,
  rowSizeAlign: "right",
  rowPinnedWidth: 3,
  rowPinnedAlign: "left",
  rowMetaHashLength: 8,
  rowMarkerWidth: 0,
  rowMarkerAlign: "left",
  rowMarkerGap: 1,
  rowMetaPreviewGap: 2,
  rowPreviewReservedWidth: 20,
  rowPreviewMaxWidth: 0,
  rowSpacing: 0,
  alternateRows: true,
  refreshIntervalMs: 1000,
  mouseEnabled: true,
  mouseScrollRows: 3,
  showScrollbar: true,
  scrollbarWidth: 1,
  scrollbarPlacement: "right",
  scrollbarAlign: "left",
  showMetadata: true,
  showRowMetadata: true,
  showPreviewPane: true,
  showFullPreviewMetadata: true,
  showPreviewGutter: true,
  showFullPreviewGutter: true,
  highlightSearchMatches: true,
  showEmptyStateHelp: true,
  panelPaddingX: 1,
  panelPaddingY: 0,
};

export const compactLayoutDefaults: Partial<UiConfig> = {
  maxPreviewLines: 18,
  helpOverlayHeight: 10,
  minPaneWidth: 18,
  splitPaneGap: 0,
  splitPaneWidthInset: 2,
  fullPreviewWidthInset: 2,
  previewTextWidthInset: 4,
  fullPreviewTextWidthInset: 4,
  imagePreviewRowInset: 4,
  fullPreviewImageRowInset: 4,
  fullPreviewScrollInsetRows: 1,
  previewGutterWidth: 3,
  fullPreviewGutterWidth: 3,
  previewMetaHeight: 2,
  fullPreviewMetaHeight: 1,
  previewMetaLineSpacing: 0,
  fullPreviewMetaLineSpacing: 0,
  fullPreviewMetaHashLength: 12,
  headerBrandMaxWidth: 0,
  headerFilterMaxWidth: 0,
  headerQueryMaxWidth: 0,
  headerModeMaxWidth: 0,
  imagePreviewNoticeSpacing: 0,
  fullPreviewImageNoticeSpacing: 0,
  statusSeparatorPadding: 1,
  statusSeparatorPaddingLeft: 1,
  statusSeparatorPaddingRight: 1,
  statusOperationMaxWidth: 0,
  statusWatcherMaxWidth: 0,
  statusHintMaxWidth: 0,
  searchOverlayPromptMaxWidth: 0,
  searchOverlayQueryMaxWidth: 0,
  searchOverlayCursorMaxWidth: 0,
  dangerOverlayPromptMaxWidth: 0,
  dangerOverlayHintMaxWidth: 0,
  helpOverlayActionMaxWidth: 0,
  frameTitlePadding: 0,
  frameTitlePaddingLeft: 0,
  frameTitlePaddingRight: 0,
  overlayPaddingX: 0,
  searchOverlayPaddingX: 0,
  dangerOverlayPaddingX: 0,
  helpOverlayPaddingX: 0,
  listPaddingX: 0,
  previewPaddingX: 0,
  fullPreviewPaddingX: 0,
  previewMetaPaddingX: 0,
  previewMetaPaddingY: 0,
  fullPreviewMetaPaddingX: 0,
  fullPreviewMetaPaddingY: 0,
  emptyStatePaddingX: 0,
  emptyStatePaddingY: 0,
  emptyStateLineSpacing: 0,
  helpKeyWidth: 18,
  rowSizeWidth: 5,
  rowMarkerWidth: 0,
  rowMetaPreviewGap: 1,
  rowPreviewReservedWidth: 20,
  showEmptyStateHelp: false,
  panelPaddingX: 0,
};

