import { For, Show } from "solid-js";
import { entryAccent, entryMeta, entryPreviewSegments, fitRowMeta, scrollbarCells } from "../presentation";
import { visibleEntries, visibleEntryCapacity } from "../state";
import {
  formatTemplate,
  paddedTitle,
  surface,
  textStyle,
  type ResolvedTuiConfig,
  type TuiListContentPartName,
  type TuiStatusLineToneName,
  type TuiSurfaceStyle,
} from "../tui-config";
import type { Entry } from "../types";

export function EntryList(props: {
  config: ResolvedTuiConfig;
  entries: Entry[];
  selectedIndex: number;
  selectedIds: Set<number>;
  rows: number;
  width: number;
  widthPercent?: number;
  query: string;
  onSelectEntry: (index: number, options: { extend: boolean; toggle: boolean }) => void;
  onScroll: (direction: -1 | 1) => void;
}) {
  const style = () => surface(props.config, "list");
  const rowSpacerStyle = () => surface(props.config, "rowSpacer");
  const layout = () => props.config.layout;
  const chrome = () => props.config.chrome;
  const visibleCount = () => visibleEntryCapacity(props.rows, layout().rowSpacing);
  const visible = () => visibleEntries(props.entries, props.selectedIndex, visibleCount());
  return (
    <box
      width={`${props.widthPercent ?? layout().listWidthPercent}%`}
      border={chrome().listBorder ? true : undefined}
      borderStyle={chrome().listBorder ? chrome().listBorderStyle : undefined}
      borderColor={chrome().listBorder ? style().border : undefined}
      backgroundColor={style().bg}
      paddingX={layout().listPaddingX}
      paddingY={layout().listPaddingY}
      title={chrome().listBorder && chrome().showListTitle ? paddedTitle(props.config.labels.historyTitle, layout().frameTitlePadding) : undefined}
      titleAlignment={chrome().listTitleAlignment}
      bottomTitle={
        chrome().listBorder && chrome().showListPositionTitle && props.entries.length > visibleCount()
          ? paddedTitle(listPositionTitle(props.config, props.selectedIndex, props.entries.length), layout().frameTitlePadding)
          : undefined
      }
      bottomTitleAlignment={chrome().listBottomTitleAlignment}
      onMouseScroll={(event: any) => {
        const direction = scrollDirection(event);
        if (!direction) return;
        event.preventDefault();
        props.onScroll(direction);
      }}
    >
      <Show
        when={props.entries.length > 0}
        fallback={<EmptyList config={props.config} query={props.query} />}
      >
        <box flexDirection="row" width="100%" height="100%">
          <box flexDirection="column" flexGrow={1}>
            <For each={visible()}>
              {(row, index) => (
                <>
                  <EntryRow
                    config={props.config}
                    entry={row.entry}
                    index={row.index}
                    width={props.width}
                    query={props.query}
                    selected={row.index === props.selectedIndex}
                    marked={props.selectedIds.has(row.entry.id)}
                    onMouseDown={(event: any) => {
                      if (event.button !== 0) return;
                      event.preventDefault();
                      props.onSelectEntry(row.index, { extend: Boolean(event.modifiers?.shift), toggle: Boolean(event.modifiers?.ctrl) });
                    }}
                  />
                  <Show when={layout().rowSpacing > 0 && index() < visible().length - 1}>
                    <box width="100%" height={layout().rowSpacing} backgroundColor={rowSpacerStyle().bg} />
                  </Show>
                </>
              )}
            </For>
          </box>
          <Show when={layout().showScrollbar}>
            <Scrollbar config={props.config} total={props.entries.length} selectedIndex={props.selectedIndex} rows={props.rows} />
          </Show>
        </box>
      </Show>
    </box>
  );
}

function EntryRow(props: {
  config: ResolvedTuiConfig;
  entry: Entry;
  index: number;
  width: number;
  query: string;
  selected: boolean;
  marked: boolean;
  onMouseDown: (event: any) => void;
}) {
  const rowStyle = () => {
    if (props.selected && props.marked) return surface(props.config, "selectedMarkedRow");
    if (props.selected) return surface(props.config, "selectedRow");
    if (props.marked) return surface(props.config, "markedRow");
    if (props.config.layout.alternateRows && props.index % 2 === 1) return surface(props.config, "alternateRow");
    return surface(props.config, "list");
  };
  const bg = () => rowStyle().bg;
  const marker = () =>
    props.selected && props.marked
      ? props.config.chrome.selectedMarkedMarker
      : props.selected
        ? props.config.chrome.selectedMarker
        : props.marked
          ? props.config.chrome.markedMarker
          : props.config.chrome.normalMarker;
  const layout = () => props.config.layout;
  const rowPrefixWidth = () => marker().length + layout().rowMarkerGap;
  const reservedRowMetaWidth = () => Math.max(rowPrefixWidth() + layout().rowMetaPreviewGap, layout().rowPreviewReservedWidth);
  const metadataWidth = () => Math.max(0, reservedRowMetaWidth() - rowPrefixWidth() - layout().rowMetaPreviewGap);
  const availablePreviewWidth = () => Math.max(1, props.width - (layout().showRowMetadata ? reservedRowMetaWidth() : rowPrefixWidth()));
  const highlightQuery = () => (layout().highlightSearchMatches ? props.query : "");
  const previewWidth = () => {
    const maxWidth = layout().rowPreviewMaxWidth;
    return maxWidth > 0 ? Math.min(availablePreviewWidth(), maxWidth) : availablePreviewWidth();
  };
  return (
    <box height={1} backgroundColor={bg()} onMouseDown={props.onMouseDown}>
      <text style={textStyle(rowStyle(), listContentColor(props.config, rowStyle(), props.entry, "preview"))}>
        <span style={textStyle(rowStyle(), listContentColor(props.config, rowStyle(), props.entry, "marker"))}>{marker()}</span>
        <span style={textStyle(rowStyle(), listContentColor(props.config, rowStyle(), props.entry, "markerGap"))}>{" ".repeat(layout().rowMarkerGap)}</span>
        <Show when={layout().showRowMetadata}>
          <span style={textStyle(rowStyle(), listContentColor(props.config, rowStyle(), props.entry, "metadata"))}>
            {fitRowMeta(entryMeta(props.entry, Date.now(), props.config.labels, layout()), metadataWidth(), props.config.labels)}
          </span>
          <span style={textStyle(rowStyle(), listContentColor(props.config, rowStyle(), props.entry, "metadataGap"))}>{" ".repeat(layout().rowMetaPreviewGap)}</span>
        </Show>
        <For each={entryPreviewSegments(props.entry, previewWidth(), highlightQuery(), props.config.labels)}>
          {(segment) => (
            <span style={textStyle(rowStyle(), listContentColor(props.config, rowStyle(), props.entry, segment.match ? "searchMatch" : "preview"))}>
              {segment.text}
            </span>
          )}
        </For>
      </text>
    </box>
  );
}

function listPositionTitle(config: ResolvedTuiConfig, selectedIndex: number, total: number): string {
  return formatTemplate(config.labels.listPositionTemplate, {
    index: selectedIndex + 1,
    total,
  });
}

function scrollDirection(event: any): -1 | 1 | null {
  if (event.scroll?.direction === "up") return -1;
  if (event.scroll?.direction === "down") return 1;
  if (event.button === 4) return -1;
  if (event.button === 5) return 1;
  return null;
}

function Scrollbar(props: { config: ResolvedTuiConfig; total: number; selectedIndex: number; rows: number }) {
  const style = () => surface(props.config, "scrollbar");
  const width = () => props.config.layout.scrollbarWidth;
  return (
    <box width={width()} flexDirection="column" backgroundColor={style().bg}>
      <For each={scrollbarCells(props.total, props.selectedIndex, props.rows, props.config.chrome.scrollbarThumb, props.config.chrome.scrollbarTrack)}>
        {(cell) => (
          <text style={textStyle(style(), listContentColor(props.config, style(), null, cell === props.config.chrome.scrollbarThumb ? "scrollbarThumb" : "scrollbarTrack"))}>
            {scrollbarCell(cell, width())}
          </text>
        )}
      </For>
    </box>
  );
}

function scrollbarCell(value: string, width: number): string {
  const chars = Array.from(value);
  if (chars.length >= width) return chars.slice(0, width).join("");
  return `${value}${" ".repeat(width - chars.length)}`;
}

function EmptyList(props: { config: ResolvedTuiConfig; query: string }) {
  const style = () => surface(props.config, "emptyState");
  const labels = () => props.config.labels;
  return (
    <box flexDirection="column" paddingX={props.config.layout.emptyStatePaddingX} paddingY={props.config.layout.emptyStatePaddingY} backgroundColor={style().bg}>
      <text style={textStyle(style(), listContentColor(props.config, style(), null, "emptyTitle"))}>{props.query ? labels().noMatchesTitle : labels().noHistoryTitle}</text>
      <Show when={props.config.layout.showEmptyStateHelp}>
        <text style={textStyle(style(), listContentColor(props.config, style(), null, "emptyHelp"))}>
          {props.query ? labels().noMatchesHelp : labels().noHistoryHelp}
        </text>
      </Show>
    </box>
  );
}

function listContentColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, entry: Entry | null, part: TuiListContentPartName): string {
  return configuredToneColor(style, config.listContentTones[part], autoListContentColor(style, entry, part));
}

function autoListContentColor(style: TuiSurfaceStyle, entry: Entry | null, part: TuiListContentPartName): string {
  switch (part) {
    case "marker":
    case "metadata":
      return entry ? entryAccent(style, entry) : style.accent;
    case "markerGap":
    case "metadataGap":
    case "emptyHelp":
    case "scrollbarTrack":
      return style.muted;
    case "searchMatch":
      return style.search;
    case "scrollbarThumb":
      return style.accent;
    case "emptyTitle":
    case "preview":
    default:
      return style.fg;
  }
}

function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}
