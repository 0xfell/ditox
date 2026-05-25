import type { EntryFilter, ListResponse, RpcResponse } from "./types";

let nextId = 1;

export class RpcError extends Error {
  constructor(
    message: string,
    public readonly code: number,
    options?: ErrorOptions,
  ) {
    super(message, options);
  }
}

export async function listEntries(query: string, filter: EntryFilter): Promise<ListResponse> {
  return call<ListResponse>("entries.list", { query, filter, limit: 100 });
}

export async function addEntry(content: string): Promise<{ id: number }> {
  return call<{ id: number }>("entries.add", { content });
}

export async function copyEntry(id: number): Promise<{ copied: boolean }> {
  return call<{ copied: boolean }>("entries.copy", { id });
}

export async function bulkCopyEntries(ids: number[]): Promise<{ copied: boolean }> {
  return call<{ copied: boolean }>("entries.bulk_copy", { ids });
}

export async function outputEntries(ids: number[]): Promise<{ content: string }> {
  return call<{ content: string }>("entries.output", { ids });
}

export async function pasteEntry(id: number, targetWindow?: string): Promise<{ pasted: boolean }> {
  return call<{ pasted: boolean }>("entries.paste", { id, target_window: targetWindow });
}

export async function deleteEntry(id: number): Promise<{ deleted: boolean }> {
  return call<{ deleted: boolean }>("entries.delete", { id });
}

export async function favoriteEntry(id: number, favorite: boolean): Promise<{ updated: boolean }> {
  return call<{ updated: boolean }>("entries.favorite", { id, favorite });
}

export async function clearEntries(kind: "all" | "text" | "images"): Promise<{ deleted: number }> {
  return call<{ deleted: number }>("entries.clear", { kind });
}

async function call<T>(method: string, params: Record<string, unknown>): Promise<T> {
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
    throw new RpcError(`${ditoxd} not found or not executable`, -32000, { cause: error });
  }

  if (proc.exitCode !== 0) {
    throw new RpcError(formatProcessError(method, proc.stderr?.toString() ?? "", proc.exitCode), -32000);
  }

  const response = parseFrame<RpcResponse<T>>(proc.stdout?.toString() ?? "");
  if ("error" in response) throw new RpcError(formatRpcError(method, response.error.message), response.error.code);
  return response.result;
}

function formatProcessError(method: string, stderr: string, exitCode: number | null): string {
  const text = stderr.trim();
  if (text.includes("FileNotFound") && method.includes("copy")) return "wl-copy was not found or could not be started";
  if (text.includes("FileNotFound") && method.includes("paste")) return "wl-copy or hyprctl was not found";
  if (text.includes("ClipboardWriteFailed")) return "failed to write the clipboard with wl-copy";
  if (text.includes("PasteBackFailed")) return "failed to paste through Hyprland";
  return text || `ditoxd exited with ${exitCode ?? "unknown status"}`;
}

function formatRpcError(method: string, message: string): string {
  if (message === "FileNotFound" && method.includes("copy")) return "wl-copy was not found or could not be started";
  if (message === "FileNotFound" && method.includes("paste")) return "wl-copy or hyprctl was not found";
  if (message === "ClipboardWriteFailed") return "failed to write the clipboard with wl-copy";
  if (message === "PasteBackFailed") return "failed to paste through Hyprland";
  return message;
}

export function parseFrame<T>(raw: string): T {
  const separator = raw.includes("\r\n\r\n") ? "\r\n\r\n" : "\n\n";
  const index = raw.indexOf(separator);
  const body = index >= 0 ? raw.slice(index + separator.length) : raw;
  return JSON.parse(body.trim()) as T;
}
