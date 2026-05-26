import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import { themeNames, themes } from "./theme";
import {
  clipseKeyBindingAliasNames,
  headerLinePartNames,
  helpActionNames,
  imagePreviewNoticeVisibilityNames,
  imagePreviewRendererNames,
  listContentPartNames,
  overlayPlacementNames,
  overlayContentPartNames,
  overlayToneNames,
  previewContentPartNames,
  resolveTuiConfig,
  statusLinePartNames,
  statusLineToneNames,
  terminalAltScreenNames,
  terminalCursorStyleNames,
  terminalScreenModeNames,
  titleAlignmentNames,
  tuiSurfaceNames,
} from "./tui-config";

type JsonObject = Record<string, any>;

const schemaPath = join(import.meta.dir, "../tui-config.schema.json");
const examplePath = join(import.meta.dir, "../tui-config.example.json");

function readJson(path: string): JsonObject {
  return JSON.parse(readFileSync(path, "utf8")) as JsonObject;
}

function sorted(values: Iterable<string>): string[] {
  return [...values].sort((a, b) => a.localeCompare(b));
}

describe("tui config schema", () => {
  test("is valid JSON and linked from the example config", () => {
    const schema = readJson(schemaPath);
    const example = readJson(examplePath);

    expect(schema.$schema).toBe("https://json-schema.org/draft/2020-12/schema");
    expect(schema.title).toBe("Ditox TUI config");
    expect(example.$schema).toBe("./tui-config.schema.json");
  });

  test("documents every example top-level config block", () => {
    const schema = readJson(schemaPath);
    const example = readJson(examplePath);

    const documented = new Set(Object.keys(schema.properties));
    expect(Object.keys(example).filter((key) => !documented.has(key))).toEqual([]);
  });

  test("keeps surface names in sync with runtime config", () => {
    const schema = readJson(schemaPath);
    const schemaSurfaces = schema.properties.styles.propertyNames.enum as string[];

    expect(sorted(schemaSurfaces)).toEqual(sorted(tuiSurfaceNames));
  });

  test("keeps structural config blocks in sync with resolved defaults", () => {
    const schema = readJson(schemaPath);
    const config = resolveTuiConfig();

    expect(sorted(Object.keys(schema.properties.layout.properties))).toEqual(sorted(Object.keys(config.layout)));
    expect(sorted(Object.keys(schema.properties.terminal.properties).filter((key) => key !== "alt_screen"))).toEqual(sorted(Object.keys(config.terminal)));
    expect(schema.properties.terminal.properties.alt_screen.$ref).toBe("#/$defs/terminalAltScreen");
    expect(sorted(Object.keys(schema.properties.chrome.properties))).toEqual(sorted(Object.keys(config.chrome)));
    expect(sorted(Object.keys(schema.properties.behavior.properties))).toEqual(sorted(Object.keys(config.behavior)));
    expect(sorted(Object.keys(schema.properties.statusTones.properties))).toEqual(sorted(Object.keys(config.statusTones)));
    expect(sorted(Object.keys(schema.properties.headerLineTones.properties))).toEqual(sorted(headerLinePartNames));
    expect(sorted(Object.keys(schema.properties.statusLineTones.properties))).toEqual(sorted(statusLinePartNames));
    expect(sorted(Object.keys(schema.properties.overlayBorderTones.properties))).toEqual(sorted(overlayToneNames));
    expect(sorted(Object.keys(schema.properties.overlayContentTones.properties))).toEqual(sorted(overlayContentPartNames));
    expect(sorted(Object.keys(schema.properties.listContentTones.properties))).toEqual(sorted(listContentPartNames));
    expect(sorted(Object.keys(schema.properties.previewContentTones.properties))).toEqual(sorted(previewContentPartNames));
    expect(sorted(schema.properties.labels.propertyNames.enum as string[])).toEqual(sorted(Object.keys(config.labels)));
  });

  test("keeps theme colors and keybinding names in sync", () => {
    const schema = readJson(schemaPath);
    const config = resolveTuiConfig();
    const themeColorKeys = Object.keys(themes.ditoxDark).filter((key) => key !== "name");
    const themeSchemaKeys = Object.keys(schema.properties.theme.oneOf[1].properties.colors.properties);
    const themeSchemaNames = schema.$defs.themeName.enum as string[];
    const keyBindingSchemaNames = schema.properties.keyBindings.propertyNames.enum as string[];

    expect(sorted(themeSchemaKeys)).toEqual(sorted(themeColorKeys));
    expect(sorted(themeSchemaNames)).toEqual(sorted(themeNames));
    expect(sorted(keyBindingSchemaNames)).toEqual(sorted([...Object.keys(config.keyBindings), ...clipseKeyBindingAliasNames]));
  });

  test("keeps help action names in sync", () => {
    const schema = readJson(schemaPath);

    expect(sorted(schema.$defs.helpAction.enum as string[])).toEqual(sorted(helpActionNames));
    expect(sorted(schema.$defs.statusLineTone.enum as string[])).toEqual(sorted(statusLineToneNames));
    expect(sorted(schema.$defs.terminalAltScreen.enum as string[])).toEqual(sorted(terminalAltScreenNames));
    expect(sorted(schema.$defs.terminalScreenMode.enum as string[])).toEqual(sorted(terminalScreenModeNames));
    expect(sorted(schema.$defs.terminalCursorStyle.enum as string[])).toEqual(sorted(terminalCursorStyleNames));
    expect(sorted(schema.$defs.titleAlignment.enum as string[])).toEqual(sorted(titleAlignmentNames));
    expect(sorted(schema.$defs.imagePreviewNoticeVisibility.enum as string[])).toEqual(sorted(imagePreviewNoticeVisibilityNames));
    expect(sorted(schema.$defs.imagePreviewRenderer.enum as string[])).toEqual(sorted(imagePreviewRendererNames));
    expect(sorted(schema.$defs.overlayPlacement.enum as string[])).toEqual(sorted(overlayPlacementNames));
  });
});
