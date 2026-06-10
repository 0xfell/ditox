import { useRenderer } from "@opentui/solid";
import { createEffect, createMemo, createSignal, onCleanup, type Accessor } from "solid-js";
import {
  imageBlockPreview,
  imageBlockPreviewAsync,
  imageBlockPreviewCached,
  shouldLoadImageBlockPreviewAsync,
  type ImageBlockPreview,
  type ImagePreviewFallbackLabels,
  type ImageProtocolCapabilities,
} from "./image-preview";
import type { Entry } from "./types";
import type { ImagePreviewMode } from "./ui-config";

export type ImageBlockPreviewRequest = {
  entry: Entry | undefined;
  maxWidth: number;
  maxRows: number;
  background: string;
  mode: ImagePreviewMode;
  labels: Partial<ImagePreviewFallbackLabels>;
  blockGlyph: string;
  capabilities: Partial<ImageProtocolCapabilities> | undefined;
  /** Decode delay applied only while the request is changing rapidly (fast
   * navigation); the first request after an idle period decodes immediately. */
  debounceMs?: number;
};

// Refresh polls rebuild the entry list, so the selected entry (and the
// capabilities object) arrive as new object identities every tick even when
// nothing the preview depends on changed. Compare requests by value so the
// decode effect only re-runs for real changes; otherwise every poll would
// re-set the preview signal (flickering async formats through their
// "decoding" placeholder) and re-walk the decode path.
export function sameImageBlockPreviewRequest(a: ImageBlockPreviewRequest, b: ImageBlockPreviewRequest): boolean {
  return (
    sameRequestEntry(a.entry, b.entry) &&
    a.maxWidth === b.maxWidth &&
    a.maxRows === b.maxRows &&
    a.background === b.background &&
    a.mode === b.mode &&
    a.labels === b.labels &&
    a.blockGlyph === b.blockGlyph &&
    a.capabilities?.kittyGraphics === b.capabilities?.kittyGraphics &&
    a.capabilities?.sixel === b.capabilities?.sixel &&
    a.capabilities?.nativeRenderer === b.capabilities?.nativeRenderer &&
    a.debounceMs === b.debounceMs
  );
}

function sameRequestEntry(a: Entry | undefined, b: Entry | undefined): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  return a.id === b.id && a.kind === b.kind && a.hash === b.hash && a.mime === b.mime && a.blob_path === b.blob_path;
}

export function createImageBlockPreview(request: Accessor<ImageBlockPreviewRequest>): Accessor<ImageBlockPreview> {
  const renderer = useRenderer();
  const stableRequest = createMemo(request, undefined, { equals: sameImageBlockPreviewRequest });
  const [loaded, setLoaded] = createSignal<ImageBlockPreview | null>(null);
  let lastRequestAt = 0;
  createEffect(() => {
    const current = stableRequest();
    // Track request churn across ALL entries (text included) so holding an
    // arrow key through a mixed list keeps the rapid-navigation window open.
    const now = Date.now();
    const sinceLastChange = now - lastRequestAt;
    lastRequestAt = now;
    const wantsAsync = shouldLoadImageBlockPreviewAsync(current.entry, current.mode);
    const cached = wantsAsync ? cachedPreview(current) : null;
    setLoaded(cached ?? syncPreview(current));
    if (!wantsAsync || cached) return;

    let disposed = false;
    const startDecode = () => {
      void imageBlockPreviewAsync(
        current.entry,
        current.maxWidth,
        current.maxRows,
        current.background,
        current.mode,
        current.labels,
        current.blockGlyph,
        current.capabilities,
      ).then((preview) => {
        if (!disposed) {
          setLoaded(preview);
          // Nothing else may be scheduled when the decode lands, so ask for a
          // fresh frame explicitly; the placeholder would otherwise linger
          // until an unrelated re-render.
          renderer.requestRender();
        }
      });
    };
    // Decoding blocks this thread for tens of milliseconds on large images.
    // While the user is skimming the list (requests arriving faster than the
    // debounce window) defer the decode; entries only pay once the selection
    // settles. The first request after an idle period decodes immediately.
    const debounceMs = Math.max(0, current.debounceMs ?? 0);
    if (debounceMs > 0 && sinceLastChange < debounceMs) {
      const timer = setTimeout(startDecode, debounceMs);
      onCleanup(() => {
        disposed = true;
        clearTimeout(timer);
      });
      return;
    }
    startDecode();
    onCleanup(() => {
      disposed = true;
    });
  });
  return () => loaded() ?? syncPreview(stableRequest());
}

function syncPreview(request: ImageBlockPreviewRequest): ImageBlockPreview {
  return imageBlockPreview(
    request.entry,
    request.maxWidth,
    request.maxRows,
    request.background,
    request.mode,
    request.labels,
    request.blockGlyph,
    request.capabilities,
  );
}

function cachedPreview(request: ImageBlockPreviewRequest): ImageBlockPreview | null {
  return imageBlockPreviewCached(
    request.entry,
    request.maxWidth,
    request.maxRows,
    request.background,
    request.mode,
    request.labels,
    request.blockGlyph,
    request.capabilities,
  );
}
