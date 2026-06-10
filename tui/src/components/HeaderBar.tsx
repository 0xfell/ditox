import { For } from "solid-js";
import type { UiState } from "../state";
import { formatFilter, truncateText } from "../presentation";
import {
  formatTemplate,
  headerLinePartNames,
  paddedTitle,
  surface,
  templateSegments,
  textStyle,
  type ResolvedTuiConfig,
  type TuiHeaderLinePartName,
  type TuiSurfaceStyle,
} from "../tui-config";
import { configuredMax, configuredToneColor, justifyContent, templateWidth, verticalJustify } from "./style-utils";

export function HeaderBar(props: { config: ResolvedTuiConfig; state: UiState; selectedCount: number; width?: number }) {
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
  const lineValues = () =>
    fittedHeaderLineValues(
      props.config,
      {
        brand: labels().brand,
        sectionSeparator: labels().headerSectionSeparator,
        labelSeparator: labels().headerLabelSeparator,
        filterLabel: labels().filterLabel,
        filter: filter(),
        queryLabel: labels().queryLabel,
        query: query(),
        modeLabel: labels().modeLabel,
        mode: selected(),
      },
      props.width === undefined ? Number.MAX_SAFE_INTEGER : Math.max(0, props.width - layout().headerPaddingX * 2),
    );
  const lineParts = () => templateSegments(labels().headerLineTemplate, lineValues());
  return (
    <box
      height={layout().headerHeight}
      border={props.config.chrome.headerBorder ? true : undefined}
      borderStyle={props.config.chrome.headerBorder ? props.config.chrome.headerBorderStyle : undefined}
      borderColor={props.config.chrome.headerBorder ? style().border : undefined}
      backgroundColor={style().bg}
      paddingX={layout().headerPaddingX}
      paddingY={layout().headerPaddingY}
      title={
        props.config.chrome.headerBorder && props.config.chrome.showHeaderTitle
          ? paddedTitle(labels().appTitle, layout().frameTitlePaddingLeft, layout().frameTitlePaddingRight)
          : undefined
      }
      titleAlignment={props.config.chrome.headerTitleAlignment}
    >
      <box width="100%" flexGrow={1} flexDirection="column" justifyContent={verticalJustify(layout().headerVerticalAlign)}>
        <box width="100%" flexDirection="row" justifyContent={justifyContent(layout().headerContentAlign)}>
          <text style={textStyle(style())}>
            <For each={lineParts()}>
              {(part) => <span style={textStyle(style(), headerPartColor(props.config, part.key, props.state.pinnedOnly))}>{part.text}</span>}
            </For>
          </text>
        </box>
      </box>
    </box>
  );
}

type HeaderLineValues = Record<"brand" | "sectionSeparator" | "labelSeparator" | "filterLabel" | "filter" | "queryLabel" | "query" | "modeLabel" | "mode", string>;

function fittedHeaderLineValues(config: ResolvedTuiConfig, values: HeaderLineValues, width: number): HeaderLineValues {
  const fitted = {
    ...values,
    brand: configuredMax(values.brand, config.layout.headerBrandMaxWidth, config),
    filter: configuredMax(values.filter, config.layout.headerFilterMaxWidth, config),
    query: configuredMax(values.query, config.layout.headerQueryMaxWidth, config),
    mode: configuredMax(values.mode, config.layout.headerModeMaxWidth, config),
  };
  const shrinkOrder: Array<keyof HeaderLineValues> = ["query", "mode", "filter", "brand", "queryLabel", "modeLabel", "filterLabel", "sectionSeparator", "labelSeparator"];
  for (const key of shrinkOrder) {
    const overflow = templateWidth(config.labels.headerLineTemplate, fitted) - width;
    if (overflow <= 0) break;
    if (fitted[key].length === 0) continue;
    fitted[key] = truncateText(fitted[key], Math.max(0, fitted[key].length - overflow), config.labels);
  }
  return fitted;
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

function isHeaderLinePartName(value: string | null): value is TuiHeaderLinePartName {
  return typeof value === "string" && (headerLinePartNames as readonly string[]).includes(value);
}
