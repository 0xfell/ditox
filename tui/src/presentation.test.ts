import { describe, expect, test } from "bun:test";
import {
  entryAccent,
  entryMeta,
  entryPreview,
  entryPreviewSegments,
  fitRowMeta,
  formatBytes,
  formatClearedStatus,
  formatClearKind,
  formatCopiedCountStatus,
  formatDeletedStatus,
  formatEntriesStatus,
  formatFilter,
  formatViewStatus,
  previewModel,
  previewMetaTemplateValues,
  previewWindow,
  scrollbarCells,
  statusTone,
  truncateText,
  watcherStatusView,
} from "./presentation";
import { resolveTuiConfig } from "./tui-config";
import type { Entry } from "./types";

const entry: Entry = {
  id: 7,
  kind: "text",
  mime: "text/plain;charset=utf-8",
  content: "hello\nworld",
  preview: "hello",
  hash: "abc123",
  favorite: false,
  created_at_ms: 1000,
  byte_len: 11,
  source_app: null,
  blob_path: null,
  image_width: null,
  image_height: null,
};

describe("presentation", () => {
  test("formats byte sizes for row metadata", () => {
    expect(formatBytes(42)).toBe("42 B");
    expect(formatBytes(1536)).toBe("1.5 KiB");
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0 MiB");
    expect(formatBytes(1536, { sizeKibUnit: "KB" })).toBe("1.5 KB");
  });

  test("builds stable compact entry metadata", () => {
    expect(entryMeta({ ...entry, favorite: true }, 66_000)).toBe("TXT 1m   11 B PIN");
    expect(entryMeta({ ...entry, favorite: true }, 66_000, { kindText: "STR", rowPinnedLabel: "sv" })).toBe("STR 1m   11 B sv ");
    expect(entryMeta({ ...entry, favorite: true }, 66_000, { ageMinutesUnit: "min", sizeBytesUnit: "bytes", rowPinnedLabel: "*" })).toBe(
      "TXT 1min 11 bytes *  ",
    );
    expect(
      entryMeta(
        { ...entry, favorite: true },
        66_000,
        {
          rowMetaTemplate: "{kind}|{age}|{size}|{pinnedSlot}",
          rowPinnedSlotTemplate: "[{pinnedRaw}]",
          rowPinnedLabel: "saved",
          ageMinutesUnit: "min",
        },
        { rowAgeWidth: 5, rowSizeWidth: 4, rowPinnedWidth: 2 },
      ),
    ).toBe("TXT| 1min|11 B|[saved]");
    expect(
      entryMeta(
        { ...entry, favorite: false },
        66_000,
        { rowMetaTemplate: "{kind}|{age}|{size}|{pinnedSlot}", rowUnpinnedSlotTemplate: "[-]" },
        { rowAgeWidth: 1, rowSizeWidth: 3, rowPinnedWidth: 0 },
      ),
    ).toBe("TXT|1m|11 B|[-]");
  });

  test("builds rich row metadata template values", () => {
    expect(
      entryMeta(
        {
          ...entry,
          favorite: true,
          source_app: "browser",
          blob_path: "/tmp/ditox/blob.png",
          image_width: 32,
          image_height: 24,
          hash: "abcdef1234567890",
        },
        66_000,
        {
          entryIdPrefix: "clip:",
          pinned: "saved",
          rowPinnedLabel: "SV",
          rowMetaTemplate: "{entryIdPrefix}{id}|{sourceApp}|{mime}|{dimensions}|{hash}|{hashFull}|{blob}|{pinnedRaw}|{pinned}",
        },
        { rowAgeWidth: 1, rowSizeWidth: 1, rowPinnedWidth: 2, rowMetaHashLength: 6 },
      ),
    ).toBe("clip:7|browser|text/plain;charset=utf-8|32x24|abcdef|abcdef1234567890|/tmp/ditox/blob.png|SV|SV");
  });

  test("fits custom row metadata into its reserved slot", () => {
    expect(fitRowMeta("SRC:very-long-browser-name", 12)).toBe("SRC:very-...");
    expect(fitRowMeta("TXT", 6)).toBe("TXT   ");
    expect(fitRowMeta("abcdef", 4, { textTruncationMarker: "~" })).toBe("abc~");
    expect(fitRowMeta("hidden", 0)).toBe("");
  });

  test("formats filter labels from configuration", () => {
    expect(formatFilter("images")).toBe("IMAGES");
    expect(formatFilter("images", { filterImages: "PICS" })).toBe("PICS");
    expect(formatFilter("favorites", { filterFavorites: "SAVED" })).toBe("SAVED");
    expect(formatFilter("today", { filterToday: "RECENT" })).toBe("RECENT");
  });

  test("formats clear-kind labels from configuration", () => {
    expect(formatClearKind("all")).toBe("all");
    expect(formatClearKind("text", { clearKindText: "strings" })).toBe("strings");
    expect(formatClearKind("images", { clearKindImages: "pictures" })).toBe("pictures");
  });

  test("formats view status without changing configured casing", () => {
    expect(formatViewStatus(true)).toBe("pinned view");
    expect(formatViewStatus(false)).toBe("all view");
    expect(formatViewStatus(true, { statusPinnedView: "SAVED", statusViewSuffix: "mode" })).toBe("SAVED mode");
    expect(formatViewStatus(false, { statusAllView: "EVERYTHING", statusViewSuffix: "mode" })).toBe("EVERYTHING mode");
  });

  test("formats operation statuses from templates", () => {
    expect(formatCopiedCountStatus(2)).toBe("copied 2");
    expect(formatEntriesStatus(7)).toBe("7 entries");
    expect(formatEntriesStatus(7, { statusEntries: "clips", statusEntriesTemplate: "{entries}: {count}" })).toBe("clips: 7");
    expect(formatDeletedStatus(3, { statusDeletedTemplate: "{count} gone" })).toBe("3 gone");
    expect(formatClearedStatus(4, true)).toBe("cleared 4; kept pinned");
    expect(
      formatClearedStatus(5, false, {
        statusClearedPrefix: "wiped",
        statusClearedTemplate: "{prefix}: {count} ({pinned})",
        statusIncludedPinned: "saved included",
      }),
    ).toBe("wiped: 5 (saved included)");
  });

  test("formats truncation and whitespace through labels", () => {
    expect(truncateText("alpha beta gamma", 10)).toBe("alpha b...");
    expect(truncateText("alpha\nbeta\tgamma", 12, { textTruncationMarker: "~", textWhitespaceReplacement: "_" })).toBe("alpha_beta_~");
    expect(truncateText("abcdef", 2, { textTruncationMarker: "~~" })).toBe("~~");
    expect(entryPreview({ ...entry, preview: "alpha\nbeta gamma" }, 11, { textTruncationMarker: "~", textWhitespaceReplacement: "_" })).toBe("alpha_beta~");
  });

  test("segments row preview text for configurable search highlighting", () => {
    expect(entryPreviewSegments({ ...entry, preview: "Alpha Needle Beta needle" }, 80, "needle")).toEqual([
      { text: "Alpha ", match: false },
      { text: "Needle", match: true },
      { text: " Beta ", match: false },
      { text: "needle", match: true },
    ]);
    expect(entryPreviewSegments({ ...entry, preview: "Alpha\nNeedle" }, 80, "alpha needle", { textWhitespaceReplacement: "_" })).toEqual([
      { text: "Alpha_Needle", match: true },
    ]);
    expect(entryPreviewSegments({ ...entry, preview: "Needle Value" }, 80, "nvl")).toEqual([
      { text: "N", match: true },
      { text: "eedle ", match: false },
      { text: "V", match: true },
      { text: "a", match: false },
      { text: "l", match: true },
      { text: "ue", match: false },
    ]);
    expect(entryPreviewSegments({ ...entry, preview: "ab-c" }, 80, "abc")).toEqual([
      { text: "ab", match: true },
      { text: "-", match: false },
      { text: "c", match: true },
    ]);
    expect(entryPreviewSegments({ ...entry, preview: "Alpha Needle Beta" }, 9, "beta")).toEqual([{ text: "Alpha ...", match: false }]);
  });

  test("resolves entry accent colors from per-surface style", () => {
    const style = {
      accent: "#111111",
      favorite: "#222222",
      image: "#333333",
    };

    expect(entryAccent(style, entry)).toBe("#111111");
    expect(entryAccent(style, { ...entry, favorite: true })).toBe("#222222");
    expect(entryAccent(style, { ...entry, kind: "image" })).toBe("#333333");
  });

  test("models image preview rows with semantic gutters", () => {
    const rows = previewModel({ ...entry, kind: "image", image_width: 2, image_height: 3 }, 6);
    expect(rows.map((row) => `${row.gutter}:${row.text}`)).toContain("dims:2x3");
    const custom = previewModel(
      { ...entry, kind: "image", image_width: null, image_height: null, blob_path: null },
      6,
      {
        previewDimensionsGutter: "sizepx",
        previewUnknownDimensions: "none",
        previewBlobGutter: "file",
        previewBlobMissing: "missing",
        sizeBytesUnit: "octets",
      },
    );
    expect(custom.map((row) => `${row.gutter}:${row.text}`)).toContain("sizepx:none");
    expect(custom.map((row) => `${row.gutter}:${row.text}`)).toContain("file:missing");
    expect(custom.map((row) => `${row.gutter}:${row.text}`)).toContain("size:11 octets");
    expect(
      previewModel({ ...entry, kind: "image", image_width: 2, image_height: 3, blob_path: "/tmp/blob" }, 6, {}, { previewImageFields: ["mime", "blob"] }).map(
        (row) => `${row.gutter}:${row.text}`,
      ),
    ).toEqual(["mime:text/plain;charset=utf-8", "blob:/tmp/blob"]);
  });

  test("builds rich preview metadata template values", () => {
    const values = previewMetaTemplateValues(
      {
        ...entry,
        favorite: true,
        source_app: "browser",
        blob_path: "/tmp/blob",
        image_width: 64,
        image_height: 32,
        hash: "abcdef123456",
        created_at_ms: 1_000,
      },
      { pinned: "saved", previewPinnedSuffixTemplate: " [{pinnedRaw}]", sizeBytesUnit: "bytes" },
      6,
      3_000,
    );

    expect(values).toMatchObject({
      kind: "TXT",
      id: 7,
      hash: "abcdef",
      hashShort: "abcdef",
      hashFull: "abcdef123456",
      sourceApp: "browser",
      blob: "/tmp/blob",
      dimensions: "64x32",
      size: "11 bytes",
      age: "2s",
      pinned: "saved",
      pinnedRaw: "saved",
      pinnedSuffix: " [saved]",
    });
  });

  test("creates a bounded scrollbar thumb", () => {
    expect(scrollbarCells(10, 5, 4)).toEqual(["|", "|", "#", "|"]);
  });

  test("windows preview lines by offset", () => {
    const rows = previewModel({ ...entry, content: "a\nb\nc\nd" }, 10);
    expect(previewWindow(rows, 1, 2).map((row) => row.text)).toEqual(["b", "c"]);
    expect(previewWindow(rows, 99, 2).map((row) => row.text)).toEqual(["d"]);
    expect(previewModel({ ...entry, content: "a\nb" }, 10, {}, 1).map((row) => row.gutter)).toEqual(["1", "2"]);
  });

  test("maps status text to semantic tones", () => {
    expect(statusTone("copied 2")).toBe("success");
    expect(statusTone("stored 2", { statusCopiedCountPrefix: "stored" })).toBe("success");
    expect(statusTone("PasteBackFailed")).toBe("error");
    expect(statusTone("paste transport failed", { errorPasteBackFailed: "paste transport failed" })).toBe("error");
    expect(statusTone("paused watcher")).toBe("warning");
    expect(statusTone("archived 2", {}, { success: ["archived"], warning: ["holding"], error: ["broken"] })).toBe("success");
    expect(statusTone("holding refresh", {}, { success: ["archived"], warning: ["holding"], error: ["broken"] })).toBe("warning");
    expect(statusTone("broken pipe", {}, { success: ["archived"], warning: ["holding"], error: ["broken"] })).toBe("error");
  });

  test("models watcher status with text and semantic tones", () => {
    const labels = resolveTuiConfig().labels;
    expect(watcherStatusView(null, labels).text).toBe("watcher stopped");
    expect(watcherStatusView(null, labels).tone).toBe("muted");
    expect(watcherStatusView({ running: true, paused: false, backend: "wl-clipboard", poll_interval_ms: 500, last_seen_ms: 1_000, last_error: null }, labels, 3_000)).toEqual({
      text: "watcher live 2s",
      tone: "success",
    });
    expect(watcherStatusView({ running: false, paused: true, backend: "wl-clipboard", poll_interval_ms: 500, last_seen_ms: 1_000, last_error: null }, labels, 3_000)).toEqual({
      text: "watcher paused",
      tone: "warning",
    });
    expect(watcherStatusView({ running: false, paused: false, backend: "wl-clipboard", poll_interval_ms: 500, last_seen_ms: 1_000, last_error: null }, labels, 65_000)).toEqual({
      text: "watcher stale 1m",
      tone: "warning",
    });
    expect(watcherStatusView({ running: false, paused: false, backend: "wl-clipboard", poll_interval_ms: 500, last_seen_ms: null, last_error: "boom" }, { ...labels, watcherErrorSeparator: " -> " })).toEqual({
      text: "watcher stopped -> boom",
      tone: "error",
    });
    expect(
      watcherStatusView(
        { running: true, paused: false, backend: "wl-clipboard", poll_interval_ms: 500, last_seen_ms: 1_000, last_error: null },
        { ...labels, ageSecondsUnit: "sec" },
        3_000,
      ).text,
    ).toBe("watcher live 2sec");
  });
});
