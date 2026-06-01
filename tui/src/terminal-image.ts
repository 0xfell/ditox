import type { PixelResolution } from "@opentui/core";
import type { DecodedImage, ImageProtocolCapabilities } from "./image-preview";
import { scaleImageToRgba } from "./image-preview";
import type { ImagePreviewAlign, ImagePreviewMode, ImagePreviewRenderer } from "./ui-config";

export type NativeImageProtocol = "kitty" | "sixel";

export type TerminalImageState = {
  columns: number;
  rows: number;
  resolution: PixelResolution | null;
  capabilities: Partial<ImageProtocolCapabilities>;
};

export type NativeImagePlacement = {
  protocol: NativeImageProtocol;
  cols: number;
  rows: number;
  pixelWidth: number;
  pixelHeight: number;
  contentPixelWidth: number;
  contentPixelHeight: number;
  contentOffsetX: number;
  contentOffsetY: number;
  leftPad: number;
};

export type NativeImageRequest = {
  protocol: NativeImageProtocol;
  sourceKey: string;
  image: DecodedImage;
  background: string;
  screenX: number;
  screenY: number;
  cols: number;
  rows: number;
  pixelWidth: number;
  pixelHeight: number;
  contentPixelWidth: number;
  contentPixelHeight: number;
  contentOffsetX: number;
  contentOffsetY: number;
  frameId?: number;
};

export type TerminalImageManagerLike = {
  queue: (request: NativeImageRequest) => void;
  clear: () => void;
};

type TerminalWriter = (chunk: string) => unknown;

export function selectNativeImageProtocol(
  mode: ImagePreviewMode,
  renderer: ImagePreviewRenderer,
  capabilities: Partial<ImageProtocolCapabilities> | undefined,
  resolution: PixelResolution | null | undefined,
): NativeImageProtocol | null {
  if (mode === "metadata") return null;
  if (renderer === "text" || renderer === "opentui") return null;
  if (!resolution || resolution.width <= 0 || resolution.height <= 0) return null;
  if (mode === "kitty") return capabilities?.kittyGraphics === true ? "kitty" : null;
  if (mode === "sixel") return capabilities?.sixel === true ? "sixel" : null;
  if (capabilities?.kittyGraphics === true) return "kitty";
  if (capabilities?.sixel === true) return "sixel";
  return null;
}

export function computeNativeImagePlacement(options: {
  imageWidth: number;
  imageHeight: number;
  maxCols: number;
  maxRows: number;
  contentWidth: number;
  terminalColumns: number;
  terminalRows: number;
  resolution: PixelResolution;
  align: ImagePreviewAlign;
  protocol: NativeImageProtocol;
}): NativeImagePlacement | null {
  const terminalColumns = Math.max(1, Math.floor(options.terminalColumns));
  const terminalRows = Math.max(1, Math.floor(options.terminalRows));
  const cellWidth = options.resolution.width / terminalColumns;
  const cellHeight = options.resolution.height / terminalRows;
  if (!Number.isFinite(cellWidth) || !Number.isFinite(cellHeight) || cellWidth <= 0 || cellHeight <= 0) return null;

  const maxCols = Math.max(1, Math.min(Math.floor(options.maxCols), Math.floor(options.contentWidth)));
  const maxRows = Math.max(1, Math.floor(options.maxRows));
  const maxPixelWidth = Math.max(1, Math.floor(maxCols * cellWidth));
  const maxPixelHeight = Math.max(1, Math.floor(maxRows * cellHeight));
  const scale = Math.min(1, maxPixelWidth / options.imageWidth, maxPixelHeight / options.imageHeight);
  const contentPixelWidth = Math.max(1, Math.floor(options.imageWidth * scale));
  const contentPixelHeight = Math.max(1, Math.floor(options.imageHeight * scale));
  const cols = Math.max(1, Math.min(maxCols, Math.ceil(contentPixelWidth / cellWidth)));
  const rows = Math.max(1, Math.min(maxRows, Math.ceil(contentPixelHeight / cellHeight)));
  const pixelWidth = Math.max(1, Math.round(cols * cellWidth));
  const pixelHeight = Math.max(1, Math.round(rows * cellHeight));
  const contentOffsetX = Math.max(0, Math.floor((pixelWidth - contentPixelWidth) / 2));
  const contentOffsetY = Math.max(0, Math.floor((pixelHeight - contentPixelHeight) / 2));
  const leftPad = imageAlignPadding(Math.max(maxCols, Math.floor(options.contentWidth)), cols, options.align);
  return { protocol: options.protocol, cols, rows, pixelWidth, pixelHeight, contentPixelWidth, contentPixelHeight, contentOffsetX, contentOffsetY, leftPad };
}

export class TerminalImageManager implements TerminalImageManagerLike {
  private pending: NativeImageRequest | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private lastFrameKey = "";
  private lastKittyPayloadKey: string | null = null;
  private lastKittyImageId: number | null = null;

  constructor(private readonly writer: TerminalWriter = (chunk) => process.stdout.write(chunk)) {}

  queue(request: NativeImageRequest): void {
    const frameKey = `${request.frameId ?? "unknown"}:${request.protocol}:${request.sourceKey}:${request.screenX},${request.screenY}:${request.cols}x${request.rows}:${request.pixelWidth}x${request.pixelHeight}:${request.contentPixelWidth}x${request.contentPixelHeight}:${request.contentOffsetX},${request.contentOffsetY}:${request.background}`;
    if (frameKey === this.lastFrameKey) return;
    this.lastFrameKey = frameKey;
    this.pending = request;
    if (this.timer) return;
    this.timer = setTimeout(() => {
      this.timer = null;
      this.flush();
    }, 0);
  }

  clear(): void {
    this.pending = null;
    this.lastFrameKey = "";
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    if (this.lastKittyImageId !== null) {
      this.writer(kittyCursorWrapped(1, 1, `${kittyDeleteVisibleImages()}${kittyDeleteImage(this.lastKittyImageId, true)}`));
      this.lastKittyImageId = null;
      this.lastKittyPayloadKey = null;
    }
  }

  destroy(): void {
    this.clear();
  }

  private flush(): void {
    const request = this.pending;
    this.pending = null;
    if (!request) return;

    const pixels = renderContainedRgba(request);
    if (request.protocol === "kitty") {
      this.writer(this.kittySequence(request, pixels));
      return;
    }
    this.writer(
      cursorWrapped(
        request.screenY + 1,
        request.screenX + 1,
        sixelEncodeRgba(pixels, request.pixelWidth, request.pixelHeight),
      ),
    );
  }

  private kittySequence(request: NativeImageRequest, pixels: Uint8Array): string {
    const payloadKey = `${request.sourceKey}:${request.pixelWidth}x${request.pixelHeight}:${request.background}`;
    const imageId = stableImageId(payloadKey);
    const transmit = payloadKey === this.lastKittyPayloadKey ? "" : kittyTransmitRgba(imageId, pixels, request.pixelWidth, request.pixelHeight);
    const deleteOld = this.lastKittyImageId !== null && this.lastKittyImageId !== imageId ? kittyDeleteImage(this.lastKittyImageId, true) : "";
    this.lastKittyPayloadKey = payloadKey;
    this.lastKittyImageId = imageId;
    return cursorWrapped(
      request.screenY + 1,
      request.screenX + 1,
      `${kittyDeleteVisibleImages()}${deleteOld}${transmit}${kittyPlaceImage(imageId, request.cols, request.rows)}`,
    );
  }
}

function renderContainedRgba(request: NativeImageRequest): Uint8Array {
  const bg = parseHexColor(request.background);
  const pixels = new Uint8Array(request.pixelWidth * request.pixelHeight * 4);
  for (let offset = 0; offset < pixels.length; offset += 4) {
    pixels[offset] = bg.r;
    pixels[offset + 1] = bg.g;
    pixels[offset + 2] = bg.b;
    pixels[offset + 3] = 255;
  }

  const scaled = scaleImageToRgba(request.image, request.contentPixelWidth, request.contentPixelHeight, request.background);
  for (let y = 0; y < request.contentPixelHeight; y += 1) {
    const targetY = request.contentOffsetY + y;
    if (targetY < 0 || targetY >= request.pixelHeight) continue;
    for (let x = 0; x < request.contentPixelWidth; x += 1) {
      const targetX = request.contentOffsetX + x;
      if (targetX < 0 || targetX >= request.pixelWidth) continue;
      const source = (y * request.contentPixelWidth + x) * 4;
      const target = (targetY * request.pixelWidth + targetX) * 4;
      pixels[target] = scaled[source]!;
      pixels[target + 1] = scaled[source + 1]!;
      pixels[target + 2] = scaled[source + 2]!;
      pixels[target + 3] = scaled[source + 3]!;
    }
  }
  return pixels;
}

export function kittyTransmitRgba(imageId: number, pixels: Uint8Array, width: number, height: number): string {
  const payload = Buffer.from(pixels).toString("base64");
  const chunks = payload.match(/.{1,4096}/g) ?? [""];
  return chunks
    .map((chunk, index) => {
      const more = index < chunks.length - 1 ? 1 : 0;
      const options =
        index === 0
          ? `a=t,f=32,s=${width},v=${height},i=${imageId},m=${more},q=2`
          : `m=${more},q=2`;
      return `\x1b_G${options};${chunk}\x1b\\`;
    })
    .join("");
}

export function kittyPlaceImage(imageId: number, cols: number, rows: number): string {
  return `\x1b_Ga=p,i=${imageId},p=1,c=${cols},r=${rows},C=1,q=2;\x1b\\`;
}

export function kittyDeleteVisibleImages(): string {
  return "\x1b_Ga=d,q=2;\x1b\\";
}

export function kittyDeleteImage(imageId: number, freeData = false): string {
  return `\x1b_Ga=d,d=${freeData ? "I" : "i"},i=${imageId},q=2;\x1b\\`;
}

export function sixelEncodeRgba(pixels: Uint8Array, width: number, height: number): string {
  const colorIndexes = new Uint16Array(width * height);
  const used = new Set<number>();
  for (let index = 0; index < width * height; index += 1) {
    const source = index * 4;
    const colorIndex = sixelPaletteIndex(pixels[source]!, pixels[source + 1]!, pixels[source + 2]!);
    colorIndexes[index] = colorIndex;
    used.add(colorIndex);
  }

  const defined = new Set<number>();
  let out = "\x1bPq";
  for (let bandY = 0; bandY < height; bandY += 6) {
    const bandColors = colorsInBand(colorIndexes, width, height, bandY);
    for (const colorIndex of bandColors) {
      if (!defined.has(colorIndex)) {
        out += sixelColorDefinition(colorIndex);
        defined.add(colorIndex);
      } else {
        out += `#${colorIndex}`;
      }
      out += sixelBandRun(colorIndexes, width, height, bandY, colorIndex);
      out += "$";
    }
    out += "-";
  }
  return `${out}\x1b\\`;
}

function sixelBandRun(colorIndexes: Uint16Array, width: number, height: number, bandY: number, colorIndex: number): string {
  let out = "";
  let lastChar = "";
  let runLength = 0;
  const flush = () => {
    if (!lastChar) return;
    out += runLength >= 4 ? `!${runLength}${lastChar}` : lastChar.repeat(runLength);
    lastChar = "";
    runLength = 0;
  };

  for (let x = 0; x < width; x += 1) {
    let bits = 0;
    for (let bit = 0; bit < 6; bit += 1) {
      const y = bandY + bit;
      if (y >= height) continue;
      if (colorIndexes[y * width + x] === colorIndex) bits |= 1 << bit;
    }
    const char = String.fromCharCode(63 + bits);
    if (char === lastChar) {
      runLength += 1;
    } else {
      flush();
      lastChar = char;
      runLength = 1;
    }
  }
  flush();
  return out;
}

function colorsInBand(colorIndexes: Uint16Array, width: number, height: number, bandY: number): number[] {
  const colors = new Set<number>();
  for (let y = bandY; y < Math.min(height, bandY + 6); y += 1) {
    for (let x = 0; x < width; x += 1) colors.add(colorIndexes[y * width + x]!);
  }
  return [...colors].sort((left, right) => left - right);
}

function sixelPaletteIndex(r: number, g: number, b: number): number {
  const red = Math.round(r / 51);
  const green = Math.round(g / 51);
  const blue = Math.round(b / 51);
  return red * 36 + green * 6 + blue;
}

function sixelColorDefinition(colorIndex: number): string {
  const red = Math.floor(colorIndex / 36);
  const green = Math.floor((colorIndex % 36) / 6);
  const blue = colorIndex % 6;
  return `#${colorIndex};2;${Math.round((red / 5) * 100)};${Math.round((green / 5) * 100)};${Math.round((blue / 5) * 100)}`;
}

function stableImageId(key: string): number {
  let hash = 2166136261;
  for (let index = 0; index < key.length; index += 1) {
    hash ^= key.charCodeAt(index);
    hash = Math.imul(hash, 16777619) >>> 0;
  }
  return (hash & 0x7fffffff) || 1;
}

function imageAlignPadding(width: number, imageWidth: number, align: ImagePreviewAlign): number {
  const extra = Math.max(0, Math.floor(width) - Math.floor(imageWidth));
  if (align === "right") return extra;
  if (align === "center") return Math.floor(extra / 2);
  return 0;
}

function kittyCursorWrapped(row: number, col: number, sequence: string): string {
  return cursorWrapped(row, col, sequence);
}

function cursorWrapped(row: number, col: number, sequence: string): string {
  return `\x1b7\x1b[${Math.max(1, Math.floor(row))};${Math.max(1, Math.floor(col))}H${sequence}\x1b8`;
}

function parseHexColor(value: string): { r: number; g: number; b: number } {
  const match = /^#([0-9a-f]{6})$/i.exec(value.trim());
  if (!match) return { r: 0, g: 0, b: 0 };
  const int = Number.parseInt(match[1]!, 16);
  return { r: (int >> 16) & 0xff, g: (int >> 8) & 0xff, b: int & 0xff };
}
