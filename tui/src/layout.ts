import { isBlockPreviewMime } from "./image-preview";
import { visibleFullPreviewLineCapacity } from "./state";
import type { ResolvedTuiConfig } from "./tui-config";
import type { Entry } from "./types";

// Single source for preview layout math. App.tsx (rendering) and keymap.ts
// (scroll bounds) must agree on these numbers, so they both derive them from
// the pure functions below instead of duplicating the formulas.

// Mirror FullPreview's own width math so wrapped-line counts used for scroll
// bounds match what FullPreview actually renders.
export function fullPreviewWidth(config: ResolvedTuiConfig, contentWidth: number): number {
  return Math.max(config.layout.minPaneWidth, contentWidth - config.layout.fullPreviewWidthInset);
}

export function fullPreviewTextWidth(config: ResolvedTuiConfig, contentWidth: number): number {
  return Math.max(
    1,
    fullPreviewWidth(config, contentWidth) - (config.layout.showFullPreviewGutter ? config.layout.fullPreviewTextWidthInset : config.layout.fullPreviewPaddingX * 2),
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

export function fullPreviewVisibleRows(entry: Entry | undefined, config: ResolvedTuiConfig, rows: number): number {
  return visibleFullPreviewLineCapacity(
    rows,
    config.layout.fullPreviewLineSpacing,
    config.layout.fullPreviewScrollInsetRows,
    fullPreviewReservedRows(entry, config, rows),
  );
}
