import type { TuiTheme } from "../theme";
import { statusTone } from "../presentation";

export function StatusLine(props: { theme: TuiTheme; status: string }) {
  const color = () => {
    switch (statusTone(props.status)) {
      case "error":
        return props.theme.accentError;
      case "success":
        return props.theme.accentSuccess;
      case "warning":
        return props.theme.accentWarning;
      default:
        return props.theme.textMuted;
    }
  };
  return (
    <box height={1} backgroundColor={props.theme.bgBase} paddingX={1}>
      <text style={{ fg: props.theme.textDim, bg: props.theme.bgBase }}>
        enter paste  ctrl+y copy  y bulk copy  / search  ? help
        <span style={{ fg: props.theme.textDim, bg: props.theme.bgBase }}>  |  </span>
        <span style={{ fg: color(), bg: props.theme.bgBase }}>{props.status || "ready"}</span>
      </text>
    </box>
  );
}
