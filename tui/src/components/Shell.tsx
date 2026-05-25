import type { JSX } from "solid-js";
import type { TuiTheme } from "../theme";

export function Shell(props: { theme: TuiTheme; children: JSX.Element }) {
  return (
    <box flexDirection="column" width="100%" height="100%" backgroundColor={props.theme.bgBase}>
      {props.children}
    </box>
  );
}
