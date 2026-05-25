import { For, Show } from "solid-js";
import type { UiState } from "../state";
import type { TuiTheme } from "../theme";

export function ModeOverlay(props: { theme: TuiTheme; state: UiState }) {
  return (
    <>
      <Show when={props.state.mode === "search"}>
        <OverlayFrame theme={props.theme} title=" search " height={3} tone="search">
          <text style={{ fg: props.theme.accentSearch, bg: props.theme.bgElevated }}>/{props.state.query}</text>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "confirm-delete"}>
        <OverlayFrame theme={props.theme} title=" confirm delete " height={3} tone="danger">
          <text style={{ fg: props.theme.accentError, bg: props.theme.bgElevated }}>Delete selected entry?</text>
          <text style={{ fg: props.theme.textMuted, bg: props.theme.bgElevated }}>  y confirm  n cancel</text>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "confirm-clear"}>
        <OverlayFrame theme={props.theme} title=" confirm clear " height={3} tone="danger">
          <text style={{ fg: props.theme.accentError, bg: props.theme.bgElevated }}>Clear {props.state.clearKind}?</text>
          <text style={{ fg: props.theme.textMuted, bg: props.theme.bgElevated }}>  y confirm  n cancel</text>
        </OverlayFrame>
      </Show>
      <Show when={props.state.mode === "help"}>
        <OverlayFrame theme={props.theme} title=" keymap " height={7} tone="command">
          <For each={helpRows}>
            {(row) => (
              <text style={{ fg: props.theme.textSecondary, bg: props.theme.bgElevated }}>
                <span style={{ fg: props.theme.accentCommand, bg: props.theme.bgElevated }}>{row.keys.padEnd(18, " ")}</span>
                {row.action}
              </text>
            )}
          </For>
        </OverlayFrame>
      </Show>
    </>
  );
}

function OverlayFrame(props: { theme: TuiTheme; title: string; height: number; tone: "search" | "danger" | "command"; children: any }) {
  return (
    <box
      height={props.height}
      border
      borderStyle="single"
      borderColor={overlayBorder(props.theme, props.tone)}
      backgroundColor={props.theme.bgElevated}
      paddingX={1}
      title={props.title}
    >
      {props.children}
    </box>
  );
}

const helpRows = [
  { keys: "j/k up/down", action: "move selection" },
  { keys: "enter", action: "paste into previous Hyprland window" },
  { keys: "ctrl+y / y", action: "copy current / copy selected set" },
  { keys: "space / p", action: "select item / toggle pin" },
  { keys: "/ / tab", action: "search / cycle filter" },
  { keys: "d / ca ct ci", action: "delete / clear all text images" },
];

function overlayBorder(theme: TuiTheme, tone: "search" | "danger" | "command"): string {
  if (tone === "danger") return theme.accentError;
  if (tone === "search") return theme.accentSearch;
  return theme.borderFocused;
}
