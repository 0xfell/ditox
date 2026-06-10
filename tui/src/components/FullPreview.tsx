import { createEffect, For, Show } from "solid-js";
import { type ImageBlockPreview as ImageBlockPreviewModel, type ImageProtocolCapabilities } from "../image-preview";
import { entryAccent, previewMetaTemplateValues, previewModel, previewWindow, truncateText } from "../presentation";
import { visibleFullPreviewLineCapacity } from "../state";
import { selectNativeImageProtocol, type TerminalImageManagerLike, type TerminalImageState } from "../terminal-image";
import { formatTemplate, paddedTitle, surface, textStyle, type ResolvedTuiConfig } from "../tui-config";
import type { Entry } from "../types";
import { createImageBlockPreview } from "../use-image-block-preview";
import { ImagePreviewRows } from "./ImagePreviewRows";
import { imagePreviewBackground, imagePreviewNotice, previewBorderColor, previewContentColor, previewLineToneColor } from "./preview-shared";
import { fitGutter, justifyContent, verticalJustify } from "./style-utils";

export function FullPreview(props: {
  config: ResolvedTuiConfig;
  entry: Entry | undefined;
  rows: number;
  width: number;
  offset: number;
  imageCapabilities?: Partial<ImageProtocolCapabilities>;
  imageTerminal?: TerminalImageState;
  imageManager?: TerminalImageManagerLike;
  onScroll: (direction: -1 | 1) => void;
}) {
  const style = () => surface(props.config, "fullPreview");
  const gutterStyle = () => surface(props.config, "fullPreviewGutter");
  const metaStyle = () => surface(props.config, "fullPreviewMeta");
  const spacerStyle = () => surface(props.config, "fullPreviewSpacer");
  const labels = () => props.config.labels;
  const layout = () => props.config.layout;
  const textWidth = () => Math.max(1, props.width - (layout().showFullPreviewGutter ? layout().fullPreviewTextWidthInset : layout().fullPreviewPaddingX * 2));
  const blockPreview = createImageBlockPreview(() => ({
    entry: props.entry,
    maxWidth: Math.min(textWidth(), layout().fullPreviewImageMaxWidth),
    maxRows: Math.min(Math.max(2, props.rows - layout().fullPreviewImageRowInset), layout().fullPreviewImageMaxRows),
    background: imagePreviewBackground(layout().fullPreviewImageBackground, style().bg),
    mode: layout().fullPreviewImageMode,
    labels: labels(),
    blockGlyph: layout().fullPreviewImageBlockGlyph,
    capabilities: props.imageTerminal?.capabilities ?? props.imageCapabilities,
  }));
  const lines = () =>
    previewModel(props.entry, layout().maxFullPreviewLines, labels(), layout(), textWidth());
  const imageNoticeText = () => imagePreviewNotice(blockPreview(), layout().fullPreviewImageNoticeVisibility, labels().fullImagePreviewSourceTemplate);
  const imageFallbackVisible = () => props.entry?.kind === "image" && blockPreview().kind === "fallback" && layout().fullPreviewImageMode !== "metadata";
  const imageNoticeSpacingRows = () =>
    props.entry?.kind === "image" && blockPreview().kind === "rendered" && imageNoticeText() ? layout().fullPreviewImageNoticeSpacing : 0;
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
        selectNativeImageProtocol(layout().fullPreviewImageMode, layout().fullPreviewImageRenderer, terminal.capabilities, resolution),
    );
  };
  createEffect(() => {
    if (!nativeImageActive()) props.imageManager?.clear();
  });
  const metadataRows = () => (layout().showFullPreviewMetadata && props.entry ? layout().fullPreviewMetaHeight : 0);
  const visibleRows = () => visibleFullPreviewLineCapacity(props.rows, layout().fullPreviewLineSpacing, layout().fullPreviewScrollInsetRows, metadataRows() + imagePreviewRows());
  const visible = () => previewWindow(lines(), props.offset, visibleRows());
  const metaDetailsVisible = () => layout().fullPreviewMetaHeight >= 2 + layout().fullPreviewMetaLineSpacing;
  const bottomTitle = () => {
    if (!props.entry) return paddedTitle(labels().previewBackHint, layout().frameTitlePaddingLeft, layout().frameTitlePaddingRight);
    const total = lines().length;
    const end = Math.min(total, props.offset + visibleRows());
    return paddedTitle(
      formatTemplate(labels().fullPreviewBottomTitleTemplate, {
        entryIdPrefix: labels().entryIdPrefix,
        id: props.entry.id,
        start: props.offset + 1,
        end,
        total,
        separator: labels().previewMetaSeparator,
        back: labels().previewBackHint,
      }),
      layout().frameTitlePaddingLeft,
      layout().frameTitlePaddingRight,
    );
  };
  return (
    <box
      flexGrow={1}
      border={props.config.chrome.fullPreviewBorder ? true : undefined}
      borderStyle={props.config.chrome.fullPreviewBorder ? props.config.chrome.fullPreviewBorderStyle : undefined}
      borderColor={props.config.chrome.fullPreviewBorder ? previewBorderColor(props.config, style(), props.entry, "fullBorder") : undefined}
      backgroundColor={style().bg}
      paddingX={layout().fullPreviewPaddingX}
      paddingY={layout().fullPreviewPaddingY}
      title={
        props.config.chrome.fullPreviewBorder && props.config.chrome.showFullPreviewTitle
          ? paddedTitle(labels().previewModeTitle, layout().frameTitlePaddingLeft, layout().frameTitlePaddingRight)
          : undefined
      }
      titleAlignment={props.config.chrome.fullPreviewTitleAlignment}
      bottomTitle={props.config.chrome.fullPreviewBorder && props.config.chrome.showFullPreviewBottomTitle ? bottomTitle() : undefined}
      bottomTitleAlignment={props.config.chrome.fullPreviewBottomTitleAlignment}
      onMouseScroll={(event: any) => {
        const direction = scrollDirection(event);
        if (!direction) return;
        event.preventDefault();
        props.onScroll(direction);
      }}
    >
      <Show when={layout().showFullPreviewMetadata ? props.entry : undefined}>
        {(entry) => (
          <box
            height={layout().fullPreviewMetaHeight}
            flexDirection="column"
            backgroundColor={metaStyle().bg}
            paddingX={layout().fullPreviewMetaPaddingX}
            paddingY={layout().fullPreviewMetaPaddingY}
          >
            <box width="100%" flexGrow={1} flexDirection="column" justifyContent={verticalJustify(layout().fullPreviewMetaVerticalAlign)}>
              <box width="100%" flexDirection="row" justifyContent={justifyContent(layout().fullPreviewMetaContentAlign)}>
                <text style={textStyle(metaStyle(), previewContentColor(props.config, metaStyle(), "fullMetaHeader", entryAccent(metaStyle(), entry())))}>
                  {fullPreviewMetaHeader(props.config, entry())}
                </text>
              </box>
              <Show when={metaDetailsVisible() && layout().fullPreviewMetaLineSpacing > 0}>
                <box width="100%" height={layout().fullPreviewMetaLineSpacing} backgroundColor={metaStyle().bg} />
              </Show>
              <Show when={metaDetailsVisible()}>
                <box width="100%" flexDirection="row" justifyContent={justifyContent(layout().fullPreviewMetaContentAlign)}>
                  <text style={textStyle(metaStyle(), previewContentColor(props.config, metaStyle(), "fullMetaDetails", metaStyle().fg))}>
                    {fullPreviewMetaDetails(props.config, entry())}
                  </text>
                </box>
              </Show>
            </box>
          </box>
        )}
      </Show>
      <box width="100%" flexGrow={1} flexDirection="column" justifyContent={verticalJustify(layout().fullPreviewBodyVerticalAlign)}>
        <Show when={props.entry?.kind === "image" && blockPreview().kind === "rendered"}>
          <ImagePreviewRows
            preview={blockPreview()}
            renderer={layout().fullPreviewImageRenderer}
            mode={layout().fullPreviewImageMode}
            blockGlyph={layout().fullPreviewImageBlockGlyph}
            align={layout().fullPreviewImageAlign}
            width={textWidth()}
            background={imagePreviewBackground(layout().fullPreviewImageBackground, style().bg)}
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
            <box height={1} flexDirection="row" justifyContent={justifyContent(layout().fullPreviewContentAlign)} backgroundColor={style().bg}>
              <text style={textStyle(style(), previewContentColor(props.config, style(), "fullImageNotice", style().muted))}>{notice()}</text>
            </box>
          )}
        </Show>
        <Show when={imageFallbackVisible()}>
          <box height={1} flexDirection="row" justifyContent={justifyContent(layout().fullPreviewContentAlign)} backgroundColor={style().bg}>
            <text style={textStyle(style())}>
              <span style={textStyle(style(), previewContentColor(props.config, style(), "fullImageFallbackPrefix", style().muted))}>
                {labels().fullImagePreviewFallbackPrefix}
              </span>
              <span style={textStyle(style(), previewContentColor(props.config, style(), "fullImageFallbackSeparator", style().muted))}>
                {labels().fullImagePreviewFallbackSeparator}
              </span>
              <span style={textStyle(style(), previewContentColor(props.config, style(), "fullImageFallbackReason", style().muted))}>
                {(blockPreview() as Extract<ImageBlockPreviewModel, { kind: "fallback" }>).reason}
              </span>
            </text>
          </box>
        </Show>
        <For each={visible()}>
          {(line, index) => (
            <>
              <box height={1} flexDirection="row" backgroundColor={style().bg}>
                <Show when={layout().showFullPreviewGutter}>
                  <text style={textStyle(gutterStyle(), previewContentColor(props.config, gutterStyle(), "fullGutter", gutterStyle().muted))}>
                    {fitGutter(line.gutter, layout().fullPreviewGutterWidth, layout().fullPreviewGutterAlign)}
                  </text>
                  <text style={textStyle(gutterStyle(), previewContentColor(props.config, gutterStyle(), "fullGutterSeparator", gutterStyle().muted))}>
                    {labels().fullPreviewGutterSeparator}
                  </text>
                </Show>
                <box flexGrow={1} flexDirection="row" justifyContent={justifyContent(layout().fullPreviewContentAlign)}>
                  <text style={textStyle(style(), previewLineToneColor("full", props.config, style(), props.entry, line.tone, index()))}>
                    {truncateText(line.text, textWidth(), labels())}
                  </text>
                </box>
              </box>
              <Show when={layout().fullPreviewLineSpacing > 0 && index() < visible().length - 1}>
                <box width="100%" height={layout().fullPreviewLineSpacing} backgroundColor={spacerStyle().bg} />
              </Show>
            </>
          )}
        </For>
      </box>
    </box>
  );
}

function fullPreviewMetaHeader(config: ResolvedTuiConfig, entry: Entry): string {
  const labels = config.labels;
  return formatTemplate(labels.fullPreviewMetaHeaderTemplate, previewMetaTemplateValues(entry, labels, config.layout.fullPreviewMetaHashLength));
}

function fullPreviewMetaDetails(config: ResolvedTuiConfig, entry: Entry): string {
  const labels = config.labels;
  return formatTemplate(labels.fullPreviewMetaDetailsTemplate, previewMetaTemplateValues(entry, labels, config.layout.fullPreviewMetaHashLength));
}

function scrollDirection(event: any): -1 | 1 | null {
  if (event.scroll?.direction === "up") return -1;
  if (event.scroll?.direction === "down") return 1;
  if (event.button === 4) return -1;
  if (event.button === 5) return 1;
  return null;
}


