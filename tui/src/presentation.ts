import type { Entry, EntryFilter, WatcherStatus } from "./types";
import type { TuiTheme } from "./theme";
import type { TuiLabels, TuiStatusToneMatchers, TuiSurfaceStyle } from "./tui-config";
import type { ContentAlign, PreviewImageField, UiConfig } from "./ui-config";

export type PreviewLine = {
  gutter: string;
  text: string;
  tone: "primary" | "secondary" | "muted" | "accent" | "error" | "success";
};

export type TextSegment = {
  text: string;
  match: boolean;
};

export type EntryAccentStyle = Pick<TuiSurfaceStyle, "accent" | "favorite" | "image"> | TuiTheme;

export function entryAccent(style: EntryAccentStyle, entry: Entry): string {
  if ("favorite" in style) {
    if (entry.favorite) return style.favorite;
    if (entry.kind === "image") return style.image;
    return style.accent;
  }
  if (entry.favorite) return style.accentFavorite;
  if (entry.kind === "image") return style.accentImage;
  return style.accentText;
}

type SizeLabelSet = Pick<TuiLabels, "sizeBytesUnit" | "sizeKibUnit" | "sizeMibUnit">;
type AgeLabelSet = Pick<TuiLabels, "ageSecondsUnit" | "ageMinutesUnit" | "ageHoursUnit" | "ageDaysUnit">;
type TextFormatLabelSet = Pick<TuiLabels, "textTruncationMarker" | "textWhitespaceReplacement">;
type EntryLabelSet = Pick<
  TuiLabels,
  | "kindText"
  | "kindImage"
  | "pinned"
  | "rowPinnedLabel"
  | "rowMetaTemplate"
  | "rowPinnedSlotTemplate"
  | "rowUnpinnedSlotTemplate"
  | "entryIdPrefix"
  | "previewUnknownDimensions"
  | "previewBlobMissing"
> &
  SizeLabelSet &
  AgeLabelSet &
  TextFormatLabelSet;
type EntryMetaLayout = Pick<
  UiConfig,
  "rowAgeWidth" | "rowAgeAlign" | "rowSizeWidth" | "rowSizeAlign" | "rowPinnedWidth" | "rowPinnedAlign" | "rowMetaHashLength"
>;
type FilterLabelSet = Pick<TuiLabels, "filterAll" | "filterText" | "filterImages" | "filterFavorites" | "filterToday">;
type ClearKindLabelSet = Pick<TuiLabels, "clearKindAll" | "clearKindText" | "clearKindImages">;
type ViewStatusLabelSet = Pick<TuiLabels, "statusPinnedView" | "statusAllView" | "statusViewSuffix">;
type OperationStatusLabelSet = Pick<
  TuiLabels,
  | "statusCopiedCountPrefix"
  | "statusCopiedCountTemplate"
  | "statusDeletedPrefix"
  | "statusDeletedTemplate"
  | "statusClearedPrefix"
  | "statusClearedTemplate"
  | "statusEntries"
  | "statusEntriesTemplate"
  | "statusKeptPinned"
  | "statusIncludedPinned"
>;
type PinStatusLabelSet = Pick<TuiLabels, "entryIdPrefix" | "statusPinnedPrefix" | "statusUnpinnedPrefix" | "statusPinTemplate">;
type WatcherLabelSet = Pick<
  TuiLabels,
  | "watcherRunning"
  | "watcherPaused"
  | "watcherStale"
  | "watcherStopped"
  | "watcherErrorSeparator"
  | "watcherRunningTemplate"
  | "watcherPausedTemplate"
  | "watcherStaleTemplate"
  | "watcherStoppedTemplate"
  | "watcherErrorTemplate"
> &
  AgeLabelSet;
type PreviewLabelSet = Pick<
  TuiLabels,
  | "noEntryTitle"
  | "noEntryHelp"
  | "previewTypeGutter"
  | "previewMimeGutter"
  | "previewSizeGutter"
  | "previewDimensionsGutter"
  | "previewHashGutter"
  | "previewBlobGutter"
  | "previewImageEntry"
  | "previewUnknownDimensions"
  | "previewBlobMissing"
  | "previewGutterSeparator"
  | "previewTextGutterTemplate"
> &
  SizeLabelSet &
  TextFormatLabelSet;
type PreviewLayoutSet = Pick<UiConfig, "previewLineNumberWidth" | "previewImageFields">;
type PreviewMetaTemplateLabelSet = Pick<
  TuiLabels,
  | "kindText"
  | "kindImage"
  | "entryIdPrefix"
  | "pinned"
  | "previewUnknownDimensions"
  | "previewBlobMissing"
  | "previewMetaHashLabel"
  | "previewMetaSeparator"
  | "previewMetaLabelSeparator"
  | "previewPinnedSuffixTemplate"
> &
  SizeLabelSet &
  AgeLabelSet;

const defaultTextFormatLabels: TextFormatLabelSet = {
  textTruncationMarker: "...",
  textWhitespaceReplacement: " ",
};

const defaultSizeLabels: SizeLabelSet = {
  sizeBytesUnit: "B",
  sizeKibUnit: "KiB",
  sizeMibUnit: "MiB",
};

const defaultAgeLabels: AgeLabelSet = {
  ageSecondsUnit: "s",
  ageMinutesUnit: "m",
  ageHoursUnit: "h",
  ageDaysUnit: "d",
};

const defaultEntryLabels: EntryLabelSet = {
  kindText: "TXT",
  kindImage: "IMG",
  pinned: "PIN",
  rowPinnedLabel: "PIN",
  rowMetaTemplate: "{kind} {age} {size}{pinnedSlot}",
  rowPinnedSlotTemplate: " {pinned}",
  rowUnpinnedSlotTemplate: "    ",
  entryIdPrefix: "#",
  previewUnknownDimensions: "unknown",
  previewBlobMissing: "not stored",
  ...defaultSizeLabels,
  ...defaultAgeLabels,
  ...defaultTextFormatLabels,
};

const defaultEntryMetaLayout: EntryMetaLayout = {
  rowAgeWidth: 2,
  rowAgeAlign: "right",
  rowSizeWidth: 6,
  rowSizeAlign: "right",
  rowPinnedWidth: 3,
  rowPinnedAlign: "left",
  rowMetaHashLength: 8,
};

const defaultFilterLabels: FilterLabelSet = {
  filterAll: "ALL",
  filterText: "TEXT",
  filterImages: "IMAGES",
  filterFavorites: "FAVORITES",
  filterToday: "TODAY",
};

const defaultClearKindLabels: ClearKindLabelSet = {
  clearKindAll: "all",
  clearKindText: "text",
  clearKindImages: "images",
};

const defaultViewStatusLabels: ViewStatusLabelSet = {
  statusPinnedView: "pinned",
  statusAllView: "all",
  statusViewSuffix: "view",
};

const defaultOperationStatusLabels: OperationStatusLabelSet = {
  statusCopiedCountPrefix: "copied",
  statusCopiedCountTemplate: "{prefix} {count}",
  statusDeletedPrefix: "deleted",
  statusDeletedTemplate: "{prefix} {count}",
  statusClearedPrefix: "cleared",
  statusClearedTemplate: "{prefix} {count}; {pinned}",
  statusEntries: "entries",
  statusEntriesTemplate: "{count} {entries}",
  statusKeptPinned: "kept pinned",
  statusIncludedPinned: "included pinned",
};

const defaultPinStatusLabels: PinStatusLabelSet = {
  entryIdPrefix: "#",
  statusPinnedPrefix: "pinned",
  statusUnpinnedPrefix: "unpinned",
  statusPinTemplate: "{prefix} {entryIdPrefix}{id}",
};

const defaultWatcherLabels: WatcherLabelSet = {
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
  ...defaultAgeLabels,
};

const defaultStatusToneMatchers: TuiStatusToneMatchers = {
  error: ["error", "failed", "not found", "exited", "unavailable"],
  success: ["copied", "pasted", "cleared", "pinned", "unpinned"],
  warning: ["paused"],
};

const defaultPreviewLabels: PreviewLabelSet = {
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
  previewGutterSeparator: "  ",
  previewTextGutterTemplate: "{linePadded}",
  ...defaultSizeLabels,
  ...defaultTextFormatLabels,
};

const defaultPreviewLayout: PreviewLayoutSet = {
  previewLineNumberWidth: 3,
  previewImageFields: ["type", "mime", "size", "dimensions", "hash", "blob"],
};

const defaultPreviewMetaTemplateLabels: PreviewMetaTemplateLabelSet = {
  kindText: "TXT",
  kindImage: "IMG",
  entryIdPrefix: "#",
  pinned: "pinned",
  previewUnknownDimensions: "unknown",
  previewBlobMissing: "not stored",
  previewMetaHashLabel: "hash",
  previewMetaSeparator: "  ",
  previewMetaLabelSeparator: " ",
  previewPinnedSuffixTemplate: "{separator}{pinned}",
  ...defaultSizeLabels,
  ...defaultAgeLabels,
};

export function entryKindLabel(entry: Entry, labels: Partial<EntryLabelSet> = {}): string {
  return entry.kind === "image" ? (labels.kindImage ?? defaultEntryLabels.kindImage) : (labels.kindText ?? defaultEntryLabels.kindText);
}

/** Most recent activity time for an entry: the later of when it was added and
 * when it was last used (copied/pasted). Drives both list ordering (backend)
 * and the displayed age so a re-used entry reads as recent. */
export function entryActivityMs(entry: Entry): number {
  return Math.max(entry.created_at_ms, entry.last_used_at_ms ?? 0);
}

export function entryMeta(entry: Entry, now = Date.now(), labels: Partial<EntryLabelSet> = {}, layout: Partial<EntryMetaLayout> = {}): string {
  const text = { ...defaultEntryLabels, ...labels };
  const metrics = { ...defaultEntryMetaLayout, ...layout };
  const age = fitText(formatAge(entryActivityMs(entry), now, text), metrics.rowAgeWidth, metrics.rowAgeAlign, false);
  const size = fitText(formatBytes(entry.byte_len, text), metrics.rowSizeWidth, metrics.rowSizeAlign, false);
  const pinnedRaw = text.rowPinnedLabel;
  const pinned = fitText(pinnedRaw, metrics.rowPinnedWidth, metrics.rowPinnedAlign, true);
  const pinnedSlot = entry.favorite
    ? applyTemplate(text.rowPinnedSlotTemplate, { pinned, pinnedRaw })
    : applyTemplate(text.rowUnpinnedSlotTemplate, { pinned, pinnedRaw });
  const dimensions =
    entry.image_width !== null && entry.image_height !== null ? `${entry.image_width}x${entry.image_height}` : text.previewUnknownDimensions;
  const hashLength = Math.max(0, Math.floor(metrics.rowMetaHashLength));
  return applyTemplate(text.rowMetaTemplate, {
    kind: entryKindLabel(entry, text),
    entryIdPrefix: text.entryIdPrefix,
    id: entry.id,
    age,
    size,
    hash: entry.hash.slice(0, hashLength),
    hashShort: entry.hash.slice(0, hashLength),
    hashFull: entry.hash,
    mime: entry.mime,
    dimensions,
    sourceApp: entry.source_app ?? "",
    blob: entry.blob_path ?? text.previewBlobMissing,
    pinned,
    pinnedRaw,
    pinnedSlot,
  });
}

export function fitRowMeta(value: string, width: number, labels: Partial<TextFormatLabelSet> = {}, align: ContentAlign = "left"): string {
  const target = Math.max(0, Math.floor(width));
  if (target === 0) return "";
  const clipped = truncateText(value, target, labels);
  const extra = Math.max(0, target - clipped.length);
  if (align === "right") return `${" ".repeat(extra)}${clipped}`;
  if (align === "center") {
    const left = Math.floor(extra / 2);
    return `${" ".repeat(left)}${clipped}${" ".repeat(extra - left)}`;
  }
  return `${clipped}${" ".repeat(extra)}`;
}

export function entryPreview(entry: Entry, width: number, labels: Partial<Pick<TuiLabels, "emptyEntryPreview"> & TextFormatLabelSet> = {}): string {
  const raw = entry.preview ?? entry.content ?? labels.emptyEntryPreview ?? "(empty)";
  return truncateText(typeof raw === "string" ? raw : String(raw), width, labels);
}

export function entryPreviewSegments(
  entry: Entry,
  width: number,
  query: string,
  labels: Partial<Pick<TuiLabels, "emptyEntryPreview"> & TextFormatLabelSet> = {},
): TextSegment[] {
  const preview = entryPreview(entry, width, labels);
  const needle = normalizeMatchText(query, labels).trim();
  if (preview.length === 0 || needle.length === 0) return [{ text: String(preview ?? ""), match: false }];

  const literalSegments = literalPreviewSegments(preview, needle);
  if (literalSegments.some((segment) => segment.match)) return literalSegments;

  return fuzzyPreviewSegments(preview, needle) ?? [{ text: String(preview ?? ""), match: false }];
}

function literalPreviewSegments(preview: string, needle: string): TextSegment[] {
  const haystack = preview.toLocaleLowerCase();
  const match = needle.toLocaleLowerCase();
  let cursor = 0;
  let index = haystack.indexOf(match, cursor);
  const ranges: Array<[number, number]> = [];

  while (index >= 0) {
    ranges.push([index, index + needle.length]);
    cursor = index + needle.length;
    index = haystack.indexOf(match, cursor);
  }

  return previewSegmentsFromRanges(preview, ranges);
}

function fuzzyPreviewSegments(preview: string, needle: string): TextSegment[] | null {
  const haystack = preview.toLocaleLowerCase();
  const chars = [...needle.toLocaleLowerCase()].filter((char) => char.trim().length > 0);
  if (chars.length === 0) return null;

  const ranges: Array<[number, number]> = [];
  let cursor = 0;
  for (const char of chars) {
    const index = haystack.indexOf(char, cursor);
    if (index < 0) return null;
    ranges.push([index, index + char.length]);
    cursor = index + char.length;
  }

  return previewSegmentsFromRanges(preview, ranges);
}

function previewSegmentsFromRanges(preview: string, ranges: Array<[number, number]>): TextSegment[] {
  const safePreview = String(preview ?? "");
  if (ranges.length === 0) return [{ text: safePreview, match: false }];
  const segments: TextSegment[] = [];
  let cursor = 0;
  for (const [start, end] of ranges) {
    if (start > cursor) segments.push({ text: safePreview.slice(cursor, start), match: false });
    const previous = segments.at(-1);
    const text = safePreview.slice(start, end);
    if (previous?.match) previous.text = String(previous.text ?? "") + text;
    else segments.push({ text, match: true });
    cursor = end;
  }
  if (cursor < safePreview.length) segments.push({ text: safePreview.slice(cursor), match: false });
  return segments;
}

export function statusTone(
  status: string,
  labels?: Partial<TuiLabels>,
  matchers: TuiStatusToneMatchers = defaultStatusToneMatchers,
): "muted" | "error" | "success" | "warning" {
  const value = status.toLowerCase();
  const errorLabels = [
    labels?.errorDitoxdMissing,
    labels?.errorClipboardToolMissing,
    labels?.errorPasteToolMissing,
    labels?.errorClipboardWriteFailed,
    labels?.errorPasteBackFailed,
    labels?.errorDitoxdExited,
    labels?.errorProcessTemplate,
    labels?.errorRpcTemplate,
  ];
  if (matchesAnyStatusToken(value, [...matchers.error, ...errorLabels], true)) return "error";
  const successLabels = [
    labels?.statusCopied,
    labels?.statusPasted,
    labels?.statusCopiedCountPrefix,
    labels?.statusClearedPrefix,
    labels?.statusDeletedPrefix,
    labels?.statusPinnedPrefix,
    labels?.statusUnpinnedPrefix,
  ];
  if (matchesAnyStatusToken(value, [...matchers.success, ...successLabels])) return "success";
  if (matchesAnyStatusToken(value, matchers.warning)) return "warning";
  return "muted";
}

function matchesAnyStatusToken(value: string, tokens: Array<string | undefined>, stripTemplates = false): boolean {
  return tokens.some((token) => {
    if (typeof token !== "string" || token.length === 0) return false;
    const normalized = (stripTemplates ? token.replace(/\{[^}]+\}/g, "") : token).trim().toLowerCase();
    if (normalized.length === 0) return false;
    if (value.includes(normalized)) return true;
    return stripTemplates && matchesStatusTemplate(value, token);
  });
}

function matchesStatusTemplate(value: string, token: string): boolean {
  const staticText = token.replace(/\{[^}]+\}/g, "").trim();
  if (staticText.length === 0) return false;
  const pattern = token
    .toLowerCase()
    .split(/\{[^}]+\}/g)
    .map(escapeRegExp)
    .join(".*");
  return new RegExp(pattern).test(value);
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export type WatcherStatusView = {
  text: string;
  tone: "muted" | "error" | "success" | "warning";
};

export function watcherStatusView(status: WatcherStatus | null, labels: Partial<WatcherLabelSet>, now = Date.now()): WatcherStatusView {
  const text = { ...defaultWatcherLabels, ...labels };
  if (!status) return { text: watcherStatusText(text.watcherStoppedTemplate, text.watcherStopped, "", text), tone: "muted" };
  if (status.last_error) return { text: watcherStatusText(text.watcherErrorTemplate, text.watcherStopped, "", text, status.last_error), tone: "error" };
  if (status.paused) return { text: watcherStatusText(text.watcherPausedTemplate, text.watcherPaused, watcherAge(status.last_seen_ms, now, text), text), tone: "warning" };
  if (status.running) return { text: watcherStatusText(text.watcherRunningTemplate, text.watcherRunning, watcherAge(status.last_seen_ms, now, text), text), tone: "success" };
  if (status.last_seen_ms !== null) return { text: watcherStatusText(text.watcherStaleTemplate, text.watcherStale, watcherAge(status.last_seen_ms, now, text), text), tone: "warning" };
  return { text: watcherStatusText(text.watcherStoppedTemplate, text.watcherStopped, "", text), tone: "error" };
}

function watcherAge(lastSeenMs: number | null, now: number, labels: Partial<AgeLabelSet>): string {
  if (lastSeenMs === null) return "";
  return formatAge(lastSeenMs, now, labels);
}

function watcherStatusText(template: string, status: string, age: string, labels: WatcherLabelSet, error = ""): string {
  return applyTemplate(template, {
    status,
    age,
    error,
    separator: labels.watcherErrorSeparator,
  });
}

export function formatFilter(filter: EntryFilter, labels: Partial<FilterLabelSet> = {}): string {
  const text = { ...defaultFilterLabels, ...labels };
  switch (filter) {
    case "text":
      return text.filterText;
    case "images":
      return text.filterImages;
    case "favorites":
      return text.filterFavorites;
    case "today":
      return text.filterToday;
    default:
      return text.filterAll;
  }
}

export function formatClearKind(kind: "all" | "text" | "images", labels: Partial<ClearKindLabelSet> = {}): string {
  const text = { ...defaultClearKindLabels, ...labels };
  switch (kind) {
    case "text":
      return text.clearKindText;
    case "images":
      return text.clearKindImages;
    default:
      return text.clearKindAll;
  }
}

export function formatViewStatus(pinnedOnly: boolean, labels: Partial<ViewStatusLabelSet> = {}): string {
  const text = { ...defaultViewStatusLabels, ...labels };
  return `${pinnedOnly ? text.statusPinnedView : text.statusAllView} ${text.statusViewSuffix}`;
}

export function formatCopiedCountStatus(count: number, labels: Partial<OperationStatusLabelSet> = {}): string {
  const text = { ...defaultOperationStatusLabels, ...labels };
  return applyTemplate(text.statusCopiedCountTemplate, {
    prefix: text.statusCopiedCountPrefix,
    count: String(count),
    pinned: "",
  });
}

export function formatDeletedStatus(count: number, labels: Partial<OperationStatusLabelSet> = {}): string {
  const text = { ...defaultOperationStatusLabels, ...labels };
  return applyTemplate(text.statusDeletedTemplate, {
    prefix: text.statusDeletedPrefix,
    count: String(count),
    pinned: "",
  });
}

export function formatClearedStatus(count: number, preservePinned: boolean, labels: Partial<OperationStatusLabelSet> = {}): string {
  const text = { ...defaultOperationStatusLabels, ...labels };
  return applyTemplate(text.statusClearedTemplate, {
    prefix: text.statusClearedPrefix,
    count: String(count),
    pinned: preservePinned ? text.statusKeptPinned : text.statusIncludedPinned,
  });
}

export function formatPinStatus(entry: Entry, pinned: boolean, labels: Partial<PinStatusLabelSet> = {}): string {
  const text = { ...defaultPinStatusLabels, ...labels };
  return applyTemplate(text.statusPinTemplate, {
    prefix: pinned ? text.statusPinnedPrefix : text.statusUnpinnedPrefix,
    id: String(entry.id),
    entryIdPrefix: text.entryIdPrefix,
    pinned: pinned ? "true" : "false",
  });
}

export function formatEntriesStatus(count: number, labels: Partial<OperationStatusLabelSet> = {}): string {
  const text = { ...defaultOperationStatusLabels, ...labels };
  return applyTemplate(text.statusEntriesTemplate, {
    prefix: "",
    count: String(count),
    entries: text.statusEntries,
    pinned: "",
  });
}

export function previewModel(
  entry: Entry | undefined,
  maxLines: number,
  labels: Partial<PreviewLabelSet> = {},
  layout: Partial<PreviewLayoutSet> | number = {},
  wrapWidth = 0,
): PreviewLine[] {
  const text = { ...defaultPreviewLabels, ...labels };
  const previewLayout: PreviewLayoutSet =
    typeof layout === "number"
      ? { ...defaultPreviewLayout, previewLineNumberWidth: layout }
      : {
          ...defaultPreviewLayout,
          ...layout,
          previewImageFields: Array.isArray(layout.previewImageFields) ? layout.previewImageFields : defaultPreviewLayout.previewImageFields,
        };
  if (!entry) {
    return [
      { gutter: "", text: text.noEntryTitle, tone: "muted" },
      { gutter: "", text: text.noEntryHelp, tone: "secondary" },
    ];
  }

  if (entry.kind === "image") {
    const dimensions =
      entry.image_width !== null && entry.image_height !== null ? `${entry.image_width}x${entry.image_height}` : text.previewUnknownDimensions;
    const rows: Record<PreviewImageField, PreviewLine> = {
      type: { gutter: text.previewTypeGutter, text: text.previewImageEntry, tone: "accent" },
      mime: { gutter: text.previewMimeGutter, text: entry.mime, tone: "primary" },
      size: { gutter: text.previewSizeGutter, text: formatBytes(entry.byte_len, text), tone: "primary" },
      dimensions: { gutter: text.previewDimensionsGutter, text: dimensions, tone: "primary" },
      hash: { gutter: text.previewHashGutter, text: entry.hash, tone: "secondary" },
      blob: { gutter: text.previewBlobGutter, text: entry.blob_path ?? text.previewBlobMissing, tone: entry.blob_path ? "secondary" : "muted" },
    };
    return previewLayout.previewImageFields.map((field) => rows[field]).slice(0, maxLines);
  }

  const sourceLines = entry.content.split(/\r?\n/);
  if (sourceLines.length === 0) return [{ gutter: "1", text: "", tone: "primary" }];
  const rows: PreviewLine[] = [];
  for (let index = 0; index < sourceLines.length && rows.length < maxLines; index += 1) {
    const lineNumber = String(index + 1);
    const linePadded = lineNumber.padStart(Math.max(1, previewLayout.previewLineNumberWidth), " ");
    const gutter = applyTemplate(text.previewTextGutterTemplate, { line: lineNumber, lineNumber, linePadded, lineNumberPadded: linePadded });
    // Soft-wrap long lines so the full string is visible instead of being
    // hard-truncated at the pane edge. Continuation rows keep a blank gutter so
    // the line number only shows once.
    const segments = wrapWidth > 0 ? wrapPreviewText(sourceLines[index] ?? "", wrapWidth) : [sourceLines[index] ?? ""];
    for (let segment = 0; segment < segments.length && rows.length < maxLines; segment += 1) {
      rows.push({ gutter: segment === 0 ? gutter : "", text: segments[segment] ?? "", tone: "primary" });
    }
  }
  return rows;
}

/** Soft-wrap a single logical line to `width` columns, breaking on spaces when
 * possible and hard-splitting words that are longer than the width. */
export function wrapPreviewText(value: string, width: number): string[] {
  const target = Math.max(1, Math.floor(width));
  const text = String(value ?? "");
  if (text.length <= target) return [text];
  const segments: string[] = [];
  let remaining = text;
  while (remaining.length > target) {
    const window = remaining.slice(0, target + 1);
    const lastSpace = window.lastIndexOf(" ");
    if (lastSpace <= 0) {
      segments.push(remaining.slice(0, target));
      remaining = remaining.slice(target);
    } else {
      segments.push(remaining.slice(0, lastSpace));
      remaining = remaining.slice(lastSpace + 1);
    }
  }
  if (remaining.length > 0) segments.push(remaining);
  return segments.length > 0 ? segments : [""];
}

export function previewMetaTemplateValues(
  entry: Entry,
  labels: Partial<PreviewMetaTemplateLabelSet> = {},
  hashLength = 12,
  now = Date.now(),
): Record<string, string | number> {
  const text = { ...defaultPreviewMetaTemplateLabels, ...labels };
  const pinnedRaw = text.pinned;
  const pinned = entry.favorite ? text.pinned : "";
  const dimensions =
    entry.image_width !== null && entry.image_height !== null ? `${entry.image_width}x${entry.image_height}` : text.previewUnknownDimensions;
  const pinnedSuffix = entry.favorite
    ? applyTemplate(text.previewPinnedSuffixTemplate, {
        separator: text.previewMetaSeparator,
        pinned,
        pinnedRaw,
      })
    : "";
  return {
    kind: entryKindLabel(entry, text),
    entryIdPrefix: text.entryIdPrefix,
    id: entry.id,
    separator: text.previewMetaSeparator,
    hashLabel: text.previewMetaHashLabel,
    hashLabelSeparator: text.previewMetaLabelSeparator,
    hash: entry.hash.slice(0, Math.max(0, Math.floor(hashLength))),
    hashShort: entry.hash.slice(0, Math.max(0, Math.floor(hashLength))),
    hashFull: entry.hash,
    mime: entry.mime,
    size: formatBytes(entry.byte_len, text),
    age: formatAge(entryActivityMs(entry), now, text),
    dimensions,
    sourceApp: entry.source_app ?? "",
    blob: entry.blob_path ?? text.previewBlobMissing,
    pinned,
    pinnedRaw,
    pinnedSuffix,
  };
}

function applyTemplate(template: string, values: Record<string, string | number>): string {
  return Object.entries(values).reduce((current, [key, value]) => current.replaceAll(`{${key}}`, String(value)), template);
}

function fitText(value: string, width: number, align: ContentAlign, clip: boolean): string {
  const target = Math.max(0, Math.floor(width));
  if (target === 0) return "";
  const chars = Array.from(value);
  const clipped = clip && chars.length > target ? chars.slice(0, target).join("") : value;
  const extra = Math.max(0, target - Array.from(clipped).length);
  if (align === "right") return `${" ".repeat(extra)}${clipped}`;
  if (align === "center") {
    const left = Math.floor(extra / 2);
    return `${" ".repeat(left)}${clipped}${" ".repeat(extra - left)}`;
  }
  return `${clipped}${" ".repeat(extra)}`;
}

export function previewWindow(lines: PreviewLine[], offset: number, rows: number): PreviewLine[] {
  const start = Math.min(Math.max(0, offset), Math.max(0, lines.length - 1));
  return lines.slice(start, start + Math.max(0, rows));
}

export function scrollbarCells(total: number, selectedIndex: number, rows: number, thumb = "#", track = "|"): string[] {
  if (rows <= 0) return [];
  if (total <= rows) return Array.from({ length: rows }, () => " ");
  const thumbSize = Math.max(1, Math.floor((rows / total) * rows));
  const maxStart = rows - thumbSize;
  const start = Math.round((selectedIndex / Math.max(1, total - 1)) * maxStart);
  return Array.from({ length: rows }, (_, index) => (index >= start && index < start + thumbSize ? thumb : track));
}

export function formatBytes(bytes: number, labels: Partial<SizeLabelSet> = {}): string {
  const text = { ...defaultSizeLabels, ...labels };
  if (bytes < 1024) return `${bytes} ${text.sizeBytesUnit}`;
  const kib = bytes / 1024;
  if (kib < 1024) return `${kib.toFixed(kib >= 10 ? 0 : 1)} ${text.sizeKibUnit}`;
  const mib = kib / 1024;
  return `${mib.toFixed(mib >= 10 ? 0 : 1)} ${text.sizeMibUnit}`;
}

export function formatAge(timestampMs: number, now = Date.now(), labels: Partial<AgeLabelSet> = {}): string {
  const text = { ...defaultAgeLabels, ...labels };
  const seconds = Math.max(0, Math.floor((now - timestampMs) / 1000));
  if (seconds < 60) return `${seconds}${text.ageSecondsUnit}`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}${text.ageMinutesUnit}`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}${text.ageHoursUnit}`;
  return `${Math.floor(hours / 24)}${text.ageDaysUnit}`;
}

export function truncateText(value: string, width: number, labels: Partial<TextFormatLabelSet> = {}): string {
  if (width <= 0) return "";
  const text = { ...defaultTextFormatLabels, ...labels };
  const whitespaceReplacement = text.textWhitespaceReplacement.length > 0 ? text.textWhitespaceReplacement : defaultTextFormatLabels.textWhitespaceReplacement;
  const marker = text.textTruncationMarker.length > 0 ? text.textTruncationMarker : defaultTextFormatLabels.textTruncationMarker;
  const clean = String(value ?? "").replace(/\s+/g, whitespaceReplacement);
  if (clean.length <= width) return clean;
  if (width <= marker.length) return marker.slice(0, width).padEnd(width, marker[0] ?? ".");
  return `${clean.slice(0, width - marker.length)}${marker}`;
}

function normalizeMatchText(value: string, labels: Partial<TextFormatLabelSet>): string {
  const text = { ...defaultTextFormatLabels, ...labels };
  const whitespaceReplacement = text.textWhitespaceReplacement.length > 0 ? text.textWhitespaceReplacement : defaultTextFormatLabels.textWhitespaceReplacement;
  return String(value ?? "").replace(/\s+/g, whitespaceReplacement);
}
