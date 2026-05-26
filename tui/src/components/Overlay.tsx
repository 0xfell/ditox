import { For, Show } from "solid-js";
import { formatClearKind } from "../presentation";
import { selectedIdsOrCurrent, selectedSetIncludesPinned, type UiState } from "../state";
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

export function ModeOverlay(props: { config: ResolvedTuiConfig; state: UiState }) {
  const searchStyle = () => overlaySurface(props.config, "search");
  const dangerStyle = () => overlaySurface(props.config, "danger");
  const helpStyle = () => overlaySurface(props.config, "command");
  const labels = () => props.config.labels;
  const deleteCount = () => selectedIdsOrCurrent(props.state).length;
  const confirmHint = () =>
    formatTemplate(labels().confirmHintTemplate, {
      indent: " ".repeat(props.config.layout.confirmHintIndent),
      hint: labels().confirmHint,
    });
  const searchInputParts = () =>
    templateSegments(labels().searchInputTemplate, {
      prompt: labels().searchPrompt,
      query: props.state.query,
      cursor: labels().searchCursor,
    });
  const deletePrompt = () =>
    deleteCount() > 1
      ? formatTemplate(labels().deleteManyTemplate, {
          message: labels().deleteMany,
          count: deleteCount(),
        })
      : formatTemplate(labels().deleteOneTemplate, {
          message: labels().deleteOne,
          count: deleteCount(),
        });
  const clearPrompt = () =>
    formatTemplate(labels().clearPromptTemplate, {
      prefix: labels().clearPrefix,
      kind: formatClearKind(props.state.clearKind, labels()),
    });
  const deletingPinned = () => selectedSetIncludesPinned(props.state);
  return (
    <>
      <Show when={props.state.mode === "search"}>
        <OverlayFrame config={props.config} title={labels().searchTitle} height={props.config.layout.searchOverlayHeight} tone="search">
          <OverlayLine>
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
          <OverlayLine>
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "deletePrompt"))}>{deletePrompt()}</text>
          </OverlayLine>
          <Show when={deletingPinned()}>
            <OverlayLine>
              <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "deleteWarning"))}>{labels().deletePinnedWarning}</text>
            </OverlayLine>
          </Show>
          <OverlayLine>
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "confirmHint"))}>{confirmHint()}</text>
          </OverlayLine>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "confirm-clear"}>
        <OverlayFrame config={props.config} title={labels().clearTitle} height={props.config.layout.clearOverlayHeight} tone="danger">
          <OverlayLine>
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "clearPrompt"))}>{clearPrompt()}</text>
          </OverlayLine>
          <OverlayLine>
            <text
              style={textStyle(
                dangerStyle(),
                overlayContentColor(props.config, dangerStyle(), props.state.clearPreserveFavorites ? "clearSafeHint" : "clearUnsafeHint"),
              )}
            >
              {props.state.clearPreserveFavorites ? labels().clearPinnedSafeHint : labels().clearPinnedUnsafeHint}
            </text>
          </OverlayLine>
          <OverlayLine>
            <text style={textStyle(dangerStyle(), overlayContentColor(props.config, dangerStyle(), "confirmHint"))}>{confirmHint()}</text>
          </OverlayLine>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "help"}>
        <OverlayFrame config={props.config} title={labels().helpTitle} height={props.config.layout.helpOverlayHeight} tone="command">
          <For each={helpRows(props.config)}>
            {(row) => (
              <OverlayLine>
                <text style={textStyle(helpStyle())}>
                  <span style={textStyle(helpStyle(), overlayContentColor(props.config, helpStyle(), "helpKey"))}>{row.keys.padEnd(props.config.layout.helpKeyWidth, " ")}</span>
                  <span style={textStyle(helpStyle(), overlayContentColor(props.config, helpStyle(), "helpAction"))}>{row.action}</span>
                </text>
              </OverlayLine>
            )}
          </For>
        </OverlayFrame>
      </Show>
    </>
  );
}

function OverlayLine(props: { children: any }) {
  return (
    <box height={1} flexDirection="row">
      {props.children}
    </box>
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
      title={overlayBorderVisible(props.config, props.tone) && overlayTitleVisible(props.config, props.tone) ? paddedTitle(props.title, props.config.layout.frameTitlePadding) : undefined}
      titleAlignment={overlayTitleAlignment(props.config, props.tone)}
    >
      <box flexDirection="column" flexGrow={1}>
        {props.children}
      </box>
    </box>
  );
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

function overlayPaddingX(config: ResolvedTuiConfig, tone: TuiOverlayToneName): number {
  if (tone === "search") return config.layout.searchOverlayPaddingX;
  if (tone === "danger") return config.layout.dangerOverlayPaddingX;
  return config.layout.helpOverlayPaddingX;
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
