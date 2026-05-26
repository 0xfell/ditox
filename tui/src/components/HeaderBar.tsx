import { For } from "solid-js";
import type { UiState } from "../state";
import { formatFilter } from "../presentation";
import {
  formatTemplate,
  headerLinePartNames,
  paddedTitle,
  surface,
  templateSegments,
  textStyle,
  type ResolvedTuiConfig,
  type TuiHeaderLinePartName,
  type TuiStatusLineToneName,
  type TuiSurfaceStyle,
} from "../tui-config";

export function HeaderBar(props: { config: ResolvedTuiConfig; state: UiState; selectedCount: number }) {
  const style = () => surface(props.config, "header");
  const labels = () => props.config.labels;
  const layout = () => props.config.layout;
  const query = () => props.state.query || labels().queryEmpty;
  const filter = () => (props.state.pinnedOnly ? labels().pinnedViewTitle : formatFilter(props.state.filter, labels()));
  const selected = () => {
    if (props.state.mode === "preview") return labels().previewModeTitle;
    return props.selectedCount > 0
      ? formatTemplate(labels().selectedCountTemplate, { prefix: labels().selectedPrefix, count: props.selectedCount })
      : labels().singleSelection;
  };
  const lineParts = () =>
    templateSegments(labels().headerLineTemplate, {
      brand: labels().brand,
      sectionSeparator: labels().headerSectionSeparator,
      labelSeparator: labels().headerLabelSeparator,
      filterLabel: labels().filterLabel,
      filter: filter(),
      queryLabel: labels().queryLabel,
      query: query(),
      modeLabel: labels().modeLabel,
      mode: selected(),
    });
  return (
    <box
      height={layout().headerHeight}
      border={props.config.chrome.headerBorder ? true : undefined}
      borderStyle={props.config.chrome.headerBorder ? props.config.chrome.headerBorderStyle : undefined}
      borderColor={props.config.chrome.headerBorder ? style().border : undefined}
      backgroundColor={style().bg}
      paddingX={layout().headerPaddingX}
      paddingY={layout().headerPaddingY}
      title={props.config.chrome.headerBorder && props.config.chrome.showHeaderTitle ? paddedTitle(labels().appTitle, layout().frameTitlePadding) : undefined}
      titleAlignment={props.config.chrome.headerTitleAlignment}
    >
      <text style={textStyle(style())}>
        <For each={lineParts()}>
          {(part) => <span style={textStyle(style(), headerPartColor(props.config, part.key, props.state.pinnedOnly))}>{part.text}</span>}
        </For>
      </text>
    </box>
  );
}

function headerPartColor(config: ResolvedTuiConfig, key: string | null, pinnedOnly: boolean): string {
  const style = surface(config, "header");
  if (!isHeaderLinePartName(key)) return style.muted;
  return configuredToneColor(style, config.headerLineTones[key], autoHeaderPartColor(style, key, pinnedOnly));
}

function autoHeaderPartColor(style: TuiSurfaceStyle, key: TuiHeaderLinePartName, pinnedOnly: boolean): string {
  switch (key) {
    case "brand":
      return style.accent;
    case "filter":
      return pinnedOnly ? style.favorite : style.fg;
    case "query":
      return style.search;
    case "mode":
      return style.secondary;
    case "filterLabel":
    case "queryLabel":
    case "modeLabel":
    case "sectionSeparator":
    case "labelSeparator":
    default:
      return style.muted;
  }
}

function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}

function isHeaderLinePartName(value: string | null): value is TuiHeaderLinePartName {
  return typeof value === "string" && (headerLinePartNames as readonly string[]).includes(value);
}
