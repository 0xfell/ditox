import { createTextAttributes } from "@opentui/core";
import type { TuiTheme } from "../theme";
import {
  tuiSurfaceNames,
  type ResolvedTuiConfig,
  type TuiConfigFile,
  type TuiSurfaceName,
  type TuiSurfaceStyle,
  type TuiTextStyle,
} from "./types";

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

export function mergeStyles(theme: TuiTheme, styles: TuiConfigFile["styles"]): Record<TuiSurfaceName, TuiSurfaceStyle> {
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

export function isColor(value: string): boolean {
  return isHexColor(value) || /^[a-zA-Z]+$/.test(value);
}

export function isHexColor(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}
