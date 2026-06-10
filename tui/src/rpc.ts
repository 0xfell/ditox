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
  | "errorUnknownStatus"
  | "errorProcessTemplate"
  | "errorRpcTemplate"
>;

const defaultErrorLabels: RpcErrorLabelSet = {
  errorDitoxdMissing: "{binary} not found or not executable",
  errorClipboardToolMissing: "wl-copy was not found or could not be started",
  errorPasteToolMissing: "wl-copy or hyprctl was not found",
  errorClipboardWriteFailed: "failed to write the clipboard with wl-copy",
  errorPasteBackFailed: "failed to paste through Hyprland",
  errorDitoxdExited: "ditoxd exited with {status}",
  errorUnknownStatus: "unknown status",
  errorProcessTemplate: "{message}",
  errorRpcTemplate: "{message}",
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

export type ImageBytesResponse = {
  mime: string;
  data: Uint8Array;
  width: number | null;
  height: number | null;
};

export async function getImageBytes(id: number, labels?: Partial<RpcErrorLabelSet>): Promise<ImageBytesResponse> {
  const result = await call<{ mime: string; data: string; width: number | null; height: number | null }>("entries.get_image", { id }, labels);
  return { ...result, data: Uint8Array.from(Buffer.from(result.data, "base64")) };
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

type PendingRequest = {
  method: string;
  labels?: Partial<RpcErrorLabelSet>;
  resolve: (body: string) => void;
  reject: (error: Error) => void;
};

// One long-lived `ditoxd serve --stdio` session per TUI process. Requests are
// written as Content-Length frames and the server answers strictly in order,
// so responses are matched FIFO. Spawning a fresh process per call (the old
// model) paid process startup on every keystroke action.
class RpcConnection {
  private buffer: Buffer = Buffer.alloc(0);
  private readonly queue: PendingRequest[] = [];
  private stderrText = "";
  private failure: { exitCode: number | null } | null = null;
  private readonly proc: Bun.Subprocess<"pipe", "pipe", "pipe">;

  constructor(binary: string) {
    this.proc = Bun.spawn([binary, "serve", "--stdio"], {
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
    });
    // The session must never keep the TUI process alive: on exit the pipe
    // closes and the server quits on stdin EOF.
    this.proc.unref();
    void this.readStdout();
    void this.readStderr();
    void this.proc.exited.then((exitCode) => this.handleExit(exitCode));
  }

  get alive(): boolean {
    return this.failure === null;
  }

  request(method: string, body: string, labels?: Partial<RpcErrorLabelSet>): Promise<string> {
    return new Promise<string>((resolve, reject) => {
      if (this.failure) {
        reject(new RpcError(formatProcessError(method, this.stderrText, this.failure.exitCode, labels), -32000));
        return;
      }
      const pending: PendingRequest = { method, labels, resolve, reject };
      this.queue.push(pending);
      try {
        this.proc.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
        void this.proc.stdin.flush();
      } catch (error) {
        const index = this.queue.indexOf(pending);
        if (index >= 0) this.queue.splice(index, 1);
        reject(new RpcError(formatProcessError(method, this.stderrText, this.proc.exitCode, labels), -32000, { cause: error }));
      }
    });
  }

  private async readStdout(): Promise<void> {
    try {
      for await (const chunk of this.proc.stdout) {
        this.buffer = Buffer.concat([this.buffer, Buffer.from(chunk)]);
        this.drainFrames();
      }
    } catch {
      // Stream closed; handleExit settles any remaining requests.
    }
  }

  private drainFrames(): void {
    while (true) {
      const frame = extractResponseFrame(this.buffer);
      if (!frame) return;
      this.buffer = this.buffer.subarray(frame.consumed);
      this.queue.shift()?.resolve(frame.body);
    }
  }

  private async readStderr(): Promise<void> {
    try {
      for await (const chunk of this.proc.stderr) this.stderrText += Buffer.from(chunk).toString();
    } catch {
      // Stream closed.
    }
  }

  private handleExit(exitCode: number | null): void {
    this.failure = { exitCode };
    for (const pending of this.queue.splice(0)) {
      pending.reject(new RpcError(formatProcessError(pending.method, this.stderrText, exitCode, pending.labels), -32000));
    }
    if (connection === this) connection = null;
  }
}

let connection: RpcConnection | null = null;

function getConnection(labels?: Partial<RpcErrorLabelSet>): RpcConnection {
  if (connection?.alive) return connection;
  const binary = Bun.env.DITOXD ?? "ditoxd";
  try {
    connection = new RpcConnection(binary);
  } catch (error) {
    connection = null;
    throw new RpcError(formatDitoxdMissing(binary, labels), -32000, { cause: error });
  }
  return connection;
}

function extractResponseFrame(buffer: Buffer): { body: string; consumed: number } | null {
  const headerEnd = buffer.indexOf("\r\n\r\n");
  if (headerEnd < 0) return null;
  const match = /content-length:\s*(\d+)/i.exec(buffer.subarray(0, headerEnd).toString());
  if (!match) return null;
  const length = Number.parseInt(match[1]!, 10);
  const bodyStart = headerEnd + 4;
  if (buffer.length < bodyStart + length) return null;
  return { body: buffer.subarray(bodyStart, bodyStart + length).toString(), consumed: bodyStart + length };
}

async function call<T>(method: string, params: Record<string, unknown>, labels?: Partial<RpcErrorLabelSet>): Promise<T> {
  const id = `tui-${nextId++}`;
  const body = JSON.stringify({ jsonrpc: "2.0", id, method, params });
  const raw = await getConnection(labels).request(method, body, labels);
  const response = JSON.parse(raw) as RpcResponse<T>;
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
  const status = String(exitCode ?? textLabels.errorUnknownStatus);
  const message = text || textLabels.errorDitoxdExited.replaceAll("{status}", status);
  return formatErrorTemplate(textLabels.errorProcessTemplate, { method, message, status });
}

export function formatRpcError(method: string, message: string, labels?: Partial<RpcErrorLabelSet>): string {
  const textLabels = errorLabels(labels);
  if (message === "FileNotFound" && method.includes("copy")) return textLabels.errorClipboardToolMissing;
  if (message === "FileNotFound" && method.includes("paste")) return textLabels.errorPasteToolMissing;
  if (message === "ClipboardWriteFailed") return textLabels.errorClipboardWriteFailed;
  if (message === "PasteBackFailed") return textLabels.errorPasteBackFailed;
  return formatErrorTemplate(textLabels.errorRpcTemplate, { method, message });
}

function errorLabels(labels: Partial<RpcErrorLabelSet> | undefined): RpcErrorLabelSet {
  return { ...defaultErrorLabels, ...labels };
}

function formatErrorTemplate(template: string, values: Record<string, string>): string {
  return Object.entries(values).reduce((output, [key, value]) => output.replaceAll(`{${key}}`, value), template);
}

export function parseFrame<T>(raw: string): T {
  const separator = raw.includes("\r\n\r\n") ? "\r\n\r\n" : "\n\n";
  const index = raw.indexOf(separator);
  const body = index >= 0 ? raw.slice(index + separator.length) : raw;
  return JSON.parse(body.trim()) as T;
}
