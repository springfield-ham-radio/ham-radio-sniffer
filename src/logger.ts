import type { ILogLayer } from 'loglayer';
import { ConsoleTransport, LogLayer } from 'loglayer';

/**
 * Creates the default console logger used when callers do not inject one.
 */
export function createDefaultLogger(): ILogLayer {
  return new LogLayer({
    transport: [
      new ConsoleTransport({
        logger: console,
        level: 'info',
      }),
    ],
  });
}
