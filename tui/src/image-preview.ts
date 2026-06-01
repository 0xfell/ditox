import { readFileSync } from "node:fs";
import { inflateSync } from "node:zlib";
import { decode as decodeWebp } from "@jsquash/webp";
import * as jpeg from "jpeg-js";
import type { ImagePreviewMode } from "./ui-config";
import type { Entry } from "./types";
import type { TuiLabels } from "./tui-config";

export type Rgba = { r: number; g: number; b: number; a: number };

export type DecodedImage = {
  width: number;
  height: number;
  pixels: Uint8Array;
};

export type DecodedPng = DecodedImage;

export type ImageCell = {
  char: string;
  fg: string;
  bg: string;
};

export type ImageNativeBuffer = {
  cols: number;
  cellRows: number;
  pixelRows: number;
  pixels: Uint8Array;
  alignedBytesPerRow: number;
};

export type ImagePreviewProtocol = "kitty" | "sixel";

export type ImageProtocolCapabilities = {
  kittyGraphics: boolean | null;
  sixel: boolean | null;
  nativeRenderer: boolean;
};

export type ImageBlockPreview =
  | {
      kind: "rendered";
      rows: ImageCell[][];
      native: ImageNativeBuffer;
      image: DecodedImage;
      bounds: { maxCols: number; maxRows: number };
      source: string;
      notice: string | null;
      protocol: ImagePreviewProtocol | null;
    }
  | { kind: "fallback"; reason: string };

export type ImagePreviewFallbackLabels = Pick<
  TuiLabels,
  | "imagePreviewNotImage"
  | "imagePreviewBlocksDisabled"
  | "imagePreviewBlobMissing"
  | "imagePreviewDecodePending"
  | "imagePreviewUnsupportedMime"
  | "imagePreviewUnsupportedBytes"
  | "imagePreviewDecodeFailed"
  | "imagePreviewBlocksSource"
  | "imagePreviewKittyFallbackSource"
  | "imagePreviewSixelFallbackSource"
  | "imagePreviewKittyProtocolName"
  | "imagePreviewSixelProtocolName"
  | "imagePreviewProtocolUnknown"
  | "imagePreviewProtocolUnsupported"
  | "imagePreviewProtocolRendererUnavailable"
>;

const pngSignature = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]);
const previewCache = new Map<string, ImageBlockPreview>();
const asyncPreviewCache = new Map<string, ImageBlockPreview | Promise<ImageBlockPreview>>();
const defaultFallbackLabels: ImagePreviewFallbackLabels = {
  imagePreviewNotImage: "not an image entry",
  imagePreviewBlocksDisabled: "image blocks disabled",
  imagePreviewBlobMissing: "image blob is not stored",
  imagePreviewDecodePending: "decoding image preview",
  imagePreviewUnsupportedMime: "{mime} block preview is not supported yet",
  imagePreviewUnsupportedBytes: "unsupported image bytes",
  imagePreviewDecodeFailed: "{error}",
  imagePreviewBlocksSource: "image blocks",
  imagePreviewKittyFallbackSource: "kitty fallback blocks",
  imagePreviewSixelFallbackSource: "sixel fallback blocks",
  imagePreviewKittyProtocolName: "Kitty",
  imagePreviewSixelProtocolName: "Sixel",
  imagePreviewProtocolUnknown: "{protocol} support unknown; showing block fallback",
  imagePreviewProtocolUnsupported: "{protocol} not detected; showing block fallback",
  imagePreviewProtocolRendererUnavailable: "{protocol} detected; native renderer unavailable; showing block fallback",
};

export function imageBlockPreview(
  entry: Entry | undefined,
  maxWidth: number,
  maxRows: number,
  background: string,
  mode: ImagePreviewMode,
  labels?: Partial<ImagePreviewFallbackLabels>,
  glyph = "▀",
  capabilities?: Partial<ImageProtocolCapabilities>,
): ImageBlockPreview {
  const text = fallbackLabels(labels);
  if (!entry || entry.kind !== "image") return { kind: "fallback", reason: text.imagePreviewNotImage };
  if (mode === "metadata") return { kind: "fallback", reason: text.imagePreviewBlocksDisabled };
  if (!entry.blob_path) return { kind: "fallback", reason: text.imagePreviewBlobMissing };
  if (!isBlockPreviewMime(entry.mime)) return { kind: "fallback", reason: unsupportedMimeReason(entry.mime, text) };
  if (isAsyncBlockPreviewMime(entry.mime)) return { kind: "fallback", reason: text.imagePreviewDecodePending };

  const width = Math.max(1, Math.floor(maxWidth));
  const rows = Math.max(1, Math.floor(maxRows));
  const notice = imagePreviewProtocolNotice(mode, capabilities, text);
  const cacheKey = `${entry.blob_path}:${width}:${rows}:${background}:${glyph}:${fallbackLabelKey(text)}:${notice ?? ""}`;
  const cached = previewCache.get(cacheKey);
  if (cached) return cached;

  let result: ImageBlockPreview;
  try {
    result = imageBlockPreviewFromBytes(readFileSync(entry.blob_path), width, rows, background, entry.mime, text, glyph, previewSourcePrefix(mode, text), notice, imagePreviewProtocol(mode));
  } catch (error) {
    result = { kind: "fallback", reason: decodeErrorReason(error, text) };
  }
  previewCache.set(cacheKey, result);
  return result;
}

export async function imageBlockPreviewAsync(
  entry: Entry | undefined,
  maxWidth: number,
  maxRows: number,
  background: string,
  mode: ImagePreviewMode,
  labels?: Partial<ImagePreviewFallbackLabels>,
  glyph = "▀",
  capabilities?: Partial<ImageProtocolCapabilities>,
): Promise<ImageBlockPreview> {
  const text = fallbackLabels(labels);
  if (!entry || entry.kind !== "image") return { kind: "fallback", reason: text.imagePreviewNotImage };
  if (mode === "metadata") return { kind: "fallback", reason: text.imagePreviewBlocksDisabled };
  if (!entry.blob_path) return { kind: "fallback", reason: text.imagePreviewBlobMissing };
  if (!isBlockPreviewMime(entry.mime)) return { kind: "fallback", reason: unsupportedMimeReason(entry.mime, text) };

  const width = Math.max(1, Math.floor(maxWidth));
  const rows = Math.max(1, Math.floor(maxRows));
  const notice = imagePreviewProtocolNotice(mode, capabilities, text);
  const cacheKey = `${entry.blob_path}:${width}:${rows}:${background}:${glyph}:${fallbackLabelKey(text)}:${notice ?? ""}:async`;
  const cached = asyncPreviewCache.get(cacheKey) ?? previewCache.get(cacheKey.replace(/:async$/, ""));
  if (cached) return await cached;

  const pending = (async () => {
    let result: ImageBlockPreview;
    try {
      result = await imageBlockPreviewFromBytesAsync(
        readFileSync(entry.blob_path!),
        width,
        rows,
        background,
        entry.mime,
        text,
        glyph,
        previewSourcePrefix(mode, text),
        notice,
        imagePreviewProtocol(mode),
      );
    } catch (error) {
      result = { kind: "fallback", reason: decodeErrorReason(error, text) };
    }
    asyncPreviewCache.set(cacheKey, result);
    return result;
  })();
  asyncPreviewCache.set(cacheKey, pending);
  return await pending;
}

export function shouldLoadImageBlockPreviewAsync(entry: Entry | undefined, mode: ImagePreviewMode): boolean {
  return Boolean(entry && entry.kind === "image" && mode !== "metadata" && entry.blob_path && isAsyncBlockPreviewMime(entry.mime));
}

export function imageBlockPreviewFromBytes(
  bytes: Uint8Array,
  maxWidth: number,
  maxRows: number,
  background = "#000000",
  mime = "auto",
  labels?: Partial<ImagePreviewFallbackLabels>,
  glyph = "▀",
  sourcePrefix = "image blocks",
  notice: string | null = null,
  protocol: ImagePreviewProtocol | null = null,
): ImageBlockPreview {
  const text = fallbackLabels(labels);
  try {
    return renderImageBlocks(decodeImage(bytes, mime), maxWidth, maxRows, background, glyph, sourcePrefix, notice, protocol);
  } catch (error) {
    return { kind: "fallback", reason: imagePreviewErrorReason(error, mime, text) };
  }
}

export async function imageBlockPreviewFromBytesAsync(
  bytes: Uint8Array,
  maxWidth: number,
  maxRows: number,
  background = "#000000",
  mime = "auto",
  labels?: Partial<ImagePreviewFallbackLabels>,
  glyph = "▀",
  sourcePrefix = "image blocks",
  notice: string | null = null,
  protocol: ImagePreviewProtocol | null = null,
): Promise<ImageBlockPreview> {
  const text = fallbackLabels(labels);
  try {
    return renderImageBlocks(await decodeImageAsync(bytes, mime), maxWidth, maxRows, background, glyph, sourcePrefix, notice, protocol);
  } catch (error) {
    return { kind: "fallback", reason: imagePreviewErrorReason(error, mime, text) };
  }
}

export function decodeImage(bytes: Uint8Array, mime = "auto"): DecodedImage {
  if (mime === "image/png") return decodePng(bytes);
  if (mime === "image/bmp") return decodeBmp(bytes);
  if (mime === "image/gif") return decodeGif(bytes);
  if (mime === "image/jpeg" || mime === "image/jpg") return decodeJpegImage(bytes);
  if (mime === "auto") {
    if (isPng(bytes)) return decodePng(bytes);
    if (isBmp(bytes)) return decodeBmp(bytes);
    if (isGif(bytes)) return decodeGif(bytes);
    if (isJpeg(bytes)) return decodeJpegImage(bytes);
  }
  throw new Error(mime === "auto" ? "unsupported image bytes" : `${mime} block preview is not supported yet`);
}

export async function decodeImageAsync(bytes: Uint8Array, mime = "auto"): Promise<DecodedImage> {
  if (mime === "image/webp") return await decodeWebpImage(bytes);
  if (mime === "auto" && isWebp(bytes)) return await decodeWebpImage(bytes);
  return decodeImage(bytes, mime);
}

export async function decodeWebpImage(bytes: Uint8Array): Promise<DecodedImage> {
  if (!isWebp(bytes)) throw new Error("not a WebP file");
  const buffer = Uint8Array.from(bytes).buffer;
  const decoded = (await decodeWebp(buffer)) as { width: number; height: number; data: Uint8Array | Uint8ClampedArray };
  if (decoded.width <= 0 || decoded.height <= 0) throw new Error("WebP has invalid dimensions");
  if (decoded.data.length < decoded.width * decoded.height * 4) throw new Error("truncated WebP pixels");
  return { width: decoded.width, height: decoded.height, pixels: Uint8Array.from(decoded.data) };
}

export function imagePreviewProtocolNotice(
  mode: ImagePreviewMode,
  capabilities?: Partial<ImageProtocolCapabilities>,
  labels?: Partial<ImagePreviewFallbackLabels>,
): string | null {
  const protocol = imagePreviewProtocol(mode);
  if (!protocol) return null;
  const text = fallbackLabels(labels);
  const protocolName = protocol === "kitty" ? text.imagePreviewKittyProtocolName : text.imagePreviewSixelProtocolName;
  const supported = protocol === "kitty" ? capabilities?.kittyGraphics : capabilities?.sixel;
  if (supported === false) return text.imagePreviewProtocolUnsupported.replaceAll("{protocol}", protocolName);
  if (supported === true && capabilities?.nativeRenderer === true) return null;
  if (supported === true) return text.imagePreviewProtocolRendererUnavailable.replaceAll("{protocol}", protocolName);
  return text.imagePreviewProtocolUnknown.replaceAll("{protocol}", protocolName);
}

function imagePreviewErrorReason(error: unknown, mime: string, labels: ImagePreviewFallbackLabels): string {
  const message = error instanceof Error ? error.message : String(error);
  if (message === "unsupported image bytes") return labels.imagePreviewUnsupportedBytes;
  if (message.endsWith(" block preview is not supported yet")) return unsupportedMimeReason(mime, labels);
  return decodeErrorReason(error, labels);
}

function decodeErrorReason(error: unknown, labels: ImagePreviewFallbackLabels): string {
  const message = error instanceof Error ? error.message : String(error);
  return labels.imagePreviewDecodeFailed.replaceAll("{error}", message);
}

function unsupportedMimeReason(mime: string, labels: ImagePreviewFallbackLabels): string {
  return labels.imagePreviewUnsupportedMime.replaceAll("{mime}", mime);
}

function fallbackLabels(labels: Partial<ImagePreviewFallbackLabels> | undefined): ImagePreviewFallbackLabels {
  return { ...defaultFallbackLabels, ...labels };
}

function fallbackLabelKey(labels: ImagePreviewFallbackLabels): string {
  return JSON.stringify(labels);
}

export function decodePng(bytes: Uint8Array): DecodedPng {
  if (!isPng(bytes)) {
    throw new Error("not a PNG file");
  }

  let offset = pngSignature.length;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let interlace = 0;
  let palette: Uint8Array | null = null;
  let transparency: Uint8Array | null = null;
  const idatChunks: Uint8Array[] = [];

  while (offset + 12 <= bytes.length) {
    const length = readU32(bytes, offset);
    offset += 4;
    const type = String.fromCharCode(bytes[offset]!, bytes[offset + 1]!, bytes[offset + 2]!, bytes[offset + 3]!);
    offset += 4;
    if (offset + length + 4 > bytes.length) throw new Error("truncated PNG chunk");
    const data = bytes.subarray(offset, offset + length);
    offset += length + 4;

    if (type === "IHDR") {
      width = readU32(data, 0);
      height = readU32(data, 4);
      bitDepth = data[8]!;
      colorType = data[9]!;
      interlace = data[12]!;
    } else if (type === "PLTE") {
      palette = Uint8Array.from(data);
    } else if (type === "tRNS") {
      transparency = Uint8Array.from(data);
    } else if (type === "IDAT") {
      idatChunks.push(Uint8Array.from(data));
    } else if (type === "IEND") {
      break;
    }
  }

  if (width <= 0 || height <= 0) throw new Error("PNG is missing IHDR");
  if (bitDepth !== 8) throw new Error(`unsupported PNG bit depth ${bitDepth}`);
  if (interlace !== 0) throw new Error("interlaced PNG preview is not supported");
  if (idatChunks.length === 0) throw new Error("PNG is missing image data");

  const bpp = bytesPerPixel(colorType);
  const stride = width * bpp;
  const inflated = inflateSync(Buffer.concat(idatChunks.map((chunk) => Buffer.from(chunk))));
  const raw = unfilterPng(inflated, width, height, bpp, stride);
  const pixels = pngScanlinesToRgba(raw, width, height, colorType, palette, transparency);
  return { width, height, pixels };
}

export function decodeBmp(bytes: Uint8Array): DecodedImage {
  if (!isBmp(bytes)) throw new Error("not a BMP file");
  if (bytes.length < 54) throw new Error("truncated BMP header");

  const pixelOffset = readU32LE(bytes, 10);
  const dibSize = readU32LE(bytes, 14);
  if (dibSize < 40) throw new Error(`unsupported BMP DIB header ${dibSize}`);

  const width = readI32LE(bytes, 18);
  const signedHeight = readI32LE(bytes, 22);
  const height = Math.abs(signedHeight);
  const topDown = signedHeight < 0;
  const planes = readU16LE(bytes, 26);
  const bitDepth = readU16LE(bytes, 28);
  const compression = readU32LE(bytes, 30);
  if (width <= 0 || height <= 0) throw new Error("BMP has invalid dimensions");
  if (planes !== 1) throw new Error(`unsupported BMP plane count ${planes}`);
  if (compression !== 0) throw new Error("compressed BMP preview is not supported");
  if (bitDepth !== 24 && bitDepth !== 32) throw new Error(`unsupported BMP bit depth ${bitDepth}`);

  const bytesPerPixel = bitDepth / 8;
  const stride = Math.floor((bitDepth * width + 31) / 32) * 4;
  if (pixelOffset + stride * height > bytes.length) throw new Error("truncated BMP pixels");

  const pixels = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    const sourceY = topDown ? y : height - 1 - y;
    const sourceRow = pixelOffset + sourceY * stride;
    for (let x = 0; x < width; x += 1) {
      const source = sourceRow + x * bytesPerPixel;
      const target = (y * width + x) * 4;
      pixels[target] = bytes[source + 2]!;
      pixels[target + 1] = bytes[source + 1]!;
      pixels[target + 2] = bytes[source]!;
      pixels[target + 3] = 255;
    }
  }
  return { width, height, pixels };
}

export function decodeGif(bytes: Uint8Array): DecodedImage {
  if (!isGif(bytes)) throw new Error("not a GIF file");
  if (bytes.length < 13) throw new Error("truncated GIF header");

  const screenWidth = readU16LE(bytes, 6);
  const screenHeight = readU16LE(bytes, 8);
  if (screenWidth <= 0 || screenHeight <= 0) throw new Error("GIF has invalid dimensions");

  const packed = bytes[10]!;
  const hasGlobalColorTable = (packed & 0x80) !== 0;
  const globalColorCount = 1 << ((packed & 0x07) + 1);
  let offset = 13;
  let globalColorTable: Uint8Array | null = null;
  if (hasGlobalColorTable) {
    const byteLength = globalColorCount * 3;
    if (offset + byteLength > bytes.length) throw new Error("truncated GIF color table");
    globalColorTable = bytes.subarray(offset, offset + byteLength);
    offset += byteLength;
  }

  let transparentIndex: number | null = null;
  while (offset < bytes.length) {
    const block = bytes[offset++]!;
    if (block === 0x3b) break;

    if (block === 0x21) {
      const label = bytes[offset++]!;
      if (label === 0xf9) {
        if (offset >= bytes.length) throw new Error("truncated GIF graphic control extension");
        const blockSize = bytes[offset++]!;
        if (blockSize !== 4 || offset + 5 > bytes.length) throw new Error("invalid GIF graphic control extension");
        const gcePacked = bytes[offset]!;
        transparentIndex = (gcePacked & 0x01) !== 0 ? bytes[offset + 3]! : null;
        offset += 4;
        if (bytes[offset++] !== 0) throw new Error("unterminated GIF graphic control extension");
      } else {
        offset = skipGifSubBlocks(bytes, offset);
      }
      continue;
    }

    if (block !== 0x2c) throw new Error(`unsupported GIF block 0x${block.toString(16)}`);
    if (offset + 9 > bytes.length) throw new Error("truncated GIF image descriptor");

    const imageLeft = readU16LE(bytes, offset);
    const imageTop = readU16LE(bytes, offset + 2);
    const width = readU16LE(bytes, offset + 4);
    const height = readU16LE(bytes, offset + 6);
    const imagePacked = bytes[offset + 8]!;
    offset += 9;

    if (width <= 0 || height <= 0) throw new Error("GIF image has invalid dimensions");
    const hasLocalColorTable = (imagePacked & 0x80) !== 0;
    const interlaced = (imagePacked & 0x40) !== 0;
    const localColorCount = 1 << ((imagePacked & 0x07) + 1);
    let colorTable = globalColorTable;
    if (hasLocalColorTable) {
      const byteLength = localColorCount * 3;
      if (offset + byteLength > bytes.length) throw new Error("truncated GIF local color table");
      colorTable = bytes.subarray(offset, offset + byteLength);
      offset += byteLength;
    }
    if (!colorTable) throw new Error("GIF is missing a color table");
    if (offset >= bytes.length) throw new Error("truncated GIF image data");

    const minCodeSize = bytes[offset++]!;
    const imageData = readGifSubBlocks(bytes, offset);
    offset = imageData.nextOffset;
    const indices = decodeGifLzw(imageData.bytes, minCodeSize, width * height);
    const pixels = gifIndicesToRgba(indices, width, height, colorTable, transparentIndex, interlaced);
    if (imageLeft !== 0 || imageTop !== 0) return { width, height, pixels };
    return { width, height, pixels };
  }

  throw new Error("GIF is missing image data");
}

export function decodeJpegImage(bytes: Uint8Array): DecodedImage {
  if (!isJpeg(bytes)) throw new Error("not a JPEG file");
  const decoded = jpeg.decode(Buffer.from(bytes), {
    useTArray: true,
    formatAsRGBA: true,
    maxResolutionInMP: 32,
    maxMemoryUsageInMB: 128,
  });
  if (decoded.width <= 0 || decoded.height <= 0) throw new Error("JPEG has invalid dimensions");
  if (decoded.data.length < decoded.width * decoded.height * 4) throw new Error("truncated JPEG pixels");
  return { width: decoded.width, height: decoded.height, pixels: Uint8Array.from(decoded.data) };
}

export function renderImageBlocks(
  image: DecodedImage,
  maxWidth: number,
  maxRows: number,
  background = "#000000",
  glyph = "▀",
  sourcePrefix = "image blocks",
  notice: string | null = null,
  protocol: ImagePreviewProtocol | null = null,
): ImageBlockPreview {
  const bg = parseHexColor(background) ?? { r: 0, g: 0, b: 0, a: 255 };
  const cellGlyph = Array.from(glyph)[0] ?? "▀";
  const target = targetSize(image.width, image.height, maxWidth, maxRows);
  const native = nativeBuffer(image, target, bg);
  const rows: ImageCell[][] = [];
  for (let row = 0; row < target.cellRows; row += 1) {
    const cells: ImageCell[] = [];
    for (let col = 0; col < target.cols; col += 1) {
      const top = averagePixel(image, col, row * 2, target.cols, target.pixelRows, bg);
      const bottom = averagePixel(image, col, row * 2 + 1, target.cols, target.pixelRows, bg);
      cells.push({ char: cellGlyph, fg: rgbToHex(top), bg: rgbToHex(bottom) });
    }
    rows.push(cells);
  }
  return {
    kind: "rendered",
    rows,
    native,
    image,
    bounds: { maxCols: Math.max(1, Math.floor(maxWidth)), maxRows: Math.max(1, Math.floor(maxRows)) },
    source: `${sourcePrefix} ${target.cols}x${target.cellRows}`,
    notice,
    protocol,
  };
}

function imagePreviewProtocol(mode: ImagePreviewMode): ImagePreviewProtocol | null {
  if (mode === "kitty" || mode === "sixel") return mode;
  return null;
}

function previewSourcePrefix(mode: ImagePreviewMode, labels: ImagePreviewFallbackLabels): string {
  if (mode === "kitty") return labels.imagePreviewKittyFallbackSource;
  if (mode === "sixel") return labels.imagePreviewSixelFallbackSource;
  return labels.imagePreviewBlocksSource;
}

export function isBlockPreviewMime(mime: string): boolean {
  return (
    mime === "image/png" ||
    mime === "image/bmp" ||
    mime === "image/gif" ||
    mime === "image/jpeg" ||
    mime === "image/jpg" ||
    mime === "image/webp"
  );
}

function isAsyncBlockPreviewMime(mime: string): boolean {
  return mime === "image/webp";
}

function isPng(bytes: Uint8Array): boolean {
  return bytes.length >= pngSignature.length && pngSignature.every((byte, index) => bytes[index] === byte);
}

function isBmp(bytes: Uint8Array): boolean {
  return bytes.length >= 2 && bytes[0] === 0x42 && bytes[1] === 0x4d;
}

function isGif(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 6 &&
    bytes[0] === 0x47 &&
    bytes[1] === 0x49 &&
    bytes[2] === 0x46 &&
    bytes[3] === 0x38 &&
    (bytes[4] === 0x37 || bytes[4] === 0x39) &&
    bytes[5] === 0x61
  );
}

function isJpeg(bytes: Uint8Array): boolean {
  return bytes.length >= 4 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[bytes.length - 2] === 0xff && bytes[bytes.length - 1] === 0xd9;
}

function isWebp(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 12 &&
    bytes[0] === 0x52 &&
    bytes[1] === 0x49 &&
    bytes[2] === 0x46 &&
    bytes[3] === 0x46 &&
    bytes[8] === 0x57 &&
    bytes[9] === 0x45 &&
    bytes[10] === 0x42 &&
    bytes[11] === 0x50
  );
}

function readU32(bytes: Uint8Array, offset: number): number {
  return ((bytes[offset]! << 24) | (bytes[offset + 1]! << 16) | (bytes[offset + 2]! << 8) | bytes[offset + 3]!) >>> 0;
}

function readU16LE(bytes: Uint8Array, offset: number): number {
  return bytes[offset]! | (bytes[offset + 1]! << 8);
}

function readU32LE(bytes: Uint8Array, offset: number): number {
  return (bytes[offset]! | (bytes[offset + 1]! << 8) | (bytes[offset + 2]! << 16) | (bytes[offset + 3]! << 24)) >>> 0;
}

function readI32LE(bytes: Uint8Array, offset: number): number {
  return readU32LE(bytes, offset) | 0;
}

function skipGifSubBlocks(bytes: Uint8Array, offset: number): number {
  while (offset < bytes.length) {
    const length = bytes[offset++]!;
    if (length === 0) return offset;
    offset += length;
    if (offset > bytes.length) throw new Error("truncated GIF sub-block");
  }
  throw new Error("unterminated GIF sub-blocks");
}

function readGifSubBlocks(bytes: Uint8Array, offset: number): { bytes: Uint8Array; nextOffset: number } {
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (offset < bytes.length) {
    const length = bytes[offset++]!;
    if (length === 0) {
      const out = new Uint8Array(total);
      let cursor = 0;
      for (const chunk of chunks) {
        out.set(chunk, cursor);
        cursor += chunk.length;
      }
      return { bytes: out, nextOffset: offset };
    }
    if (offset + length > bytes.length) throw new Error("truncated GIF sub-block");
    const chunk = bytes.subarray(offset, offset + length);
    chunks.push(chunk);
    total += chunk.length;
    offset += length;
  }
  throw new Error("unterminated GIF sub-blocks");
}

function decodeGifLzw(bytes: Uint8Array, minCodeSize: number, expectedPixels: number): Uint8Array {
  if (minCodeSize < 2 || minCodeSize > 8) throw new Error(`unsupported GIF LZW code size ${minCodeSize}`);
  const clearCode = 1 << minCodeSize;
  const endCode = clearCode + 1;
  let codeSize = minCodeSize + 1;
  let nextCode = endCode + 1;
  let dict = initialGifDictionary(clearCode);
  let bitOffset = 0;
  let previous: number[] | null = null;
  const out: number[] = [];

  while (bitOffset + codeSize <= bytes.length * 8) {
    const code = readGifCode(bytes, bitOffset, codeSize);
    bitOffset += codeSize;

    if (code === clearCode) {
      dict = initialGifDictionary(clearCode);
      codeSize = minCodeSize + 1;
      nextCode = endCode + 1;
      previous = null;
      continue;
    }
    if (code === endCode) break;

    let entry = dict[code];
    if (!entry && previous && code === nextCode) entry = [...previous, previous[0]!];
    if (!entry) throw new Error("invalid GIF LZW code");
    out.push(...entry);

    if (previous && nextCode < 4096) {
      dict[nextCode++] = [...previous, entry[0]!];
      if (nextCode >= 1 << codeSize && codeSize < 12) codeSize += 1;
    }
    previous = entry;
    if (out.length >= expectedPixels) break;
  }

  if (out.length < expectedPixels) throw new Error("truncated GIF pixels");
  return Uint8Array.from(out.slice(0, expectedPixels));
}

function initialGifDictionary(clearCode: number): Array<number[] | undefined> {
  const dict: Array<number[] | undefined> = [];
  for (let index = 0; index < clearCode; index += 1) dict[index] = [index];
  return dict;
}

function readGifCode(bytes: Uint8Array, bitOffset: number, size: number): number {
  let code = 0;
  for (let bit = 0; bit < size; bit += 1) {
    const absolute = bitOffset + bit;
    const byte = bytes[Math.floor(absolute / 8)]!;
    code |= ((byte >> (absolute % 8)) & 1) << bit;
  }
  return code;
}

function gifIndicesToRgba(
  indices: Uint8Array,
  width: number,
  height: number,
  colorTable: Uint8Array,
  transparentIndex: number | null,
  interlaced: boolean,
): Uint8Array {
  const pixels = new Uint8Array(width * height * 4);
  let source = 0;
  const writePixel = (x: number, y: number) => {
    const colorIndex = indices[source++]!;
    const colorOffset = colorIndex * 3;
    if (colorOffset + 2 >= colorTable.length) throw new Error("GIF color index is out of range");
    const target = (y * width + x) * 4;
    pixels[target] = colorTable[colorOffset]!;
    pixels[target + 1] = colorTable[colorOffset + 1]!;
    pixels[target + 2] = colorTable[colorOffset + 2]!;
    pixels[target + 3] = transparentIndex === colorIndex ? 0 : 255;
  };

  if (!interlaced) {
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) writePixel(x, y);
    }
    return pixels;
  }

  const starts = [0, 4, 2, 1];
  const steps = [8, 8, 4, 2];
  for (let pass = 0; pass < starts.length; pass += 1) {
    for (let y = starts[pass]!; y < height; y += steps[pass]!) {
      for (let x = 0; x < width; x += 1) writePixel(x, y);
    }
  }
  return pixels;
}

function bytesPerPixel(colorType: number): number {
  switch (colorType) {
    case 0:
    case 3:
      return 1;
    case 4:
      return 2;
    case 2:
      return 3;
    case 6:
      return 4;
    default:
      throw new Error(`unsupported PNG color type ${colorType}`);
  }
}

function unfilterPng(inflated: Uint8Array, width: number, height: number, bpp: number, stride: number): Uint8Array {
  const expected = height * (stride + 1);
  if (inflated.length < expected) throw new Error("truncated PNG scanlines");
  const out = new Uint8Array(height * stride);
  let input = 0;
  for (let y = 0; y < height; y += 1) {
    const filter = inflated[input++]!;
    const rowStart = y * stride;
    const prevStart = rowStart - stride;
    for (let x = 0; x < stride; x += 1) {
      const raw = inflated[input++]!;
      const left = x >= bpp ? out[rowStart + x - bpp]! : 0;
      const up = y > 0 ? out[prevStart + x]! : 0;
      const upLeft = y > 0 && x >= bpp ? out[prevStart + x - bpp]! : 0;
      out[rowStart + x] = unfilterByte(filter, raw, left, up, upLeft);
    }
  }
  return out;
}

function unfilterByte(filter: number, raw: number, left: number, up: number, upLeft: number): number {
  switch (filter) {
    case 0:
      return raw;
    case 1:
      return (raw + left) & 0xff;
    case 2:
      return (raw + up) & 0xff;
    case 3:
      return (raw + Math.floor((left + up) / 2)) & 0xff;
    case 4:
      return (raw + paeth(left, up, upLeft)) & 0xff;
    default:
      throw new Error(`unsupported PNG filter ${filter}`);
  }
}

function paeth(a: number, b: number, c: number): number {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  return pb <= pc ? b : c;
}

function pngScanlinesToRgba(
  raw: Uint8Array,
  width: number,
  height: number,
  colorType: number,
  palette: Uint8Array | null,
  transparency: Uint8Array | null,
): Uint8Array {
  const pixels = new Uint8Array(width * height * 4);
  let source = 0;
  let target = 0;
  for (let index = 0; index < width * height; index += 1) {
    if (colorType === 0) {
      const gray = raw[source++]!;
      pixels.set([gray, gray, gray, 255], target);
    } else if (colorType === 2) {
      pixels.set([raw[source++]!, raw[source++]!, raw[source++]!, 255], target);
    } else if (colorType === 3) {
      const paletteIndex = raw[source++]!;
      const paletteOffset = paletteIndex * 3;
      if (!palette || paletteOffset + 2 >= palette.length) throw new Error("PNG palette index is out of range");
      pixels.set([palette[paletteOffset]!, palette[paletteOffset + 1]!, palette[paletteOffset + 2]!, transparency?.[paletteIndex] ?? 255], target);
    } else if (colorType === 4) {
      const gray = raw[source++]!;
      pixels.set([gray, gray, gray, raw[source++]!], target);
    } else if (colorType === 6) {
      pixels.set([raw[source++]!, raw[source++]!, raw[source++]!, raw[source++]!], target);
    }
    target += 4;
  }
  return pixels;
}

function targetSize(imageWidth: number, imageHeight: number, maxWidth: number, maxRows: number): { cols: number; pixelRows: number; cellRows: number } {
  let cols = Math.max(1, Math.min(Math.floor(maxWidth), imageWidth));
  let pixelRows = Math.max(1, Math.round((cols * imageHeight) / imageWidth));
  if (Math.ceil(pixelRows / 2) > maxRows) {
    pixelRows = Math.max(1, Math.floor(maxRows) * 2);
    cols = Math.max(1, Math.min(Math.floor((pixelRows * imageWidth) / imageHeight), Math.floor(maxWidth)));
  }
  const cellRows = Math.max(1, Math.ceil(pixelRows / 2));
  return { cols, pixelRows, cellRows };
}

function averagePixel(image: DecodedImage, outX: number, outY: number, outWidth: number, outHeight: number, background: Rgba): Rgba {
  const clampedY = Math.min(outY, outHeight - 1);
  const startX = Math.max(0, Math.floor((outX * image.width) / outWidth));
  const endX = Math.min(image.width, Math.max(startX + 1, Math.ceil(((outX + 1) * image.width) / outWidth)));
  const startY = Math.max(0, Math.floor((clampedY * image.height) / outHeight));
  const endY = Math.min(image.height, Math.max(startY + 1, Math.ceil(((clampedY + 1) * image.height) / outHeight)));
  let r = 0;
  let g = 0;
  let b = 0;
  let count = 0;
  for (let sourceY = startY; sourceY < endY; sourceY += 1) {
    for (let sourceX = startX; sourceX < endX; sourceX += 1) {
      const offset = (sourceY * image.width + sourceX) * 4;
      const alpha = image.pixels[offset + 3]! / 255;
      r += image.pixels[offset]! * alpha + background.r * (1 - alpha);
      g += image.pixels[offset + 1]! * alpha + background.g * (1 - alpha);
      b += image.pixels[offset + 2]! * alpha + background.b * (1 - alpha);
      count += 1;
    }
  }
  return {
    r: Math.round(r / count),
    g: Math.round(g / count),
    b: Math.round(b / count),
    a: 255,
  };
}

export function scaleImageToRgba(image: DecodedImage, width: number, height: number, background = "#000000"): Uint8Array {
  const outWidth = Math.max(1, Math.floor(width));
  const outHeight = Math.max(1, Math.floor(height));
  const bg = parseHexColor(background) ?? { r: 0, g: 0, b: 0, a: 255 };
  const pixels = new Uint8Array(outWidth * outHeight * 4);
  for (let y = 0; y < outHeight; y += 1) {
    for (let x = 0; x < outWidth; x += 1) {
      const color = averagePixel(image, x, y, outWidth, outHeight, bg);
      const offset = (y * outWidth + x) * 4;
      pixels[offset] = color.r;
      pixels[offset + 1] = color.g;
      pixels[offset + 2] = color.b;
      pixels[offset + 3] = 255;
    }
  }
  return pixels;
}

function nativeBuffer(
  image: DecodedImage,
  target: { cols: number; pixelRows: number; cellRows: number },
  background: Rgba,
): ImageNativeBuffer {
  const pixelRows = target.cellRows * 2;
  const pixels = new Uint8Array(target.cols * pixelRows * 4);
  for (let y = 0; y < pixelRows; y += 1) {
    for (let x = 0; x < target.cols; x += 1) {
      const color = averagePixel(image, x, y, target.cols, target.pixelRows, background);
      const offset = (y * target.cols + x) * 4;
      pixels[offset] = color.r;
      pixels[offset + 1] = color.g;
      pixels[offset + 2] = color.b;
      pixels[offset + 3] = 255;
    }
  }
  return { cols: target.cols, cellRows: target.cellRows, pixelRows, pixels, alignedBytesPerRow: target.cols * 4 };
}

function blendOver(source: Rgba, background: Rgba): Rgba {
  const alpha = source.a / 255;
  return {
    r: Math.round(source.r * alpha + background.r * (1 - alpha)),
    g: Math.round(source.g * alpha + background.g * (1 - alpha)),
    b: Math.round(source.b * alpha + background.b * (1 - alpha)),
    a: 255,
  };
}

function parseHexColor(value: string): Rgba | null {
  const match = /^#([0-9a-f]{6})$/i.exec(value.trim());
  if (!match) return null;
  const int = Number.parseInt(match[1]!, 16);
  return { r: (int >> 16) & 0xff, g: (int >> 8) & 0xff, b: int & 0xff, a: 255 };
}

function rgbToHex(color: Rgba): string {
  return `#${hex(color.r)}${hex(color.g)}${hex(color.b)}`;
}

function hex(value: number): string {
  return Math.max(0, Math.min(255, value)).toString(16).padStart(2, "0");
}
