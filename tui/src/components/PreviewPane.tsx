import { For, Show } from "solid-js";
import { entryAccent, formatBytes, previewModel, truncateText } from "../presentation";
import type { UiConfig } from "../ui-config";
import type { TuiTheme } from "../theme";
import type { Entry } from "../types";

export function PreviewPane(props: { theme: TuiTheme; config: UiConfig; entry: Entry | undefined; rows: number; width: number }) {
  const lines = () => previewModel(props.entry, Math.min(props.config.maxPreviewLines, props.rows));
  return (
    <box
      width={`${props.config.previewWidthPercent}%`}
      border
      borderStyle="single"
      borderColor={props.entry ? entryAccent(props.theme, props.entry) : props.theme.border}
      backgroundColor={props.theme.bgPanel}
      paddingX={props.config.panelPaddingX}
      paddingY={props.config.panelPaddingY}
      title=" preview "
      bottomTitle={props.entry ? ` #${props.entry.id} ` : undefined}
    >
      <Show when={props.config.showMetadata ? props.entry : undefined}>
        {(entry) => <PreviewMeta theme={props.theme} entry={entry()} />}
      </Show>
      <For each={lines()}>
        {(line) => (
          <box height={1} flexDirection="row" backgroundColor={props.theme.bgPanel}>
            <text style={{ fg: props.theme.textDim, bg: props.theme.bgPanel }}>{line.gutter.padStart(4, " ")}</text>
            <text style={{ fg: props.theme.textDim, bg: props.theme.bgPanel }}>  </text>
            <text style={{ fg: toneColor(props.theme, line.tone), bg: props.theme.bgPanel }}>
              {truncateText(line.text, Math.max(8, props.width - 8))}
            </text>
          </box>
        )}
      </For>
    </box>
  );
}

function PreviewMeta(props: { theme: TuiTheme; entry: Entry }) {
  return (
    <box height={3} flexDirection="column" backgroundColor={props.theme.bgSubtle} paddingX={1}>
      <text style={{ fg: entryAccent(props.theme, props.entry), bg: props.theme.bgSubtle }}>
        {props.entry.kind.toUpperCase()} #{props.entry.id}
        <span style={{ fg: props.theme.textDim, bg: props.theme.bgSubtle }}>  hash </span>
        <span style={{ fg: props.theme.textSecondary, bg: props.theme.bgSubtle }}>{props.entry.hash.slice(0, 12)}</span>
      </text>
      <text style={{ fg: props.theme.textMuted, bg: props.theme.bgSubtle }}>
        {props.entry.mime}  {formatBytes(props.entry.byte_len)}
        {props.entry.favorite ? "  pinned" : ""}
      </text>
    </box>
  );
}

function toneColor(theme: TuiTheme, tone: "primary" | "secondary" | "muted" | "accent" | "error" | "success"): string {
  switch (tone) {
    case "secondary":
      return theme.textSecondary;
    case "muted":
      return theme.textMuted;
    case "accent":
      return theme.accentCommand;
    case "error":
      return theme.accentError;
    case "success":
      return theme.accentSuccess;
    default:
      return theme.textPrimary;
  }
}
