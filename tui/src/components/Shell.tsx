import type { JSX } from "solid-js";
import { surface, type ResolvedTuiConfig } from "../tui-config";

export function Shell(props: { config: ResolvedTuiConfig; children: JSX.Element }) {
  const style = () => surface(props.config, "shell");
  const layout = () => props.config.layout;
  return (
    <box flexDirection="column" width="100%" height="100%" backgroundColor={style().bg} paddingX={layout().shellPaddingX} paddingY={layout().shellPaddingY}>
      {props.children}
    </box>
  );
}
