import { describe, it } from 'node:test';
import { expect } from 'chai';
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

    expect(fakeSniffer.started).to.equal(true);
    expect(status.running).to.equal(true);
    expect(status.computerPort).to.equal('/dev/computer');
    expect(status.radioPort).to.equal('/dev/radio');
    expect(status.baudRate).to.equal(19200);
    expect(status.logFile).to.equal('test-sniffer.json');
    expect(status.packetCount).to.equal(0);
  });

  it('should reject a second start while running', () => {
    const session = new SnifferSession({
      logger: new MockLogLayer(),
      createSniffer: () => new FakeRadioSniffer() as unknown as RadioSniffer,
    });

    session.start({ computerPort: '/dev/computer', radioPort: '/dev/radio' });

    expect(() => {
      session.start({ computerPort: '/dev/computer', radioPort: '/dev/radio' });
    })
      .to.throw(SnifferConflictError)
      .with.property('message', 'Sniffer is already running');
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

    expect(session.getPackets()).to.have.length(1);
    expect(session.getPackets()[0]?.id).to.equal(1);
    expect(session.getStatus().packetCount).to.equal(1);
    expect(events).to.include('packet');

    const stopped = session.stop();
    expect(fakeSniffer.stopped).to.equal(true);
    expect(stopped.running).to.equal(false);
    expect(session.getLogData()).to.deep.equal({
      metadata: { startTime: '2026-08-28T19:00:00.000Z', totalEntries: 1, version: '1.0.0' },
      entries: [{ timestamp: '000.010', elapsedMs: 10, direction: 'SEND', data: [0x50] }],
    });
  });
});
