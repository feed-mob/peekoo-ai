import { error, warn, info, debug, trace } from "@tauri-apps/plugin-log";

function formatConsoleArgs(args: unknown[]): string {
  if (args.length === 0) return "";
  const first = args[0];
  if (typeof first === "string" && first.includes("%")) {
    let idx = 0;
    const formatted = first.replace(/%[sdiffoO]/g, () => {
      idx++;
      const val = args[idx];
      if (val === undefined) return "";
      if (typeof val === "object") {
        try {
          return JSON.stringify(val);
        } catch {
          return String(val);
        }
      }
      return String(val);
    });
    const remaining = args.slice(idx + 1);
    if (remaining.length > 0) {
      return `${formatted} ${remaining.map((p) => (typeof p === "object" ? JSON.stringify(p) : String(p))).join(" ")}`;
    }
    return formatted;
  }
  return args
    .map((arg) => {
      if (typeof arg === "object") {
        try {
          return JSON.stringify(arg);
        } catch {
          return String(arg);
        }
      }
      return String(arg);
    })
    .join(" ");
}

/**
 * Rewrites `console.*` methods to forward messages through the Tauri log
 * plugin. The original browser console output is preserved, and each
 * message is also sent to the Rust backend where it is persisted to
 * the log file.
 *
 * Call once at application startup, before any other code runs.
 */
export function forwardConsole(): void {
  const originalConsole = {
    log: console.log.bind(console),
    debug: console.debug.bind(console),
    info: console.info.bind(console),
    warn: console.warn.bind(console),
    error: console.error.bind(console),
  };

  console.log = (...args: unknown[]) => {
    originalConsole.log(...args);
    trace(formatConsoleArgs(args)).catch(() => {});
  };
  console.debug = (...args: unknown[]) => {
    originalConsole.debug(...args);
    debug(formatConsoleArgs(args)).catch(() => {});
  };
  console.info = (...args: unknown[]) => {
    originalConsole.info(...args);
    info(formatConsoleArgs(args)).catch(() => {});
  };
  console.warn = (...args: unknown[]) => {
    originalConsole.warn(...args);
    warn(formatConsoleArgs(args)).catch(() => {});
  };
  console.error = (...args: unknown[]) => {
    originalConsole.error(...args);
    error(formatConsoleArgs(args)).catch(() => {});
  };
}
