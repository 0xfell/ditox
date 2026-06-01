import type { BoxRenderable, OptimizedBuffer } from "@opentui/core";
import { For, onCleanup } from "solid-js";
import type { ImageBlockPreview as ImageBlockPreviewModel } from "../image-preview";
import type { TerminalImageManagerLike, TerminalImageState } from "../terminal-image";
import { computeNativeImagePlacement, selectNativeImageProtocol } from "../terminal-image";
import type { Entry } from "../types";
import type { ImagePreviewAlign, ImagePreviewMode, ImagePreviewRenderer } from "../ui-config";

export function ImagePreviewRows(props: {
  preview: ImageBlockPreviewModel;
  renderer: ImagePreviewRenderer;
  mode: ImagePreviewMode;
  blockGlyph: string;
  align?: ImagePreviewAlign;
  width?: number;
  background?: string;
  entry?: Entry;
  terminal?: TerminalImageState;
  imageManager?: TerminalImageManagerLike;
}) {
  if (props.preview.kind !== "rendered") return null;
  const nativeRows = nativeImageRows(props);
  if (nativeRows) return nativeRows;

  const imageWidth = props.preview.native.cols;
  const contentWidth = Math.max(imageWidth, Math.floor(props.width ?? imageWidth));
  const imageHeight = props.preview.native.cellRows;
  const leftPad = imageAlignPadding(contentWidth, imageWidth, props.align ?? "left");
  const imageRows = shouldUseOpenTuiRenderer(props.renderer, props.blockGlyph) ? <OpenTuiImageRows preview={props.preview} /> : <TextImageRows preview={props.preview} />;

  return (
    <box flexDirection="row" width={contentWidth} height={imageHeight} flexShrink={0}>
      {leftPad > 0 ? <box width={leftPad} height={imageHeight} flexShrink={0} /> : null}
      {imageRows}
    </box>
  );
}

function shouldUseOpenTuiRenderer(renderer: ImagePreviewRenderer, _blockGlyph: string): boolean {
  if (renderer === "opentui") return true;
  if (renderer === "text") return false;
  return false;
}

function imageAlignPadding(width: number, imageWidth: number, align: ImagePreviewAlign): number {
  const extra = Math.max(0, width - imageWidth);
  if (align === "right") return extra;
  if (align === "center") return Math.floor(extra / 2);
  return 0;
}

function OpenTuiImageRows(props: { preview: Extract<ImageBlockPreviewModel, { kind: "rendered" }> }) {
  const drawImage = function (this: BoxRenderable, buffer: OptimizedBuffer) {
    const image = props.preview.native;
    const pixels = image.pixels as unknown as Parameters<OptimizedBuffer["drawSuperSampleBuffer"]>[2];
    buffer.drawSuperSampleBuffer(this.screenX, this.screenY, pixels, image.pixels.length, "rgba8unorm", image.alignedBytesPerRow);
  };
  return <box width={props.preview.native.cols} height={props.preview.native.cellRows} flexShrink={0} renderAfter={drawImage} />;
}

function nativeImageRows(props: {
  preview: ImageBlockPreviewModel;
  renderer: ImagePreviewRenderer;
  mode: ImagePreviewMode;
  align?: ImagePreviewAlign;
  width?: number;
  background?: string;
  entry?: Entry;
  terminal?: TerminalImageState;
  imageManager?: TerminalImageManagerLike;
}) {
  if (props.preview.kind !== "rendered") return null;
  const terminal = props.terminal;
  const resolution = terminal?.resolution ?? null;
  const protocol = selectNativeImageProtocol(props.mode, props.renderer, terminal?.capabilities, resolution);
  if (!terminal || !resolution || !protocol || !props.imageManager) return null;

  const contentWidth = Math.max(1, Math.floor(props.width ?? props.preview.bounds.maxCols));
  const placement = computeNativeImagePlacement({
    imageWidth: props.preview.image.width,
    imageHeight: props.preview.image.height,
    maxCols: props.preview.bounds.maxCols,
    maxRows: props.preview.native.cellRows,
    contentWidth,
    terminalColumns: terminal.columns,
    terminalRows: terminal.rows,
    resolution,
    align: props.align ?? "left",
    protocol,
  });
  if (!placement) return null;

  return (
    <box width={contentWidth} height={placement.rows} flexDirection="row" flexShrink={0} backgroundColor={props.background}>
      {placement.leftPad > 0 ? <box width={placement.leftPad} height={placement.rows} flexShrink={0} backgroundColor={props.background} /> : null}
      <NativeTerminalImage
        preview={props.preview}
        sourceKey={nativeImageSourceKey(props.entry, props.preview)}
        background={props.background ?? "#000000"}
        manager={props.imageManager}
        placement={placement}
      />
    </box>
  );
}

function NativeTerminalImage(props: {
  preview: Extract<ImageBlockPreviewModel, { kind: "rendered" }>;
  sourceKey: string;
  background: string;
  manager: TerminalImageManagerLike;
  placement: NonNullable<ReturnType<typeof computeNativeImagePlacement>>;
}) {
  onCleanup(() => props.manager.clear());
  const drawImage = function (this: BoxRenderable) {
    props.manager.queue({
      protocol: props.placement.protocol,
      sourceKey: props.sourceKey,
      image: props.preview.image,
      background: props.background,
      screenX: this.screenX,
      screenY: this.screenY,
      cols: props.placement.cols,
      rows: props.placement.rows,
      pixelWidth: props.placement.pixelWidth,
      pixelHeight: props.placement.pixelHeight,
      contentPixelWidth: props.placement.contentPixelWidth,
      contentPixelHeight: props.placement.contentPixelHeight,
      contentOffsetX: props.placement.contentOffsetX,
      contentOffsetY: props.placement.contentOffsetY,
      frameId: this.ctx.frameId,
    });
  };
  return <box width={props.placement.cols} height={props.placement.rows} flexShrink={0} backgroundColor={props.background} renderAfter={drawImage} />;
}

function TextImageRows(props: { preview: Extract<ImageBlockPreviewModel, { kind: "rendered" }> }) {
  return (
    <box flexDirection="column" flexShrink={0}>
      <For each={props.preview.rows}>
        {(row) => (
          <text>
            <For each={row}>{(cell) => <span style={{ fg: cell.fg, bg: cell.bg }}>{cell.char}</span>}</For>
          </text>
        )}
      </For>
    </box>
  );
}

function nativeImageSourceKey(entry: Entry | undefined, preview: Extract<ImageBlockPreviewModel, { kind: "rendered" }>): string {
  if (entry?.kind === "image") return `${entry.hash}:${entry.mime}:${entry.blob_path ?? ""}`;
  return `${preview.image.width}x${preview.image.height}:${preview.source}`;
}
