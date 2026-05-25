import type { EntryFilter, ListResponse, RpcResponse } from "./types";

let nextId = 1;

export class RpcError extends Error {
  constructor(
    message: string,
    public readonly code: number,
  ) {
    super(message);
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

export async function pasteEntry(id: number, targetWindow?: string): Promise<{ pasted: boolean }> {
  return call<{ pasted: boolean }>("entries.paste", { id, target_window: targetWindow });
}

export async function deleteEntry(id: number): Promise<{ deleted: boolean }> {
  return call<{ deleted: boolean }>("entries.delete", { id });
}

export async function favoriteEntry(id: number, favorite: boolean): Promise<{ updated: boolean }> {
  return call<{ updated: boolean }>("entries.favorite", { id, favorite });
}

async function call<T>(method: string, params: Record<string, unknown>): Promise<T> {
  const id = `tui-${nextId++}`;
  const body = JSON.stringify({ jsonrpc: "2.0", id, method, params });
  const frame = `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`;
  const ditoxd = Bun.env.DITOXD ?? "ditoxd";

  const proc = Bun.spawnSync({
    cmd: [ditoxd, "serve", "--stdio"],
    stdin: Buffer.from(frame),
    stdout: "pipe",
    stderr: "pipe",
  });

  if (proc.exitCode !== 0) {
    throw new RpcError(proc.stderr.toString() || `ditoxd exited with ${proc.exitCode}`, -32000);
  }

  const response = parseFrame<RpcResponse<T>>(proc.stdout.toString());
  if ("error" in response) throw new RpcError(response.error.message, response.error.code);
  return response.result;
}

export function parseFrame<T>(raw: string): T {
  const separator = raw.includes("\r\n\r\n") ? "\r\n\r\n" : "\n\n";
  const index = raw.indexOf(separator);
  const body = index >= 0 ? raw.slice(index + separator.length) : raw;
  return JSON.parse(body.trim()) as T;
}
