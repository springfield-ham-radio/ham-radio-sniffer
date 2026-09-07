import type { ILogLayer } from 'loglayer';
import { ConsoleTransport, LogLayer } from 'loglayer';

export type SnifferLogLevel = 'debug' | 'info' | 'warn' | 'error';

const LOG_LEVELS = new Set<string>(['debug', 'info', 'warn', 'error']);

/**
 * Resolve the sniffer console log level from `SNIFFER_LOG_LEVEL` or `LOG_LEVEL`.
 *
 * `debug` logs every serial chunk as hex. `info` logs port open/close and the
 * first bytes seen on each port so a silent UART is obvious without debug.
 */
export function resolveSnifferLogLevel(value: string | undefined = process.env.SNIFFER_LOG_LEVEL ?? process.env.LOG_LEVEL): SnifferLogLevel {
  const normalized = value?.trim().toLowerCase();

  if (normalized === 'trace') {
    return 'debug';
  }

  if (normalized === 'warning') {
    return 'warn';
  }

  if (normalized === 'fatal') {
    return 'error';
  }

  if (normalized && LOG_LEVELS.has(normalized)) {
    return normalized as SnifferLogLevel;
  }

  return 'info';
}

/**
 * Creates the default console logger used when callers do not inject one.
 */
export function createDefaultLogger(): ILogLayer {
  const level = resolveSnifferLogLevel();

  return new LogLayer({
    transport: [
      new ConsoleTransport({
        logger: console,
        level,
      }),
    ],
  });
}
