export type ThemeName = "ditoxDark" | "ditoxLight";

export type TuiTheme = {
  name: ThemeName;
  bgBase: string;
  bgPanel: string;
  bgElevated: string;
  bgSelected: string;
  bgSubtle: string;
  border: string;
  borderFocused: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  textDim: string;
  selectionFg: string;
  accentText: string;
  accentImage: string;
  accentFavorite: string;
  accentError: string;
  accentSuccess: string;
  accentWarning: string;
  accentSearch: string;
  accentCommand: string;
  scrollbarTrack: string;
  scrollbarThumb: string;
};

export const themes: Record<ThemeName, TuiTheme> = {
  ditoxDark: {
    name: "ditoxDark",
    bgBase: "#101214",
    bgPanel: "#15191d",
    bgElevated: "#1c2227",
    bgSelected: "#26323a",
    bgSubtle: "#181d21",
    border: "#354049",
    borderFocused: "#5ab0c9",
    textPrimary: "#eef2f3",
    textSecondary: "#b9c4c9",
    textMuted: "#7e8c93",
    textDim: "#59676f",
    selectionFg: "#ffffff",
    accentText: "#7fd27c",
    accentImage: "#78a8ff",
    accentFavorite: "#f2c14e",
    accentError: "#ef6f6c",
    accentSuccess: "#72d49b",
    accentWarning: "#d9a441",
    accentSearch: "#c58cff",
    accentCommand: "#8fd5ff",
    scrollbarTrack: "#242b31",
    scrollbarThumb: "#6d7d86",
  },
  ditoxLight: {
    name: "ditoxLight",
    bgBase: "#f6f7f4",
    bgPanel: "#ffffff",
    bgElevated: "#eef1ed",
    bgSelected: "#dbe8eb",
    bgSubtle: "#e8ece8",
    border: "#c2cbc5",
    borderFocused: "#327f94",
    textPrimary: "#172023",
    textSecondary: "#3f4d52",
    textMuted: "#6d797d",
    textDim: "#8f999c",
    selectionFg: "#0b1417",
    accentText: "#197b44",
    accentImage: "#245fb8",
    accentFavorite: "#9a6b00",
    accentError: "#b53330",
    accentSuccess: "#207a4a",
    accentWarning: "#906318",
    accentSearch: "#6f3aa6",
    accentCommand: "#237486",
    scrollbarTrack: "#d9dfda",
    scrollbarThumb: "#6e7f82",
  },
};

export function currentTheme(): TuiTheme {
  const requested = Bun.env.DITOX_TUI_THEME;
  if (requested === "ditoxLight") return themes.ditoxLight;
  return themes.ditoxDark;
}
