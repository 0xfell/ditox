/**
 * Ditox TUI performance benchmark.
 *
 * Measures the hot paths behind list navigation and image previews so
 * optimizations can be compared factually:
 *
 *   bun bench/perf.ts            # human-readable table
 *   bun bench/perf.ts --json out.json
 *
 * The synthetic PNGs use per-pixel gradients so zlib cannot degenerate into
 * trivial RLE; decode cost is representative of real screenshots.
 */
import { deflateSync } from "node:zlib";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  decodePng,
  imageBlockPreview,
  imageBlockPreviewAsync,
  imageBlockPreviewFromBytes,
  renderImageBlocks,
  scaleImageToRgba,
  type DecodedImage,
} from "../src/image-preview";
import { kittyTransmitRgba } from "../src/terminal-image";
import { previewModel, wrapPreviewText } from "../src/presentation";
import { initialState, moveSelection, type UiState } from "../src/state";
import type { Entry } from "../src/types";

type BenchResult = {
  name: string;
  msPerOp: number;
  opsPerSec: number;
  iterations: number;
};

const results: BenchResult[] = [];

function bench(name: string, fn: () => void, opts: { minMs?: number; warmup?: number } = {}): void {
  const warmup = opts.warmup ?? 2;
  const minMs = opts.minMs ?? 400;
  for (let i = 0; i < warmup; i += 1) fn();
  let iterations = 0;
  const start = Bun.nanoseconds();
  let elapsed = 0;
  while (elapsed < minMs * 1e6) {
    fn();
    iterations += 1;
    elapsed = Bun.nanoseconds() - start;
  }
  const msPerOp = elapsed / 1e6 / iterations;
  results.push({ name, msPerOp: round(msPerOp), opsPerSec: round(1000 / msPerOp), iterations });
}

async function benchAsync(name: string, fn: () => Promise<void>, opts: { minMs?: number; warmup?: number } = {}): Promise<void> {
  const warmup = opts.warmup ?? 2;
  const minMs = opts.minMs ?? 400;
  for (let i = 0; i < warmup; i += 1) await fn();
  let iterations = 0;
  const start = Bun.nanoseconds();
  let elapsed = 0;
  while (elapsed < minMs * 1e6) {
    await fn();
    iterations += 1;
    elapsed = Bun.nanoseconds() - start;
  }
  const msPerOp = elapsed / 1e6 / iterations;
  results.push({ name, msPerOp: round(msPerOp), opsPerSec: round(1000 / msPerOp), iterations });
}

function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}

// --- synthetic inputs -------------------------------------------------------

function syntheticPng(width: number, height: number): Uint8Array {
  const stride = width * 4;
  const raw = new Uint8Array(height * (1 + stride));
  let offset = 0;
  for (let y = 0; y < height; y += 1) {
    raw[offset++] = 0; // filter: none
    for (let x = 0; x < width; x += 1) {
      raw[offset++] = (x * 7 + y * 3) & 0xff;
      raw[offset++] = (x * 2 + y * 11) & 0xff;
      raw[offset++] = (x ^ y) & 0xff;
      raw[offset++] = 255;
    }
  }
  return concatBytes(
    Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk("IHDR", concatBytes(u32(width), u32(height), Uint8Array.from([8, 6, 0, 0, 0]))),
    pngChunk("IDAT", deflateSync(raw, { level: 6 })),
    pngChunk("IEND", new Uint8Array()),
  );
}

function pngChunk(type: string, data: Uint8Array): Uint8Array {
  const typeBytes = Uint8Array.from([...type].map((char) => char.charCodeAt(0)));
  return concatBytes(u32(data.length), typeBytes, data, Uint8Array.from([0, 0, 0, 0]));
}

function u32(value: number): Uint8Array {
  return Uint8Array.from([(value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff]);
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

function imageEntry(id: number, blobPath: string, byteLen: number): Entry {
  return {
    id,
    kind: "image",
    mime: "image/png",
    content: "hash",
    preview: "image/png",
    hash: `image-${id}`.padEnd(64, "0"),
    favorite: false,
    created_at_ms: Date.now(),
    last_used_at_ms: null,
    byte_len: byteLen,
    source_app: null,
    blob_path: blobPath,
    image_width: null,
    image_height: null,
  };
}

function textEntry(id: number): Entry {
  return {
    id,
    kind: "text",
    mime: "text/plain",
    content: `clip ${id}`,
    preview: `clip ${id}`,
    hash: `text-${id}`.padEnd(64, "0"),
    favorite: false,
    created_at_ms: Date.now() - id,
    last_used_at_ms: null,
    byte_len: 16,
    source_app: null,
    blob_path: null,
    image_width: null,
    image_height: null,
  };
}

// --- benches ----------------------------------------------------------------

const png720 = syntheticPng(1280, 720);
const png1440 = syntheticPng(2560, 1440);
const decoded1080: DecodedImage = (() => {
  const png = syntheticPng(1920, 1080);
  return decodePng(png);
})();

const dir = mkdtempSync(join(tmpdir(), "ditox-bench-"));
const blobPath = join(dir, "bench-1440.png");
writeFileSync(blobPath, png1440);
const benchEntry = imageEntry(1, blobPath, png1440.length);

bench("png decode 1280x720", () => {
  decodePng(png720);
});

bench("png decode 2560x1440", () => {
  decodePng(png1440);
}, { minMs: 600 });

bench("blocks pipeline (decode+render) 1280x720 -> 40x20 cells", () => {
  imageBlockPreviewFromBytes(png720, 40, 20, "#101010", "image/png");
});

bench("blocks render only (pre-decoded) 1920x1080 -> 56x28 cells", () => {
  renderImageBlocks(decoded1080, 56, 28, "#101010");
});

let uncachedWidth = 40;
await benchAsync("entry preview new geometry, same blob 2560x1440", async () => {
  // Distinct geometry per call defeats the rendered-preview cache, simulating
  // pane resizes / split<->full transitions on an already-seen blob. With a
  // decoded-image cache only the cell render is paid; without one every
  // geometry change re-reads and re-decodes the full PNG.
  uncachedWidth += 1;
  // Cycle through more geometries than the rendered-preview LRU holds so
  // every iteration misses it and the measurement stays honest.
  if (uncachedWidth > 99) uncachedWidth = 40;
  await imageBlockPreviewAsync(benchEntry, uncachedWidth, 24, "#101010", "blocks");
}, { minMs: 800 });

let toggleSplit = true;
await benchAsync("entry preview size toggle (split<->full) 2560x1440", async () => {
  // Alternates between two geometries like toggling split/full preview.
  // Without a decoded-image cache each toggle re-reads + re-decodes.
  toggleSplit = !toggleSplit;
  await imageBlockPreviewAsync(benchEntry, toggleSplit ? 38 : 96, toggleSplit ? 18 : 38, "#101010", "blocks");
}, { minMs: 800 });

bench("kitty scale 1920x1080 -> 408x528 px", () => {
  scaleImageToRgba(decoded1080, 408, 528, "#101010");
});

const kittyPixels = scaleImageToRgba(decoded1080, 408, 528, "#101010");
bench("kitty transmit encode 408x528 rgba", () => {
  kittyTransmitRgba(7, kittyPixels, 408, 528);
});

const longContent = Array.from({ length: 1000 }, (_, i) => `line ${i} ${"persistently-long-token ".repeat(7)}`).join("\n");
const longEntry: Entry = { ...textEntry(1), content: longContent, byte_len: longContent.length };
bench("previewModel 1000 wrapped lines @60 cols", () => {
  previewModel(longEntry, 64, {}, {}, 60);
});

const oneLongLine = "word ".repeat(4000);
bench("wrapPreviewText 20k chars @80 cols", () => {
  wrapPreviewText(oneLongLine, 80);
});

const navEntries = Array.from({ length: 1000 }, (_, i) => textEntry(i + 1));
const navState: UiState = { ...initialState(), entries: navEntries };
bench("moveSelection x1000 over 1000 entries", () => {
  let state = navState;
  for (let i = 0; i < 1000; i += 1) state = moveSelection(state, 1);
});

// Defeat dead-code elimination concerns trivially by touching results.
void imageBlockPreview;

rmSync(dir, { recursive: true, force: true });

// --- output -----------------------------------------------------------------

const jsonFlag = process.argv.indexOf("--json");
if (jsonFlag >= 0 && process.argv[jsonFlag + 1]) {
  writeFileSync(process.argv[jsonFlag + 1]!, JSON.stringify(results, null, 2));
}
console.log(`ditox tui benchmark — bun ${Bun.version}`);
for (const result of results) {
  console.log(`${result.name.padEnd(58)} ${String(result.msPerOp).padStart(10)} ms/op  ${String(result.opsPerSec).padStart(12)} ops/s  (${result.iterations}x)`);
}
