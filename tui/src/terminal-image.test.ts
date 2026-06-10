import { describe, expect, test } from "bun:test";
import {
  TerminalImageManager,
  computeNativeImagePlacement,
  kittyDeleteImage,
  kittyDeleteVisibleImages,
  kittyPlaceImage,
  kittyTransmitRgba,
  selectNativeImageProtocol,
  sixelEncodeRgba,
} from "./terminal-image";
import type { DecodedImage } from "./image-preview";

describe("terminal image previews", () => {
  test("selects native protocols from renderer, mode, capabilities, and pixel resolution", () => {
    const resolution = { width: 1200, height: 800 };
    const capabilities = { kittyGraphics: true, sixel: true, nativeRenderer: true };

    expect(selectNativeImageProtocol("blocks", "auto", capabilities, resolution)).toBe("kitty");
    expect(selectNativeImageProtocol("blocks", "native", { sixel: true, nativeRenderer: true }, resolution)).toBe("sixel");
    expect(selectNativeImageProtocol("kitty", "auto", capabilities, resolution)).toBe("kitty");
    expect(selectNativeImageProtocol("sixel", "auto", capabilities, resolution)).toBe("sixel");
    expect(selectNativeImageProtocol("kitty", "text", capabilities, resolution)).toBeNull();
    expect(selectNativeImageProtocol("kitty", "opentui", capabilities, resolution)).toBeNull();
    expect(selectNativeImageProtocol("kitty", "auto", { kittyGraphics: false }, resolution)).toBeNull();
    expect(selectNativeImageProtocol("blocks", "auto", capabilities, null)).toBeNull();
  });

  test("maps terminal cell geometry to a pixel-bounded image placement", () => {
    const placement = computeNativeImagePlacement({
      imageWidth: 800,
      imageHeight: 400,
      maxCols: 80,
      maxRows: 10,
      contentWidth: 100,
      terminalColumns: 100,
      terminalRows: 40,
      resolution: { width: 1000, height: 800 },
      align: "center",
      protocol: "kitty",
    });

    expect(placement).toEqual({
      protocol: "kitty",
      cols: 40,
      rows: 10,
      pixelWidth: 400,
      pixelHeight: 200,
      contentPixelWidth: 400,
      contentPixelHeight: 200,
      contentOffsetX: 0,
      contentOffsetY: 0,
      leftPad: 30,
    });
  });

  test("keeps native previews at source size when the image is smaller than the pane", () => {
    const placement = computeNativeImagePlacement({
      imageWidth: 20,
      imageHeight: 20,
      maxCols: 80,
      maxRows: 20,
      contentWidth: 80,
      terminalColumns: 100,
      terminalRows: 40,
      resolution: { width: 1000, height: 800 },
      align: "right",
      protocol: "sixel",
    });

    expect(placement?.pixelWidth).toBe(20);
    expect(placement?.pixelHeight).toBe(20);
    expect(placement?.contentPixelWidth).toBe(20);
    expect(placement?.contentPixelHeight).toBe(20);
    expect(placement?.cols).toBe(2);
    expect(placement?.rows).toBe(1);
    expect(placement?.leftPad).toBe(78);
  });

  test("sends exact whole-cell pixel rectangles to avoid terminal-side rescaling", () => {
    const placement = computeNativeImagePlacement({
      imageWidth: 100,
      imageHeight: 50,
      maxCols: 80,
      maxRows: 20,
      contentWidth: 80,
      terminalColumns: 100,
      terminalRows: 40,
      resolution: { width: 900, height: 680 },
      align: "left",
      protocol: "kitty",
    });

    expect(placement).toMatchObject({
      cols: 12,
      rows: 3,
      pixelWidth: 108,
      pixelHeight: 51,
      contentPixelWidth: 100,
      contentPixelHeight: 50,
      contentOffsetX: 4,
      contentOffsetY: 0,
    });
  });

  test("encodes Kitty RGBA transmit and placement commands", () => {
    const pixels = Uint8Array.from([255, 0, 0, 255, 0, 255, 0, 255]);
    const transmit = kittyTransmitRgba(123, pixels, 2, 1);
    expect(transmit).toContain("\x1b_Ga=t,f=32,s=2,v=1,i=123");
    expect(transmit).not.toContain("a=T");
    expect(transmit).toContain(Buffer.from(pixels).toString("base64"));
    expect(transmit).toContain("\x1b\\");

    expect(kittyPlaceImage(123, 8, 4)).toBe("\x1b_Ga=p,i=123,p=1,c=8,r=4,C=1,q=2;\x1b\\");
    expect(kittyDeleteVisibleImages()).toBe("\x1b_Ga=d,q=2;\x1b\\");
    expect(kittyDeleteImage(123)).toBe("\x1b_Ga=d,d=i,i=123,q=2;\x1b\\");
    expect(kittyDeleteImage(123, true)).toBe("\x1b_Ga=d,d=I,i=123,q=2;\x1b\\");
  });

  test("encodes Sixel image data with color definitions and run-length pixels", () => {
    const pixels = Uint8Array.from([
      255, 0, 0, 255,
      255, 0, 0, 255,
      0, 0, 255, 255,
      0, 0, 255, 255,
    ]);
    const sixel = sixelEncodeRgba(pixels, 2, 2);
    expect(sixel.startsWith("\x1bPq")).toBe(true);
    expect(sixel).toContain("#180;2;100;0;0");
    expect(sixel).toContain("#5;2;0;0;100");
    expect(sixel.endsWith("\x1b\\")).toBe(true);
  });

  test("writes native graphics after a frame and reuses transmitted Kitty payloads", async () => {
    const writes: string[] = [];
    const manager = new TerminalImageManager((chunk) => writes.push(chunk));
    const image: DecodedImage = { width: 1, height: 1, pixels: Uint8Array.from([255, 0, 0, 255]) };

    manager.queue({
      protocol: "kitty",
      sourceKey: "red",
      image,
      background: "#000000",
      screenX: 4,
      screenY: 2,
      cols: 1,
      rows: 1,
      pixelWidth: 1,
      pixelHeight: 1,
      contentPixelWidth: 1,
      contentPixelHeight: 1,
      contentOffsetX: 0,
      contentOffsetY: 0,
    });
    await new Promise((resolve) => setTimeout(resolve, 1));
    expect(writes).toHaveLength(1);
    expect(writes[0]).toContain("\x1b[3;5H");
    expect(writes[0]).not.toContain("a=d");
    expect(writes[0]).toContain("a=t");
    expect(writes[0]).toContain("a=p");
    expect(writes[0]).not.toContain("a=T");

    manager.queue({
      protocol: "kitty",
      sourceKey: "red",
      image,
      background: "#000000",
      screenX: 5,
      screenY: 2,
      cols: 1,
      rows: 1,
      pixelWidth: 1,
      pixelHeight: 1,
      contentPixelWidth: 1,
      contentPixelHeight: 1,
      contentOffsetX: 0,
      contentOffsetY: 0,
    });
    await new Promise((resolve) => setTimeout(resolve, 1));
    expect(writes).toHaveLength(2);
    expect(writes[1]).not.toContain("a=d");
    expect(writes[1]).not.toContain("a=t");
    expect(writes[1]).toContain("a=p");

    manager.clear();
    expect(writes.at(-1)).toContain("a=d");
    expect(writes.at(-1)).toContain("d=I");
  });

  test("skips re-emitting an identical placement queued on a later frame", async () => {
    const writes: string[] = [];
    const manager = new TerminalImageManager((chunk) => writes.push(chunk));
    const image: DecodedImage = { width: 1, height: 1, pixels: Uint8Array.from([255, 0, 0, 255]) };

    const request = {
      protocol: "kitty" as const,
      sourceKey: "red",
      image,
      background: "#000000",
      screenX: 4,
      screenY: 2,
      cols: 1,
      rows: 1,
      pixelWidth: 1,
      pixelHeight: 1,
      contentPixelWidth: 1,
      contentPixelHeight: 1,
      contentOffsetX: 0,
      contentOffsetY: 0,
    };

    manager.queue(request);
    await new Promise((resolve) => setTimeout(resolve, 1));
    manager.queue({ ...request });
    manager.queue({ ...request });
    await new Promise((resolve) => setTimeout(resolve, 1));

    expect(writes).toHaveLength(1);

    // After clear, the same placement must re-emit (the screen was wiped).
    manager.clear();
    manager.queue({ ...request });
    await new Promise((resolve) => setTimeout(resolve, 1));
    expect(writes.filter((chunk) => chunk.includes("a=p"))).toHaveLength(2);
  });

  test("frees old Kitty image data only after the new placement is established", async () => {
    const writes: string[] = [];
    const manager = new TerminalImageManager((chunk) => writes.push(chunk));
    const image: DecodedImage = { width: 1, height: 1, pixels: Uint8Array.from([255, 0, 0, 255]) };

    const request = {
      protocol: "kitty" as const,
      image,
      background: "#000000",
      screenX: 0,
      screenY: 0,
      cols: 1,
      rows: 1,
      pixelWidth: 1,
      pixelHeight: 1,
      contentPixelWidth: 1,
      contentPixelHeight: 1,
      contentOffsetX: 0,
      contentOffsetY: 0,
    };

    manager.queue({ ...request, sourceKey: "red" });
    await new Promise((resolve) => setTimeout(resolve, 1));
    manager.queue({ ...request, sourceKey: "blue" });
    await new Promise((resolve) => setTimeout(resolve, 1));

    expect(writes).toHaveLength(2);
    expect(writes[1]).not.toContain("a=d,q=2");
    expect(writes[1]).toContain("d=I");
    expect(writes[1]).toContain("a=t");
    expect(writes[1]).toContain("a=p");
    // The new placement must come before the old image is freed.
    expect(writes[1]!.indexOf("a=p")).toBeLessThan(writes[1]!.indexOf("d=I"));
  });
});
