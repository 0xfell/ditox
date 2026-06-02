import { describe, expect, test } from "bun:test";
import { testRender } from "@opentui/solid";
import { TextAttributes, type CapturedFrame, type CapturedSpan } from "@opentui/core";
import { Resvg } from "@resvg/resvg-js";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { EntryList } from "./components/EntryList";
import { FullPreview } from "./components/FullPreview";
import { HeaderBar } from "./components/HeaderBar";
import { ModeOverlay } from "./components/Overlay";
import { PreviewPane } from "./components/PreviewPane";
import { Shell } from "./components/Shell";
import { StatusLine } from "./components/StatusLine";
import { initialState } from "./state";
import { resolveTuiConfig } from "./tui-config";
import type { Entry } from "./types";

const fixedNow = 1_700_000_000_000;
const goldenDir = join(import.meta.dir, "__goldens__");
const artifactDir = Bun.env.DITOX_TUI_ARTIFACT_DIR ?? join(import.meta.dir, "..", "artifacts", "frames");

describe("OpenTUI golden frames", () => {
  test("matches persisted shell golden", async () => {
    const config = resolveTuiConfig({
      labels: {
        brand: "GOLD",
        pinnedViewTitle: "SAVED",
        selectedPrefix: "marked",
        headerSectionSeparator: "::",
        headerLabelSeparator: "=",
        previewHashGutter: "sha",
        previewMetaHashLabel: "digest",
        previewMetaSeparator: " ~ ",
        previewMetaLabelSeparator: ":",
        sizeKibUnit: "KB",
        ageSecondsUnit: "sec",
      },
      chrome: {
        panelBorderStyle: "single",
        selectedMarker: ">",
        markedMarker: "+",
        normalMarker: ".",
        statusSeparator: "/",
      },
      layout: {
        listWidthPercent: 44,
        frameTitlePadding: 0,
        headerPaddingX: 0,
        statusPaddingX: 0,
        previewMetaPaddingX: 0,
        statusSeparatorPadding: 1,
        rowMarkerGap: 1,
        rowMetaPreviewGap: 2,
        rowPreviewReservedWidth: 22,
        previewMetaHashLength: 10,
        imagePreviewMode: "metadata",
        maxPreviewLines: 4,
      },
    });
    const entries = [
      textEntry(1, "saved golden clip with enough text for truncation", true),
      imageEntry(2),
      textEntry(3, "plain golden clip", false),
    ];
    const state = {
      ...initialState(),
      entries,
      selectedIndex: 0,
      selectedIds: new Set([1, 2]),
      pinnedOnly: true,
      status: "stored 2",
      watcher: {
        running: false,
        paused: true,
        backend: "wayland",
        poll_interval_ms: 750,
        last_seen_ms: fixedNow - 3_000,
        last_error: null,
      },
    };

    await expectGolden(
      "shell-pinned-wide",
      () => (
        <Shell config={config}>
          <HeaderBar config={config} state={state} selectedCount={state.selectedIds.size} />
          <box flexGrow={1} flexDirection="row">
            <EntryList
              config={config}
              entries={entries}
              selectedIndex={state.selectedIndex}
              selectedIds={state.selectedIds}
              rows={7}
              width={42}
              query={state.query}
              onSelectEntry={() => {}}
              onScroll={() => {}}
            />
            <PreviewPane config={config} entry={entries[state.selectedIndex]} rows={7} width={60} />
          </box>
          <StatusLine config={config} status={state.status} watcher={state.watcher} width={108} />
        </Shell>
      ),
      108,
      15,
    );
  });

  test("matches persisted overlay golden", async () => {
    const config = resolveTuiConfig({
      labels: {
        helpTitle: "bindings",
        helpPaste: "send selected clip",
        helpSearchCopyMatches: "copy filtered clips",
      },
      layout: {
        frameTitlePadding: 0,
        overlayPaddingX: 0,
        helpKeyWidth: 34,
      },
      keyBindings: {
        copyPaste: "ctrl+p",
        searchCopyMatches: "ctrl+g",
      },
    });

    await expectGolden(
      "help-overlay",
      () => (
        <Shell config={config}>
          <ModeOverlay config={config} state={{ ...initialState(), mode: "help" }} width={64} />
        </Shell>
      ),
      64,
      17,
    );
  });

  test("matches persisted full-preview golden", async () => {
    const config = resolveTuiConfig({
      labels: {
        previewModeTitle: "reader",
        previewBackHint: "leave",
        previewGutterSeparator: ":",
        kindText: "STR",
      },
      layout: {
        frameTitlePadding: 0,
        previewLineNumberWidth: 2,
        previewGutterWidth: 2,
        maxFullPreviewLines: 8,
        panelPaddingX: 0,
      },
      chrome: {
        panelBorderStyle: "single",
      },
    });

    await expectGolden(
      "full-preview",
      () => (
        <Shell config={config}>
          <FullPreview
            config={config}
            entry={textEntry(8, "alpha\nbeta\ngamma\ndelta\nepsilon", false)}
            rows={7}
            width={48}
            offset={2}
            onScroll={() => {}}
          />
        </Shell>
      ),
      52,
      9,
    );
  });

  test("exports deterministic text, spans, SVG, and PNG review artifacts", async () => {
    const dir = mkdtempSync(join(tmpdir(), "ditox-tui-artifacts-"));
    const previousEnabled = Bun.env.DITOX_TUI_ARTIFACTS;
    const previousDir = Bun.env.DITOX_TUI_ARTIFACT_DIR;
    Bun.env.DITOX_TUI_ARTIFACTS = "1";
    Bun.env.DITOX_TUI_ARTIFACT_DIR = dir;
    try {
      const config = resolveTuiConfig({
        labels: { brand: "ART" },
        layout: { frameTitlePadding: 0 },
      });
      const capture = await withFixedNow(() =>
        captureFrame(
          () => (
            <Shell config={config}>
              <HeaderBar config={config} state={{ ...initialState(), status: "artifact" }} selectedCount={0} />
            </Shell>
          ),
          32,
          4,
        ),
      );

      writeFrameArtifact("artifact-smoke", capture);

      expect(readFileSync(join(dir, "artifact-smoke.frame.txt"), "utf8")).toContain("ART");
      expect(JSON.parse(readFileSync(join(dir, "artifact-smoke.spans.json"), "utf8")).cols).toBe(32);
      expect(readFileSync(join(dir, "artifact-smoke.svg"), "utf8")).toContain("<svg");
      expect([...readFileSync(join(dir, "artifact-smoke.png")).slice(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
    } finally {
      if (previousEnabled === undefined) delete Bun.env.DITOX_TUI_ARTIFACTS;
      else Bun.env.DITOX_TUI_ARTIFACTS = previousEnabled;
      if (previousDir === undefined) delete Bun.env.DITOX_TUI_ARTIFACT_DIR;
      else Bun.env.DITOX_TUI_ARTIFACT_DIR = previousDir;
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

async function expectGolden(name: string, node: () => any, width: number, height: number): Promise<void> {
  const capture = await withFixedNow(() => captureFrame(node, width, height));
  const frame = capture.text;
  expectFrameWithin(frame, width, height);
  writeFrameArtifact(name, capture);

  const path = join(goldenDir, `${name}.frame.txt`);
  if (Bun.env.DITOX_UPDATE_GOLDENS === "1") {
    mkdirSync(goldenDir, { recursive: true });
    writeFileSync(path, frame);
    return;
  }

  expect(existsSync(path), `missing golden frame: ${path}; run DITOX_UPDATE_GOLDENS=1 bun test src/tui-golden.test.tsx`).toBe(true);
  expect(frame).toBe(readFileSync(path, "utf8"));
}

async function withFixedNow<T>(run: () => Promise<T>): Promise<T> {
  const originalNow = Date.now;
  Date.now = () => fixedNow;
  try {
    return await run();
  } finally {
    Date.now = originalNow;
  }
}

type CapturedTuiFrame = {
  text: string;
  spans: CapturedFrame;
};

async function captureFrame(node: () => any, width: number, height: number): Promise<CapturedTuiFrame> {
  const view = await testRender(node, { width, height });
  try {
    await view.renderOnce();
    return { text: view.captureCharFrame(), spans: view.captureSpans() };
  } finally {
    view.renderer.destroy();
  }
}

function writeFrameArtifact(name: string, capture: CapturedTuiFrame): void {
  if (Bun.env.DITOX_TUI_ARTIFACTS !== "1") return;
  const dir = Bun.env.DITOX_TUI_ARTIFACT_DIR ?? artifactDir;
  mkdirSync(dir, { recursive: true });
  const svg = frameToSvg(capture.spans);
  writeFileSync(join(dir, `${name}.frame.txt`), capture.text);
  writeFileSync(join(dir, `${name}.spans.json`), JSON.stringify(capture.spans, null, 2));
  writeFileSync(join(dir, `${name}.svg`), svg);
  writeFileSync(join(dir, `${name}.png`), svgToPng(svg));
}

function svgToPng(svg: string): Uint8Array {
  return new Resvg(svg, {
    fitTo: { mode: "original" },
    font: {
      loadSystemFonts: true,
      defaultFontFamily: "monospace",
    },
  }).render().asPng();
}

function frameToSvg(frame: CapturedFrame): string {
  const cellWidth = 9;
  const cellHeight = 18;
  const fontSize = 14;
  const baseline = 14;
  const width = frame.cols * cellWidth;
  const height = frame.rows * cellHeight;
  const lines = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`,
    `<style>text{font-family:"JetBrains Mono","Fira Code","SFMono-Regular",Consolas,monospace;font-size:${fontSize}px;dominant-baseline:alphabetic;white-space:pre}</style>`,
  ];

  for (let y = 0; y < frame.lines.length; y += 1) {
    let x = 0;
    for (const span of frame.lines[y]?.spans ?? []) {
      const spanWidth = Math.max(0, span.width) * cellWidth;
      const posX = x * cellWidth;
      const posY = y * cellHeight;
      lines.push(`<rect x="${posX}" y="${posY}" width="${spanWidth}" height="${cellHeight}" fill="${rgbaToCss(span.bg)}"/>`);
      if (span.text.trim().length > 0 && !hasTextAttribute(span, TextAttributes.HIDDEN)) {
        lines.push(
          `<text x="${posX}" y="${posY + baseline}" fill="${rgbaToCss(span.fg)}"${svgTextAttributes(span)}>${escapeXml(span.text)}</text>`,
        );
      }
      x += span.width;
    }
  }

  lines.push("</svg>\n");
  return lines.join("\n");
}

function svgTextAttributes(span: CapturedSpan): string {
  const attrs = [];
  if (hasTextAttribute(span, TextAttributes.BOLD)) attrs.push(`font-weight="700"`);
  if (hasTextAttribute(span, TextAttributes.ITALIC)) attrs.push(`font-style="italic"`);
  const decorations = [];
  if (hasTextAttribute(span, TextAttributes.UNDERLINE)) decorations.push("underline");
  if (hasTextAttribute(span, TextAttributes.STRIKETHROUGH)) decorations.push("line-through");
  if (decorations.length > 0) attrs.push(`text-decoration="${decorations.join(" ")}"`);
  if (hasTextAttribute(span, TextAttributes.DIM)) attrs.push(`opacity="0.68"`);
  return attrs.length > 0 ? ` ${attrs.join(" ")}` : "";
}

function hasTextAttribute(span: CapturedSpan, mask: number): boolean {
  return (Number(span.attributes ?? 0) & mask) === mask;
}

function rgbaToCss(color: unknown): string {
  const [r, g, b, a] = rgbaBytes(color);
  if (a >= 255) return `rgb(${r} ${g} ${b})`;
  return `rgb(${r} ${g} ${b} / ${(a / 255).toFixed(3)})`;
}

function rgbaBytes(color: unknown): [number, number, number, number] {
  const maybeColor = color as { buffer?: Uint8Array | number[]; toInts?: () => number[] } | undefined;
  if (maybeColor && typeof maybeColor.toInts === "function") {
    const ints = maybeColor.toInts();
    return [byteValue(ints[0]), byteValue(ints[1]), byteValue(ints[2]), byteValue(ints[3] ?? 255)];
  }

  const raw = maybeColor?.buffer ?? color;
  const bytes =
    raw instanceof Uint8Array
      ? [...raw]
      : Array.isArray(raw)
        ? raw
        : [(raw as Record<string, unknown> | undefined)?.["0"], (raw as Record<string, unknown> | undefined)?.["1"], (raw as Record<string, unknown> | undefined)?.["2"], (raw as Record<string, unknown> | undefined)?.["3"]];
  return [byteValue(bytes[0]), byteValue(bytes[1]), byteValue(bytes[2]), byteValue(bytes[3] ?? 255)];
}

function byteValue(value: unknown): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return 0;
  if (numeric >= 0 && numeric <= 1) return Math.round(numeric * 255);
  return Math.max(0, Math.min(255, Math.round(numeric)));
}

function escapeXml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

function expectFrameWithin(frame: string, width: number, height: number): void {
  const lines = frame.replace(/\n$/, "").split("\n");
  expect(lines.length).toBeLessThanOrEqual(height);
  for (const line of lines) expect([...line].length).toBeLessThanOrEqual(width);
}

function textEntry(id: number, content: string, favorite: boolean): Entry {
  return {
    id,
    kind: "text",
    mime: "text/plain",
    content,
    preview: content,
    hash: `text-${id}`.padEnd(64, "0"),
    favorite,
    created_at_ms: fixedNow - id * 1000,
    last_used_at_ms: null,
    byte_len: Buffer.byteLength(content),
    source_app: null,
    blob_path: null,
    image_width: null,
    image_height: null,
  };
}

function imageEntry(id: number): Entry {
  return {
    id,
    kind: "image",
    mime: "image/png",
    content: "hash",
    preview: "image/png 32x24",
    hash: `image-${id}`.padEnd(64, "0"),
    favorite: false,
    created_at_ms: fixedNow - id * 1000,
    last_used_at_ms: null,
    byte_len: 2048,
    source_app: null,
    blob_path: null,
    image_width: 32,
    image_height: 24,
  };
}
