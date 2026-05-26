import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import {
  imageBlockPreview,
  imageBlockPreviewAsync,
  shouldLoadImageBlockPreviewAsync,
  type ImageBlockPreview as ImageBlockPreviewModel,
  type ImageProtocolCapabilities,
} from "../image-preview";
import { entryAccent, previewMetaTemplateValues, previewModel, truncateText } from "../presentation";
import { visiblePreviewLineCapacity } from "../state";
import type { ContentAlign, VerticalAlign } from "../ui-config";
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

export function PreviewPane(props: {
  config: ResolvedTuiConfig;
  entry: Entry | undefined;
  rows: number;
  width: number;
  widthPercent?: number;
  imageCapabilities?: Partial<ImageProtocolCapabilities>;
}) {
  const style = () => surface(props.config, "preview");
  const gutterStyle = () => surface(props.config, "previewGutter");
  const spacerStyle = () => surface(props.config, "previewSpacer");
  const layout = () => props.config.layout;
  const labels = () => props.config.labels;
  const [loadedBlockPreview, setLoadedBlockPreview] = createSignal<ImageBlockPreviewModel | null>(null);
  const textWidth = () => Math.max(1, props.width - (layout().showPreviewGutter ? layout().previewTextWidthInset : layout().previewPaddingX * 2));
  const blockPreviewRequest = createMemo(() => ({
    entry: props.entry,
    maxWidth: Math.min(textWidth(), layout().imagePreviewMaxWidth),
    maxRows: Math.min(Math.max(2, props.rows - layout().imagePreviewRowInset), layout().imagePreviewMaxRows),
    background: imagePreviewBackground(props.config, style().bg),
    mode: layout().imagePreviewMode,
    labels: labels(),
    blockGlyph: layout().imagePreviewBlockGlyph,
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
  const imageNoticeText = () => imagePreviewNotice(props.config, blockPreview());
  const imageFallbackVisible = () => props.entry?.kind === "image" && blockPreview().kind === "fallback" && layout().imagePreviewMode !== "metadata";
  const imageNoticeSpacingRows = () =>
    props.entry?.kind === "image" && blockPreview().kind === "rendered" && imageNoticeText() ? layout().imagePreviewNoticeSpacing : 0;
  const imagePreviewRows = () => {
    if (props.entry?.kind !== "image") return 0;
    const preview = blockPreview();
    const renderedRows = preview.kind === "rendered" ? preview.native.cellRows : 0;
    return renderedRows + imageNoticeSpacingRows() + (imageNoticeText() ? 1 : 0) + (imageFallbackVisible() ? 1 : 0);
  };
  const metadataRows = () => (layout().showMetadata && props.entry ? layout().previewMetaHeight : 0);
  const visibleTextRows = () =>
    visiblePreviewLineCapacity(Math.min(layout().maxPreviewLines, Math.max(0, props.rows - metadataRows() - imagePreviewRows())), layout().previewLineSpacing);
  const lines = () =>
    previewModel(
      props.entry,
      visibleTextRows(),
      labels(),
      layout(),
    );
  return (
    <box
      width={`${props.widthPercent ?? layout().previewWidthPercent}%`}
      border={props.config.chrome.previewBorder ? true : undefined}
      borderStyle={props.config.chrome.previewBorder ? props.config.chrome.previewBorderStyle : undefined}
      borderColor={props.config.chrome.previewBorder ? splitBorderColor(props.config, style(), props.entry) : undefined}
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
        {(entry) => <PreviewMeta config={props.config} entry={entry()} />}
      </Show>
      <box width="100%" flexGrow={1} flexDirection="column" justifyContent={verticalJustify(layout().previewBodyVerticalAlign)}>
        <Show when={props.entry?.kind === "image" && blockPreview().kind === "rendered"}>
          <ImagePreviewRows
            preview={blockPreview()}
            renderer={layout().imagePreviewRenderer}
            blockGlyph={layout().imagePreviewBlockGlyph}
            align={layout().imagePreviewAlign}
            width={textWidth()}
          />
        </Show>
        <Show when={imageNoticeSpacingRows() > 0}>
          <box width="100%" height={imageNoticeSpacingRows()} backgroundColor={spacerStyle().bg} />
        </Show>
        <Show when={imageNoticeText()}>
          {(notice) => (
            <box height={1} flexDirection="row" justifyContent={justifyContent(layout().previewContentAlign)} backgroundColor={style().bg}>
              <text style={textStyle(style(), splitContentColor(props.config, style(), "splitImageNotice", style().muted))}>{notice()}</text>
            </box>
          )}
        </Show>
        <Show when={imageFallbackVisible()}>
          <box height={1} flexDirection="row" justifyContent={justifyContent(layout().previewContentAlign)} backgroundColor={style().bg}>
            <text style={textStyle(style())}>
              <span style={textStyle(style(), splitContentColor(props.config, style(), "splitImageFallbackPrefix", style().muted))}>
                {labels().splitImagePreviewFallbackPrefix}
              </span>
              <span style={textStyle(style(), splitContentColor(props.config, style(), "splitImageFallbackSeparator", style().muted))}>
                {labels().splitImagePreviewFallbackSeparator}
              </span>
              <span style={textStyle(style(), splitContentColor(props.config, style(), "splitImageFallbackReason", style().muted))}>
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
                  <text style={textStyle(gutterStyle(), splitContentColor(props.config, gutterStyle(), "splitGutter", gutterStyle().muted))}>
                    {fitGutter(line.gutter, layout().previewGutterWidth, layout().previewGutterAlign)}
                  </text>
                  <text style={textStyle(gutterStyle(), splitContentColor(props.config, gutterStyle(), "splitGutterSeparator", gutterStyle().muted))}>
                    {labels().previewGutterSeparator}
                  </text>
                </Show>
                <box flexGrow={1} flexDirection="row" justifyContent={justifyContent(layout().previewContentAlign)}>
                  <text style={textStyle(style(), splitLineToneColor(props.config, style(), props.entry, line.tone, index()))}>
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

function imagePreviewBackground(config: ResolvedTuiConfig, fallback: string): string {
  return config.layout.imagePreviewBackground === "auto" ? fallback : config.layout.imagePreviewBackground;
}

function imagePreviewNotice(config: ResolvedTuiConfig, preview: ImageBlockPreviewModel): string | null {
  if (preview.kind !== "rendered") return null;
  if (config.layout.imagePreviewNoticeVisibility === "never") return null;
  if (preview.notice) return preview.notice;
  if (config.layout.imagePreviewNoticeVisibility === "always") {
    return formatTemplate(config.labels.splitImagePreviewSourceTemplate, { source: preview.source });
  }
  return null;
}

function PreviewMeta(props: { config: ResolvedTuiConfig; entry: Entry }) {
  const style = () => surface(props.config, "previewMeta");
  const labels = () => props.config.labels;
  const values = () => previewMetaTemplateValues(props.entry, labels(), props.config.layout.previewMetaHashLength);
  const header = () =>
    formatTemplate(labels().previewMetaHeaderTemplate, values());
  const details = () =>
    formatTemplate(labels().previewMetaDetailsTemplate, values());
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
          <text style={textStyle(style(), splitContentColor(props.config, style(), "splitMetaHeader", style().accent))}>{header()}</text>
        </box>
        <Show when={detailsVisible() && props.config.layout.previewMetaLineSpacing > 0}>
          <box width="100%" height={props.config.layout.previewMetaLineSpacing} backgroundColor={style().bg} />
        </Show>
        <Show when={detailsVisible()}>
          <box width="100%" flexDirection="row" justifyContent={justifyContent(props.config.layout.previewMetaContentAlign)}>
            <text style={textStyle(style(), splitContentColor(props.config, style(), "splitMetaDetails", style().fg))}>{details()}</text>
          </box>
        </Show>
      </box>
    </box>
  );
}

function justifyContent(align: ContentAlign): "flex-start" | "center" | "flex-end" {
  if (align === "right") return "flex-end";
  if (align === "center") return "center";
  return "flex-start";
}

function verticalJustify(align: VerticalAlign): "flex-start" | "center" | "flex-end" {
  if (align === "bottom") return "flex-end";
  if (align === "center") return "center";
  return "flex-start";
}

function fitGutter(value: string, width: number, align: ContentAlign): string {
  const target = Math.max(1, Math.floor(width));
  const trimmed = value.trim();
  const raw = trimmed.length > 0 ? trimmed : value;
  const clipped = Array.from(raw).slice(0, target).join("");
  const extra = Math.max(0, target - Array.from(clipped).length);
  if (align === "right") return `${" ".repeat(extra)}${clipped}`;
  if (align === "center") {
    const left = Math.floor(extra / 2);
    return `${" ".repeat(left)}${clipped}${" ".repeat(extra - left)}`;
  }
  return `${clipped}${" ".repeat(extra)}`;
}

function previewEntryTitle(config: ResolvedTuiConfig, entry: Entry): string {
  return formatTemplate(config.labels.previewEntryTitleTemplate, {
    entryIdPrefix: config.labels.entryIdPrefix,
    id: entry.id,
  });
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

function splitBorderColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, entry: Entry | undefined): string {
  return splitContentColor(config, style, "splitBorder", entry ? entryAccent(style, entry) : style.border);
}

function splitLineToneColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, entry: Entry | undefined, tone: PreviewLineTone, index: number): string {
  if (!entry) return splitContentColor(config, style, index === 0 ? "splitEmptyTitle" : "splitEmptyHelp", toneColor(style, tone));
  return splitContentColor(config, style, splitLineTonePart(tone), toneColor(style, tone));
}

function splitLineTonePart(tone: PreviewLineTone): TuiPreviewContentPartName {
  switch (tone) {
    case "secondary":
      return "splitSecondary";
    case "muted":
      return "splitMuted";
    case "accent":
      return "splitAccent";
    case "error":
      return "splitError";
    case "success":
      return "splitSuccess";
    default:
      return "splitPrimary";
  }
}

function splitContentColor(config: ResolvedTuiConfig, style: TuiSurfaceStyle, part: TuiPreviewContentPartName, autoColor: string): string {
  return configuredToneColor(style, config.previewContentTones[part], autoColor);
}

function configuredToneColor(style: TuiSurfaceStyle, tone: TuiStatusLineToneName, autoColor: string): string {
  if (tone === "auto") return autoColor;
  return style[tone];
}
