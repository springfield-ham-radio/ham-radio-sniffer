import { EventEmitter } from 'node:events';
import { ByteLengthParser } from '@serialport/parser-byte-length';
import { SerialLogger } from '@springfield/ham-radio-driver';
import type { ILogLayer } from 'loglayer';
import { SerialPort } from 'serialport';
import { createDefaultLogger } from './logger.ts';
import type { SnifferDirection, SnifferPacket } from '../shared/types/sniffer.ts';

export interface BridgedSerialPort {
  readonly isOpen: boolean;
  pipe(transform: unknown): { on(event: 'data', listener: (data: Buffer) => void): unknown };
  write(data: Buffer | Uint8Array): boolean;
  close(callback?: (error?: Error | null) => void): void;
  on(event: 'error' | 'open', listener: (...args: unknown[]) => void): unknown;
}

export type SerialPortFactory = (options: { path: string; baudRate: number }) => BridgedSerialPort;

export interface SnifferTrafficLogger {
  logSend(data: Uint8Array, description?: string): void;
  logReceive(data: Uint8Array, description?: string): void;
  close(): void;
  getLogFilePath(): string;
  getLogData(): unknown;
}

export interface RadioSnifferOptions {
  computerPort: string;
  radioPort: string;
  baudRate?: number;
  logFile?: string;
  logger?: ILogLayer;
  serialPortFactory?: SerialPortFactory;
  trafficLogger?: SnifferTrafficLogger;
  packetIdleMs?: number;
}

export interface RadioSnifferEvents {
  packet: [packet: Omit<SnifferPacket, 'id'>];
  portError: [error: Error, source: 'computer' | 'radio'];
}

function createSerialPort({ path, baudRate }: { path: string; baudRate: number }): BridgedSerialPort {
  return new SerialPort({ path, baudRate });
}

/**
 * Bridges two serial ports and emits coalesced packets for live inspection.
 *
 * Byte-length parsing still forwards every byte immediately so timing of the
 * clone protocol is preserved. Packet events are coalesced on direction change
 * or idle so the web API and UI can render readable frames instead of one
 * event per byte.
 */
export class RadioSniffer extends EventEmitter<RadioSnifferEvents> {
  private computerPort: BridgedSerialPort | undefined;
  private radioPort: BridgedSerialPort | undefined;
  private readonly options: RadioSnifferOptions;
  private readonly logger: ILogLayer;
  private readonly serialPortFactory: SerialPortFactory;
  private readonly trafficLogger: SnifferTrafficLogger;
  private readonly packetIdleMs: number;
  private readonly startedAt: number;
  private pendingDirection: SnifferDirection | undefined;
  private pendingBytes: number[] = [];
  private pendingStartedAt = 0;
  private pendingDescription: string | undefined;
  private idleTimer: ReturnType<typeof setTimeout> | undefined;
  private running = false;

  constructor(options: RadioSnifferOptions) {
    super();
    this.options = options;
    this.logger = options.logger ?? createDefaultLogger();
    this.serialPortFactory = options.serialPortFactory ?? createSerialPort;
    this.trafficLogger = options.trafficLogger ?? new SerialLogger(options.logFile ?? this.generateLogFileName());
    this.packetIdleMs = options.packetIdleMs ?? 15;
    this.startedAt = Date.now();
  }

  public getLogFilePath(): string {
    return this.trafficLogger.getLogFilePath();
  }

  public getLogData(): unknown {
    return this.trafficLogger.getLogData();
  }

  public start(): void {
    if (this.running) {
      return;
    }

    const baudRate = this.options.baudRate ?? 9600;

    this.logger
      .withMetadata({
        logFile: this.getLogFilePath(),
        computerPort: this.options.computerPort,
        radioPort: this.options.radioPort,
        baudRate,
      })
      .info('Sniffer started');

    this.computerPort = this.openPort(this.options.computerPort, baudRate, 'computer');
    this.radioPort = this.openPort(this.options.radioPort, baudRate, 'radio');
    this.setupDataHandlers();
    this.running = true;
  }

  public stop(): void {
    if (!this.running && !this.computerPort && !this.radioPort) {
      return;
    }

    this.logger.info('Stopping sniffer...');
    this.flushPendingPacket();
    this.clearIdleTimer();
    this.trafficLogger.close();
    this.closePort(this.computerPort);
    this.closePort(this.radioPort);
    this.computerPort = undefined;
    this.radioPort = undefined;
    this.running = false;
  }

  private openPort(path: string, baudRate: number, source: 'computer' | 'radio'): BridgedSerialPort {
    this.logger.withMetadata({ port: path, baudRate }).debug(`Opening ${source} port`);

    const port = this.serialPortFactory({ path, baudRate });

    port.on('error', (error: unknown) => {
      const portError = error instanceof Error ? error : new Error(String(error));
      this.logger.withError(portError).error(`${source} port error`);
      this.emit('portError', portError, source);
    });

    port.on('open', () => {
      this.logger.info(`${source === 'computer' ? 'Computer' : 'Radio'} port opened successfully`);
    });

    return port;
  }

  private setupDataHandlers(): void {
    if (!this.computerPort || !this.radioPort) {
      return;
    }

    const computerParser = this.computerPort.pipe(new ByteLengthParser({ length: 1 }));
    const radioParser = this.radioPort.pipe(new ByteLengthParser({ length: 1 }));

    computerParser.on('data', (data: Buffer) => {
      this.radioPort?.write(data);
      const bytes = Uint8Array.from(data);
      this.trafficLogger.logSend(bytes, 'Computer to Radio');
      this.bufferPacket(bytes, 'COMPUTER->RADIO', 'Computer to Radio');
    });

    radioParser.on('data', (data: Buffer) => {
      this.computerPort?.write(data);
      const bytes = Uint8Array.from(data);
      this.trafficLogger.logReceive(bytes, 'Radio to Computer');
      this.bufferPacket(bytes, 'RADIO->COMPUTER', 'Radio to Computer');
    });
  }

  private bufferPacket(data: Uint8Array, direction: SnifferDirection, description: string): void {
    if (this.pendingDirection && this.pendingDirection !== direction) {
      this.flushPendingPacket();
    }

    if (!this.pendingDirection) {
      this.pendingDirection = direction;
      this.pendingStartedAt = Date.now();
      this.pendingDescription = description;
    }

    this.pendingBytes.push(...data);

    if (this.packetIdleMs <= 0) {
      this.flushPendingPacket();
      return;
    }

    this.clearIdleTimer();
    this.idleTimer = setTimeout(() => {
      this.flushPendingPacket();
    }, this.packetIdleMs);
  }

  private flushPendingPacket(): void {
    this.clearIdleTimer();

    if (!this.pendingDirection || this.pendingBytes.length === 0) {
      this.pendingDirection = undefined;
      this.pendingBytes = [];
      this.pendingDescription = undefined;
      return;
    }

    const elapsedMs = this.pendingStartedAt - this.startedAt;
    const packet: Omit<SnifferPacket, 'id'> = {
      timestamp: this.formatTimestamp(elapsedMs),
      elapsedMs,
      direction: this.pendingDirection,
      data: [...this.pendingBytes],
      description: this.pendingDescription,
    };

    this.pendingDirection = undefined;
    this.pendingBytes = [];
    this.pendingDescription = undefined;
    this.emit('packet', packet);
  }

  private formatTimestamp(elapsedMs: number): string {
    const seconds = Math.floor(elapsedMs / 1000);
    const milliseconds = Math.abs(elapsedMs % 1000);

    return `${seconds.toString().padStart(3, '0')}.${milliseconds.toString().padStart(3, '0')}`;
  }

  private generateLogFileName(): string {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `radio-sniffer-${timestamp}.json`;
  }

  private closePort(port: BridgedSerialPort | undefined): void {
    if (!port?.isOpen) {
      return;
    }

    port.close();
  }

  private clearIdleTimer(): void {
    if (this.idleTimer) {
      clearTimeout(this.idleTimer);
      this.idleTimer = undefined;
    }
  }
}
