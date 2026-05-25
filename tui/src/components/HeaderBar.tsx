import type { UiState } from "../state";
import type { TuiTheme } from "../theme";
import { formatFilter } from "../presentation";

export function HeaderBar(props: { theme: TuiTheme; state: UiState; selectedCount: number }) {
  const query = () => props.state.query || "-";
  const selected = () => (props.selectedCount > 0 ? `selected ${props.selectedCount}` : "single");
  return (
    <box
      height={3}
      border
      borderStyle="single"
      borderColor={props.theme.borderFocused}
      backgroundColor={props.theme.bgPanel}
      paddingX={1}
      title=" ditox "
    >
      <text style={{ fg: props.theme.textPrimary, bg: props.theme.bgPanel }}>
        <span style={{ fg: props.theme.accentCommand, bg: props.theme.bgPanel }}>DITOX</span>
        <span style={{ fg: props.theme.textDim, bg: props.theme.bgPanel }}>  filter </span>
        <span style={{ fg: props.theme.textPrimary, bg: props.theme.bgPanel }}>{formatFilter(props.state.filter)}</span>
        <span style={{ fg: props.theme.textDim, bg: props.theme.bgPanel }}>  query </span>
        <span style={{ fg: props.theme.accentSearch, bg: props.theme.bgPanel }}>{query()}</span>
        <span style={{ fg: props.theme.textDim, bg: props.theme.bgPanel }}>  mode </span>
        <span style={{ fg: props.theme.textSecondary, bg: props.theme.bgPanel }}>{selected()}</span>
      </text>
    </box>
  );
}
