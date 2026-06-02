import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { createTextAttributes } from "@opentui/core";
import { themeNames, themes, type ThemeName, type TuiTheme } from "./theme";
import type {
  ContentAlign,
  ImagePreviewAlign,
  ImagePreviewMode,
  ImagePreviewNoticeVisibility,
  ImagePreviewRenderer,
  OverlayPlacement,
  PreviewImageField,
  ScrollbarPlacement,
  UiConfig,
  VerticalAlign,
} from "./ui-config";
import type { EntryFilter } from "./types";

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

type EnvMap = Record<string, string | undefined>;

const defaultChrome: TuiChromeConfig = {
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
  selectedMarker: ">",
  selectedMarkedMarker: "*",
  markedMarker: "+",
  normalMarker: "|",
  scrollbarThumb: "#",
  scrollbarTrack: "|",
  statusSeparator: "|",
};

const defaultTerminal: TuiTerminalConfig = {
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

const defaultKeyLabels: KeyDisplayLabels = {
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

const defaultStatusTones: TuiStatusToneMatchers = {
  error: ["error", "failed", "not found", "exited", "unavailable"],
  success: ["copied", "pasted", "cleared", "pinned", "unpinned"],
  warning: ["paused"],
};

const defaultStatusLineTones: TuiStatusLineTones = {
  operation: "auto",
  watcher: "auto",
  hint: "muted",
  separator: "muted",
};

const defaultHeaderLineTones: TuiHeaderLineTones = {
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

const defaultOverlayBorderTones: TuiOverlayBorderTones = {
  search: "search",
  danger: "error",
  command: "border",
};

const defaultOverlayContentTones: TuiOverlayContentTones = {
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

const defaultListContentTones: TuiListContentTones = {
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

const defaultPreviewContentTones: TuiPreviewContentTones = {
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

const defaultLabels: TuiLabels = {
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

const defaultKeyBindings: TuiKeyBindings = {
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

const defaultFilterOrder: EntryFilter[] = ["all", "text", "images", "favorites", "today"];
const defaultHelpOrder: TuiHelpActionName[] = [
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
const defaultPreviewImageFields: PreviewImageField[] = ["type", "mime", "size", "dimensions", "hash", "blob"];
const defaultStartup: TuiStartupConfig = {
  filter: "all",
  pinnedOnly: false,
  query: "",
};
const defaultBehavior: TuiBehaviorConfig = {
  liveSearch: true,
  liveSearchDebounceMs: 120,
  clearQueryOnSearchOpen: true,
  restoreQueryOnSearchCancel: true,
  exitAfterPaste: true,
  exitAfterCopy: false,
  exitAfterBulkCopy: false,
  exitAfterSearchCopy: false,
};

const compatKeyBindingAliases: Record<CompatKeyBindingAlias, keyof TuiKeyBindings> = {
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

const defaultLayout: UiConfig = {
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

const compactLayoutDefaults: Partial<UiConfig> = {
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

export function currentTuiConfig(): ResolvedTuiConfig {
  return loadTuiConfig(Bun.env);
}

export function loadTuiConfig(
  env: EnvMap = Bun.env,
  readFile: (path: string) => string = (path) => readFileSync(path, "utf8"),
): ResolvedTuiConfig {
  const sourcePath = configPath(env);
  let fileConfig: TuiConfigFile = {};
  try {
    fileConfig = JSON.parse(readFile(sourcePath)) as TuiConfigFile;
  } catch {
    fileConfig = {};
  }
  fileConfig = mergeCompatThemeFile(fileConfig, sourcePath, env, readFile);
  return resolveTuiConfig(fileConfig, env, sourcePath);
}

export function resolveTuiConfig(fileConfig: TuiConfigFile = {}, env: EnvMap = {}, sourcePath: string | null = null): ResolvedTuiConfig {
  const preset = themePreset(fileConfig.theme, env);
  const theme = mergeTheme(preset, typeof fileConfig.theme === "object" ? fileConfig.theme.colors : undefined);
  const terminal = resolveTerminalConfig(fileConfig.terminal, env);
  const compactMode = envBool(env, "DITOX_TUI_COMPACT", boolValue(fileConfig.layout?.compactMode, defaultLayout.compactMode));
  const configuredListWidth = splitListWidth(fileConfig.layout);
  const envPreviewWidth = envNumber(env, "DITOX_TUI_PREVIEW_WIDTH", Number.NaN);
  const listWidthFallback = Number.isFinite(envPreviewWidth) ? 100 - envPreviewWidth : configuredListWidth;
  const listWidthPercent = clampNumber(envNumber(env, "DITOX_TUI_LIST_WIDTH", listWidthFallback), 32, 68);
  const showMetadata = envBool(env, "DITOX_TUI_METADATA", boolValue(fileConfig.layout?.showMetadata, boolValue(fileConfig.enableDescription, defaultLayout.showMetadata)));
  const compactFallback = <K extends keyof UiConfig>(key: K): UiConfig[K] =>
    compactMode && compactLayoutDefaults[key] !== undefined ? (compactLayoutDefaults[key] as UiConfig[K]) : defaultLayout[key];
  const panelPaddingX = clampNumber(numberValue(fileConfig.layout?.panelPaddingX, compactFallback("panelPaddingX")), 0, 4);
  const panelPaddingY = clampNumber(numberValue(fileConfig.layout?.panelPaddingY, defaultLayout.panelPaddingY), 0, 2);
  const overlayPaddingX = clampNumber(numberValue(fileConfig.layout?.overlayPaddingX, compactFallback("overlayPaddingX")), 0, 4);
  const overlayPaddingY = clampNumber(numberValue(fileConfig.layout?.overlayPaddingY, defaultLayout.overlayPaddingY), 0, 3);
  const overlayContentAlign = contentAlignValue(env.DITOX_TUI_OVERLAY_CONTENT_ALIGN ?? fileConfig.layout?.overlayContentAlign, defaultLayout.overlayContentAlign);
  const overlayVerticalAlign = verticalAlignValue(
    env.DITOX_TUI_OVERLAY_VERTICAL_ALIGN ?? fileConfig.layout?.overlayVerticalAlign,
    defaultLayout.overlayVerticalAlign,
  );
  const overlayLineSpacing = clampNumber(
    envNumber(env, "DITOX_TUI_OVERLAY_LINE_SPACING", numberValue(fileConfig.layout?.overlayLineSpacing, defaultLayout.overlayLineSpacing)),
    0,
    3,
  );
  const emptyStateContentAlign = contentAlignValue(
    env.DITOX_TUI_EMPTY_STATE_ALIGN ?? fileConfig.layout?.emptyStateContentAlign,
    defaultLayout.emptyStateContentAlign,
  );
  const previewGutterWidth = clampNumber(numberValue(fileConfig.layout?.previewGutterWidth, compactFallback("previewGutterWidth")), 1, 12);
  const previewGutterAlign = contentAlignValue(env.DITOX_TUI_PREVIEW_GUTTER_ALIGN ?? fileConfig.layout?.previewGutterAlign, defaultLayout.previewGutterAlign);
  const previewTextWidthInset = clampNumber(numberValue(fileConfig.layout?.previewTextWidthInset, compactFallback("previewTextWidthInset")), 0, 24);
  const previewContentAlign = contentAlignValue(env.DITOX_TUI_PREVIEW_CONTENT_ALIGN ?? fileConfig.layout?.previewContentAlign, defaultLayout.previewContentAlign);
  const statusSeparatorPadding = clampNumber(
    envNumber(env, "DITOX_TUI_STATUS_SEPARATOR_PADDING", numberValue(fileConfig.layout?.statusSeparatorPadding, compactFallback("statusSeparatorPadding"))),
    0,
    6,
  );
  const statusSeparatorPaddingLeft = clampNumber(
    envNumber(env, "DITOX_TUI_STATUS_SEPARATOR_PADDING_LEFT", numberValue(fileConfig.layout?.statusSeparatorPaddingLeft, statusSeparatorPadding)),
    0,
    6,
  );
  const statusSeparatorPaddingRight = clampNumber(
    envNumber(env, "DITOX_TUI_STATUS_SEPARATOR_PADDING_RIGHT", numberValue(fileConfig.layout?.statusSeparatorPaddingRight, statusSeparatorPadding)),
    0,
    6,
  );
  const previewMetaLineSpacing = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_PREVIEW_META_LINE_SPACING",
      numberValue(fileConfig.layout?.previewMetaLineSpacing, defaultLayout.previewMetaLineSpacing),
    ),
    0,
    2,
  );
  const fullPreviewMetaLineSpacing = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_FULL_PREVIEW_META_LINE_SPACING",
      numberValue(fileConfig.layout?.fullPreviewMetaLineSpacing, previewMetaLineSpacing),
    ),
    0,
    2,
  );
  const previewMetaHashLength = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_PREVIEW_META_HASH_LENGTH",
      numberValue(fileConfig.layout?.previewMetaHashLength, defaultLayout.previewMetaHashLength),
    ),
    6,
    64,
  );
  const fullPreviewMetaHashLength = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_FULL_PREVIEW_META_HASH_LENGTH",
      numberValue(fileConfig.layout?.fullPreviewMetaHashLength, previewMetaHashLength),
    ),
    6,
    64,
  );
  const frameTitlePadding = clampNumber(
    envNumber(env, "DITOX_TUI_TITLE_PADDING", numberValue(fileConfig.layout?.frameTitlePadding, compactFallback("frameTitlePadding"))),
    0,
    4,
  );
  const frameTitlePaddingLeft = clampNumber(
    envNumber(env, "DITOX_TUI_TITLE_PADDING_LEFT", numberValue(fileConfig.layout?.frameTitlePaddingLeft, frameTitlePadding)),
    0,
    4,
  );
  const frameTitlePaddingRight = clampNumber(
    envNumber(env, "DITOX_TUI_TITLE_PADDING_RIGHT", numberValue(fileConfig.layout?.frameTitlePaddingRight, frameTitlePadding)),
    0,
    4,
  );
  const fullPreviewTextWidthInset = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_FULL_PREVIEW_TEXT_WIDTH_INSET",
      numberValue(fileConfig.layout?.fullPreviewTextWidthInset, numberValue(fileConfig.layout?.previewTextWidthInset, compactFallback("fullPreviewTextWidthInset"))),
    ),
    0,
    24,
  );
  const fullPreviewContentAlign = contentAlignValue(
    env.DITOX_TUI_FULL_PREVIEW_CONTENT_ALIGN ?? fileConfig.layout?.fullPreviewContentAlign,
    previewContentAlign,
  );
  const previewBodyVerticalAlign = verticalAlignValue(
    env.DITOX_TUI_PREVIEW_BODY_VERTICAL_ALIGN ?? fileConfig.layout?.previewBodyVerticalAlign,
    defaultLayout.previewBodyVerticalAlign,
  );
  const imagePreviewRowInset = clampNumber(numberValue(fileConfig.layout?.imagePreviewRowInset, compactFallback("imagePreviewRowInset")), 0, 20);
  const imagePreviewMode = imagePreviewModeValue(
    env.DITOX_TUI_IMAGE_PREVIEW,
    fileConfig.layout?.imagePreviewMode,
    compatImagePreviewMode(fileConfig.imageDisplay, defaultLayout.imagePreviewMode),
  );
  const compatImageMaxWidth = compatImagePreviewMaxWidth(fileConfig.imageDisplay, defaultLayout.imagePreviewMaxWidth);
  const hasSplitImageMaxWidthOverride =
    env.DITOX_TUI_IMAGE_PREVIEW_MAX_WIDTH !== undefined || fileConfig.layout?.imagePreviewMaxWidth !== undefined || compatImageMaxWidth !== defaultLayout.imagePreviewMaxWidth;
  const imagePreviewMaxWidth = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_IMAGE_PREVIEW_MAX_WIDTH",
      numberValue(fileConfig.layout?.imagePreviewMaxWidth, compatImageMaxWidth),
    ),
    8,
    120,
  );
  const compatImageMaxRows = compatImagePreviewMaxRows(fileConfig.imageDisplay, defaultLayout.imagePreviewMaxRows);
  const hasSplitImageMaxRowsOverride =
    env.DITOX_TUI_IMAGE_PREVIEW_MAX_ROWS !== undefined || fileConfig.layout?.imagePreviewMaxRows !== undefined || compatImageMaxRows !== defaultLayout.imagePreviewMaxRows;
  const imagePreviewMaxRows = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_IMAGE_PREVIEW_MAX_ROWS",
      numberValue(fileConfig.layout?.imagePreviewMaxRows, compatImageMaxRows),
    ),
    2,
    60,
  );
  const imagePreviewRenderer = imagePreviewRendererValue(
    env.DITOX_TUI_IMAGE_RENDERER ?? fileConfig.layout?.imagePreviewRenderer,
    defaultLayout.imagePreviewRenderer,
  );
  const imagePreviewAlign = imagePreviewAlignValue(env.DITOX_TUI_IMAGE_ALIGN ?? fileConfig.layout?.imagePreviewAlign, defaultLayout.imagePreviewAlign);
  const imagePreviewBlockGlyph = glyphValue(env.DITOX_TUI_IMAGE_BLOCK_GLYPH ?? fileConfig.layout?.imagePreviewBlockGlyph, defaultLayout.imagePreviewBlockGlyph);
  const imagePreviewBackground = imagePreviewBackgroundValue(env.DITOX_TUI_IMAGE_BACKGROUND ?? fileConfig.layout?.imagePreviewBackground, defaultLayout.imagePreviewBackground);
  const imagePreviewNoticeVisibility = imagePreviewNoticeVisibilityValue(
    env.DITOX_TUI_IMAGE_NOTICE ?? fileConfig.layout?.imagePreviewNoticeVisibility,
    defaultLayout.imagePreviewNoticeVisibility,
  );
  const imagePreviewNoticeSpacing = clampNumber(
    envNumber(
      env,
      "DITOX_TUI_IMAGE_NOTICE_SPACING",
      numberValue(fileConfig.layout?.imagePreviewNoticeSpacing, compactFallback("imagePreviewNoticeSpacing")),
    ),
    0,
    4,
  );
  const layout: UiConfig = {
    compactMode,
    listWidthPercent,
    previewWidthPercent: 100 - listWidthPercent,
    maxPreviewLines: clampNumber(
      envNumber(env, "DITOX_TUI_MAX_PREVIEW_LINES", numberValue(fileConfig.layout?.maxPreviewLines, compactFallback("maxPreviewLines"))),
      8,
      120,
    ),
    maxFullPreviewLines: clampNumber(
      envNumber(env, "DITOX_TUI_MAX_FULL_PREVIEW_LINES", numberValue(fileConfig.layout?.maxFullPreviewLines, defaultLayout.maxFullPreviewLines)),
      24,
      10000,
    ),
    historyLimit: clampNumber(envNumber(env, "DITOX_TUI_HISTORY_LIMIT", numberValue(fileConfig.layout?.historyLimit, defaultLayout.historyLimit)), 1, 500),
    imagePreviewMode,
    fullPreviewImageMode: imagePreviewModeValue(
      env.DITOX_TUI_FULL_PREVIEW_IMAGE_MODE,
      fileConfig.layout?.fullPreviewImageMode,
      imagePreviewMode,
    ),
    imagePreviewMaxWidth,
    imagePreviewMaxRows,
    fullPreviewImageMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_FULL_PREVIEW_IMAGE_MAX_WIDTH",
        numberValue(fileConfig.layout?.fullPreviewImageMaxWidth, hasSplitImageMaxWidthOverride ? imagePreviewMaxWidth : defaultLayout.fullPreviewImageMaxWidth),
      ),
      8,
      120,
    ),
    fullPreviewImageMaxRows: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_FULL_PREVIEW_IMAGE_MAX_ROWS",
        numberValue(fileConfig.layout?.fullPreviewImageMaxRows, hasSplitImageMaxRowsOverride ? imagePreviewMaxRows : defaultLayout.fullPreviewImageMaxRows),
      ),
      2,
      60,
    ),
    imagePreviewRenderer,
    fullPreviewImageRenderer: imagePreviewRendererValue(env.DITOX_TUI_FULL_PREVIEW_IMAGE_RENDERER ?? fileConfig.layout?.fullPreviewImageRenderer, imagePreviewRenderer),
    imagePreviewAlign,
    fullPreviewImageAlign: imagePreviewAlignValue(env.DITOX_TUI_FULL_PREVIEW_IMAGE_ALIGN ?? fileConfig.layout?.fullPreviewImageAlign, imagePreviewAlign),
    imagePreviewBlockGlyph,
    fullPreviewImageBlockGlyph: glyphValue(env.DITOX_TUI_FULL_PREVIEW_IMAGE_BLOCK_GLYPH ?? fileConfig.layout?.fullPreviewImageBlockGlyph, imagePreviewBlockGlyph),
    imagePreviewBackground,
    fullPreviewImageBackground: imagePreviewBackgroundValue(env.DITOX_TUI_FULL_PREVIEW_IMAGE_BACKGROUND ?? fileConfig.layout?.fullPreviewImageBackground, imagePreviewBackground),
    imagePreviewNoticeVisibility,
    fullPreviewImageNoticeVisibility: imagePreviewNoticeVisibilityValue(
      env.DITOX_TUI_FULL_PREVIEW_IMAGE_NOTICE ?? fileConfig.layout?.fullPreviewImageNoticeVisibility,
      imagePreviewNoticeVisibility,
    ),
    imagePreviewNoticeSpacing,
    fullPreviewImageNoticeSpacing: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_FULL_PREVIEW_IMAGE_NOTICE_SPACING",
        numberValue(fileConfig.layout?.fullPreviewImageNoticeSpacing, imagePreviewNoticeSpacing),
      ),
      0,
      4,
    ),
    headerHeight: clampNumber(numberValue(fileConfig.layout?.headerHeight, defaultLayout.headerHeight), 1, 6),
    statusHeight: clampNumber(numberValue(fileConfig.layout?.statusHeight, defaultLayout.statusHeight), 1, 3),
    showHeader: envBool(env, "DITOX_TUI_HEADER", boolValue(fileConfig.layout?.showHeader, defaultLayout.showHeader)),
    showStatusLine: envBool(env, "DITOX_TUI_STATUS_LINE", boolValue(fileConfig.layout?.showStatusLine, defaultLayout.showStatusLine)),
    searchOverlayHeight: clampNumber(numberValue(fileConfig.layout?.searchOverlayHeight, defaultLayout.searchOverlayHeight), 1, 8),
    confirmOverlayHeight: clampNumber(numberValue(fileConfig.layout?.confirmOverlayHeight, defaultLayout.confirmOverlayHeight), 1, 8),
    confirmPinnedExtraRows: clampNumber(numberValue(fileConfig.layout?.confirmPinnedExtraRows, defaultLayout.confirmPinnedExtraRows), 0, 4),
    clearOverlayHeight: clampNumber(numberValue(fileConfig.layout?.clearOverlayHeight, defaultLayout.clearOverlayHeight), 1, 8),
    helpOverlayHeight: clampNumber(numberValue(fileConfig.layout?.helpOverlayHeight, compactFallback("helpOverlayHeight")), 8, 40),
    overlayPlacement: overlayPlacementValue(env.DITOX_TUI_OVERLAY_PLACEMENT ?? fileConfig.layout?.overlayPlacement, defaultLayout.overlayPlacement),
    minPaneWidth: clampNumber(numberValue(fileConfig.layout?.minPaneWidth, compactFallback("minPaneWidth")), 12, 80),
    splitPaneGap: clampNumber(envNumber(env, "DITOX_TUI_SPLIT_PANE_GAP", numberValue(fileConfig.layout?.splitPaneGap, compactFallback("splitPaneGap"))), 0, 8),
    splitPaneWidthInset: clampNumber(numberValue(fileConfig.layout?.splitPaneWidthInset, compactFallback("splitPaneWidthInset")), 0, 20),
    fullPreviewWidthInset: clampNumber(numberValue(fileConfig.layout?.fullPreviewWidthInset, compactFallback("fullPreviewWidthInset")), 0, 20),
    previewTextWidthInset,
    fullPreviewTextWidthInset,
    previewContentAlign,
    fullPreviewContentAlign,
    previewBodyVerticalAlign,
    fullPreviewBodyVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_FULL_PREVIEW_BODY_VERTICAL_ALIGN ?? fileConfig.layout?.fullPreviewBodyVerticalAlign,
      previewBodyVerticalAlign,
    ),
    imagePreviewRowInset,
    fullPreviewImageRowInset: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_FULL_PREVIEW_IMAGE_ROW_INSET",
        numberValue(fileConfig.layout?.fullPreviewImageRowInset, numberValue(fileConfig.layout?.imagePreviewRowInset, compactFallback("fullPreviewImageRowInset"))),
      ),
      0,
      20,
    ),
    fullPreviewScrollInsetRows: clampNumber(numberValue(fileConfig.layout?.fullPreviewScrollInsetRows, compactFallback("fullPreviewScrollInsetRows")), 0, 12),
    previewLineNumberWidth: clampNumber(numberValue(fileConfig.layout?.previewLineNumberWidth, defaultLayout.previewLineNumberWidth), 1, 8),
    previewGutterWidth,
    fullPreviewGutterWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_FULL_PREVIEW_GUTTER_WIDTH",
        numberValue(fileConfig.layout?.fullPreviewGutterWidth, numberValue(fileConfig.layout?.previewGutterWidth, compactFallback("fullPreviewGutterWidth"))),
      ),
      1,
      12,
    ),
    previewGutterAlign,
    fullPreviewGutterAlign: contentAlignValue(
      env.DITOX_TUI_FULL_PREVIEW_GUTTER_ALIGN ?? fileConfig.layout?.fullPreviewGutterAlign,
      previewGutterAlign,
    ),
    previewLineSpacing: clampNumber(numberValue(fileConfig.layout?.previewLineSpacing, defaultLayout.previewLineSpacing), 0, 3),
    fullPreviewLineSpacing: clampNumber(numberValue(fileConfig.layout?.fullPreviewLineSpacing, defaultLayout.fullPreviewLineSpacing), 0, 3),
    previewMetaHeight: clampNumber(
      envNumber(env, "DITOX_TUI_PREVIEW_META_HEIGHT", numberValue(fileConfig.layout?.previewMetaHeight, compactFallback("previewMetaHeight"))),
      1,
      4,
    ),
    fullPreviewMetaHeight: clampNumber(
      envNumber(env, "DITOX_TUI_FULL_PREVIEW_META_HEIGHT", numberValue(fileConfig.layout?.fullPreviewMetaHeight, compactFallback("fullPreviewMetaHeight"))),
      1,
      4,
    ),
    previewMetaLineSpacing,
    fullPreviewMetaLineSpacing,
    previewMetaHashLength,
    fullPreviewMetaHashLength,
    previewImageFields: previewImageFieldsValue(fileConfig.layout?.previewImageFields),
    headerBrandMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_HEADER_BRAND_MAX_WIDTH", numberValue(fileConfig.layout?.headerBrandMaxWidth, defaultLayout.headerBrandMaxWidth)),
      0,
      200,
    ),
    headerFilterMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_HEADER_FILTER_MAX_WIDTH", numberValue(fileConfig.layout?.headerFilterMaxWidth, defaultLayout.headerFilterMaxWidth)),
      0,
      200,
    ),
    headerQueryMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_HEADER_QUERY_MAX_WIDTH", numberValue(fileConfig.layout?.headerQueryMaxWidth, defaultLayout.headerQueryMaxWidth)),
      0,
      200,
    ),
    headerModeMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_HEADER_MODE_MAX_WIDTH", numberValue(fileConfig.layout?.headerModeMaxWidth, defaultLayout.headerModeMaxWidth)),
      0,
      200,
    ),
    statusSeparatorPadding,
    statusSeparatorPaddingLeft,
    statusSeparatorPaddingRight,
    statusOperationMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_STATUS_OPERATION_MAX_WIDTH", numberValue(fileConfig.layout?.statusOperationMaxWidth, defaultLayout.statusOperationMaxWidth)),
      0,
      200,
    ),
    statusWatcherMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_STATUS_WATCHER_MAX_WIDTH", numberValue(fileConfig.layout?.statusWatcherMaxWidth, defaultLayout.statusWatcherMaxWidth)),
      0,
      200,
    ),
    statusHintMaxWidth: clampNumber(
      envNumber(env, "DITOX_TUI_STATUS_HINT_MAX_WIDTH", numberValue(fileConfig.layout?.statusHintMaxWidth, defaultLayout.statusHintMaxWidth)),
      0,
      200,
    ),
    searchOverlayPromptMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_SEARCH_OVERLAY_PROMPT_MAX_WIDTH",
        numberValue(fileConfig.layout?.searchOverlayPromptMaxWidth, defaultLayout.searchOverlayPromptMaxWidth),
      ),
      0,
      200,
    ),
    searchOverlayQueryMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_SEARCH_OVERLAY_QUERY_MAX_WIDTH",
        numberValue(fileConfig.layout?.searchOverlayQueryMaxWidth, defaultLayout.searchOverlayQueryMaxWidth),
      ),
      0,
      200,
    ),
    searchOverlayCursorMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_SEARCH_OVERLAY_CURSOR_MAX_WIDTH",
        numberValue(fileConfig.layout?.searchOverlayCursorMaxWidth, defaultLayout.searchOverlayCursorMaxWidth),
      ),
      0,
      200,
    ),
    dangerOverlayPromptMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_DANGER_OVERLAY_PROMPT_MAX_WIDTH",
        numberValue(fileConfig.layout?.dangerOverlayPromptMaxWidth, defaultLayout.dangerOverlayPromptMaxWidth),
      ),
      0,
      200,
    ),
    dangerOverlayHintMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_DANGER_OVERLAY_HINT_MAX_WIDTH",
        numberValue(fileConfig.layout?.dangerOverlayHintMaxWidth, defaultLayout.dangerOverlayHintMaxWidth),
      ),
      0,
      200,
    ),
    helpOverlayActionMaxWidth: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_HELP_OVERLAY_ACTION_MAX_WIDTH",
        numberValue(fileConfig.layout?.helpOverlayActionMaxWidth, defaultLayout.helpOverlayActionMaxWidth),
      ),
      0,
      200,
    ),
    frameTitlePadding,
    frameTitlePaddingLeft,
    frameTitlePaddingRight,
    shellPaddingX: clampNumber(envNumber(env, "DITOX_TUI_SHELL_PADDING_X", numberValue(fileConfig.layout?.shellPaddingX, defaultLayout.shellPaddingX)), 0, 6),
    shellPaddingY: clampNumber(envNumber(env, "DITOX_TUI_SHELL_PADDING_Y", numberValue(fileConfig.layout?.shellPaddingY, defaultLayout.shellPaddingY)), 0, 4),
    headerPaddingX: clampNumber(numberValue(fileConfig.layout?.headerPaddingX, defaultLayout.headerPaddingX), 0, 4),
    headerPaddingY: clampNumber(numberValue(fileConfig.layout?.headerPaddingY, defaultLayout.headerPaddingY), 0, 3),
    statusPaddingX: clampNumber(numberValue(fileConfig.layout?.statusPaddingX, defaultLayout.statusPaddingX), 0, 4),
    statusPaddingY: clampNumber(numberValue(fileConfig.layout?.statusPaddingY, defaultLayout.statusPaddingY), 0, 3),
    headerContentAlign: contentAlignValue(env.DITOX_TUI_HEADER_CONTENT_ALIGN ?? fileConfig.layout?.headerContentAlign, defaultLayout.headerContentAlign),
    statusContentAlign: contentAlignValue(env.DITOX_TUI_STATUS_CONTENT_ALIGN ?? fileConfig.layout?.statusContentAlign, defaultLayout.statusContentAlign),
    headerVerticalAlign: verticalAlignValue(env.DITOX_TUI_HEADER_VERTICAL_ALIGN ?? fileConfig.layout?.headerVerticalAlign, defaultLayout.headerVerticalAlign),
    statusVerticalAlign: verticalAlignValue(env.DITOX_TUI_STATUS_VERTICAL_ALIGN ?? fileConfig.layout?.statusVerticalAlign, defaultLayout.statusVerticalAlign),
    overlayPaddingX,
    overlayPaddingY,
    overlayContentAlign,
    overlayVerticalAlign,
    overlayLineSpacing,
    searchOverlayPaddingX: clampNumber(numberValue(fileConfig.layout?.searchOverlayPaddingX, overlayPaddingX), 0, 4),
    searchOverlayPaddingY: clampNumber(numberValue(fileConfig.layout?.searchOverlayPaddingY, overlayPaddingY), 0, 3),
    searchOverlayContentAlign: contentAlignValue(env.DITOX_TUI_SEARCH_OVERLAY_CONTENT_ALIGN ?? fileConfig.layout?.searchOverlayContentAlign, overlayContentAlign),
    searchOverlayVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_SEARCH_OVERLAY_VERTICAL_ALIGN ?? fileConfig.layout?.searchOverlayVerticalAlign,
      overlayVerticalAlign,
    ),
    searchOverlayLineSpacing: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_SEARCH_OVERLAY_LINE_SPACING",
        numberValue(fileConfig.layout?.searchOverlayLineSpacing, overlayLineSpacing),
      ),
      0,
      3,
    ),
    dangerOverlayPaddingX: clampNumber(numberValue(fileConfig.layout?.dangerOverlayPaddingX, overlayPaddingX), 0, 4),
    dangerOverlayPaddingY: clampNumber(numberValue(fileConfig.layout?.dangerOverlayPaddingY, overlayPaddingY), 0, 3),
    dangerOverlayContentAlign: contentAlignValue(env.DITOX_TUI_DANGER_OVERLAY_CONTENT_ALIGN ?? fileConfig.layout?.dangerOverlayContentAlign, overlayContentAlign),
    dangerOverlayVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_DANGER_OVERLAY_VERTICAL_ALIGN ?? fileConfig.layout?.dangerOverlayVerticalAlign,
      overlayVerticalAlign,
    ),
    dangerOverlayLineSpacing: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_DANGER_OVERLAY_LINE_SPACING",
        numberValue(fileConfig.layout?.dangerOverlayLineSpacing, overlayLineSpacing),
      ),
      0,
      3,
    ),
    helpOverlayPaddingX: clampNumber(numberValue(fileConfig.layout?.helpOverlayPaddingX, overlayPaddingX), 0, 4),
    helpOverlayPaddingY: clampNumber(numberValue(fileConfig.layout?.helpOverlayPaddingY, overlayPaddingY), 0, 3),
    helpOverlayContentAlign: contentAlignValue(env.DITOX_TUI_HELP_OVERLAY_CONTENT_ALIGN ?? fileConfig.layout?.helpOverlayContentAlign, overlayContentAlign),
    helpOverlayVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_HELP_OVERLAY_VERTICAL_ALIGN ?? fileConfig.layout?.helpOverlayVerticalAlign,
      overlayVerticalAlign,
    ),
    helpOverlayLineSpacing: clampNumber(
      envNumber(
        env,
        "DITOX_TUI_HELP_OVERLAY_LINE_SPACING",
        numberValue(fileConfig.layout?.helpOverlayLineSpacing, overlayLineSpacing),
      ),
      0,
      3,
    ),
    listPaddingX: clampNumber(numberValue(fileConfig.layout?.listPaddingX, panelPaddingX), 0, 4),
    listPaddingY: clampNumber(numberValue(fileConfig.layout?.listPaddingY, panelPaddingY), 0, 2),
    previewPaddingX: clampNumber(numberValue(fileConfig.layout?.previewPaddingX, panelPaddingX), 0, 4),
    previewPaddingY: clampNumber(numberValue(fileConfig.layout?.previewPaddingY, panelPaddingY), 0, 2),
    fullPreviewPaddingX: clampNumber(numberValue(fileConfig.layout?.fullPreviewPaddingX, panelPaddingX), 0, 4),
    fullPreviewPaddingY: clampNumber(numberValue(fileConfig.layout?.fullPreviewPaddingY, panelPaddingY), 0, 2),
    previewMetaPaddingX: clampNumber(numberValue(fileConfig.layout?.previewMetaPaddingX, compactFallback("previewMetaPaddingX")), 0, 4),
    previewMetaPaddingY: clampNumber(numberValue(fileConfig.layout?.previewMetaPaddingY, compactFallback("previewMetaPaddingY")), 0, 2),
    fullPreviewMetaPaddingX: clampNumber(
      envNumber(env, "DITOX_TUI_FULL_PREVIEW_META_PADDING_X", numberValue(fileConfig.layout?.fullPreviewMetaPaddingX, compactFallback("fullPreviewMetaPaddingX"))),
      0,
      4,
    ),
    fullPreviewMetaPaddingY: clampNumber(numberValue(fileConfig.layout?.fullPreviewMetaPaddingY, compactFallback("fullPreviewMetaPaddingY")), 0, 2),
    previewMetaContentAlign: contentAlignValue(
      env.DITOX_TUI_PREVIEW_META_CONTENT_ALIGN ?? fileConfig.layout?.previewMetaContentAlign,
      previewContentAlign,
    ),
    fullPreviewMetaContentAlign: contentAlignValue(
      env.DITOX_TUI_FULL_PREVIEW_META_CONTENT_ALIGN ?? fileConfig.layout?.fullPreviewMetaContentAlign,
      fullPreviewContentAlign,
    ),
    previewMetaVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_PREVIEW_META_VERTICAL_ALIGN ?? fileConfig.layout?.previewMetaVerticalAlign,
      defaultLayout.previewMetaVerticalAlign,
    ),
    fullPreviewMetaVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_FULL_PREVIEW_META_VERTICAL_ALIGN ?? fileConfig.layout?.fullPreviewMetaVerticalAlign,
      verticalAlignValue(
        env.DITOX_TUI_PREVIEW_META_VERTICAL_ALIGN ?? fileConfig.layout?.previewMetaVerticalAlign,
        defaultLayout.previewMetaVerticalAlign,
      ),
    ),
    emptyStatePaddingX: clampNumber(numberValue(fileConfig.layout?.emptyStatePaddingX, compactFallback("emptyStatePaddingX")), 0, 4),
    emptyStatePaddingY: clampNumber(numberValue(fileConfig.layout?.emptyStatePaddingY, compactFallback("emptyStatePaddingY")), 0, 2),
    emptyStateContentAlign,
    emptyStateTitleAlign: contentAlignValue(
      env.DITOX_TUI_EMPTY_STATE_TITLE_ALIGN ?? fileConfig.layout?.emptyStateTitleAlign,
      emptyStateContentAlign,
    ),
    emptyStateHelpAlign: contentAlignValue(
      env.DITOX_TUI_EMPTY_STATE_HELP_ALIGN ?? fileConfig.layout?.emptyStateHelpAlign,
      emptyStateContentAlign,
    ),
    emptyStateVerticalAlign: verticalAlignValue(
      env.DITOX_TUI_EMPTY_STATE_VERTICAL_ALIGN ?? fileConfig.layout?.emptyStateVerticalAlign,
      defaultLayout.emptyStateVerticalAlign,
    ),
    emptyStateLineSpacing: clampNumber(
      envNumber(env, "DITOX_TUI_EMPTY_STATE_LINE_SPACING", numberValue(fileConfig.layout?.emptyStateLineSpacing, defaultLayout.emptyStateLineSpacing)),
      0,
      4,
    ),
    helpKeyWidth: clampNumber(numberValue(fileConfig.layout?.helpKeyWidth, compactFallback("helpKeyWidth")), 8, 48),
    helpKeyAlign: contentAlignValue(env.DITOX_TUI_HELP_KEY_ALIGN ?? fileConfig.layout?.helpKeyAlign, defaultLayout.helpKeyAlign),
    confirmHintIndent: clampNumber(numberValue(fileConfig.layout?.confirmHintIndent, defaultLayout.confirmHintIndent), 0, 8),
    rowContentAlign: contentAlignValue(env.DITOX_TUI_ROW_CONTENT_ALIGN ?? fileConfig.layout?.rowContentAlign, defaultLayout.rowContentAlign),
    rowMetadataAlign: contentAlignValue(env.DITOX_TUI_ROW_METADATA_ALIGN ?? fileConfig.layout?.rowMetadataAlign, defaultLayout.rowMetadataAlign),
    rowPreviewAlign: contentAlignValue(env.DITOX_TUI_ROW_PREVIEW_ALIGN ?? fileConfig.layout?.rowPreviewAlign, defaultLayout.rowPreviewAlign),
    rowAgeWidth: clampNumber(numberValue(fileConfig.layout?.rowAgeWidth, defaultLayout.rowAgeWidth), 0, 12),
    rowAgeAlign: contentAlignValue(env.DITOX_TUI_ROW_AGE_ALIGN ?? fileConfig.layout?.rowAgeAlign, defaultLayout.rowAgeAlign),
    rowSizeWidth: clampNumber(numberValue(fileConfig.layout?.rowSizeWidth, compactFallback("rowSizeWidth")), 0, 16),
    rowSizeAlign: contentAlignValue(env.DITOX_TUI_ROW_SIZE_ALIGN ?? fileConfig.layout?.rowSizeAlign, defaultLayout.rowSizeAlign),
    rowPinnedWidth: clampNumber(numberValue(fileConfig.layout?.rowPinnedWidth, defaultLayout.rowPinnedWidth), 0, 12),
    rowPinnedAlign: contentAlignValue(env.DITOX_TUI_ROW_PINNED_ALIGN ?? fileConfig.layout?.rowPinnedAlign, defaultLayout.rowPinnedAlign),
    rowMetaHashLength: clampNumber(numberValue(fileConfig.layout?.rowMetaHashLength, defaultLayout.rowMetaHashLength), 4, 64),
    rowMarkerWidth: clampNumber(
      envNumber(env, "DITOX_TUI_ROW_MARKER_WIDTH", numberValue(fileConfig.layout?.rowMarkerWidth, defaultLayout.rowMarkerWidth)),
      0,
      12,
    ),
    rowMarkerAlign: contentAlignValue(env.DITOX_TUI_ROW_MARKER_ALIGN ?? fileConfig.layout?.rowMarkerAlign, defaultLayout.rowMarkerAlign),
    rowMarkerGap: clampNumber(numberValue(fileConfig.layout?.rowMarkerGap, defaultLayout.rowMarkerGap), 0, 4),
    rowMetaPreviewGap: clampNumber(numberValue(fileConfig.layout?.rowMetaPreviewGap, compactFallback("rowMetaPreviewGap")), 0, 8),
    rowPreviewReservedWidth: clampNumber(
      numberValue(fileConfig.layout?.rowPreviewReservedWidth, compactFallback("rowPreviewReservedWidth")),
      8,
      80,
    ),
    rowPreviewMaxWidth: clampNumber(
      numberValue(fileConfig.layout?.rowPreviewMaxWidth, numberValue(fileConfig.maxEntryLength, defaultLayout.rowPreviewMaxWidth)),
      0,
      240,
    ),
    rowSpacing: clampNumber(numberValue(fileConfig.layout?.rowSpacing, defaultLayout.rowSpacing), 0, 3),
    alternateRows: envBool(env, "DITOX_TUI_ALTERNATE_ROWS", boolValue(fileConfig.layout?.alternateRows, defaultLayout.alternateRows)),
    refreshIntervalMs: clampNumber(
      envNumber(env, "DITOX_TUI_REFRESH_MS", numberValue(fileConfig.layout?.refreshIntervalMs, numberValue(fileConfig.pollInterval, defaultLayout.refreshIntervalMs))),
      0,
      60000,
    ),
    mouseEnabled: envBool(env, "DITOX_TUI_MOUSE", boolValue(fileConfig.layout?.mouseEnabled, boolValue(fileConfig.enableMouse, defaultLayout.mouseEnabled))),
    mouseScrollRows: clampNumber(
      envNumber(env, "DITOX_TUI_MOUSE_SCROLL_ROWS", numberValue(fileConfig.layout?.mouseScrollRows, defaultLayout.mouseScrollRows)),
      1,
      20,
    ),
    showScrollbar: envBool(env, "DITOX_TUI_SCROLLBAR", boolValue(fileConfig.layout?.showScrollbar, defaultLayout.showScrollbar)),
    scrollbarWidth: clampNumber(
      envNumber(env, "DITOX_TUI_SCROLLBAR_WIDTH", numberValue(fileConfig.layout?.scrollbarWidth, defaultLayout.scrollbarWidth)),
      1,
      4,
    ),
    scrollbarPlacement: scrollbarPlacementValue(env.DITOX_TUI_SCROLLBAR_PLACEMENT ?? fileConfig.layout?.scrollbarPlacement, defaultLayout.scrollbarPlacement),
    scrollbarAlign: contentAlignValue(env.DITOX_TUI_SCROLLBAR_ALIGN ?? fileConfig.layout?.scrollbarAlign, defaultLayout.scrollbarAlign),
    showMetadata,
    showRowMetadata: envBool(env, "DITOX_TUI_ROW_METADATA", boolValue(fileConfig.layout?.showRowMetadata, defaultLayout.showRowMetadata)),
    showPreviewPane: envBool(env, "DITOX_TUI_PREVIEW_PANE", boolValue(fileConfig.layout?.showPreviewPane, defaultLayout.showPreviewPane)),
    showFullPreviewMetadata: envBool(
      env,
      "DITOX_TUI_FULL_PREVIEW_METADATA",
      boolValue(fileConfig.layout?.showFullPreviewMetadata, showMetadata),
    ),
    showPreviewGutter: envBool(env, "DITOX_TUI_PREVIEW_GUTTER", boolValue(fileConfig.layout?.showPreviewGutter, defaultLayout.showPreviewGutter)),
    showFullPreviewGutter: envBool(
      env,
      "DITOX_TUI_FULL_PREVIEW_GUTTER",
      boolValue(fileConfig.layout?.showFullPreviewGutter, defaultLayout.showFullPreviewGutter),
    ),
    highlightSearchMatches: envBool(
      env,
      "DITOX_TUI_SEARCH_HIGHLIGHT",
      boolValue(fileConfig.layout?.highlightSearchMatches, defaultLayout.highlightSearchMatches),
    ),
    showEmptyStateHelp: envBool(env, "DITOX_TUI_EMPTY_HELP", boolValue(fileConfig.layout?.showEmptyStateHelp, compactFallback("showEmptyStateHelp"))),
    panelPaddingX,
    panelPaddingY,
  };

  const labels = { ...defaultLabels, ...fileConfig.labels };
  if (fileConfig.labels?.fullPreviewGutterSeparator === undefined && fileConfig.labels?.previewGutterSeparator !== undefined) {
    labels.fullPreviewGutterSeparator = labels.previewGutterSeparator;
  }
  if (fileConfig.labels?.splitImagePreviewFallbackPrefix === undefined && fileConfig.labels?.imagePreviewFallbackPrefix !== undefined) {
    labels.splitImagePreviewFallbackPrefix = labels.imagePreviewFallbackPrefix;
  }
  if (fileConfig.labels?.splitImagePreviewFallbackSeparator === undefined && fileConfig.labels?.imagePreviewFallbackSeparator !== undefined) {
    labels.splitImagePreviewFallbackSeparator = labels.imagePreviewFallbackSeparator;
  }
  if (fileConfig.labels?.fullImagePreviewFallbackPrefix === undefined && fileConfig.labels?.imagePreviewFallbackPrefix !== undefined) {
    labels.fullImagePreviewFallbackPrefix = labels.imagePreviewFallbackPrefix;
  }
  if (fileConfig.labels?.fullImagePreviewFallbackSeparator === undefined && fileConfig.labels?.imagePreviewFallbackSeparator !== undefined) {
    labels.fullImagePreviewFallbackSeparator = labels.imagePreviewFallbackSeparator;
  }
  if (fileConfig.labels?.splitImagePreviewSourceTemplate === undefined && fileConfig.labels?.imagePreviewSourceTemplate !== undefined) {
    labels.splitImagePreviewSourceTemplate = labels.imagePreviewSourceTemplate;
  }
  if (fileConfig.labels?.fullImagePreviewSourceTemplate === undefined && fileConfig.labels?.imagePreviewSourceTemplate !== undefined) {
    labels.fullImagePreviewSourceTemplate = labels.imagePreviewSourceTemplate;
  }
  if (fileConfig.labels?.fullPreviewMetaHeaderTemplate === undefined && fileConfig.labels?.fullPreviewMetaTemplate !== undefined) {
    labels.fullPreviewMetaHeaderTemplate = labels.fullPreviewMetaTemplate;
  }

  return {
    sourcePath,
    theme,
    terminal,
    layout,
    chrome: mergeChrome(fileConfig.chrome),
    styles: mergeStyles(theme, fileConfig.styles),
    labels,
    keyBindings: mergeKeyBindings(fileConfig.keyBindings),
    keyLabels: mergeKeyLabels(fileConfig.keyLabels),
    statusTones: mergeStatusTones(fileConfig.statusTones),
    headerLineTones: mergeHeaderLineTones(fileConfig.headerLineTones),
    statusLineTones: mergeStatusLineTones(fileConfig.statusLineTones),
    overlayBorderTones: mergeOverlayBorderTones(fileConfig.overlayBorderTones),
    overlayContentTones: mergeOverlayContentTones(fileConfig.overlayContentTones),
    listContentTones: mergeListContentTones(fileConfig.listContentTones),
    previewContentTones: mergePreviewContentTones(fileConfig.previewContentTones),
    filterOrder: filterOrderValue(fileConfig.filterOrder),
    helpOrder: helpOrderValue(fileConfig.helpOrder),
    startup: startupValue(fileConfig.startup, env),
    behavior: behaviorValue(fileConfig.behavior, env),
  };
}

export function surface(config: ResolvedTuiConfig, name: TuiSurfaceName): TuiSurfaceStyle {
  return config.styles[name];
}

export function textStyle(style: TuiSurfaceStyle, fg: string = style.fg): TuiTextStyle {
  const attributes = {
    bold: style.bold,
    dim: style.dim,
    italic: style.italic,
    underline: style.underline,
    blink: style.blink,
    inverse: style.inverse,
    hidden: style.hidden,
    strikethrough: style.strikethrough,
  };
  return {
    fg,
    bg: style.bg,
    attributes: createTextAttributes(attributes),
    ...attributes,
  };
}

export function keyDisplay(keys: string[], labels: Pick<TuiLabels, "keyAlternativeSeparator"> = defaultLabels, keyLabels: KeyDisplayLabels = defaultKeyLabels): string {
  return keys.map((key) => displayKey(key, keyLabels)).join(labels.keyAlternativeSeparator);
}

export function paddedTitle(value: string, paddingLeft: number, paddingRight = paddingLeft): string {
  const left = " ".repeat(Math.max(0, Math.floor(paddingLeft)));
  const right = " ".repeat(Math.max(0, Math.floor(paddingRight)));
  return `${left}${value}${right}`;
}

export function helpRows(config: ResolvedTuiConfig): Array<{ keys: string; action: string }> {
  const keys = config.keyBindings;
  const labels = config.labels;
  const keyLabels = config.keyLabels;
  const rows: Record<TuiHelpActionName, { keys: string; action: string }> = {
    moveSelection: { keys: keyGroup(labels, keyLabels, keys.up, keys.down), action: labels.helpMoveSelection },
    pageSelection: { keys: keyGroup(labels, keyLabels, keys.pageUp, keys.pageDown), action: labels.helpPageSelection },
    firstLastEntry: { keys: keyGroup(labels, keyLabels, keys.home, keys.end), action: labels.helpFirstLastEntry },
    quit: { keys: keyGroup(labels, keyLabels, keys.quit, keys.forceQuit), action: labels.helpQuit },
    preview: { keys: keyDisplay(keys.preview, labels, keyLabels), action: labels.helpPreview },
    previewNavigation: {
      keys: keyGroup(labels, keyLabels, keys.previewUp, keys.previewDown, keys.previewPageUp, keys.previewPageDown),
      action: labels.helpPreviewNavigation,
    },
    previewBack: { keys: keyDisplay(keys.previewBack, labels, keyLabels), action: labels.helpPreviewBack },
    pinnedView: { keys: keyDisplay(keys.togglePinnedView, labels, keyLabels), action: labels.helpPinnedView },
    paste: { keys: keyDisplay(keys.copyPaste, labels, keyLabels), action: labels.helpPaste },
    copySet: { keys: keyGroup(labels, keyLabels, keys.copyOnly, keys.bulkCopy), action: labels.helpCopySet },
    output: { keys: keyDisplay(keys.output, labels, keyLabels), action: labels.helpOutput },
    markSingle: { keys: keyGroup(labels, keyLabels, keys.selectToggle, keys.selectSingle, keys.clearSelection), action: labels.helpMarkSingle },
    rangeSelect: { keys: keyGroup(labels, keyLabels, keys.selectUp, keys.selectDown), action: labels.helpRangeSelect },
    searchFilter: { keys: keyGroup(labels, keyLabels, keys.search, keys.nextFilter), action: labels.helpSearchFilter },
    searchEdit: {
      keys: keyGroup(labels, keyLabels, keys.searchBackspace, keys.searchApply, keys.searchCancel),
      action: labels.helpSearchEdit,
    },
    searchCopyMatches: { keys: keyDisplay(keys.searchCopyMatches, labels, keyLabels), action: labels.helpSearchCopyMatches },
    pinDelete: { keys: keyGroup(labels, keyLabels, keys.toggleFavorite, keys.delete), action: labels.helpPinDelete },
    clearHistory: { keys: keyGroup(labels, keyLabels, keys.clearAll, keys.clearText, keys.clearImages), action: labels.helpClearHistory },
    clearAllIncludingPinned: { keys: keyDisplay(keys.clearAllIncludingPinned, labels, keyLabels), action: labels.helpClearAllIncludingPinned },
    confirmChoice: { keys: keyGroup(labels, keyLabels, keys.confirmYes, keys.confirmNo), action: labels.helpConfirmChoice },
  };
  return config.helpOrder.map((name) => rows[name]);
}

export function statusHint(config: ResolvedTuiConfig, mode: TuiStatusHintMode = "browse"): string {
  const keys = config.keyBindings;
  const labels = config.labels;
  const keyLabels = config.keyLabels;
  return formatTemplate(statusHintTemplate(labels, mode), {
    pasteKeys: keyDisplay(keys.copyPaste, labels, keyLabels),
    paste: labels.statusPasteHint,
    copyKeys: keyDisplay(keys.copyOnly, labels, keyLabels),
    copy: labels.statusCopyHint,
    previewKeys: keyDisplay(keys.preview, labels, keyLabels),
    preview: labels.statusPreviewHint,
    searchKeys: keyDisplay(keys.search, labels, keyLabels),
    search: labels.statusSearchHint,
    filterKeys: keyDisplay(keys.nextFilter, labels, keyLabels),
    filter: labels.statusFilterHint,
    pinnedKeys: keyDisplay(keys.togglePinnedView, labels, keyLabels),
    pinned: labels.statusPinnedHint,
    deleteKeys: keyDisplay(keys.delete, labels, keyLabels),
    delete: labels.statusDeleteHint,
    outputKeys: keyDisplay(keys.output, labels, keyLabels),
    output: labels.statusOutputHint,
    helpKeys: keyDisplay(keys.help, labels, keyLabels),
    help: labels.statusHelpHint,
    quitKeys: keyGroup(labels, keyLabels, keys.quit, keys.forceQuit),
    quit: labels.statusQuitHint,
    applyKeys: keyDisplay(keys.searchApply, labels, keyLabels),
    apply: labels.statusApplyHint,
    backspaceKeys: keyDisplay(keys.searchBackspace, labels, keyLabels),
    backspace: labels.statusBackspaceHint,
    cancelKeys: keyDisplay(keys.searchCancel, labels, keyLabels),
    cancel: labels.statusCancelHint,
    searchCopyKeys: keyDisplay(keys.searchCopyMatches, labels, keyLabels),
    searchCopy: labels.statusSearchCopyHint,
    previewBackKeys: keyDisplay(keys.previewBack, labels, keyLabels),
    previewBack: labels.statusPreviewBackHint,
    previewScrollKeys: keyGroup(labels, keyLabels, keys.previewUp, keys.previewDown, keys.previewPageUp, keys.previewPageDown),
    previewScroll: labels.statusPreviewScrollHint,
    confirmYesKeys: keyDisplay(keys.confirmYes, labels, keyLabels),
    confirmYes: labels.statusConfirmYesHint,
    confirmNoKeys: keyDisplay(keys.confirmNo, labels, keyLabels),
    confirmNo: labels.statusConfirmNoHint,
    separator: labels.statusHintSeparator,
  });
}

function statusHintTemplate(labels: TuiLabels, mode: TuiStatusHintMode): string {
  if (mode === "search") return labels.statusSearchModeHintTemplate;
  if (mode === "preview") return labels.statusPreviewModeHintTemplate;
  if (mode === "confirm") return labels.statusConfirmModeHintTemplate;
  return labels.statusHintTemplate;
}

function keyGroup(labels: Pick<TuiLabels, "keyAlternativeSeparator" | "keyGroupSeparator">, keyLabels: KeyDisplayLabels, ...groups: string[][]): string {
  return groups.map((keys) => keyDisplay(keys, labels, keyLabels)).join(labels.keyGroupSeparator);
}

export function formatTemplate(value: string, replacements: Record<string, string | number>): string {
  let out = value;
  for (const [key, replacement] of Object.entries(replacements)) {
    out = out.replaceAll(`{${key}}`, String(replacement));
  }
  return out;
}

export type TemplateSegment = {
  key: string | null;
  text: string;
};

export function templateSegments(template: string, replacements: Record<string, string | number>): TemplateSegment[] {
  const parts: TemplateSegment[] = [];
  const pattern = /\{([a-zA-Z0-9_]+)\}/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(template)) !== null) {
    if (match.index > cursor) parts.push({ key: null, text: template.slice(cursor, match.index) });
    const key = match[1]!;
    if (Object.hasOwn(replacements, key)) {
      const text = String(replacements[key]);
      if (text.length > 0) parts.push({ key, text });
    } else {
      parts.push({ key: null, text: match[0] });
    }
    cursor = match.index + match[0].length;
  }

  if (cursor < template.length) parts.push({ key: null, text: template.slice(cursor) });
  return parts;
}

function configPath(env: EnvMap): string {
  if (env.DITOX_TUI_CONFIG) return env.DITOX_TUI_CONFIG;
  const configHome = env.XDG_CONFIG_HOME || join(env.HOME || homedir(), ".config");
  return join(configHome, "ditox", "tui.json");
}

function mergeCompatThemeFile(
  fileConfig: TuiConfigFile,
  sourcePath: string,
  env: EnvMap,
  readFile: (path: string) => string,
): TuiConfigFile {
  if (!fileConfig.themeFile) return fileConfig;
  let compatTheme: CompatThemeFile;
  try {
    compatTheme = JSON.parse(readFile(resolveExternalConfigPath(fileConfig.themeFile, sourcePath, env))) as CompatThemeFile;
  } catch {
    return fileConfig;
  }
  if (compatTheme.UseCustom === false) return fileConfig;

  const derived = configFromCompatTheme(compatTheme, fileConfig.theme);
  return {
    ...fileConfig,
    theme: derived.theme,
    styles: mergeSurfaceConfigs(derived.styles, fileConfig.styles),
  };
}

function resolveExternalConfigPath(path: string, sourcePath: string, env: EnvMap): string {
  const home = env.HOME || homedir();
  if (path === "~") return home;
  if (path.startsWith("~/")) return join(home, path.slice(2));
  if (path.startsWith("$HOME/")) return join(home, path.slice(6));
  if (isAbsolute(path)) return path;
  return resolve(dirname(sourcePath), path);
}

function configFromCompatTheme(compatTheme: CompatThemeFile, existingTheme: TuiConfigFile["theme"]): Pick<TuiConfigFile, "theme" | "styles"> {
  const existingPreset = typeof existingTheme === "string" ? existingTheme : existingTheme?.preset;
  const existingColors = typeof existingTheme === "object" ? existingTheme.colors : undefined;
  const colors = { ...themeColorsFromCompatTheme(compatTheme), ...existingColors };
  return {
    theme: { preset: existingPreset, colors },
    styles: surfaceStylesFromCompatTheme(compatTheme),
  };
}

function themeColorsFromCompatTheme(theme: CompatThemeFile): Partial<ThemeColors> {
  const colors: Partial<ThemeColors> = {};
  assignColor(colors, "bgPanel", colorValue(theme, "TitleBack"));
  assignColor(colors, "bgSelected", colorValue(theme, "SelectedDesc"));
  assignColor(colors, "border", colorValue(theme, "DividerDot"));
  assignColor(colors, "borderFocused", colorValue(theme, "SelectedBorder") ?? colorValue(theme, "PreviewBorder"));
  assignColor(colors, "textPrimary", colorValue(theme, "NormalDesc"));
  assignColor(colors, "textSecondary", colorValue(theme, "TitleInfo") ?? colorValue(theme, "FilterInfo"));
  assignColor(colors, "textMuted", colorValue(theme, "DimmedDesc"));
  assignColor(colors, "selectionFg", colorValue(theme, "SelectedDesc"));
  assignColor(colors, "accentText", colorValue(theme, "NormalTitle"));
  assignColor(colors, "accentImage", colorValue(theme, "FilterInfo"));
  assignColor(colors, "accentFavorite", colorValue(theme, "PinIndicatorColor"));
  assignColor(colors, "accentSuccess", colorValue(theme, "StatusMsg"));
  assignColor(colors, "accentSearch", colorValue(theme, "FilterPrompt"));
  assignColor(colors, "accentCommand", colorValue(theme, "HelpKey") ?? colorValue(theme, "TitleInfo"));
  assignColor(colors, "scrollbarTrack", colorValue(theme, "PageInactiveDot"));
  assignColor(colors, "scrollbarThumb", colorValue(theme, "PageActiveDot"));
  return colors;
}

function surfaceStylesFromCompatTheme(theme: CompatThemeFile): TuiConfigFile["styles"] {
  return {
    header: compactStyle({
      bg: colorValue(theme, "TitleBack"),
      fg: colorValue(theme, "TitleFore"),
      border: colorValue(theme, "DividerDot") ?? colorValue(theme, "SelectedBorder"),
      accent: colorValue(theme, "TitleInfo"),
      secondary: colorValue(theme, "TitleInfo"),
      search: colorValue(theme, "FilterText") ?? colorValue(theme, "FilterPrompt"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    list: compactStyle({
      fg: colorValue(theme, "NormalDesc"),
      accent: colorValue(theme, "NormalTitle"),
      muted: colorValue(theme, "DimmedDesc"),
      secondary: colorValue(theme, "DimmedTitle"),
      search: colorValue(theme, "FilteredMatch"),
      favorite: colorValue(theme, "PinIndicatorColor"),
      border: colorValue(theme, "DividerDot"),
    }),
    alternateRow: compactStyle({
      fg: colorValue(theme, "NormalDesc"),
      accent: colorValue(theme, "NormalTitle"),
      muted: colorValue(theme, "DimmedDesc"),
      secondary: colorValue(theme, "DimmedTitle"),
      search: colorValue(theme, "FilteredMatch"),
      favorite: colorValue(theme, "PinIndicatorColor"),
      border: colorValue(theme, "DividerDot"),
    }),
    selectedRow: compactStyle({
      fg: colorValue(theme, "SelectedDesc"),
      accent: colorValue(theme, "SelectedTitle"),
      muted: colorValue(theme, "SelectedDesc"),
      border: colorValue(theme, "SelectedBorder") ?? colorValue(theme, "SelectedDescBorder"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    selectedMarkedRow: compactStyle({
      fg: colorValue(theme, "SelectedDesc"),
      accent: colorValue(theme, "PinIndicatorColor") ?? colorValue(theme, "SelectedTitle"),
      muted: colorValue(theme, "SelectedDesc"),
      border: colorValue(theme, "SelectedBorder") ?? colorValue(theme, "SelectedDescBorder"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    markedRow: compactStyle({
      fg: colorValue(theme, "NormalDesc"),
      accent: colorValue(theme, "PinIndicatorColor") ?? colorValue(theme, "SelectedTitle"),
      muted: colorValue(theme, "DimmedDesc"),
      border: colorValue(theme, "SelectedDescBorder") ?? colorValue(theme, "SelectedBorder"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    preview: compactStyle({
      fg: colorValue(theme, "PreviewedText"),
      border: colorValue(theme, "PreviewBorder"),
      accent: colorValue(theme, "FilterInfo") ?? colorValue(theme, "TitleInfo"),
      muted: colorValue(theme, "DimmedDesc"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    previewMeta: compactStyle({
      fg: colorValue(theme, "DimmedDesc"),
      border: colorValue(theme, "PreviewBorder"),
      accent: colorValue(theme, "TitleInfo") ?? colorValue(theme, "NormalTitle"),
      muted: colorValue(theme, "DimmedTitle"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    fullPreview: compactStyle({
      fg: colorValue(theme, "PreviewedText"),
      border: colorValue(theme, "PreviewBorder"),
      accent: colorValue(theme, "FilterInfo") ?? colorValue(theme, "TitleInfo"),
      muted: colorValue(theme, "DimmedDesc"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    fullPreviewMeta: compactStyle({
      fg: colorValue(theme, "DimmedDesc"),
      border: colorValue(theme, "PreviewBorder"),
      accent: colorValue(theme, "TitleInfo") ?? colorValue(theme, "NormalTitle"),
      muted: colorValue(theme, "DimmedTitle"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    overlay: compactStyle({
      fg: colorValue(theme, "HelpDesc") ?? colorValue(theme, "FilterText"),
      border: colorValue(theme, "SelectedBorder") ?? colorValue(theme, "DividerDot"),
      accent: colorValue(theme, "HelpKey"),
      muted: colorValue(theme, "DimmedDesc"),
      secondary: colorValue(theme, "FilterInfo"),
      search: colorValue(theme, "FilterPrompt") ?? colorValue(theme, "FilterCursor"),
      favorite: colorValue(theme, "PinIndicatorColor"),
    }),
    status: compactStyle({
      fg: colorValue(theme, "DimmedDesc"),
      accent: colorValue(theme, "StatusMsg"),
      success: colorValue(theme, "StatusMsg"),
      warning: colorValue(theme, "PinIndicatorColor"),
      search: colorValue(theme, "FilterPrompt"),
    }),
    scrollbar: compactStyle({
      fg: colorValue(theme, "PageActiveDot"),
      accent: colorValue(theme, "PageActiveDot"),
      muted: colorValue(theme, "PageInactiveDot"),
      border: colorValue(theme, "DividerDot"),
    }),
  };
}

function mergeSurfaceConfigs(base: TuiConfigFile["styles"], override: TuiConfigFile["styles"]): TuiConfigFile["styles"] {
  const merged: TuiConfigFile["styles"] = { ...base };
  if (!override) return merged;
  for (const name of Object.keys(override) as TuiSurfaceName[]) {
    merged[name] = { ...merged[name], ...override[name] };
  }
  return merged;
}

function compactStyle(style: Partial<TuiSurfaceStyle>): Partial<TuiSurfaceStyle> {
  const out: Partial<TuiSurfaceStyle> = {};
  for (const key of Object.keys(style) as Array<keyof TuiSurfaceStyle>) {
    const value = style[key];
    if (typeof value === "string" && isColor(value)) (out as Record<string, string>)[key] = value;
  }
  return out;
}

function colorValue(theme: CompatThemeFile, key: keyof CompatThemeFile): string | undefined {
  const value = theme[key];
  return typeof value === "string" && isColor(value) ? value : undefined;
}

function assignColor<T extends Record<string, string | undefined>, K extends keyof T>(target: T, key: K, value: string | undefined): void {
  if (value) target[key] = value as T[K];
}

function resolveTerminalConfig(terminal: TuiConfigFile["terminal"], env: EnvMap): TuiTerminalConfig {
  const backgroundColor = terminalBackgroundColorValue(
    env.DITOX_TUI_BACKGROUND_COLOR ?? env.DITOX_TUI_BACKGROUND ?? terminal?.backgroundColor,
    defaultTerminal.backgroundColor,
  );
  const title = nullableTextValue(env.DITOX_TUI_TITLE ?? terminal?.title, defaultTerminal.title);
  const footerHeight = clampNumber(
    envNumber(env, "DITOX_TUI_FOOTER_HEIGHT", numberValue(terminal?.footerHeight, defaultTerminal.footerHeight)),
    1,
    48,
  );
  const clearOnShutdown = envBool(env, "DITOX_TUI_CLEAR_ON_SHUTDOWN", boolValue(terminal?.clearOnShutdown, defaultTerminal.clearOnShutdown));
  const cursor = terminalCursorConfigValue(terminal?.cursor, env);
  const kittyKeyboard = kittyKeyboardConfigValue(terminal?.kittyKeyboard, env);
  const targetFps = optionalClampedNumber(env, "DITOX_TUI_TARGET_FPS", terminal?.targetFps, 1, 240);
  const maxFps = optionalClampedNumber(env, "DITOX_TUI_MAX_FPS", terminal?.maxFps, 1, 240);
  const debounceDelay = optionalClampedNumber(env, "DITOX_TUI_RENDER_DEBOUNCE_MS", terminal?.debounceDelay, 0, 250);
  const stdinParserMaxBufferBytes = optionalClampedNumber(
    env,
    "DITOX_TUI_STDIN_BUFFER_BYTES",
    terminal?.stdinParserMaxBufferBytes,
    4096,
    16 * 1024 * 1024,
  );
  const directScreenMode = terminalScreenModeValue(env.DITOX_TUI_SCREEN_MODE ?? terminal?.screenMode);
  if (directScreenMode) {
    return {
      altScreen: screenModeAltScreen(directScreenMode),
      screenMode: directScreenMode,
      title,
      backgroundColor,
      footerHeight,
      clearOnShutdown,
      cursor,
      kittyKeyboard,
      targetFps,
      maxFps,
      debounceDelay,
      stdinParserMaxBufferBytes,
    };
  }

  const altScreen = terminalAltScreenValue(env.DITOX_TUI_ALT_SCREEN ?? terminal?.altScreen ?? terminal?.alt_screen, defaultTerminal.altScreen);
  return {
    altScreen,
    screenMode: screenModeFromAltScreen(altScreen),
    title,
    backgroundColor,
    footerHeight,
    clearOnShutdown,
    cursor,
    kittyKeyboard,
    targetFps,
    maxFps,
    debounceDelay,
    stdinParserMaxBufferBytes,
  };
}

function terminalCursorConfigValue(cursor: TuiTerminalConfigFile["cursor"], env: EnvMap): TuiTerminalCursorConfig {
  return {
    style: terminalCursorStyleValue(env.DITOX_TUI_CURSOR_STYLE ?? cursor?.style, defaultTerminal.cursor.style),
    blinking: terminalCursorBlinkingValue(env.DITOX_TUI_CURSOR_BLINKING ?? cursor?.blinking, defaultTerminal.cursor.blinking),
    color: terminalCursorColorValue(env.DITOX_TUI_CURSOR_COLOR ?? cursor?.color, defaultTerminal.cursor.color),
  };
}

function terminalCursorBlinkingValue(value: unknown, fallback: boolean | null): boolean | null {
  if (value === null || value === "auto" || value === "none") return null;
  if (value === true || value === "1" || value === "true" || value === "yes") return true;
  if (value === false || value === "0" || value === "false" || value === "no") return false;
  return fallback;
}

function terminalCursorStyleValue(value: unknown, fallback: TuiTerminalCursorStyle | null): TuiTerminalCursorStyle | null {
  if (value === null || value === "auto" || value === "none") return null;
  if ((terminalCursorStyleNames as readonly unknown[]).includes(value)) return value as TuiTerminalCursorStyle;
  return fallback;
}

function terminalCursorColorValue(value: unknown, fallback: string | null): string | null {
  if (value === null || value === "auto" || value === "none") return null;
  if (typeof value === "string" && isHexColor(value)) return value;
  return fallback;
}

function kittyKeyboardConfigValue(value: TuiTerminalConfigFile["kittyKeyboard"], env: EnvMap): TuiKittyKeyboardConfig {
  const base = { ...defaultTerminal.kittyKeyboard, ...kittyKeyboardFileValue(value) };
  const envMode = kittyKeyboardModeValue(env.DITOX_TUI_KITTY_KEYBOARD);
  const enabled = envMode?.enabled ?? base.enabled;
  return {
    enabled,
    disambiguate: envBool(env, "DITOX_TUI_KITTY_KEYBOARD_DISAMBIGUATE", envMode?.disambiguate ?? base.disambiguate),
    alternateKeys: envBool(env, "DITOX_TUI_KITTY_KEYBOARD_ALTERNATE_KEYS", envMode?.alternateKeys ?? base.alternateKeys),
    events: envBool(env, "DITOX_TUI_KITTY_KEYBOARD_EVENTS", envMode?.events ?? base.events),
    allKeysAsEscapes: envBool(env, "DITOX_TUI_KITTY_KEYBOARD_ALL_KEYS", envMode?.allKeysAsEscapes ?? base.allKeysAsEscapes),
    reportText: envBool(env, "DITOX_TUI_KITTY_KEYBOARD_REPORT_TEXT", envMode?.reportText ?? base.reportText),
  };
}

function kittyKeyboardFileValue(value: TuiTerminalConfigFile["kittyKeyboard"]): Partial<TuiKittyKeyboardConfig> {
  if (typeof value === "boolean") return { enabled: value };
  const mode = kittyKeyboardModeValue(value);
  if (mode) return mode;
  if (!value || typeof value !== "object") return {};
  return {
    enabled: boolValue(value.enabled, defaultTerminal.kittyKeyboard.enabled),
    disambiguate: boolValue(value.disambiguate, defaultTerminal.kittyKeyboard.disambiguate),
    alternateKeys: boolValue(value.alternateKeys, defaultTerminal.kittyKeyboard.alternateKeys),
    events: boolValue(value.events, defaultTerminal.kittyKeyboard.events),
    allKeysAsEscapes: boolValue(value.allKeysAsEscapes, defaultTerminal.kittyKeyboard.allKeysAsEscapes),
    reportText: boolValue(value.reportText, defaultTerminal.kittyKeyboard.reportText),
  };
}

function kittyKeyboardModeValue(value: unknown): Partial<TuiKittyKeyboardConfig> | null {
  if (typeof value !== "string") return null;
  const normalized = value.toLowerCase();
  if (normalized === "off" || normalized === "false" || normalized === "0" || normalized === "disabled" || normalized === "none") {
    return { enabled: false };
  }
  if (normalized === "on" || normalized === "true" || normalized === "1" || normalized === "basic" || normalized === "enabled") {
    return { enabled: true };
  }
  if (normalized === "events") return { enabled: true, events: true };
  if (normalized === "all") return { enabled: true, events: true, allKeysAsEscapes: true, reportText: true };
  return null;
}

function optionalClampedNumber(env: EnvMap, envName: string, fileValue: unknown, min: number, max: number): number | null {
  const envValue = envNumber(env, envName, Number.NaN);
  if (Number.isFinite(envValue)) return clampNumber(envValue, min, max);
  if (typeof fileValue === "number" && Number.isFinite(fileValue)) return clampNumber(fileValue, min, max);
  return null;
}

function terminalBackgroundColorValue(value: unknown, fallback: string): string {
  if (value === "auto" || value === "transparent") return value;
  if (typeof value === "string" && isHexColor(value)) return value;
  return fallback;
}

function terminalAltScreenValue(value: unknown, fallback: TuiTerminalAltScreen): TuiTerminalAltScreen {
  if ((terminalAltScreenNames as readonly unknown[]).includes(value)) return value as TuiTerminalAltScreen;
  return fallback;
}

function terminalScreenModeValue(value: unknown): TuiTerminalScreenMode | null {
  if (value === "alternate-screen" || value === "alternate") return "alternate-screen";
  if (value === "main-screen" || value === "main") return "main-screen";
  if (value === "split-footer" || value === "split") return "split-footer";
  return null;
}

function screenModeFromAltScreen(altScreen: TuiTerminalAltScreen): TuiTerminalScreenMode {
  return altScreen === "never" ? "main-screen" : "alternate-screen";
}

function screenModeAltScreen(screenMode: TuiTerminalScreenMode): TuiTerminalAltScreen {
  return screenMode === "alternate-screen" ? "always" : "never";
}

function themePreset(theme: TuiConfigFile["theme"], env: EnvMap): ThemeName {
  if (isThemeName(env.DITOX_TUI_THEME)) return env.DITOX_TUI_THEME;
  if (isThemeName(theme)) return theme;
  if (typeof theme === "object" && isThemeName(theme?.preset)) return theme.preset;
  return "ditoxDark";
}

function isThemeName(value: unknown): value is ThemeName {
  return typeof value === "string" && (themeNames as readonly string[]).includes(value);
}

function splitListWidth(layout: Partial<UiConfig> | undefined): number {
  if (typeof layout?.listWidthPercent === "number" && Number.isFinite(layout.listWidthPercent)) return layout.listWidthPercent;
  if (typeof layout?.previewWidthPercent === "number" && Number.isFinite(layout.previewWidthPercent)) return 100 - layout.previewWidthPercent;
  return defaultLayout.listWidthPercent;
}

function mergeTheme(preset: ThemeName, colors: Partial<ThemeColors> | undefined): TuiTheme {
  const base = themes[preset];
  return { ...base, ...colorOverrides(colors), name: base.name };
}

function imagePreviewModeValue(envValue: string | undefined, fileValue: unknown, fallback: ImagePreviewMode): ImagePreviewMode {
  if (isImagePreviewMode(envValue)) return envValue;
  if (isImagePreviewMode(fileValue)) return fileValue;
  return fallback;
}

function imagePreviewNoticeVisibilityValue(value: unknown, fallback: ImagePreviewNoticeVisibility): ImagePreviewNoticeVisibility {
  if ((imagePreviewNoticeVisibilityNames as readonly unknown[]).includes(value)) return value as ImagePreviewNoticeVisibility;
  return fallback;
}

function imagePreviewRendererValue(value: unknown, fallback: ImagePreviewRenderer): ImagePreviewRenderer {
  if ((imagePreviewRendererNames as readonly unknown[]).includes(value)) return value as ImagePreviewRenderer;
  return fallback;
}

function imagePreviewAlignValue(value: unknown, fallback: ImagePreviewAlign): ImagePreviewAlign {
  if ((imagePreviewAlignNames as readonly unknown[]).includes(value)) return value as ImagePreviewAlign;
  return fallback;
}

function contentAlignValue(value: unknown, fallback: ContentAlign): ContentAlign {
  if ((titleAlignmentNames as readonly unknown[]).includes(value)) return value as ContentAlign;
  return fallback;
}

function verticalAlignValue(value: unknown, fallback: VerticalAlign): VerticalAlign {
  if ((verticalAlignNames as readonly unknown[]).includes(value)) return value as VerticalAlign;
  return fallback;
}

function scrollbarPlacementValue(value: unknown, fallback: ScrollbarPlacement): ScrollbarPlacement {
  if ((scrollbarPlacementNames as readonly unknown[]).includes(value)) return value as ScrollbarPlacement;
  return fallback;
}

function overlayPlacementValue(value: unknown, fallback: OverlayPlacement): OverlayPlacement {
  if ((overlayPlacementNames as readonly unknown[]).includes(value)) return value as OverlayPlacement;
  return fallback;
}

function compatImagePreviewMode(display: CompatImageDisplayConfig | undefined, fallback: ImagePreviewMode): ImagePreviewMode {
  if (!display) return fallback;
  if (display.type === "basic") return "blocks";
  if (display.type === "kitty" || display.type === "sixel") return display.type;
  return fallback;
}

function isImagePreviewMode(value: unknown): value is ImagePreviewMode {
  return value === "metadata" || value === "blocks" || value === "kitty" || value === "sixel";
}

function imagePreviewBackgroundValue(value: unknown, fallback: string): string {
  if (value === "auto") return "auto";
  if (typeof value === "string" && isHexColor(value)) return value;
  return fallback;
}

function compatImagePreviewMaxWidth(display: CompatImageDisplayConfig | undefined, fallback: number): number {
  if (!display || typeof display.scaleX !== "number" || !Number.isFinite(display.scaleX) || display.scaleX <= 0) return fallback;
  return Math.round(display.scaleX * 5);
}

function compatImagePreviewMaxRows(display: CompatImageDisplayConfig | undefined, fallback: number): number {
  if (!display || typeof display.scaleY !== "number" || !Number.isFinite(display.scaleY) || display.scaleY <= 0) return fallback;
  const heightCut = typeof display.heightCut === "number" && Number.isFinite(display.heightCut) ? Math.max(0, display.heightCut) : 0;
  return Math.round(display.scaleY * 2 - heightCut);
}

function colorOverrides(colors: Partial<ThemeColors> | undefined): Partial<ThemeColors> {
  if (!colors) return {};
  const overrides: Partial<ThemeColors> = {};
  for (const key of Object.keys(themes.ditoxDark) as Array<keyof TuiTheme>) {
    if (key === "name") continue;
    const value = colors[key as keyof ThemeColors];
    if (typeof value === "string" && isColor(value)) overrides[key as keyof ThemeColors] = value;
  }
  return overrides;
}

function mergeChrome(chrome: Partial<TuiChromeConfig> | undefined): TuiChromeConfig {
  const panelBorder = boolValue(chrome?.panelBorder, defaultChrome.panelBorder);
  const overlayBorder = boolValue(chrome?.overlayBorder, defaultChrome.overlayBorder);
  const showPanelTitles = boolValue(chrome?.showPanelTitles, defaultChrome.showPanelTitles);
  const showOverlayTitles = boolValue(chrome?.showOverlayTitles, defaultChrome.showOverlayTitles);
  const panelBorderStyle = borderStyle(chrome?.panelBorderStyle, defaultChrome.panelBorderStyle);
  const overlayBorderStyle = borderStyle(chrome?.overlayBorderStyle, defaultChrome.overlayBorderStyle);
  const panelTitleAlignment = titleAlignment(chrome?.panelTitleAlignment, defaultChrome.panelTitleAlignment);
  const panelBottomTitleAlignment = titleAlignment(chrome?.panelBottomTitleAlignment, defaultChrome.panelBottomTitleAlignment);
  const overlayTitleAlignment = titleAlignment(chrome?.overlayTitleAlignment, defaultChrome.overlayTitleAlignment);
  return {
    panelBorder,
    overlayBorder,
    headerBorder: boolValue(chrome?.headerBorder, panelBorder),
    listBorder: boolValue(chrome?.listBorder, panelBorder),
    previewBorder: boolValue(chrome?.previewBorder, panelBorder),
    fullPreviewBorder: boolValue(chrome?.fullPreviewBorder, panelBorder),
    statusBorder: boolValue(chrome?.statusBorder, defaultChrome.statusBorder),
    searchOverlayBorder: boolValue(chrome?.searchOverlayBorder, overlayBorder),
    dangerOverlayBorder: boolValue(chrome?.dangerOverlayBorder, overlayBorder),
    helpOverlayBorder: boolValue(chrome?.helpOverlayBorder, overlayBorder),
    showPanelTitles,
    showOverlayTitles,
    showHeaderTitle: boolValue(chrome?.showHeaderTitle, showPanelTitles),
    showListTitle: boolValue(chrome?.showListTitle, showPanelTitles),
    showPreviewTitle: boolValue(chrome?.showPreviewTitle, showPanelTitles),
    showFullPreviewTitle: boolValue(chrome?.showFullPreviewTitle, showPanelTitles),
    showStatusTitle: boolValue(chrome?.showStatusTitle, defaultChrome.showStatusTitle),
    showSearchOverlayTitle: boolValue(chrome?.showSearchOverlayTitle, showOverlayTitles),
    showDangerOverlayTitle: boolValue(chrome?.showDangerOverlayTitle, showOverlayTitles),
    showHelpOverlayTitle: boolValue(chrome?.showHelpOverlayTitle, showOverlayTitles),
    showListPositionTitle: boolValue(chrome?.showListPositionTitle, defaultChrome.showListPositionTitle),
    showPreviewEntryTitle: boolValue(chrome?.showPreviewEntryTitle, defaultChrome.showPreviewEntryTitle),
    showFullPreviewBottomTitle: boolValue(chrome?.showFullPreviewBottomTitle, defaultChrome.showFullPreviewBottomTitle),
    panelBorderStyle,
    overlayBorderStyle,
    headerBorderStyle: borderStyle(chrome?.headerBorderStyle, panelBorderStyle),
    listBorderStyle: borderStyle(chrome?.listBorderStyle, panelBorderStyle),
    previewBorderStyle: borderStyle(chrome?.previewBorderStyle, panelBorderStyle),
    fullPreviewBorderStyle: borderStyle(chrome?.fullPreviewBorderStyle, panelBorderStyle),
    statusBorderStyle: borderStyle(chrome?.statusBorderStyle, defaultChrome.statusBorderStyle),
    searchOverlayBorderStyle: borderStyle(chrome?.searchOverlayBorderStyle, overlayBorderStyle),
    dangerOverlayBorderStyle: borderStyle(chrome?.dangerOverlayBorderStyle, overlayBorderStyle),
    helpOverlayBorderStyle: borderStyle(chrome?.helpOverlayBorderStyle, overlayBorderStyle),
    panelTitleAlignment,
    panelBottomTitleAlignment,
    overlayTitleAlignment,
    headerTitleAlignment: titleAlignment(chrome?.headerTitleAlignment, panelTitleAlignment),
    listTitleAlignment: titleAlignment(chrome?.listTitleAlignment, panelTitleAlignment),
    previewTitleAlignment: titleAlignment(chrome?.previewTitleAlignment, panelTitleAlignment),
    fullPreviewTitleAlignment: titleAlignment(chrome?.fullPreviewTitleAlignment, panelTitleAlignment),
    statusTitleAlignment: titleAlignment(chrome?.statusTitleAlignment, defaultChrome.statusTitleAlignment),
    listBottomTitleAlignment: titleAlignment(chrome?.listBottomTitleAlignment, panelBottomTitleAlignment),
    previewBottomTitleAlignment: titleAlignment(chrome?.previewBottomTitleAlignment, panelBottomTitleAlignment),
    fullPreviewBottomTitleAlignment: titleAlignment(chrome?.fullPreviewBottomTitleAlignment, panelBottomTitleAlignment),
    searchOverlayTitleAlignment: titleAlignment(chrome?.searchOverlayTitleAlignment, overlayTitleAlignment),
    dangerOverlayTitleAlignment: titleAlignment(chrome?.dangerOverlayTitleAlignment, overlayTitleAlignment),
    helpOverlayTitleAlignment: titleAlignment(chrome?.helpOverlayTitleAlignment, overlayTitleAlignment),
    selectedMarker: shortText(chrome?.selectedMarker, defaultChrome.selectedMarker),
    selectedMarkedMarker: shortText(chrome?.selectedMarkedMarker, defaultChrome.selectedMarkedMarker),
    markedMarker: shortText(chrome?.markedMarker, defaultChrome.markedMarker),
    normalMarker: shortText(chrome?.normalMarker, defaultChrome.normalMarker),
    scrollbarThumb: glyphText(chrome?.scrollbarThumb, defaultChrome.scrollbarThumb),
    scrollbarTrack: glyphText(chrome?.scrollbarTrack, defaultChrome.scrollbarTrack),
    statusSeparator: shortText(chrome?.statusSeparator, defaultChrome.statusSeparator),
  };
}

function mergeStyles(theme: TuiTheme, styles: TuiConfigFile["styles"]): Record<TuiSurfaceName, TuiSurfaceStyle> {
  const shellDefault = surfaceDefaults(theme, theme.bgBase, theme.textPrimary, theme.border, theme.accentCommand, theme.textDim);
  const shellBase = { ...shellDefault, ...styleOverrides(styles?.shell) };
  const overlayDefault = surfaceDefaults(theme, theme.bgElevated, theme.textSecondary, theme.borderFocused, theme.accentCommand, theme.textMuted);
  const overlayBase = { ...overlayDefault, ...styleOverrides(styles?.overlay) };
  const listDefault = surfaceDefaults(theme, theme.bgPanel, theme.textPrimary, theme.border, theme.accentText, theme.textMuted);
  const listBase = { ...listDefault, ...styleOverrides(styles?.list) };
  const previewDefault = surfaceDefaults(theme, theme.bgPanel, theme.textPrimary, theme.border, theme.accentCommand, theme.textDim);
  const previewBase = { ...previewDefault, ...styleOverrides(styles?.preview) };
  const fullPreviewDefault = surfaceDefaults(theme, theme.bgPanel, theme.textPrimary, theme.borderFocused, theme.accentCommand, theme.textDim);
  const fullPreviewBase = { ...fullPreviewDefault, ...styleOverrides(styles?.fullPreview) };
  const defaults: Record<TuiSurfaceName, TuiSurfaceStyle> = {
    shell: shellBase,
    header: surfaceDefaults(theme, theme.bgPanel, theme.textPrimary, theme.borderFocused, theme.accentCommand, theme.textDim),
    list: listBase,
    alternateRow: surfaceDefaults(theme, theme.bgSubtle, theme.textPrimary, theme.border, theme.accentText, theme.textMuted),
    selectedRow: surfaceDefaults(theme, theme.bgSelected, theme.selectionFg, theme.borderFocused, theme.accentText, theme.textMuted),
    selectedMarkedRow: surfaceDefaults(theme, theme.bgSelected, theme.selectionFg, theme.borderFocused, theme.accentFavorite, theme.textMuted),
    markedRow: surfaceDefaults(theme, theme.bgPanel, theme.textPrimary, theme.border, theme.accentFavorite, theme.textMuted),
    rowSpacer: listBase,
    emptyState: listBase,
    preview: previewBase,
    previewGutter: previewBase,
    previewMeta: surfaceDefaults(theme, theme.bgSubtle, theme.textMuted, theme.border, theme.accentText, theme.textDim),
    previewSpacer: previewBase,
    fullPreview: fullPreviewBase,
    fullPreviewGutter: fullPreviewBase,
    fullPreviewMeta: surfaceDefaults(theme, theme.bgSubtle, theme.textMuted, theme.borderFocused, theme.accentText, theme.textDim),
    fullPreviewSpacer: fullPreviewBase,
    overlay: overlayBase,
    searchOverlay: overlayBase,
    dangerOverlay: overlayBase,
    helpOverlay: overlayBase,
    status: surfaceDefaults(theme, theme.bgBase, theme.textMuted, theme.border, theme.accentCommand, theme.textDim),
    scrollbar: surfaceDefaults(theme, theme.bgPanel, theme.scrollbarThumb, theme.border, theme.scrollbarThumb, theme.scrollbarTrack),
    splitPaneGap: shellBase,
  };
  const merged = { ...defaults };
  for (const name of Object.keys(defaults) as TuiSurfaceName[]) {
    merged[name] = { ...defaults[name], ...styleOverrides(styles?.[name]) };
  }
  return merged;
}

function surfaceDefaults(theme: TuiTheme, bg: string, fg: string, border: string, accent: string, muted: string): TuiSurfaceStyle {
  return {
    bg,
    fg,
    border,
    accent,
    muted,
    secondary: theme.textSecondary,
    success: theme.accentSuccess,
    warning: theme.accentWarning,
    error: theme.accentError,
    search: theme.accentSearch,
    favorite: theme.accentFavorite,
    image: theme.accentImage,
    bold: false,
    dim: false,
    italic: false,
    underline: false,
    blink: false,
    inverse: false,
    hidden: false,
    strikethrough: false,
  };
}

function styleOverrides(style: Partial<TuiSurfaceStyle> | undefined): Partial<TuiSurfaceStyle> {
  if (!style) return {};
  const out: Partial<TuiSurfaceStyle> = {};
  for (const key of ["bg", "fg", "border", "accent", "muted", "secondary", "success", "warning", "error", "search", "favorite", "image"] as Array<keyof TuiSurfaceStyle>) {
    const value = style[key];
    if (typeof value === "string" && isColor(value)) (out as Record<string, string>)[key] = value;
  }
  for (const key of ["bold", "dim", "italic", "underline", "blink", "inverse", "hidden", "strikethrough"] as Array<keyof TuiSurfaceStyle>) {
    const value = style[key];
    if (typeof value === "boolean") (out as Record<string, boolean>)[key] = value;
  }
  return out;
}

function mergeKeyBindings(bindings: TuiConfigFile["keyBindings"]): TuiKeyBindings {
  const merged = { ...defaultKeyBindings };
  if (!bindings) return merged;
  const rawBindings = bindings as Record<string, string | string[] | undefined>;
  for (const action of Object.keys(defaultKeyBindings) as Array<keyof TuiKeyBindings>) {
    const value = rawBindings[action];
    const normalized = normalizeKeys(value);
    if (normalized.length > 0) merged[action] = normalized;
  }
  for (const [alias, action] of Object.entries(compatKeyBindingAliases) as Array<[CompatKeyBindingAlias, keyof TuiKeyBindings]>) {
    if (rawBindings[action] !== undefined) continue;
    const normalized = normalizeKeys(rawBindings[alias]);
    if (normalized.length > 0) merged[action] = normalized;
  }
  return merged;
}

function mergeKeyLabels(labels: TuiConfigFile["keyLabels"]): KeyDisplayLabels {
  const merged = { ...defaultKeyLabels };
  if (!labels) return merged;
  for (const [key, value] of Object.entries(labels)) {
    if (typeof value !== "string") continue;
    merged[normalizeKey(key)] = value;
  }
  return merged;
}

function mergeStatusTones(statusTones: TuiConfigFile["statusTones"]): TuiStatusToneMatchers {
  return {
    error: stringList(statusTones?.error, defaultStatusTones.error),
    success: stringList(statusTones?.success, defaultStatusTones.success),
    warning: stringList(statusTones?.warning, defaultStatusTones.warning),
  };
}

function mergeHeaderLineTones(headerLineTones: TuiConfigFile["headerLineTones"]): TuiHeaderLineTones {
  const merged = { ...defaultHeaderLineTones };
  if (!headerLineTones) return merged;
  for (const part of headerLinePartNames) {
    const tone = headerLineTones[part];
    if (isStatusLineToneName(tone)) merged[part] = tone;
  }
  return merged;
}

function mergeStatusLineTones(statusLineTones: TuiConfigFile["statusLineTones"]): TuiStatusLineTones {
  const merged = { ...defaultStatusLineTones };
  if (!statusLineTones) return merged;
  for (const part of statusLinePartNames) {
    const tone = statusLineTones[part];
    if (isStatusLineToneName(tone)) merged[part] = tone;
  }
  return merged;
}

function mergeOverlayBorderTones(overlayBorderTones: TuiConfigFile["overlayBorderTones"]): TuiOverlayBorderTones {
  const merged = { ...defaultOverlayBorderTones };
  if (!overlayBorderTones) return merged;
  for (const toneName of overlayToneNames) {
    const tone = overlayBorderTones[toneName];
    if (isStatusLineToneName(tone)) merged[toneName] = tone;
  }
  return merged;
}

function mergeOverlayContentTones(overlayContentTones: TuiConfigFile["overlayContentTones"]): TuiOverlayContentTones {
  const merged = { ...defaultOverlayContentTones };
  if (!overlayContentTones) return merged;
  for (const part of overlayContentPartNames) {
    const tone = overlayContentTones[part];
    if (isStatusLineToneName(tone)) merged[part] = tone;
  }
  return merged;
}

function mergeListContentTones(listContentTones: TuiConfigFile["listContentTones"]): TuiListContentTones {
  const merged = { ...defaultListContentTones };
  if (!listContentTones) return merged;
  for (const part of listContentPartNames) {
    const tone = listContentTones[part];
    if (isStatusLineToneName(tone)) merged[part] = tone;
  }
  return merged;
}

function mergePreviewContentTones(previewContentTones: TuiConfigFile["previewContentTones"]): TuiPreviewContentTones {
  const merged = { ...defaultPreviewContentTones };
  if (!previewContentTones) return merged;
  for (const part of previewContentPartNames) {
    const tone = previewContentTones[part];
    if (isStatusLineToneName(tone)) merged[part] = tone;
  }
  if (isStatusLineToneName(previewContentTones.fullMeta)) {
    if (previewContentTones.fullMetaHeader === undefined) merged.fullMetaHeader = previewContentTones.fullMeta;
    if (previewContentTones.fullMetaDetails === undefined) merged.fullMetaDetails = previewContentTones.fullMeta;
  }
  return merged;
}

function isStatusLineToneName(value: unknown): value is TuiStatusLineToneName {
  return typeof value === "string" && (statusLineToneNames as readonly string[]).includes(value);
}

function filterOrderValue(order: string[] | undefined): EntryFilter[] {
  if (!Array.isArray(order)) return defaultFilterOrder;
  const seen = new Set<EntryFilter>();
  const filters: EntryFilter[] = [];
  for (const value of order) {
    if (!isEntryFilter(value) || seen.has(value)) continue;
    seen.add(value);
    filters.push(value);
  }
  return filters.length > 0 ? filters : defaultFilterOrder;
}

function helpOrderValue(order: string[] | undefined): TuiHelpActionName[] {
  if (!Array.isArray(order)) return defaultHelpOrder;
  const seen = new Set<TuiHelpActionName>();
  const actions: TuiHelpActionName[] = [];
  for (const value of order) {
    if (!isHelpActionName(value) || seen.has(value)) continue;
    seen.add(value);
    actions.push(value);
  }
  return actions.length > 0 ? actions : defaultHelpOrder;
}

function previewImageFieldsValue(fields: PreviewImageField[] | undefined): PreviewImageField[] {
  if (!Array.isArray(fields)) return defaultPreviewImageFields;
  const seen = new Set<PreviewImageField>();
  const out: PreviewImageField[] = [];
  for (const field of fields) {
    if (!isPreviewImageField(field) || seen.has(field)) continue;
    seen.add(field);
    out.push(field);
  }
  return out.length > 0 ? out : defaultPreviewImageFields;
}

function startupValue(startup: Partial<TuiStartupConfig> | undefined, env: EnvMap): TuiStartupConfig {
  return {
    filter: entryFilterValue(env.DITOX_TUI_STARTUP_FILTER, startup?.filter, defaultStartup.filter),
    pinnedOnly: envBool(env, "DITOX_TUI_STARTUP_PINNED", boolValue(startup?.pinnedOnly, defaultStartup.pinnedOnly)),
    query: textValue(env.DITOX_TUI_STARTUP_QUERY, textValue(startup?.query, defaultStartup.query)),
  };
}

function behaviorValue(behavior: Partial<TuiBehaviorConfig> | undefined, env: EnvMap): TuiBehaviorConfig {
  return {
    liveSearch: envBool(env, "DITOX_TUI_LIVE_SEARCH", boolValue(behavior?.liveSearch, defaultBehavior.liveSearch)),
    liveSearchDebounceMs: clampNumber(
      envNumber(env, "DITOX_TUI_LIVE_SEARCH_DEBOUNCE_MS", numberValue(behavior?.liveSearchDebounceMs, defaultBehavior.liveSearchDebounceMs)),
      0,
      5000,
    ),
    clearQueryOnSearchOpen: envBool(
      env,
      "DITOX_TUI_CLEAR_QUERY_ON_SEARCH_OPEN",
      boolValue(behavior?.clearQueryOnSearchOpen, defaultBehavior.clearQueryOnSearchOpen),
    ),
    restoreQueryOnSearchCancel: envBool(
      env,
      "DITOX_TUI_RESTORE_QUERY_ON_SEARCH_CANCEL",
      boolValue(behavior?.restoreQueryOnSearchCancel, defaultBehavior.restoreQueryOnSearchCancel),
    ),
    exitAfterPaste: envBool(env, "DITOX_TUI_EXIT_AFTER_PASTE", boolValue(behavior?.exitAfterPaste, defaultBehavior.exitAfterPaste)),
    exitAfterCopy: envBool(env, "DITOX_TUI_EXIT_AFTER_COPY", boolValue(behavior?.exitAfterCopy, defaultBehavior.exitAfterCopy)),
    exitAfterBulkCopy: envBool(env, "DITOX_TUI_EXIT_AFTER_BULK_COPY", boolValue(behavior?.exitAfterBulkCopy, defaultBehavior.exitAfterBulkCopy)),
    exitAfterSearchCopy: envBool(env, "DITOX_TUI_EXIT_AFTER_SEARCH_COPY", boolValue(behavior?.exitAfterSearchCopy, defaultBehavior.exitAfterSearchCopy)),
  };
}

function entryFilterValue(envValue: unknown, fileValue: unknown, fallback: EntryFilter): EntryFilter {
  if (isEntryFilter(envValue)) return envValue;
  if (isEntryFilter(fileValue)) return fileValue;
  return fallback;
}

function isEntryFilter(value: unknown): value is EntryFilter {
  return value === "all" || value === "text" || value === "images" || value === "favorites" || value === "today";
}

function isHelpActionName(value: unknown): value is TuiHelpActionName {
  return typeof value === "string" && (helpActionNames as readonly string[]).includes(value);
}

function isPreviewImageField(value: unknown): value is PreviewImageField {
  return value === "type" || value === "mime" || value === "size" || value === "dimensions" || value === "hash" || value === "blob";
}

function normalizeKeys(value: string | string[] | undefined): string[] {
  const values = typeof value === "string" ? value.split(",") : Array.isArray(value) ? value : [];
  return values.map((key) => key.trim()).filter(Boolean).map(normalizeKey);
}

export function normalizeKey(key: string): string {
  if (key === " ") return "space";
  if (/^[A-Z]$/.test(key)) return `shift+${key.toLowerCase()}`;
  const parts = key.toLowerCase().split("+");
  if (parts[parts.length - 1] === "return") parts[parts.length - 1] = "enter";
  return parts.join("+");
}

function displayKey(key: string, labels: KeyDisplayLabels): string {
  const normalized = normalizeKey(key);
  return labels[normalized] ?? key;
}

function borderStyle(value: unknown, fallback: BorderStyle): BorderStyle {
  return value === "single" || value === "double" || value === "rounded" || value === "heavy" ? value : fallback;
}

function titleAlignment(value: unknown, fallback: TitleAlignment): TitleAlignment {
  return (titleAlignmentNames as readonly unknown[]).includes(value) ? (value as TitleAlignment) : fallback;
}

function shortText(value: unknown, fallback: string): string {
  return typeof value === "string" ? value.slice(0, 2) : fallback;
}

function glyphText(value: unknown, fallback: string): string {
  return typeof value === "string" ? Array.from(value).slice(0, 4).join("") : fallback;
}

function glyphValue(value: unknown, fallback: string): string {
  if (typeof value !== "string") return fallback;
  return Array.from(value)[0] ?? fallback;
}

function textValue(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

function nullableTextValue(value: unknown, fallback: string | null): string | null {
  if (value === null) return null;
  return typeof value === "string" ? value : fallback;
}

function stringList(value: unknown, fallback: string[]): string[] {
  if (!Array.isArray(value)) return fallback;
  return value.filter((item): item is string => typeof item === "string");
}

function isColor(value: string): boolean {
  return isHexColor(value) || /^[a-zA-Z]+$/.test(value);
}

function isHexColor(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

function boolValue(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function numberValue(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function envBool(env: EnvMap, name: string, fallback: boolean): boolean {
  const value = env[name];
  if (value === "1" || value === "true" || value === "yes") return true;
  if (value === "0" || value === "false" || value === "no") return false;
  return fallback;
}

function envNumber(env: EnvMap, name: string, fallback: number): number {
  const parsed = Number(env[name]);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function clampNumber(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
