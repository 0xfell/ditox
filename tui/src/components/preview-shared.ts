import { type ImageBlockPreview } from "../image-preview";
import { entryAccent } from "../presentation";
import { formatTemplate, type ResolvedTuiConfig, type TuiPreviewContentPartName, type TuiSurfaceStyle } from "../tui-config";
import type { Entry } from "../types";
import type { ImagePreviewNoticeVisibility } from "../ui-config";
import { configuredToneColor, toneColor } from "./style-utils";

// Shared logic between the split PreviewPane ("split*" part names /
// imagePreview* config fields) and the FullPreview ("full*" part names /
// fullPreviewImage* config fields). The components pass their own surface and
// tone names; nothing here invents new ones.

export type PreviewLineTone = "primary" | "secondary" | "muted" | "accent" | "error" | "success";

export type PreviewVariant = "split" | "full";

export function imagePreviewBackground(configured: string, fallback: string): string {
  return configured === "auto" ? fallback : configured;
}

export function imagePreviewNotice(preview: ImageBlockPreview, visibility: ImagePreviewNoticeVisibility, sourceTemplate: string): string | null {
  if (preview.kind !== "rendered") return null;
  if (visibility === "never") return null;
  if (preview.notice) return preview.notice;
  if (visibility === "always") {
    return formatTemplate(sourceTemplate, { source: preview.source });
  }
  return null;
}

export function previewContentColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, part: TuiPreviewContentPartName, autoColor: string): string {
  return configuredToneColor(style, config.previewContentTones[part], autoColor);
}

export function previewBorderColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, entry: Entry | undefined, part: TuiPreviewContentPartName): string {
  return previewContentColor(config, style, part, entry ? entryAccent(style, entry) : style.border);
}

const previewLineToneParts = {
  split: {
    primary: "splitPrimary",
    secondary: "splitSecondary",
    muted: "splitMuted",
    accent: "splitAccent",
    error: "splitError",
    success: "splitSuccess",
  },
  full: {
    primary: "fullPrimary",
    secondary: "fullSecondary",
    muted: "fullMuted",
    accent: "fullAccent",
    error: "fullError",
    success: "fullSuccess",
  },
} as const satisfies Record<PreviewVariant, Record<PreviewLineTone, TuiPreviewContentPartName>>;

const previewEmptyLineParts = {
  split: { title: "splitEmptyTitle", help: "splitEmptyHelp" },
  full: { title: "fullEmptyTitle", help: "fullEmptyHelp" },
} as const satisfies Record<PreviewVariant, { title: TuiPreviewContentPartName; help: TuiPreviewContentPartName }>;

export function previewLineTonePart(variant: PreviewVariant, tone: PreviewLineTone): TuiPreviewContentPartName {
  return previewLineToneParts[variant][tone];
}

export function previewLineToneColor(
  variant: PreviewVariant,
  config: ResolvedTuiConfig,
  style: TuiSurfaceStyle,
  entry: Entry | undefined,
  tone: PreviewLineTone,
  index: number,
): string {
  const empty = previewEmptyLineParts[variant];
  if (!entry) return previewContentColor(config, style, index === 0 ? empty.title : empty.help, toneColor(style, tone));
  return previewContentColor(config, style, previewLineTonePart(variant, tone), toneColor(style, tone));
}
