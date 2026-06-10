import { truncateText } from "../presentation";
import type { ContentAlign, VerticalAlign } from "../ui-config";
import { templateSegments, type ResolvedTuiConfig, type TuiStatusLineToneName, type TuiSurfaceStyle } from "../tui-config";

export function justifyContent(align: ContentAlign): "flex-start" | "center" | "flex-end" {
  if (align === "right") return "flex-end";
  if (align === "center") return "center";
  return "flex-start";
}

export function verticalJustify(align: VerticalAlign): "flex-start" | "center" | "flex-end" {
  if (align === "bottom") return "flex-end";
  if (align === "center") return "center";
  return "flex-start";
}

export function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}

export function toneColor(style: TuiSurfaceStyle, tone: "primary" | "secondary" | "muted" | "accent" | "error" | "success"): string {
  switch (tone) {
    case "secondary":
      return style.secondary;
    case "muted":
      return style.muted;
    case "accent":
      return style.accent;
    case "error":
      return style.error;
    case "success":
      return style.success;
    default:
      return style.fg;
  }
}

export function configuredMax(value: string, maxWidth: number, config: ResolvedTuiConfig): string {
  return truncateText(value, maxWidth > 0 ? maxWidth : value.length, config.labels);
}

export function templateWidth(template: string, values: Record<string, string>): number {
  return templateSegments(template, values).reduce((width, part) => width + part.text.length, 0);
}

// Pad/clip a fixed-width cell (e.g. row markers, scrollbar glyphs) to exactly
// `width` characters with the requested alignment.
export function fitCell(value: string, width: number, align: ContentAlign): string {
  const chars = Array.from(value);
  if (chars.length >= width) return chars.slice(0, width).join("");
  const padding = width - chars.length;
  if (align === "right") return `${" ".repeat(padding)}${value}`;
  if (align === "center") {
    const left = Math.floor(padding / 2);
    return `${" ".repeat(left)}${value}${" ".repeat(padding - left)}`;
  }
  return `${value}${" ".repeat(padding)}`;
}

// Fit a preview gutter label into its column: trims surrounding whitespace
// (falling back to the raw value when it is all-whitespace) and enforces a
// minimum width of one cell. Distinct semantics from fitCell.
export function fitGutter(value: string, width: number, align: ContentAlign): string {
  const target = Math.max(1, Math.floor(width));
  const trimmed = value.trim();
  const raw = trimmed.length > 0 ? trimmed : value;
  const clipped = Array.from(raw).slice(0, target).join("");
  const extra = Math.max(0, target - Array.from(clipped).length);
  if (align === "right") return `${" ".repeat(extra)}${clipped}`;
  if (align === "center") {
    const left = Math.floor(extra / 2);
    return `${" ".repeat(left)}${clipped}${" ".repeat(extra - left)}`;
  }
  return `${clipped}${" ".repeat(extra)}`;
}

// Fit a help-overlay key column entry: clips with the configured truncation
// marker and aligns per config.layout.helpKeyAlign. Distinct semantics from
// fitCell/fitGutter (marker-aware truncation, config-driven alignment).
export function fitHelpKey(value: string, config: ResolvedTuiConfig, maxWidth = config.layout.helpKeyWidth): string {
  const width = Math.max(0, Math.floor(maxWidth));
  const chars = Array.from(value);
  const clipped = chars.length > width ? truncateText(value, width, config.labels) : value;
  const padding = Math.max(0, width - Array.from(clipped).length);
  if (config.layout.helpKeyAlign === "right") return `${" ".repeat(padding)}${clipped}`;
  if (config.layout.helpKeyAlign === "center") {
    const left = Math.floor(padding / 2);
    return `${" ".repeat(left)}${clipped}${" ".repeat(padding - left)}`;
  }
  return `${clipped}${" ".repeat(padding)}`;
}
