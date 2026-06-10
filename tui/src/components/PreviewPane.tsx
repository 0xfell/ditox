import { createEffect, For, Show } from "solid-js";
import { type ImageBlockPreview as ImageBlockPreviewModel, type ImageProtocolCapabilities } from "../image-preview";
import { previewMetaTemplateValues, previewModel, truncateText } from "../presentation";
import { visiblePreviewLineCapacity } from "../state";
import { selectNativeImageProtocol, type TerminalImageManagerLike, type TerminalImageState } from "../terminal-image";
import { formatTemplate, paddedTitle, surface, textStyle, type ResolvedTuiConfig } from "../tui-config";
import type { Entry } from "../types";
import { createImageBlockPreview } from "../use-image-block-preview";
import { ImagePreviewRows } from "./ImagePreviewRows";
import { imagePreviewBackground, imagePreviewNotice, previewBorderColor, previewContentColor, previewLineToneColor } from "./preview-shared";
import { fitGutter, justifyContent, verticalJustify } from "./style-utils";

export function PreviewPane(props: {
  config: ResolvedTuiConfig;
  entry: Entry | undefined;
  rows: number;
  width: number;
  widthPercent?: number;
  imageCapabilities?: Partial<ImageProtocolCapabilities>;
  imageTerminal?: TerminalImageState;
  imageManager?: TerminalImageManagerLike;
}) {
  const style = () => surface(props.config, "preview");
  const gutterStyle = () => surface(props.config, "previewGutter");
  const spacerStyle = () => surface(props.config, "previewSpacer");
  const layout = () => props.config.layout;
  const labels = () => props.config.labels;
  const textWidth = () => Math.max(1, props.width - (layout().showPreviewGutter ? layout().previewTextWidthInset : layout().previewPaddingX * 2));
  // Width available to the metadata header/detail text once the preview border,
  // preview padding, and metadata padding are subtracted. Used to hard-truncate
  // each line so a long header (e.g. full hash) cannot wrap into the detail row.
  const metaTextWidth = () =>
    Math.max(1, props.width - (props.config.chrome.previewBorder ? 2 : 0) - layout().previewPaddingX * 2 - layout().previewMetaPaddingX * 2);
  const blockPreview = createImageBlockPreview(() => ({
    entry: props.entry,
    maxWidth: Math.min(textWidth(), layout().imagePreviewMaxWidth),
    maxRows: Math.min(Math.max(2, props.rows - layout().imagePreviewRowInset), layout().imagePreviewMaxRows),
    background: imagePreviewBackground(layout().imagePreviewBackground, style().bg),
    mode: layout().imagePreviewMode,
    labels: labels(),
    blockGlyph: layout().imagePreviewBlockGlyph,
    capabilities: props.imageTerminal?.capabilities ?? props.imageCapabilities,
  }));
  const imageNoticeText = () => imagePreviewNotice(blockPreview(), layout().imagePreviewNoticeVisibility, labels().splitImagePreviewSourceTemplate);
  const imageFallbackVisible = () => props.entry?.kind === "image" && blockPreview().kind === "fallback" && layout().imagePreviewMode !== "metadata";
  const imageNoticeSpacingRows = () =>
    props.entry?.kind === "image" && blockPreview().kind === "rendered" && imageNoticeText() ? layout().imagePreviewNoticeSpacing : 0;
  const imagePreviewRows = () => {
    if (props.entry?.kind !== "image") return 0;
    const preview = blockPreview();
    const renderedRows = preview.kind === "rendered" ? preview.native.cellRows : 0;
    return renderedRows + imageNoticeSpacingRows() + (imageNoticeText() ? 1 : 0) + (imageFallbackVisible() ? 1 : 0);
  };
  const nativeImageActive = () => {
    if (!props.imageManager || props.entry?.kind !== "image" || blockPreview().kind !== "rendered") return false;
    const terminal = props.imageTerminal;
    const resolution = terminal?.resolution ?? null;
    return Boolean(
      terminal &&
        resolution &&
        selectNativeImageProtocol(layout().imagePreviewMode, layout().imagePreviewRenderer, terminal.capabilities, resolution),
    );
  };
  createEffect(() => {
    if (!nativeImageActive()) props.imageManager?.clear();
  });
  const metadataRows = () => (layout().showMetadata && props.entry ? layout().previewMetaHeight : 0);
  const visibleTextRows = () =>
    visiblePreviewLineCapacity(Math.min(layout().maxPreviewLines, Math.max(0, props.rows - metadataRows() - imagePreviewRows())), layout().previewLineSpacing);
  const lines = () =>
    previewModel(
      props.entry,
      visibleTextRows(),
      labels(),
      layout(),
      textWidth(),
    );
  return (
    <box
      width={`${props.widthPercent ?? layout().previewWidthPercent}%`}
      border={props.config.chrome.previewBorder ? true : undefined}
      borderStyle={props.config.chrome.previewBorder ? props.config.chrome.previewBorderStyle : undefined}
      borderColor={props.config.chrome.previewBorder ? previewBorderColor(props.config, style(), props.entry, "splitBorder") : undefined}
      backgroundColor={style().bg}
      paddingX={layout().previewPaddingX}
      paddingY={layout().previewPaddingY}
      title={
        props.config.chrome.previewBorder && props.config.chrome.showPreviewTitle
          ? paddedTitle(labels().previewTitle, layout().frameTitlePaddingLeft, layout().frameTitlePaddingRight)
          : undefined
      }
      titleAlignment={props.config.chrome.previewTitleAlignment}
      bottomTitle={
        props.config.chrome.previewBorder && props.config.chrome.showPreviewEntryTitle && props.entry
          ? paddedTitle(previewEntryTitle(props.config, props.entry), layout().frameTitlePaddingLeft, layout().frameTitlePaddingRight)
          : undefined
      }
      bottomTitleAlignment={props.config.chrome.previewBottomTitleAlignment}
    >
      <Show when={layout().showMetadata ? props.entry : undefined}>
        {(entry) => <PreviewMeta config={props.config} entry={entry()} width={metaTextWidth()} />}
      </Show>
      <box width="100%" flexGrow={1} flexDirection="column" justifyContent={verticalJustify(layout().previewBodyVerticalAlign)}>
        <Show when={props.entry?.kind === "image" && blockPreview().kind === "rendered"}>
          <ImagePreviewRows
            preview={blockPreview()}
            renderer={layout().imagePreviewRenderer}
            mode={layout().imagePreviewMode}
            blockGlyph={layout().imagePreviewBlockGlyph}
            align={layout().imagePreviewAlign}
            width={textWidth()}
            background={imagePreviewBackground(layout().imagePreviewBackground, style().bg)}
            entry={props.entry}
            terminal={props.imageTerminal}
            imageManager={props.imageManager}
          />
        </Show>
        <Show when={imageNoticeSpacingRows() > 0}>
          <box width="100%" height={imageNoticeSpacingRows()} backgroundColor={spacerStyle().bg} />
        </Show>
        <Show when={imageNoticeText()}>
          {(notice) => (
            <box height={1} flexDirection="row" justifyContent={justifyContent(layout().previewContentAlign)} backgroundColor={style().bg}>
              <text style={textStyle(style(), previewContentColor(props.config, style(), "splitImageNotice", style().muted))}>{notice()}</text>
            </box>
          )}
        </Show>
        <Show when={imageFallbackVisible()}>
          <box height={1} flexDirection="row" justifyContent={justifyContent(layout().previewContentAlign)} backgroundColor={style().bg}>
            <text style={textStyle(style())}>
              <span style={textStyle(style(), previewContentColor(props.config, style(), "splitImageFallbackPrefix", style().muted))}>
                {labels().splitImagePreviewFallbackPrefix}
              </span>
              <span style={textStyle(style(), previewContentColor(props.config, style(), "splitImageFallbackSeparator", style().muted))}>
                {labels().splitImagePreviewFallbackSeparator}
              </span>
              <span style={textStyle(style(), previewContentColor(props.config, style(), "splitImageFallbackReason", style().muted))}>
                {(blockPreview() as Extract<ImageBlockPreviewModel, { kind: "fallback" }>).reason}
              </span>
            </text>
          </box>
        </Show>
        <For each={lines()}>
          {(line, index) => (
            <>
              <box height={1} flexDirection="row" backgroundColor={style().bg}>
                <Show when={layout().showPreviewGutter}>
                  <text style={textStyle(gutterStyle(), previewContentColor(props.config, gutterStyle(), "splitGutter", gutterStyle().muted))}>
                    {fitGutter(line.gutter, layout().previewGutterWidth, layout().previewGutterAlign)}
                  </text>
                  <text style={textStyle(gutterStyle(), previewContentColor(props.config, gutterStyle(), "splitGutterSeparator", gutterStyle().muted))}>
                    {labels().previewGutterSeparator}
                  </text>
                </Show>
                <box flexGrow={1} flexDirection="row" justifyContent={justifyContent(layout().previewContentAlign)}>
                  <text style={textStyle(style(), previewLineToneColor("split", props.config, style(), props.entry, line.tone, index()))}>
                    {truncateText(line.text, textWidth(), labels())}
                  </text>
                </box>
              </box>
              <Show when={layout().previewLineSpacing > 0 && index() < lines().length - 1}>
                <box width="100%" height={layout().previewLineSpacing} backgroundColor={spacerStyle().bg} />
              </Show>
            </>
          )}
        </For>
      </box>
    </box>
  );
}

// Clip a single metadata line to the available width without wrapping. Unlike
// truncateText this preserves the template's intentional spacing (e.g. the
// double-space field separators) and only collapses newlines that would force a
// soft wrap into the adjacent metadata row.
export function clipMetaLine(value: string, width: number, marker: string): string {
  const target = Math.max(0, Math.floor(width));
  if (target === 0) return "";
  const oneLine = String(value ?? "").replace(/[\r\n]+/g, " ");
  const chars = Array.from(oneLine);
  if (chars.length <= target) return oneLine;
  const mark = marker.length > 0 ? marker : "...";
  if (target <= mark.length) return chars.slice(0, target).join("");
  return `${chars.slice(0, target - mark.length).join("")}${mark}`;
}

function PreviewMeta(props: { config: ResolvedTuiConfig; entry: Entry; width: number }) {
  const style = () => surface(props.config, "previewMeta");
  const labels = () => props.config.labels;
  const values = () => previewMetaTemplateValues(props.entry, labels(), props.config.layout.previewMetaHashLength);
  const header = () =>
    clipMetaLine(formatTemplate(labels().previewMetaHeaderTemplate, values()), props.width, labels().textTruncationMarker);
  const details = () =>
    clipMetaLine(formatTemplate(labels().previewMetaDetailsTemplate, values()), props.width, labels().textTruncationMarker);
  const detailsVisible = () => props.config.layout.previewMetaHeight >= 2 + props.config.layout.previewMetaLineSpacing;
  return (
    <box
      height={props.config.layout.previewMetaHeight}
      flexDirection="column"
      backgroundColor={style().bg}
      paddingX={props.config.layout.previewMetaPaddingX}
      paddingY={props.config.layout.previewMetaPaddingY}
    >
      <box width="100%" flexGrow={1} flexDirection="column" justifyContent={verticalJustify(props.config.layout.previewMetaVerticalAlign)}>
        <box width="100%" flexDirection="row" justifyContent={justifyContent(props.config.layout.previewMetaContentAlign)}>
          <text style={textStyle(style(), previewContentColor(props.config, style(), "splitMetaHeader", style().accent))}>{header()}</text>
        </box>
        <Show when={detailsVisible() && props.config.layout.previewMetaLineSpacing > 0}>
          <box width="100%" height={props.config.layout.previewMetaLineSpacing} backgroundColor={style().bg} />
        </Show>
        <Show when={detailsVisible()}>
          <box width="100%" flexDirection="row" justifyContent={justifyContent(props.config.layout.previewMetaContentAlign)}>
            <text style={textStyle(style(), previewContentColor(props.config, style(), "splitMetaDetails", style().fg))}>{details()}</text>
          </box>
        </Show>
      </box>
    </box>
  );
}

function previewEntryTitle(config: ResolvedTuiConfig, entry: Entry): string {
  return formatTemplate(config.labels.previewEntryTitleTemplate, {
    entryIdPrefix: config.labels.entryIdPrefix,
    id: entry.id,
  });
}


