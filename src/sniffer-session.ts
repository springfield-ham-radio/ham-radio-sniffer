import type { ILogLayer } from 'loglayer';
import type { SnifferEvent, SnifferPacket, SnifferStatus, StartSnifferRequest } from '../shared/types/sniffer.ts';
import { createDefaultLogger, resolveSnifferLogLevel } from './logger.ts';
import { RadioSniffer, type RadioSnifferOptions } from './radio-sniffer.ts';

export class SnifferConflictError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SnifferConflictError';
  }
}

export type RadioSnifferFactory = (options: RadioSnifferOptions) => RadioSniffer;

/**
 * Process-wide sniffer controller used by the HTTP API.
 *
 * Only one bridge can own the serial ports at a time. Subscribers receive
 * live packets so SSE clients can render traffic without polling.
 */
export class SnifferSession {
  private sniffer: RadioSniffer | undefined;
  private startedAt: string | undefined;
  private packets: SnifferPacket[] = [];
  private lastLogData: unknown;
  private nextPacketId = 1;
  private readonly listeners = new Set<(event: SnifferEvent) => void>();
  private readonly createSniffer: RadioSnifferFactory;
  private readonly logger: ILogLayer;
  private status: SnifferStatus = { running: false, packetCount: 0 };

  constructor(options: { createSniffer?: RadioSnifferFactory; logger?: ILogLayer } = {}) {
    this.createSniffer = options.createSniffer ?? ((snifferOptions) => new RadioSniffer(snifferOptions));
    this.logger = options.logger ?? createDefaultLogger();
    this.logger.withMetadata({ level: resolveSnifferLogLevel() }).info('Sniffer logger ready');
  }

  public getStatus(): SnifferStatus {
    const stats = this.sniffer?.getStats();

    return {
      ...this.status,
      ...stats,
      packetCount: this.packets.length,
    };
  }

  public getPackets(): SnifferPacket[] {
    return [...this.packets];
  }

  public getLogData(): unknown {
    return this.sniffer?.getLogData() ?? this.lastLogData;
  }

  public start(request: StartSnifferRequest): SnifferStatus {
    if (this.status.running) {
      throw new SnifferConflictError('Sniffer is already running');
    }

    const sniffer = this.createSniffer({
      computerPort: request.computerPort,
      radioPort: request.radioPort,
      baudRate: request.baudRate,
      logFile: request.logFile,
      rts: request.rts,
      dtr: request.dtr,
      logger: this.logger,
    });

    this.packets = [];
    this.lastLogData = undefined;
    this.nextPacketId = 1;
    this.startedAt = new Date().toISOString();
    this.sniffer = sniffer;

    sniffer.on('packet', (packet) => {
      this.handlePacket(packet);
    });

    sniffer.on('portError', (error, source) => {
      this.publish({ type: 'error', message: error.message, source });
    });

    sniffer.on('stats', () => {
      this.publish({ type: 'status', status: this.getStatus() });
    });

    try {
      sniffer.start();
    } catch (error) {
      this.sniffer = undefined;
      this.startedAt = undefined;
      throw error;
    }

    this.status = {
      running: true,
      computerPort: request.computerPort,
      radioPort: request.radioPort,
      baudRate: request.baudRate ?? 9600,
      logFile: sniffer.getLogFilePath(),
      startedAt: this.startedAt,
      packetCount: 0,
      rts: request.rts ?? true,
      dtr: request.dtr ?? true,
      ...sniffer.getStats(),
    };

    this.logger.withMetadata(this.status).info('Sniffer session started');
    this.publish({ type: 'status', status: this.getStatus() });

    return this.getStatus();
  }

  public stop(): SnifferStatus {
    if (!this.status.running && !this.sniffer) {
      return this.getStatus();
    }

    try {
      // Capture before and after close so buffered bytes are retained for save.
      const logBeforeStop = this.sniffer?.getLogData();
      const stats = this.sniffer?.getStats();
      this.sniffer?.stop();
      this.lastLogData = this.sniffer?.getLogData() ?? logBeforeStop;
      this.status = {
        ...this.status,
        ...stats,
        running: false,
        packetCount: this.packets.length,
        computerPortOpen: false,
        radioPortOpen: false,
      };
    } catch (error) {
      this.logger.withError(error).error('Failed to stop sniffer cleanly');
      this.status = {
        ...this.status,
        running: false,
        packetCount: this.packets.length,
        computerPortOpen: false,
        radioPortOpen: false,
      };
    }

    this.sniffer = undefined;

    this.logger.info('Sniffer session stopped');
    this.publish({ type: 'status', status: this.getStatus() });

    return this.getStatus();
  }

  public subscribe(listener: (event: SnifferEvent) => void): () => void {
    this.listeners.add(listener);
    listener({ type: 'status', status: this.getStatus() });

    return () => {
      this.listeners.delete(listener);
    };
  }

  private handlePacket(packet: Omit<SnifferPacket, 'id'>): void {
    const recorded: SnifferPacket = {
      ...packet,
      id: this.nextPacketId,
    };

    this.nextPacketId += 1;
    this.packets.push(recorded);
    this.status = {
      ...this.status,
      packetCount: this.packets.length,
    };
    this.publish({ type: 'packet', packet: recorded });
    this.publish({ type: 'status', status: this.getStatus() });
  }

  private publish(event: SnifferEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }
}

export const snifferSession = new SnifferSession();
