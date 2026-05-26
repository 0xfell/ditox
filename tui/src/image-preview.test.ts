import { describe, expect, test } from "bun:test";
import { deflateSync } from "node:zlib";
import * as jpeg from "jpeg-js";
import {
  decodeBmp,
  decodeGif,
  decodeJpegImage,
  decodePng,
  decodeWebpImage,
  imageBlockPreview,
  imageBlockPreviewFromBytes,
  imageBlockPreviewFromBytesAsync,
  imagePreviewProtocolNotice,
} from "./image-preview";
import type { Entry } from "./types";

describe("image preview", () => {
  test("decodes simple RGBA PNG dimensions and pixels", () => {
    const bytes = rgbaPng(2, 2, [
      [255, 0, 0, 255],
      [0, 255, 0, 255],
      [0, 0, 255, 255],
      [255, 255, 255, 255],
    ]);

    const decoded = decodePng(bytes);
    expect(decoded.width).toBe(2);
    expect(decoded.height).toBe(2);
    expect([...decoded.pixels.slice(0, 8)]).toEqual([255, 0, 0, 255, 0, 255, 0, 255]);
  });

  test("renders PNG pixels into upper-half terminal cells", () => {
    const preview = imageBlockPreviewFromBytes(
      rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
      ]),
      2,
      1,
      "#000000",
    );

    expect(preview.kind).toBe("rendered");
    if (preview.kind !== "rendered") return;
    expect(preview.rows).toHaveLength(1);
    expect(preview.rows[0]).toEqual([
      { char: "▀", fg: "#ff0000", bg: "#0000ff" },
      { char: "▀", fg: "#00ff00", bg: "#ffffff" },
    ]);
    expect(preview.native.cols).toBe(2);
    expect(preview.native.cellRows).toBe(1);
    expect(preview.native.pixelRows).toBe(2);
    expect([...preview.native.pixels.slice(0, 16)]).toEqual([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]);
    expect(preview.native.alignedBytesPerRow).toBe(8);
  });

  test("renders block previews with a configured glyph", () => {
    const preview = imageBlockPreviewFromBytes(
      rgbaPng(2, 2, [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
      ]),
      2,
      1,
      "#000000",
      "auto",
      undefined,
      "█",
    );

    expect(preview.kind).toBe("rendered");
    if (preview.kind !== "rendered") return;
    expect(preview.rows[0]).toEqual([
      { char: "█", fg: "#ff0000", bg: "#0000ff" },
      { char: "█", fg: "#00ff00", bg: "#ffffff" },
    ]);
  });

  test("labels Kitty and Sixel requests as protocol fallbacks", () => {
    const bytes = rgbaPng(2, 2, [
      [255, 0, 0, 255],
      [0, 255, 0, 255],
      [0, 0, 255, 255],
      [255, 255, 255, 255],
    ]);

    const kitty = imageBlockPreviewFromBytes(bytes, 2, 1, "#000000", "auto", undefined, "▀", "kitty fallback blocks");
    const sixel = imageBlockPreviewFromBytes(bytes, 2, 1, "#000000", "auto", undefined, "▀", "sixel fallback blocks");

    expect(kitty.kind).toBe("rendered");
    expect(sixel.kind).toBe("rendered");
    if (kitty.kind !== "rendered" || sixel.kind !== "rendered") return;
    expect(kitty.source).toBe("kitty fallback blocks 2x1");
    expect(sixel.source).toBe("sixel fallback blocks 2x1");
  });

  test("describes protocol fallback support states", () => {
    expect(imagePreviewProtocolNotice("blocks")).toBeNull();
    expect(imagePreviewProtocolNotice("kitty")).toBe("Kitty support unknown; showing block fallback");
    expect(imagePreviewProtocolNotice("kitty", { kittyGraphics: false })).toBe("Kitty not detected; showing block fallback");
    expect(imagePreviewProtocolNotice("kitty", { kittyGraphics: true, nativeRenderer: false })).toBe(
      "Kitty detected; native renderer unavailable; showing block fallback",
    );
    expect(imagePreviewProtocolNotice("sixel", { sixel: true, nativeRenderer: true })).toBeNull();
    expect(
      imagePreviewProtocolNotice(
        "sixel",
        { sixel: false },
        {
          imagePreviewProtocolUnsupported: "{protocol} unavailable",
        },
      ),
    ).toBe("Sixel unavailable");
  });

  test("decodes uncompressed BMP files for block previews", () => {
    const bytes = rgbaBmp(2, 2, [
      [255, 0, 0, 255],
      [0, 255, 0, 255],
      [0, 0, 255, 255],
      [255, 255, 255, 255],
    ]);

    const decoded = decodeBmp(bytes);
    expect(decoded.width).toBe(2);
    expect(decoded.height).toBe(2);
    expect([...decoded.pixels.slice(0, 8)]).toEqual([255, 0, 0, 255, 0, 255, 0, 255]);

    const preview = imageBlockPreviewFromBytes(bytes, 2, 1, "#000000", "image/bmp");
    expect(preview.kind).toBe("rendered");
    if (preview.kind !== "rendered") return;
    expect(preview.rows[0]).toEqual([
      { char: "▀", fg: "#ff0000", bg: "#0000ff" },
      { char: "▀", fg: "#00ff00", bg: "#ffffff" },
    ]);
  });

  test("decodes GIF first frames for block previews", () => {
    const bytes = indexedGif(
      2,
      2,
      [
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 255],
      ],
      [0, 1, 2, 3],
    );

    const decoded = decodeGif(bytes);
    expect(decoded.width).toBe(2);
    expect(decoded.height).toBe(2);
    expect([...decoded.pixels.slice(0, 8)]).toEqual([255, 0, 0, 255, 0, 255, 0, 255]);

    const preview = imageBlockPreviewFromBytes(bytes, 2, 1, "#000000", "image/gif");
    expect(preview.kind).toBe("rendered");
    if (preview.kind !== "rendered") return;
    expect(preview.rows[0]).toEqual([
      { char: "▀", fg: "#ff0000", bg: "#0000ff" },
      { char: "▀", fg: "#00ff00", bg: "#ffffff" },
    ]);
  });

  test("decodes JPEG files for block previews", () => {
    const bytes = rgbaJpeg(8, 8, [208, 32, 24, 255]);

    const decoded = decodeJpegImage(bytes);
    expect(decoded.width).toBe(8);
    expect(decoded.height).toBe(8);
    expect(decoded.pixels[0]).toBeGreaterThan(180);
    expect(decoded.pixels[1]).toBeLessThan(70);
    expect(decoded.pixels[2]).toBeLessThan(70);
    expect(decoded.pixels[3]).toBe(255);

    const preview = imageBlockPreviewFromBytes(bytes, 4, 2, "#000000", "image/jpeg");
    expect(preview.kind).toBe("rendered");
    if (preview.kind !== "rendered") return;
    expect(preview.rows).toHaveLength(2);
    expect(preview.rows[0]).toHaveLength(4);
    expect(preview.rows[0]![0]!.char).toBe("▀");
    expect(preview.rows[0]![0]!.fg).toMatch(/^#[0-9a-f]{6}$/);
    expect(preview.rows[0]![0]!.bg).toMatch(/^#[0-9a-f]{6}$/);
  });

  test("decodes WebP files for block previews", async () => {
    const bytes = webp2x2Red();

    const decoded = await decodeWebpImage(bytes);
    expect(decoded.width).toBe(2);
    expect(decoded.height).toBe(2);
    expect(decoded.pixels[0]).toBeGreaterThan(200);
    expect(decoded.pixels[1]).toBeLessThan(40);
    expect(decoded.pixels[2]).toBeLessThan(40);
    expect(decoded.pixels[3]).toBe(255);

    const preview = await imageBlockPreviewFromBytesAsync(bytes, 2, 1, "#000000", "image/webp");
    expect(preview.kind).toBe("rendered");
    if (preview.kind !== "rendered") return;
    expect(preview.rows).toHaveLength(1);
    expect(preview.rows[0]).toHaveLength(2);
    expect(preview.rows[0]![0]!.char).toBe("▀");
    expect(preview.rows[0]![0]!.fg).toMatch(/^#[0-9a-f]{6}$/);
    expect(preview.rows[0]![0]!.bg).toMatch(/^#[0-9a-f]{6}$/);
  });

  test("falls back with a readable reason for unsupported bytes", () => {
    const preview = imageBlockPreviewFromBytes(Uint8Array.from([1, 2, 3]), 12, 4);
    expect(preview.kind).toBe("fallback");
    if (preview.kind !== "fallback") return;
    expect(preview.reason).toBe("unsupported image bytes");
  });

  test("falls back with configured copy", () => {
    const labels = {
      imagePreviewNotImage: "not a picture row",
      imagePreviewBlocksDisabled: "block mode off",
      imagePreviewBlobMissing: "missing image file",
      imagePreviewDecodePending: "loading image pixels",
      imagePreviewUnsupportedMime: "cannot draw {mime}",
      imagePreviewUnsupportedBytes: "bytes cannot be decoded",
      imagePreviewDecodeFailed: "decode failed: {error}",
      imagePreviewProtocolUnknown: "{protocol} unknown",
      imagePreviewProtocolUnsupported: "{protocol} unsupported",
      imagePreviewProtocolRendererUnavailable: "{protocol} no renderer",
    };

    expect(imageBlockPreview(undefined, 12, 4, "#000000", "blocks", labels)).toEqual({
      kind: "fallback",
      reason: "not a picture row",
    });
    expect(imageBlockPreview(imageEntry({ blob_path: "/tmp/missing.png" }), 12, 4, "#000000", "metadata", labels)).toEqual({
      kind: "fallback",
      reason: "block mode off",
    });
    expect(imageBlockPreview(imageEntry({ blob_path: null }), 12, 4, "#000000", "blocks", labels)).toEqual({
      kind: "fallback",
      reason: "missing image file",
    });
    expect(imageBlockPreview(imageEntry({ mime: "image/webp", blob_path: "/tmp/webp" }), 12, 4, "#000000", "blocks", labels)).toEqual({
      kind: "fallback",
      reason: "loading image pixels",
    });

    const unsupported = imageBlockPreviewFromBytes(Uint8Array.from([1, 2, 3]), 12, 4, "#000000", "auto", labels);
    expect(unsupported.kind).toBe("fallback");
    if (unsupported.kind !== "fallback") return;
    expect(unsupported.reason).toBe("bytes cannot be decoded");

    const decodeFailure = imageBlockPreviewFromBytes(Uint8Array.from([1, 2, 3]), 12, 4, "#000000", "image/png", labels);
    expect(decodeFailure.kind).toBe("fallback");
    if (decodeFailure.kind !== "fallback") return;
    expect(decodeFailure.reason).toBe("decode failed: not a PNG file");
  });
});

function imageEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: 1,
    kind: "image",
    mime: "image/png",
    content: "hash",
    preview: "image/png",
    hash: "hash",
    favorite: false,
    created_at_ms: Date.now(),
    byte_len: 10,
    source_app: null,
    blob_path: null,
    image_width: null,
    image_height: null,
    ...overrides,
  };
}

function webp2x2Red(): Uint8Array {
  return Uint8Array.from(Buffer.from("UklGRjwAAABXRUJQVlA4IDAAAADQAQCdASoCAAIAAgA0JaACdLoB+AADsAD+8MQL/yC5YXXI1/8gP+QH/ID/+PIAAAA=", "base64"));
}

function rgbaPng(width: number, height: number, pixels: Array<[number, number, number, number]>): Uint8Array {
  const raw = new Uint8Array(height * (1 + width * 4));
  let offset = 0;
  let pixel = 0;
  for (let y = 0; y < height; y += 1) {
    raw[offset++] = 0;
    for (let x = 0; x < width; x += 1) {
      raw.set(pixels[pixel++]!, offset);
      offset += 4;
    }
  }

  return concatBytes(
    Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", concatBytes(u32(width), u32(height), Uint8Array.from([8, 6, 0, 0, 0]))),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", new Uint8Array()),
  );
}

function chunk(type: string, data: Uint8Array): Uint8Array {
  const typeBytes = Uint8Array.from([...type].map((char) => char.charCodeAt(0)));
  return concatBytes(u32(data.length), typeBytes, data, Uint8Array.from([0, 0, 0, 0]));
}

function u32(value: number): Uint8Array {
  return Uint8Array.from([(value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff]);
}

function rgbaBmp(width: number, height: number, pixels: Array<[number, number, number, number]>): Uint8Array {
  const rowStride = Math.floor((24 * width + 31) / 32) * 4;
  const pixelBytes = rowStride * height;
  const out = new Uint8Array(54 + pixelBytes);
  out[0] = 0x42;
  out[1] = 0x4d;
  writeU32LE(out, 2, out.length);
  writeU32LE(out, 10, 54);
  writeU32LE(out, 14, 40);
  writeI32LE(out, 18, width);
  writeI32LE(out, 22, height);
  writeU16LE(out, 26, 1);
  writeU16LE(out, 28, 24);
  writeU32LE(out, 34, pixelBytes);

  for (let y = 0; y < height; y += 1) {
    const sourceY = height - 1 - y;
    let offset = 54 + y * rowStride;
    for (let x = 0; x < width; x += 1) {
      const [r, g, b] = pixels[sourceY * width + x]!;
      out[offset++] = b;
      out[offset++] = g;
      out[offset++] = r;
    }
  }
  return out;
}

function rgbaJpeg(width: number, height: number, color: [number, number, number, number]): Uint8Array {
  const data = new Uint8Array(width * height * 4);
  for (let offset = 0; offset < data.length; offset += 4) data.set(color, offset);
  return Uint8Array.from(jpeg.encode({ width, height, data }, 100).data);
}

function writeU16LE(bytes: Uint8Array, offset: number, value: number): void {
  bytes[offset] = value & 0xff;
  bytes[offset + 1] = (value >>> 8) & 0xff;
}

function writeU32LE(bytes: Uint8Array, offset: number, value: number): void {
  bytes[offset] = value & 0xff;
  bytes[offset + 1] = (value >>> 8) & 0xff;
  bytes[offset + 2] = (value >>> 16) & 0xff;
  bytes[offset + 3] = (value >>> 24) & 0xff;
}

function writeI32LE(bytes: Uint8Array, offset: number, value: number): void {
  writeU32LE(bytes, offset, value >>> 0);
}

function indexedGif(width: number, height: number, palette: Array<[number, number, number]>, indices: number[]): Uint8Array {
  const colorCount = Math.max(2, 1 << Math.ceil(Math.log2(palette.length)));
  const tableSizeBits = Math.max(0, Math.log2(colorCount) - 1);
  const colorTable = new Uint8Array(colorCount * 3);
  for (const [index, color] of palette.entries()) colorTable.set(color, index * 3);
  const minCodeSize = Math.max(2, Math.ceil(Math.log2(colorCount)));
  const clear = 1 << minCodeSize;
  const end = clear + 1;
  const codes: number[] = [];
  for (let index = 0; index < indices.length; index += 2) {
    codes.push(clear, indices[index]!);
    if (indices[index + 1] !== undefined) codes.push(indices[index + 1]!);
  }
  codes.push(clear, end);
  const imageData = packFixedCodes(codes, minCodeSize + 1);

  return concatBytes(
    Uint8Array.from([...Buffer.from("GIF89a")]),
    u16le(width),
    u16le(height),
    Uint8Array.from([0x80 | 0x70 | tableSizeBits, 0, 0]),
    colorTable,
    Uint8Array.from([0x2c]),
    u16le(0),
    u16le(0),
    u16le(width),
    u16le(height),
    Uint8Array.from([0, minCodeSize, imageData.length]),
    imageData,
    Uint8Array.from([0, 0x3b]),
  );
}

function packFixedCodes(codes: number[], size: number): Uint8Array {
  const out = new Uint8Array(Math.ceil((codes.length * size) / 8));
  let bitOffset = 0;
  for (const code of codes) {
    for (let bit = 0; bit < size; bit += 1) {
      if (((code >> bit) & 1) !== 0) out[Math.floor(bitOffset / 8)]! |= 1 << (bitOffset % 8);
      bitOffset += 1;
    }
  }
  return out;
}

function u16le(value: number): Uint8Array {
  return Uint8Array.from([value & 0xff, (value >>> 8) & 0xff]);
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
