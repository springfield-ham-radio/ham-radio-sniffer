import { describe, expect, it } from 'vitest';
import { EventEmitter } from 'node:events';
import { MockLogLayer } from 'loglayer';
import { SnifferConflictError, SnifferSession } from '../../src/sniffer-session.ts';
import type { RadioSniffer } from '../../src/radio-sniffer.ts';
import type { SnifferPacket } from '../../shared/types/sniffer.ts';

class FakeRadioSniffer extends EventEmitter {
  started = false;
  stopped = false;
  private logData: unknown = { entries: [] };

  start(): void {
    this.started = true;
  }

  stop(): void {
    this.stopped = true;
  }

  getLogFilePath(): string {
    return 'test-sniffer.json';
  }

  getLogData(): unknown {
    return this.logData;
  }

  getStats(): {
    bytesComputerToRadio: number;
    bytesRadioToComputer: number;
    writeErrors: number;
    computerPortOpen: boolean;
    radioPortOpen: boolean;
  } {
    return {
      bytesComputerToRadio: 0,
      bytesRadioToComputer: 0,
      writeErrors: 0,
      computerPortOpen: this.started && !this.stopped,
      radioPortOpen: this.started && !this.stopped,
    };
  }

  emitPacket(packet: Omit<SnifferPacket, 'id'>): void {
    this.logData = {
      metadata: { startTime: '2026-08-28T19:00:00.000Z', totalEntries: 1, version: '1.0.0' },
      entries: [{ timestamp: packet.timestamp, elapsedMs: packet.elapsedMs, direction: 'SEND', data: packet.data }],
    };
    this.emit('packet', packet);
  }
}

describe('SnifferSession', () => {
  it('should start once and expose running status', () => {
    const fakeSniffer = new FakeRadioSniffer();
    const session = new SnifferSession({
      logger: new MockLogLayer(),
      createSniffer: () => fakeSniffer as unknown as RadioSniffer,
    });

    const status = session.start({
      computerPort: '/dev/computer',
      radioPort: '/dev/radio',
      baudRate: 19200,
    });

    expect(fakeSniffer.started).toBe(true);
    expect(status.running).toBe(true);
    expect(status.computerPort).toBe('/dev/computer');
    expect(status.radioPort).toBe('/dev/radio');
    expect(status.baudRate).toBe(19200);
    expect(status.logFile).toBe('test-sniffer.json');
    expect(status.packetCount).toBe(0);
    expect(status.computerPortOpen).toBe(true);
    expect(status.radioPortOpen).toBe(true);
    expect(status.bytesComputerToRadio).toBe(0);
    expect(status.writeErrors).toBe(0);
  });

  it('should reject a second start while running', () => {
    const session = new SnifferSession({
      logger: new MockLogLayer(),
      createSniffer: () => new FakeRadioSniffer() as unknown as RadioSniffer,
    });

    session.start({ computerPort: '/dev/computer', radioPort: '/dev/radio' });

    let thrown: unknown;

    try {
      session.start({ computerPort: '/dev/computer', radioPort: '/dev/radio' });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(SnifferConflictError);
    expect((thrown as Error).message).toBe('Sniffer is already running');
  });

  it('should record packets and notify subscribers', () => {
    const fakeSniffer = new FakeRadioSniffer();
    const session = new SnifferSession({
      logger: new MockLogLayer(),
      createSniffer: () => fakeSniffer as unknown as RadioSniffer,
    });
    const events: string[] = [];

    session.subscribe((event) => {
      events.push(event.type);
    });

    session.start({ computerPort: '/dev/computer', radioPort: '/dev/radio' });
    fakeSniffer.emitPacket({
      timestamp: '000.010',
      elapsedMs: 10,
      direction: 'COMPUTER->RADIO',
      data: [0x50],
    });

    expect(session.getPackets()).toHaveLength(1);
    expect(session.getPackets()[0]?.id).toBe(1);
    expect(session.getStatus().packetCount).toBe(1);
    expect(events).toContain('packet');

    const stopped = session.stop();
    expect(fakeSniffer.stopped).toBe(true);
    expect(stopped.running).toBe(false);
    expect(session.getLogData()).toEqual({
      metadata: { startTime: '2026-08-28T19:00:00.000Z', totalEntries: 1, version: '1.0.0' },
      entries: [{ timestamp: '000.010', elapsedMs: 10, direction: 'SEND', data: [0x50] }],
    });
  });
});
