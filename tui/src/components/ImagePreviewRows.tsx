import type { BoxRenderable, OptimizedBuffer } from "@opentui/core";
import { For } from "solid-js";
import type { ImageBlockPreview as ImageBlockPreviewModel } from "../image-preview";
import type { ImagePreviewRenderer } from "../ui-config";

export function ImagePreviewRows(props: { preview: ImageBlockPreviewModel; renderer: ImagePreviewRenderer; blockGlyph: string }) {
  if (props.preview.kind !== "rendered") return null;
  if (shouldUseOpenTuiRenderer(props.renderer, props.blockGlyph)) return <OpenTuiImageRows preview={props.preview} />;
  return <TextImageRows preview={props.preview} />;
}

function shouldUseOpenTuiRenderer(renderer: ImagePreviewRenderer, blockGlyph: string): boolean {
  if (renderer === "opentui") return true;
  if (renderer === "text") return false;
  return blockGlyph === "▀";
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
