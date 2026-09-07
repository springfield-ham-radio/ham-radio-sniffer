import { EventEmitter } from 'node:events';
import type { BridgedSerialPort } from '../../src/radio-sniffer.ts';

export class FakeSerialPort extends EventEmitter implements BridgedSerialPort {
  readonly path: string;
  isOpen = true;
  readonly written: Buffer[] = [];
  readonly parser = new EventEmitter();
  signals: { rts?: boolean; dtr?: boolean } | undefined;

  constructor(path: string) {
    super();
    this.path = path;
  }

  set(signals: { rts?: boolean; dtr?: boolean }, callback?: (error?: Error | null) => void): void {
    this.signals = { rts: signals.rts, dtr: signals.dtr };
    callback?.(null);
  }

  pipe(_transform: unknown): { on(event: 'data', listener: (data: Buffer) => void): unknown } {
    return this.parser;
  }

  write(data: Buffer | Uint8Array, callback?: (error?: Error | null) => void): boolean {
    this.written.push(Buffer.from(data));
    callback?.(null);
    return true;
  }

  close(_callback?: (error?: Error | null) => void): void {
    this.isOpen = false;
    this.emit('close');
  }

  emitData(data: Buffer | number[]): void {
    this.parser.emit('data', Buffer.from(data));
  }
}

export class FakeTrafficLogger {
  readonly sent: Uint8Array[] = [];
  readonly received: Uint8Array[] = [];
  closed = false;
  readonly logFilePath: string;

  constructor(logFilePath = 'test-sniffer.json') {
    this.logFilePath = logFilePath;
  }

  logSend(data: Uint8Array): void {
    this.sent.push(data);
  }

  logReceive(data: Uint8Array): void {
    this.received.push(data);
  }

  close(): void {
    this.closed = true;
  }

  getLogFilePath(): string {
    return this.logFilePath;
  }

  getLogData(): unknown {
    return {
      entries: [
        ...this.sent.map((data) => ({ direction: 'SEND', data: Array.from(data) })),
        ...this.received.map((data) => ({ direction: 'RECV', data: Array.from(data) })),
      ],
    };
  }
}
