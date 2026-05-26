import type { EntryFilter, ListResponse, RpcResponse, WatcherStatus } from "./types";
import type { TuiLabels } from "./tui-config";

let nextId = 1;

export type RpcErrorLabelSet = Pick<
  TuiLabels,
  | "errorDitoxdMissing"
  | "errorClipboardToolMissing"
  | "errorPasteToolMissing"
  | "errorClipboardWriteFailed"
  | "errorPasteBackFailed"
  | "errorDitoxdExited"
>;

const defaultErrorLabels: RpcErrorLabelSet = {
  errorDitoxdMissing: "{binary} not found or not executable",
  errorClipboardToolMissing: "wl-copy was not found or could not be started",
  errorPasteToolMissing: "wl-copy or hyprctl was not found",
  errorClipboardWriteFailed: "failed to write the clipboard with wl-copy",
  errorPasteBackFailed: "failed to paste through Hyprland",
  errorDitoxdExited: "ditoxd exited with {status}",
};

export class RpcError extends Error {
  constructor(
    message: string,
    public readonly code: number,
    options?: ErrorOptions,
  ) {
    super(message, options);
  }
}

export async function listEntries(query: string, filter: EntryFilter, limit = 100, labels?: Partial<RpcErrorLabelSet>): Promise<ListResponse> {
  return call<ListResponse>("entries.list", { query, filter, limit }, labels);
}

export async function getWatcherStatus(labels?: Partial<RpcErrorLabelSet>): Promise<WatcherStatus> {
  return call<WatcherStatus>("watcher.status", {}, labels);
}

export async function addEntry(content: string, labels?: Partial<RpcErrorLabelSet>): Promise<{ id: number }> {
  return call<{ id: number }>("entries.add", { content }, labels);
}

export async function copyEntry(id: number, labels?: Partial<RpcErrorLabelSet>): Promise<{ copied: boolean }> {
  return call<{ copied: boolean }>("entries.copy", { id }, labels);
}

export async function bulkCopyEntries(ids: number[], labels?: Partial<RpcErrorLabelSet>): Promise<{ copied: boolean }> {
  return call<{ copied: boolean }>("entries.bulk_copy", { ids }, labels);
}

export async function outputEntries(ids: number[], labels?: Partial<RpcErrorLabelSet>): Promise<{ content: string }> {
  return call<{ content: string }>("entries.output", { ids }, labels);
}

export async function pasteEntry(id: number, targetWindow?: string, labels?: Partial<RpcErrorLabelSet>): Promise<{ pasted: boolean }> {
  return call<{ pasted: boolean }>("entries.paste", { id, target_window: targetWindow }, labels);
}

export async function deleteEntry(id: number, labels?: Partial<RpcErrorLabelSet>): Promise<{ deleted: boolean }> {
  return call<{ deleted: boolean }>("entries.delete", { id }, labels);
}

export async function favoriteEntry(id: number, favorite: boolean, labels?: Partial<RpcErrorLabelSet>): Promise<{ updated: boolean }> {
  return call<{ updated: boolean }>("entries.favorite", { id, favorite }, labels);
}

export async function clearEntries(kind: "all" | "text" | "images", preserveFavorites = false, labels?: Partial<RpcErrorLabelSet>): Promise<{ deleted: number }> {
  return call<{ deleted: number }>("entries.clear", { kind, preserve_favorites: preserveFavorites }, labels);
}

async function call<T>(method: string, params: Record<string, unknown>, labels?: Partial<RpcErrorLabelSet>): Promise<T> {
  const id = `tui-${nextId++}`;
  const body = JSON.stringify({ jsonrpc: "2.0", id, method, params });
  const frame = `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`;
  const ditoxd = Bun.env.DITOXD ?? "ditoxd";

  let proc: ReturnType<typeof Bun.spawnSync>;
  try {
    proc = Bun.spawnSync({
      cmd: [ditoxd, "serve", "--stdio"],
      stdin: Buffer.from(frame),
      stdout: "pipe",
      stderr: "pipe",
    });
  } catch (error) {
    throw new RpcError(formatDitoxdMissing(ditoxd, labels), -32000, { cause: error });
  }

  if (proc.exitCode !== 0) {
    throw new RpcError(formatProcessError(method, proc.stderr?.toString() ?? "", proc.exitCode, labels), -32000);
  }

  const response = parseFrame<RpcResponse<T>>(proc.stdout?.toString() ?? "");
  if ("error" in response) throw new RpcError(formatRpcError(method, response.error.message, labels), response.error.code);
  return response.result;
}

export function formatDitoxdMissing(binary: string, labels?: Partial<RpcErrorLabelSet>): string {
  return errorLabels(labels).errorDitoxdMissing.replaceAll("{binary}", binary);
}

export function formatProcessError(method: string, stderr: string, exitCode: number | null, labels?: Partial<RpcErrorLabelSet>): string {
  const textLabels = errorLabels(labels);
  const text = stderr.trim();
  if (text.includes("FileNotFound") && method.includes("copy")) return textLabels.errorClipboardToolMissing;
  if (text.includes("FileNotFound") && method.includes("paste")) return textLabels.errorPasteToolMissing;
  if (text.includes("ClipboardWriteFailed")) return textLabels.errorClipboardWriteFailed;
  if (text.includes("PasteBackFailed")) return textLabels.errorPasteBackFailed;
  return text || textLabels.errorDitoxdExited.replaceAll("{status}", String(exitCode ?? "unknown status"));
}

export function formatRpcError(method: string, message: string, labels?: Partial<RpcErrorLabelSet>): string {
  const textLabels = errorLabels(labels);
  if (message === "FileNotFound" && method.includes("copy")) return textLabels.errorClipboardToolMissing;
  if (message === "FileNotFound" && method.includes("paste")) return textLabels.errorPasteToolMissing;
  if (message === "ClipboardWriteFailed") return textLabels.errorClipboardWriteFailed;
  if (message === "PasteBackFailed") return textLabels.errorPasteBackFailed;
  return message;
}

function errorLabels(labels: Partial<RpcErrorLabelSet> | undefined): RpcErrorLabelSet {
  return { ...defaultErrorLabels, ...labels };
}

export function parseFrame<T>(raw: string): T {
  const separator = raw.includes("\r\n\r\n") ? "\r\n\r\n" : "\n\n";
  const index = raw.indexOf(separator);
  const body = index >= 0 ? raw.slice(index + separator.length) : raw;
  return JSON.parse(body.trim()) as T;
}
