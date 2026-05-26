import { For } from "solid-js";
import { statusTone, truncateText, watcherStatusView } from "../presentation";
import {
  statusHint,
  surface,
  templateSegments,
  textStyle,
  type ResolvedTuiConfig,
  type TuiStatusHintMode,
  type TuiStatusLineToneName,
  type TuiSurfaceStyle,
} from "../tui-config";
import type { WatcherStatus } from "../types";

export function StatusLine(props: { config: ResolvedTuiConfig; status: string; watcher: WatcherStatus | null; width: number; mode?: TuiStatusHintMode }) {
  const style = () => surface(props.config, "status");
  const watcher = () => watcherStatusView(props.watcher, props.config.labels);
  const operationTone = () => statusTone(props.status, props.config.labels, props.config.statusTones);
  const watcherTone = () => watcher().tone;
  const operationText = () => props.status || props.config.labels.ready;
  const separatorText = () => `${" ".repeat(props.config.layout.statusSeparatorPadding)}${props.config.chrome.statusSeparator}${" ".repeat(props.config.layout.statusSeparatorPadding)}`;
  const fixedTemplateWidth = () =>
    templateSegments(props.config.labels.statusLineTemplate, {
      hint: "",
      separator: separatorText(),
      watcher: watcher().text,
      operation: operationText(),
    }).reduce((width, part) => width + part.text.length, 0);
  const hint = () => {
    return truncateText(
      statusHint(props.config, props.mode ?? "browse"),
      Math.max(0, props.width - fixedTemplateWidth() - props.config.layout.statusPaddingX * 2),
      props.config.labels,
    );
  };
  const lineParts = () =>
    templateSegments(props.config.labels.statusLineTemplate, {
      hint: hint(),
      separator: separatorText(),
      watcher: watcher().text,
      operation: operationText(),
    });
  return (
    <box
      height={props.config.layout.statusHeight}
      backgroundColor={style().bg}
      paddingX={props.config.layout.statusPaddingX}
      paddingY={props.config.layout.statusPaddingY}
    >
      <text style={textStyle(style(), style().muted)}>
        <For each={lineParts()}>
          {(part) => <span style={textStyle(style(), statusPartColor(props.config, part.key, watcherTone(), operationTone()))}>{part.text}</span>}
        </For>
      </text>
    </box>
  );
}

type StatusTone = "muted" | "error" | "success" | "warning";

function statusPartColor(config: ResolvedTuiConfig, key: string | null, watcher: StatusTone, operation: StatusTone): string {
  const style = surface(config, "status");
  if (key === "watcher") return configuredToneColor(style, config.statusLineTones.watcher, toneColorFromStyle(style, watcher));
  if (key === "operation") return configuredToneColor(style, config.statusLineTones.operation, toneColorFromStyle(style, operation));
  if (key === "hint") return configuredToneColor(style, config.statusLineTones.hint, style.muted);
  if (key === "separator") return configuredToneColor(style, config.statusLineTones.separator, style.muted);
  return style.muted;
}

function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}

function toneColorFromStyle(style: TuiSurfaceStyle, tone: StatusTone): string {
  switch (tone) {
    case "error":
      return style.error;
    case "success":
      return style.success;
    case "warning":
      return style.warning;
    default:
      return style.fg;
  }
}
