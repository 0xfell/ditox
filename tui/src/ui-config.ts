export type UiConfig = {
  compactMode: boolean;
  listWidthPercent: number;
  previewWidthPercent: number;
  maxPreviewLines: number;
  showScrollbar: boolean;
  showMetadata: boolean;
  panelPaddingX: number;
  panelPaddingY: number;
};

export function currentUiConfig(): UiConfig {
  const compactMode = boolEnv("DITOX_TUI_COMPACT", false);
  const listWidthPercent = clampNumber(numberEnv("DITOX_TUI_LIST_WIDTH", 46), 32, 68);
  return {
    compactMode,
    listWidthPercent,
    previewWidthPercent: 100 - listWidthPercent,
    maxPreviewLines: clampNumber(numberEnv("DITOX_TUI_MAX_PREVIEW_LINES", compactMode ? 18 : 28), 8, 80),
    showScrollbar: boolEnv("DITOX_TUI_SCROLLBAR", true),
    showMetadata: boolEnv("DITOX_TUI_METADATA", true),
    panelPaddingX: compactMode ? 0 : 1,
    panelPaddingY: 0,
  };
}

function boolEnv(name: string, fallback: boolean): boolean {
  const value = Bun.env[name];
  if (value === "1" || value === "true" || value === "yes") return true;
  if (value === "0" || value === "false" || value === "no") return false;
  return fallback;
}

function numberEnv(name: string, fallback: number): number {
  const parsed = Number(Bun.env[name]);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function clampNumber(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
