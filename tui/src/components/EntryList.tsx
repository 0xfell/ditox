import { For, Show } from "solid-js";
import { entryAccent, entryMeta, entryPreview, scrollbarCells } from "../presentation";
import { visibleEntries } from "../state";
import type { UiConfig } from "../ui-config";
import type { TuiTheme } from "../theme";
import type { Entry } from "../types";

export function EntryList(props: {
  theme: TuiTheme;
  config: UiConfig;
  entries: Entry[];
  selectedIndex: number;
  selectedIds: Set<number>;
  rows: number;
  width: number;
  query: string;
}) {
  const visible = () => visibleEntries(props.entries, props.selectedIndex, props.rows);
  return (
    <box
      width={`${props.config.listWidthPercent}%`}
      border
      borderStyle="single"
      borderColor={props.theme.border}
      backgroundColor={props.theme.bgPanel}
      paddingX={props.config.panelPaddingX}
      paddingY={props.config.panelPaddingY}
      title=" history "
      bottomTitle={props.entries.length > props.rows ? ` ${props.selectedIndex + 1}/${props.entries.length} ` : undefined}
    >
      <Show
        when={props.entries.length > 0}
        fallback={<EmptyList theme={props.theme} query={props.query} />}
      >
        <box flexDirection="row" width="100%" height="100%">
          <box flexDirection="column" flexGrow={1}>
            <For each={visible()}>
              {(row) => (
                <EntryRow
                  theme={props.theme}
                  entry={row.entry}
                  width={props.width}
                  selected={row.index === props.selectedIndex}
                  marked={props.selectedIds.has(row.entry.id)}
                />
              )}
            </For>
          </box>
          <Show when={props.config.showScrollbar}>
            <Scrollbar theme={props.theme} total={props.entries.length} selectedIndex={props.selectedIndex} rows={props.rows} />
          </Show>
        </box>
      </Show>
    </box>
  );
}

function EntryRow(props: { theme: TuiTheme; entry: Entry; width: number; selected: boolean; marked: boolean }) {
  const bg = () => (props.selected ? props.theme.bgSelected : props.theme.bgPanel);
  const fg = () => (props.selected ? props.theme.selectionFg : props.theme.textPrimary);
  const accent = () => entryAccent(props.theme, props.entry);
  const marker = () => (props.selected ? ">" : props.marked ? "+" : "|");
  const previewWidth = () => Math.max(8, props.width - 28);
  return (
    <box height={1} flexDirection="row" backgroundColor={bg()}>
      <text style={{ fg: accent(), bg: bg() }}>{marker()}</text>
      <text style={{ fg: props.theme.textMuted, bg: bg() }}> </text>
      <text style={{ fg: accent(), bg: bg() }}>{entryMeta(props.entry)}</text>
      <text style={{ fg: props.theme.textMuted, bg: bg() }}>  </text>
      <text style={{ fg: fg(), bg: bg() }}>{entryPreview(props.entry, previewWidth())}</text>
    </box>
  );
}

function Scrollbar(props: { theme: TuiTheme; total: number; selectedIndex: number; rows: number }) {
  return (
    <box width={1} flexDirection="column" backgroundColor={props.theme.bgPanel}>
      <For each={scrollbarCells(props.total, props.selectedIndex, props.rows)}>
        {(cell) => (
          <text style={{ fg: cell === "#" ? props.theme.scrollbarThumb : props.theme.scrollbarTrack, bg: props.theme.bgPanel }}>
            {cell}
          </text>
        )}
      </For>
    </box>
  );
}

function EmptyList(props: { theme: TuiTheme; query: string }) {
  return (
    <box flexDirection="column" paddingX={1} paddingY={1} backgroundColor={props.theme.bgPanel}>
      <text style={{ fg: props.theme.textSecondary, bg: props.theme.bgPanel }}>{props.query ? "No matches" : "No clipboard history"}</text>
      <text style={{ fg: props.theme.textDim, bg: props.theme.bgPanel }}>
        {props.query ? "Try a broader search." : "Start the watcher or add an entry from the CLI."}
      </text>
    </box>
  );
}
