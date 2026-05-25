import type { Entry, EntryFilter } from "./types";
import type { TuiTheme } from "./theme";

export type PreviewLine = {
  gutter: string;
  text: string;
  tone: "primary" | "secondary" | "muted" | "accent" | "error" | "success";
};

export function entryAccent(theme: TuiTheme, entry: Entry): string {
  if (entry.favorite) return theme.accentFavorite;
  if (entry.kind === "image") return theme.accentImage;
  return theme.accentText;
}

export function entryKindLabel(entry: Entry): string {
  return entry.kind === "image" ? "IMG" : "TXT";
}

export function entryMeta(entry: Entry, now = Date.now()): string {
  const age = formatAge(entry.created_at_ms, now).padStart(2, " ");
  const size = formatBytes(entry.byte_len).padStart(6, " ");
  const favorite = entry.favorite ? " PIN" : "    ";
  return `${entryKindLabel(entry)} ${age} ${size}${favorite}`;
}

export function entryPreview(entry: Entry, width: number): string {
  return truncateText(entry.preview || entry.content || "(empty)", width);
}

export function statusTone(status: string): "muted" | "error" | "success" | "warning" {
  const value = status.toLowerCase();
  if (value.includes("error") || value.includes("failed") || value.includes("not found")) return "error";
  if (value.includes("copied") || value.includes("pasted") || value.includes("cleared")) return "success";
  if (value.includes("paused")) return "warning";
  return "muted";
}

export function formatFilter(filter: EntryFilter): string {
  return filter.toUpperCase();
}

export function previewModel(entry: Entry | undefined, maxLines: number): PreviewLine[] {
  if (!entry) {
    return [
      { gutter: "", text: "No entry selected", tone: "muted" },
      { gutter: "", text: "Copy something or use `ditox add` to seed history.", tone: "secondary" },
    ];
  }

  if (entry.kind === "image") {
    const dimensions =
      entry.image_width !== null && entry.image_height !== null ? `${entry.image_width}x${entry.image_height}` : "unknown";
    const rows: PreviewLine[] = [
      { gutter: "type", text: "Image entry", tone: "accent" },
      { gutter: "mime", text: entry.mime, tone: "primary" },
      { gutter: "size", text: formatBytes(entry.byte_len), tone: "primary" },
      { gutter: "dims", text: dimensions, tone: "primary" },
      { gutter: "hash", text: entry.hash, tone: "secondary" },
      { gutter: "blob", text: entry.blob_path ?? "not stored", tone: entry.blob_path ? "secondary" : "muted" },
    ];
    return rows.slice(0, maxLines);
  }

  const lines = entry.content.split(/\r?\n/);
  if (lines.length === 0) return [{ gutter: "1", text: "", tone: "primary" }];
  return lines.slice(0, maxLines).map<PreviewLine>((line, index) => ({
    gutter: String(index + 1).padStart(3, " "),
    text: line,
    tone: "primary",
  }));
}

export function scrollbarCells(total: number, selectedIndex: number, rows: number): string[] {
  if (rows <= 0) return [];
  if (total <= rows) return Array.from({ length: rows }, () => " ");
  const thumbSize = Math.max(1, Math.floor((rows / total) * rows));
  const maxStart = rows - thumbSize;
  const start = Math.round((selectedIndex / Math.max(1, total - 1)) * maxStart);
  return Array.from({ length: rows }, (_, index) => (index >= start && index < start + thumbSize ? "#" : "|"));
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kib = bytes / 1024;
  if (kib < 1024) return `${kib.toFixed(kib >= 10 ? 0 : 1)} KiB`;
  const mib = kib / 1024;
  return `${mib.toFixed(mib >= 10 ? 0 : 1)} MiB`;
}

export function formatAge(timestampMs: number, now = Date.now()): string {
  const seconds = Math.max(0, Math.floor((now - timestampMs) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

export function truncateText(value: string, width: number): string {
  if (width <= 0) return "";
  const clean = value.replace(/\s+/g, " ");
  if (clean.length <= width) return clean;
  if (width <= 3) return ".".repeat(width);
  return `${clean.slice(0, width - 3)}...`;
}
