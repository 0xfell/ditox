import type { BoxRenderable, OptimizedBuffer } from "@opentui/core";
import { For } from "solid-js";
import type { ImageBlockPreview as ImageBlockPreviewModel } from "../image-preview";
import type { ImagePreviewAlign, ImagePreviewRenderer } from "../ui-config";

export function ImagePreviewRows(props: {
  preview: ImageBlockPreviewModel;
  renderer: ImagePreviewRenderer;
  blockGlyph: string;
  align?: ImagePreviewAlign;
  width?: number;
}) {
  if (props.preview.kind !== "rendered") return null;
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

function shouldUseOpenTuiRenderer(renderer: ImagePreviewRenderer, blockGlyph: string): boolean {
  if (renderer === "opentui") return true;
  if (renderer === "text") return false;
  return blockGlyph === "▀";
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
