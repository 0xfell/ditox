import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import {
  imageBlockPreview,
  imageBlockPreviewAsync,
  shouldLoadImageBlockPreviewAsync,
  type ImageBlockPreview as ImageBlockPreviewModel,
  type ImageProtocolCapabilities,
} from "../image-preview";
import { entryAccent, previewMetaTemplateValues, previewModel, previewWindow, truncateText } from "../presentation";
import { visibleFullPreviewLineCapacity } from "../state";
import {
  formatTemplate,
  paddedTitle,
  surface,
  textStyle,
  type ResolvedTuiConfig,
  type TuiPreviewContentPartName,
  type TuiStatusLineToneName,
  type TuiSurfaceStyle,
} from "../tui-config";
import type { Entry } from "../types";
import { ImagePreviewRows } from "./ImagePreviewRows";

export function FullPreview(props: {
  config: ResolvedTuiConfig;
  entry: Entry | undefined;
  rows: number;
  width: number;
  offset: number;
  imageCapabilities?: Partial<ImageProtocolCapabilities>;
  onScroll: (direction: -1 | 1) => void;
}) {
  const style = () => surface(props.config, "fullPreview");
  const gutterStyle = () => surface(props.config, "fullPreviewGutter");
  const metaStyle = () => surface(props.config, "fullPreviewMeta");
  const spacerStyle = () => surface(props.config, "fullPreviewSpacer");
  const labels = () => props.config.labels;
  const layout = () => props.config.layout;
  const [loadedBlockPreview, setLoadedBlockPreview] = createSignal<ImageBlockPreviewModel | null>(null);
  const textWidth = () => Math.max(1, props.width - (layout().showFullPreviewGutter ? layout().fullPreviewTextWidthInset : layout().fullPreviewPaddingX * 2));
  const blockPreviewRequest = createMemo(() => ({
    entry: props.entry,
    maxWidth: Math.min(textWidth(), layout().fullPreviewImageMaxWidth),
    maxRows: Math.min(Math.max(2, props.rows - layout().fullPreviewImageRowInset), layout().fullPreviewImageMaxRows),
    background: imagePreviewBackground(props.config, style().bg),
    mode: layout().fullPreviewImageMode,
    labels: labels(),
    blockGlyph: layout().fullPreviewImageBlockGlyph,
    capabilities: props.imageCapabilities,
  }));
  createEffect(() => {
    const request = blockPreviewRequest();
    const syncPreview = imageBlockPreview(
      request.entry,
      request.maxWidth,
      request.maxRows,
      request.background,
      request.mode,
      request.labels,
      request.blockGlyph,
      request.capabilities,
    );
    setLoadedBlockPreview(syncPreview);
    if (!shouldLoadImageBlockPreviewAsync(request.entry, request.mode)) return;

    let disposed = false;
    void imageBlockPreviewAsync(
      request.entry,
      request.maxWidth,
      request.maxRows,
      request.background,
      request.mode,
      request.labels,
      request.blockGlyph,
      request.capabilities,
    ).then((preview) => {
      if (!disposed) setLoadedBlockPreview(preview);
    });
    onCleanup(() => {
      disposed = true;
    });
  });
  const blockPreview = () => {
    const request = blockPreviewRequest();
    return (
      loadedBlockPreview() ??
      imageBlockPreview(request.entry, request.maxWidth, request.maxRows, request.background, request.mode, request.labels, request.blockGlyph, request.capabilities)
    );
  };
  const lines = () =>
    previewModel(props.entry, layout().maxFullPreviewLines, labels(), layout());
  const imageNoticeText = () => imagePreviewNotice(props.config, blockPreview());
  const imageFallbackVisible = () => props.entry?.kind === "image" && blockPreview().kind === "fallback" && layout().fullPreviewImageMode !== "metadata";
  const imagePreviewRows = () => {
    if (props.entry?.kind !== "image") return 0;
    const preview = blockPreview();
    const renderedRows = preview.kind === "rendered" ? preview.native.cellRows : 0;
    return renderedRows + (imageNoticeText() ? 1 : 0) + (imageFallbackVisible() ? 1 : 0);
  };
  const metadataRows = () => (layout().showFullPreviewMetadata && props.entry ? layout().fullPreviewMetaHeight : 0);
  const visibleRows = () => visibleFullPreviewLineCapacity(props.rows, layout().fullPreviewLineSpacing, layout().fullPreviewScrollInsetRows, metadataRows() + imagePreviewRows());
  const visible = () => previewWindow(lines(), props.offset, visibleRows());
  const bottomTitle = () => {
    if (!props.entry) return paddedTitle(labels().previewBackHint, layout().frameTitlePadding);
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
      layout().frameTitlePadding,
    );
  };
  return (
    <box
      flexGrow={1}
      border={props.config.chrome.fullPreviewBorder ? true : undefined}
      borderStyle={props.config.chrome.fullPreviewBorder ? props.config.chrome.fullPreviewBorderStyle : undefined}
      borderColor={props.config.chrome.fullPreviewBorder ? fullBorderColor(props.config, style(), props.entry) : undefined}
      backgroundColor={style().bg}
      paddingX={layout().fullPreviewPaddingX}
      paddingY={layout().fullPreviewPaddingY}
      title={props.config.chrome.fullPreviewBorder && props.config.chrome.showFullPreviewTitle ? paddedTitle(labels().previewModeTitle, layout().frameTitlePadding) : undefined}
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
          <box height={layout().fullPreviewMetaHeight} backgroundColor={metaStyle().bg} paddingX={layout().fullPreviewMetaPaddingX} paddingY={layout().fullPreviewMetaPaddingY}>
            <text style={textStyle(metaStyle(), fullContentColor(props.config, metaStyle(), "fullMeta", entryAccent(metaStyle(), entry())))}>{fullPreviewMeta(props.config, entry())}</text>
          </box>
        )}
      </Show>
      <Show when={props.entry?.kind === "image" && blockPreview().kind === "rendered"}>
        <ImagePreviewRows preview={blockPreview()} renderer={layout().fullPreviewImageRenderer} blockGlyph={layout().fullPreviewImageBlockGlyph} />
      </Show>
      <Show when={imageNoticeText()}>
        {(notice) => (
          <box height={1} flexDirection="row" backgroundColor={style().bg}>
            <text style={textStyle(style(), fullContentColor(props.config, style(), "fullImageNotice", style().muted))}>{notice()}</text>
          </box>
        )}
      </Show>
      <Show when={imageFallbackVisible()}>
        <text style={textStyle(style())}>
          <span style={textStyle(style(), fullContentColor(props.config, style(), "fullImageFallbackPrefix", style().muted))}>
            {labels().fullImagePreviewFallbackPrefix}
          </span>
          <span style={textStyle(style(), fullContentColor(props.config, style(), "fullImageFallbackSeparator", style().muted))}>
            {labels().fullImagePreviewFallbackSeparator}
          </span>
          <span style={textStyle(style(), fullContentColor(props.config, style(), "fullImageFallbackReason", style().muted))}>
            {(blockPreview() as Extract<ImageBlockPreviewModel, { kind: "fallback" }>).reason}
          </span>
        </text>
      </Show>
      <For each={visible()}>
        {(line, index) => (
          <>
            <box height={1} flexDirection="row" backgroundColor={style().bg}>
              <Show when={layout().showFullPreviewGutter}>
                <text style={textStyle(gutterStyle(), fullContentColor(props.config, gutterStyle(), "fullGutter", gutterStyle().muted))}>
                  {line.gutter.padStart(layout().fullPreviewGutterWidth, " ")}
                </text>
                <text style={textStyle(gutterStyle(), fullContentColor(props.config, gutterStyle(), "fullGutterSeparator", gutterStyle().muted))}>
                  {labels().fullPreviewGutterSeparator}
                </text>
              </Show>
              <text style={textStyle(style(), fullLineToneColor(props.config, style(), props.entry, line.tone, index()))}>
                {truncateText(line.text, textWidth(), labels())}
              </text>
            </box>
            <Show when={layout().fullPreviewLineSpacing > 0 && index() < visible().length - 1}>
              <box width="100%" height={layout().fullPreviewLineSpacing} backgroundColor={spacerStyle().bg} />
            </Show>
          </>
        )}
      </For>
    </box>
  );
}

function fullPreviewMeta(config: ResolvedTuiConfig, entry: Entry): string {
  const labels = config.labels;
  return formatTemplate(labels.fullPreviewMetaTemplate, previewMetaTemplateValues(entry, labels, config.layout.previewMetaHashLength));
}

function imagePreviewBackground(config: ResolvedTuiConfig, fallback: string): string {
  return config.layout.fullPreviewImageBackground === "auto" ? fallback : config.layout.fullPreviewImageBackground;
}

function imagePreviewNotice(config: ResolvedTuiConfig, preview: ImageBlockPreviewModel): string | null {
  if (preview.kind !== "rendered") return null;
  if (config.layout.fullPreviewImageNoticeVisibility === "never") return null;
  if (preview.notice) return preview.notice;
  if (config.layout.fullPreviewImageNoticeVisibility === "always") {
    return formatTemplate(config.labels.fullImagePreviewSourceTemplate, { source: preview.source });
  }
  return null;
}

function scrollDirection(event: any): -1 | 1 | null {
  if (event.scroll?.direction === "up") return -1;
  if (event.scroll?.direction === "down") return 1;
  if (event.button === 4) return -1;
  if (event.button === 5) return 1;
  return null;
}

function toneColor(style: TuiSurfaceStyle, tone: "primary" | "secondary" | "muted" | "accent" | "error" | "success"): string {
  switch (tone) {
    case "secondary":
      return style.secondary;
    case "muted":
      return style.muted;
    case "accent":
      return style.accent;
    case "error":
      return style.error;
    case "success":
      return style.success;
    default:
      return style.fg;
  }
}

type PreviewLineTone = "primary" | "secondary" | "muted" | "accent" | "error" | "success";

function fullBorderColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, entry: Entry | undefined): string {
  return fullContentColor(config, style, "fullBorder", entry ? entryAccent(style, entry) : style.border);
}

function fullLineToneColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, entry: Entry | undefined, tone: PreviewLineTone, index: number): string {
  if (!entry) return fullContentColor(config, style, index === 0 ? "fullEmptyTitle" : "fullEmptyHelp", toneColor(style, tone));
  return fullContentColor(config, style, fullLineTonePart(tone), toneColor(style, tone));
}

function fullLineTonePart(tone: PreviewLineTone): TuiPreviewContentPartName {
  switch (tone) {
    case "secondary":
      return "fullSecondary";
    case "muted":
      return "fullMuted";
    case "accent":
      return "fullAccent";
    case "error":
      return "fullError";
    case "success":
      return "fullSuccess";
    default:
      return "fullPrimary";
  }
}

function fullContentColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, part: TuiPreviewContentPartName, autoColor: string): string {
  return configuredToneColor(style, config.previewContentTones[part], autoColor);
}

function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}
