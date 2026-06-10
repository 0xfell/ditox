import type { CliRenderer } from "@opentui/core";

type TerminalWriter = {
  isTTY?: boolean;
  write: (chunk: string) => unknown;
};

type ShutdownTuiOptions = {
  code?: number;
  stdout?: TerminalWriter;
  exit?: (code: number) => void;
  schedule?: (callback: () => void) => unknown;
  afterDestroy?: () => void;
};

export const terminalExitResetSequence = "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1015l\x1b[?1004l\x1b[?2004l\x1b[?25h\x1b[0m";

let terminalExitResetInstalled = false;
let terminalExitResetWritten = false;
let tuiShutdownStarted = false;

export function writeTerminalExitReset(stdout: TerminalWriter = process.stdout): void {
  if (terminalExitResetWritten) return;
  if (!stdout.isTTY) return;
  stdout.write(terminalExitResetSequence);
  terminalExitResetWritten = true;
}

export function installTerminalExitReset(stdout: TerminalWriter = process.stdout): void {
  if (terminalExitResetInstalled) return;
  terminalExitResetInstalled = true;
  process.once("exit", () => writeTerminalExitReset(stdout));
}

export function shutdownTui(renderer: Pick<CliRenderer, "destroy">, options: ShutdownTuiOptions = {}): void {
  if (tuiShutdownStarted) return;
  tuiShutdownStarted = true;
  const stdout = options.stdout ?? process.stdout;
  try {
    renderer.destroy();
  } finally {
    writeTerminalExitReset(stdout);
    options.afterDestroy?.();
    const exit = options.exit ?? process.exit;
    const schedule = options.schedule ?? ((callback: () => void) => setTimeout(callback, 0));
    schedule(() => exit(options.code ?? 0));
  }
}

export function exitAfter(enabled: boolean, renderer: CliRenderer): void {
  if (!enabled) return;
  setTimeout(() => shutdownTui(renderer), 0);
}
