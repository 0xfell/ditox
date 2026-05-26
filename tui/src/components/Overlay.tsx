import { For, Show } from "solid-js";
import { formatClearKind, truncateText } from "../presentation";
import { selectedIdsOrCurrent, selectedSetIncludesPinned, type UiState } from "../state";
import type { ContentAlign, VerticalAlign } from "../ui-config";
import {
  formatTemplate,
  helpRows,
  paddedTitle,
  surface,
  templateSegments,
  textStyle,
  type BorderStyle,
  type ResolvedTuiConfig,
  type TuiOverlayContentPartName,
  type TuiOverlayToneName,
  type TuiStatusLineToneName,
  type TuiSurfaceStyle,
} from "../tui-config";

export function ModeOverlay(props: { config: ResolvedTuiConfig; state: UiState; width?: number }) {
  const searchStyle = () => overlaySurface(props.config, "search");
  const dangerStyle = () => overlaySurface(props.config, "danger");
  const helpStyle = () => overlaySurface(props.config, "command");
  const labels = () => props.config.labels;
  const searchWidth = () => overlayContentWidth(props.config, "search", props.width);
  const dangerWidth = () => overlayContentWidth(props.config, "danger", props.width);
  const helpWidth = () => overlayContentWidth(props.config, "command", props.width);
  const deleteCount = () => selectedIdsOrCurrent(props.state).length;
  const confirmHint = () =>
    fitOverlayText(
      formatTemplate(labels().confirmHintTemplate, {
        indent: " ".repeat(props.config.layout.confirmHintIndent),
        hint: labels().confirmHint,
      }),
      props.config.layout.dangerOverlayHintMaxWidth,
      dangerWidth(),
      props.config,
    );
  const searchInputParts = () =>
    templateSegments(
      labels().searchInputTemplate,
      fittedSearchInputValues(
        props.config,
        {
          prompt: labels().searchPrompt,
          query: props.state.query,
          cursor: labels().searchCursor,
        },
        searchWidth(),
      ),
    );
  const deletePrompt = () =>
    fitOverlayText(
      deleteCount() > 1
        ? formatTemplate(labels().deleteManyTemplate, {
            message: labels().deleteMany,
            count: deleteCount(),
          })
        : formatTemplate(labels().deleteOneTemplate, {
            message: labels().deleteOne,
            count: deleteCount(),
          }),
      props.config.layout.dangerOverlayPromptMaxWidth,
      dangerWidth(),
      props.config,
    );
  const clearPrompt = () =>
    fitOverlayText(
      formatTemplate(labels().clearPromptTemplate, {
        prefix: labels().clearPrefix,
        kind: formatClearKind(props.state.clearKind, labels()),
      }),
      props.config.layout.dangerOverlayPromptMaxWidth,
      dangerWidth(),
      props.config,
    );
  const dangerHint = (value: string) => fitOverlayText(value, props.config.layout.dangerOverlayHintMaxWidth, dangerWidth(), props.config);
  const helpKeyColumnWidth = () => Math.min(props.config.layout.helpKeyWidth, helpWidth());
  const helpAction = (value: string) =>
    fitOverlayText(value, props.config.layout.helpOverlayActionMaxWidth, Math.max(0, helpWidth() - helpKeyColumnWidth()), props.config);
  const deletingPinned = () => selectedSetIncludesPinned(props.state);
  const helpRowsView = () => visibleHelpRows(props.config);
  return (
    <>
      <Show when={props.state.mode === "search"}>
        <OverlayFrame config={props.config} title={labels().searchTitle} height={props.config.layout.searchOverlayHeight} tone="search">
          <OverlayLine config={props.config} tone="search">
            <text style={textStyle(searchStyle(), overlayContentColor(props.config, searchStyle(), "searchInput"))}>
              <For each={searchInputParts()}>
                {(part) => (
                  <span style={textStyle(searchStyle(), overlayContentColor(props.config, searchStyle(), searchInputPartName(part.key)))}>
                    {part.text}
                  </span>
                )}
              </For>
            </text>
          </OverlayLine>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "confirm-delete"}>
        <OverlayFrame
          config={props.config}
          title={labels().deleteTitle}
          height={props.config.layout.confirmOverlayHeight + (deletingPinned() ? props.config.layout.confirmPinnedExtraRows : 0)}
          tone="danger"
        >
          <OverlayLine config={props.config} tone="danger">
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "deletePrompt"))}>{deletePrompt()}</text>
          </OverlayLine>
          <Show when={deletingPinned()}>
            <>
              <OverlayLineGap config={props.config} tone="danger" />
              <OverlayLine config={props.config} tone="danger">
                <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "deleteWarning"))}>{dangerHint(labels().deletePinnedWarning)}</text>
              </OverlayLine>
            </>
          </Show>
          <OverlayLineGap config={props.config} tone="danger" />
          <OverlayLine config={props.config} tone="danger">
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "confirmHint"))}>{confirmHint()}</text>
          </OverlayLine>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "confirm-clear"}>
        <OverlayFrame config={props.config} title={labels().clearTitle} height={props.config.layout.clearOverlayHeight} tone="danger">
          <OverlayLine config={props.config} tone="danger">
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "clearPrompt"))}>{clearPrompt()}</text>
          </OverlayLine>
          <OverlayLineGap config={props.config} tone="danger" />
          <OverlayLine config={props.config} tone="danger">
            <text
              style={textStyle(
                dangerStyle(),
                overlayContentColor(props.config, dangerStyle(), props.state.clearPreserveFavorites ? "clearSafeHint" : "clearUnsafeHint"),
              )}
            >
              {dangerHint(props.state.clearPreserveFavorites ? labels().clearPinnedSafeHint : labels().clearPinnedUnsafeHint)}
            </text>
          </OverlayLine>
          <OverlayLineGap config={props.config} tone="danger" />
          <OverlayLine config={props.config} tone="danger">
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "confirmHint"))}>{confirmHint()}</text>
          </OverlayLine>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "help"}>
        <OverlayFrame config={props.config} title={labels().helpTitle} height={props.config.layout.helpOverlayHeight} tone="command">
          <For each={helpRowsView()}>
            {(row, index) => (
              <>
                <OverlayLine config={props.config} tone="command">
                  <text style={textStyle(helpStyle())}>
                    <span style={textStyle(helpStyle(), overlayContentColor(props.config, helpStyle(), "helpKey"))}>{fitHelpKey(row.keys, props.config, helpKeyColumnWidth())}</span>
                    <span style={textStyle(helpStyle(), overlayContentColor(props.config, helpStyle(), "helpAction"))}>{helpAction(row.action)}</span>
                  </text>
                </OverlayLine>
                <Show when={index() < helpRowsView().length - 1}>
                  <OverlayLineGap config={props.config} tone="command" />
                </Show>
              </>
            )}
          </For>
        </OverlayFrame>
      </Show>
    </>
  );
}

type SearchInputValues = Record<"prompt" | "query" | "cursor", string>;

function fittedSearchInputValues(config: ResolvedTuiConfig, values: SearchInputValues, width: number): SearchInputValues {
  const fitted = {
    prompt: configuredMax(values.prompt, config.layout.searchOverlayPromptMaxWidth, config),
    query: configuredMax(values.query, config.layout.searchOverlayQueryMaxWidth, config),
    cursor: configuredMax(values.cursor, config.layout.searchOverlayCursorMaxWidth, config),
  };
  const shrinkOrder: Array<keyof SearchInputValues> = ["query", "prompt", "cursor"];
  for (const key of shrinkOrder) {
    const overflow = searchInputWidth(config, fitted) - width;
    if (overflow <= 0) break;
    if (fitted[key].length === 0) continue;
    fitted[key] = truncateText(fitted[key], Math.max(0, fitted[key].length - overflow), config.labels);
  }
  return fitted;
}

function searchInputWidth(config: ResolvedTuiConfig, values: SearchInputValues): number {
  return templateSegments(config.labels.searchInputTemplate, values).reduce((width, part) => width + part.text.length, 0);
}

function fitOverlayText(value: string, maxWidth: number, width: number, config: ResolvedTuiConfig): string {
  return truncateText(configuredMax(value, maxWidth, config), width, config.labels);
}

function configuredMax(value: string, maxWidth: number, config: ResolvedTuiConfig): string {
  return truncateText(value, maxWidth > 0 ? maxWidth : value.length, config.labels);
}

function OverlayLine(props: { config: ResolvedTuiConfig; tone: TuiOverlayToneName; children: any }) {
  return (
    <box height={1} width="100%" flexDirection="row" justifyContent={justifyContent(overlayContentAlign(props.config, props.tone))}>
      {props.children}
    </box>
  );
}

function visibleHelpRows(config: ResolvedTuiConfig): Array<{ keys: string; action: string }> {
  const rows = helpRows(config);
  const capacity = overlayContentRows(config, "command", config.layout.helpOverlayHeight);
  const spacing = overlayLineSpacing(config, "command");
  const visible: Array<{ keys: string; action: string }> = [];
  let usedRows = 0;
  for (const row of rows) {
    const nextRows = usedRows + (visible.length > 0 ? spacing : 0) + 1;
    if (nextRows > capacity) break;
    visible.push(row);
    usedRows = nextRows;
  }
  return visible;
}

function overlayContentRows(config: ResolvedTuiConfig, tone: TuiOverlayToneName, height: number): number {
  const borderRows = overlayBorderVisible(config, tone) ? 2 : 0;
  return Math.max(0, height - borderRows - overlayPaddingY(config, tone) * 2);
}

function OverlayLineGap(props: { config: ResolvedTuiConfig; tone: TuiOverlayToneName }) {
  const height = () => overlayLineSpacing(props.config, props.tone);
  const style = () => overlaySurface(props.config, props.tone);
  return (
    <Show when={height() > 0}>
      <box width="100%" height={height()} backgroundColor={style().bg} />
    </Show>
  );
}

function OverlayFrame(props: { config: ResolvedTuiConfig; title: string; height: number; tone: TuiOverlayToneName; children: any }) {
  const style = () => overlaySurface(props.config, props.tone);
  return (
    <box
      height={props.height}
      flexDirection="column"
      border={overlayBorderVisible(props.config, props.tone) ? true : undefined}
      borderStyle={overlayBorderVisible(props.config, props.tone) ? overlayBorderStyle(props.config, props.tone) : undefined}
      borderColor={overlayBorderVisible(props.config, props.tone) ? overlayBorder(props.config, props.tone) : undefined}
      backgroundColor={style().bg}
      paddingX={overlayPaddingX(props.config, props.tone)}
      paddingY={overlayPaddingY(props.config, props.tone)}
      title={
        overlayBorderVisible(props.config, props.tone) && overlayTitleVisible(props.config, props.tone)
          ? paddedTitle(props.title, props.config.layout.frameTitlePaddingLeft, props.config.layout.frameTitlePaddingRight)
          : undefined
      }
      titleAlignment={overlayTitleAlignment(props.config, props.tone)}
    >
      <box flexDirection="column" flexGrow={1} justifyContent={verticalJustify(overlayVerticalAlign(props.config, props.tone))}>
        {props.children}
      </box>
    </box>
  );
}

function justifyContent(align: ContentAlign): "flex-start" | "center" | "flex-end" {
  if (align === "right") return "flex-end";
  if (align === "center") return "center";
  return "flex-start";
}

function verticalJustify(align: VerticalAlign): "flex-start" | "center" | "flex-end" {
  if (align === "bottom") return "flex-end";
  if (align === "center") return "center";
  return "flex-start";
}

function overlayBorder(config: ResolvedTuiConfig, tone: TuiOverlayToneName): string {
  const style = overlaySurface(config, tone);
  return configuredToneColor(style, config.overlayBorderTones[tone], autoOverlayBorderColor(style, tone));
}

function overlayBorderVisible(config: ResolvedTuiConfig, tone: TuiOverlayToneName): boolean {
  if (tone === "search") return config.chrome.searchOverlayBorder;
  if (tone === "danger") return config.chrome.dangerOverlayBorder;
  return config.chrome.helpOverlayBorder;
}

function overlayTitleVisible(config: ResolvedTuiConfig, tone: TuiOverlayToneName): boolean {
  if (tone === "search") return config.chrome.showSearchOverlayTitle;
  if (tone === "danger") return config.chrome.showDangerOverlayTitle;
  return config.chrome.showHelpOverlayTitle;
}

function overlayBorderStyle(config: ResolvedTuiConfig, tone: TuiOverlayToneName): BorderStyle {
  if (tone === "search") return config.chrome.searchOverlayBorderStyle;
  if (tone === "danger") return config.chrome.dangerOverlayBorderStyle;
  return config.chrome.helpOverlayBorderStyle;
}

function overlayTitleAlignment(config: ResolvedTuiConfig, tone: TuiOverlayToneName) {
  if (tone === "search") return config.chrome.searchOverlayTitleAlignment;
  if (tone === "danger") return config.chrome.dangerOverlayTitleAlignment;
  return config.chrome.helpOverlayTitleAlignment;
}

function overlayContentAlign(config: ResolvedTuiConfig, tone: TuiOverlayToneName) {
  if (tone === "search") return config.layout.searchOverlayContentAlign;
  if (tone === "danger") return config.layout.dangerOverlayContentAlign;
  return config.layout.helpOverlayContentAlign;
}

function overlayVerticalAlign(config: ResolvedTuiConfig, tone: TuiOverlayToneName) {
  if (tone === "search") return config.layout.searchOverlayVerticalAlign;
  if (tone === "danger") return config.layout.dangerOverlayVerticalAlign;
  return config.layout.helpOverlayVerticalAlign;
}

function overlayLineSpacing(config: ResolvedTuiConfig, tone: TuiOverlayToneName): number {
  if (tone === "search") return config.layout.searchOverlayLineSpacing;
  if (tone === "danger") return config.layout.dangerOverlayLineSpacing;
  return config.layout.helpOverlayLineSpacing;
}

function fitHelpKey(value: string, config: ResolvedTuiConfig, maxWidth = config.layout.helpKeyWidth): string {
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

function overlayPaddingX(config: ResolvedTuiConfig, tone: TuiOverlayToneName): number {
  if (tone === "search") return config.layout.searchOverlayPaddingX;
  if (tone === "danger") return config.layout.dangerOverlayPaddingX;
  return config.layout.helpOverlayPaddingX;
}

function overlayContentWidth(config: ResolvedTuiConfig, tone: TuiOverlayToneName, width: number | undefined): number {
  if (width === undefined) return Number.MAX_SAFE_INTEGER;
  const borderWidth = overlayBorderVisible(config, tone) ? 2 : 0;
  return Math.max(0, width - overlayPaddingX(config, tone) * 2 - borderWidth);
}

function overlayPaddingY(config: ResolvedTuiConfig, tone: TuiOverlayToneName): number {
  if (tone === "search") return config.layout.searchOverlayPaddingY;
  if (tone === "danger") return config.layout.dangerOverlayPaddingY;
  return config.layout.helpOverlayPaddingY;
}

function overlaySurface(config: ResolvedTuiConfig, tone: TuiOverlayToneName): TuiSurfaceStyle {
  if (tone === "search") return surface(config, "searchOverlay");
  if (tone === "danger") return surface(config, "dangerOverlay");
  return surface(config, "helpOverlay");
}

function autoOverlayBorderColor(style: TuiSurfaceStyle, tone: TuiOverlayToneName): string {
  if (tone === "search") return style.search;
  if (tone === "danger") return style.error;
  return style.border;
}

function overlayContentColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, part: TuiOverlayContentPartName): string {
  return configuredToneColor(style, config.overlayContentTones[part], autoOverlayContentColor(style, part));
}

function searchInputPartName(key: string | null): TuiOverlayContentPartName {
  if (key === "prompt") return "searchPrompt";
  if (key === "query") return "searchQuery";
  if (key === "cursor") return "searchCursor";
  return "searchInput";
}

function autoOverlayContentColor(style: TuiSurfaceStyle, part: TuiOverlayContentPartName): string {
  switch (part) {
    case "searchInput":
    case "searchPrompt":
    case "searchQuery":
      return style.search;
    case "searchCursor":
      return style.accent;
    case "deletePrompt":
    case "clearPrompt":
      return style.error;
    case "deleteWarning":
    case "clearUnsafeHint":
      return style.warning;
    case "clearSafeHint":
      return style.success;
    case "helpKey":
      return style.accent;
    case "helpAction":
      return style.fg;
    default:
      return style.muted;
  }
}

function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}
