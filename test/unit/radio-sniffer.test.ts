import { describe, expect, it } from 'vitest';
import { RadioSniffer } from '../../src/radio-sniffer.ts';
import { FakeSerialPort, FakeTrafficLogger } from '../helpers/fake-serial-port.ts';
import { MockLogLayer } from 'loglayer';

describe('RadioSniffer', () => {
  it('should forward computer bytes to the radio and emit a packet', () => {
    const computerPort = new FakeSerialPort('/dev/computer');
    const radioPort = new FakeSerialPort('/dev/radio');
    const trafficLogger = new FakeTrafficLogger();
    const ports = new Map<string, FakeSerialPort>([
      [computerPort.path, computerPort],
      [radioPort.path, radioPort],
    ]);

    const sniffer = new RadioSniffer({
      computerPort: computerPort.path,
      radioPort: radioPort.path,
      logger: new MockLogLayer(),
      trafficLogger,
      packetIdleMs: 0,
      serialPortFactory: ({ path }) => {
        const port = ports.get(path);

        if (!port) {
          throw new Error(`Unexpected port ${path}`);
        }

        return port;
      },
    });

    const packets: Array<{ direction: string; data: number[] }> = [];
    sniffer.on('packet', (packet) => {
      packets.push({ direction: packet.direction, data: packet.data });
    });

    sniffer.start();
    computerPort.emitData([0x50, 0xbb]);

    expect(radioPort.written).toHaveLength(1);
    expect(Array.from(radioPort.written[0] ?? [])).toEqual([0x50, 0xbb]);
    expect(packets).toEqual([{ direction: 'COMPUTER->RADIO', data: [0x50, 0xbb] }]);
    expect(trafficLogger.sent).toHaveLength(1);
    expect(sniffer.getStats()).toMatchObject({
      bytesComputerToRadio: 2,
      bytesRadioToComputer: 0,
      writeErrors: 0,
      computerPortOpen: true,
      radioPortOpen: true,
    });

    sniffer.stop();
    expect(computerPort.isOpen).toBe(false);
    expect(radioPort.isOpen).toBe(false);
    expect(trafficLogger.closed).toBe(true);
  });

  it('should forward radio bytes to the computer', () => {
    const computerPort = new FakeSerialPort('/dev/computer');
    const radioPort = new FakeSerialPort('/dev/radio');
    const ports = new Map<string, FakeSerialPort>([
      [computerPort.path, computerPort],
      [radioPort.path, radioPort],
    ]);

    const sniffer = new RadioSniffer({
      computerPort: computerPort.path,
      radioPort: radioPort.path,
      logger: new MockLogLayer(),
      trafficLogger: new FakeTrafficLogger(),
      packetIdleMs: 0,
      serialPortFactory: ({ path }) => ports.get(path) as FakeSerialPort,
    });

    const packets: Array<{ direction: string; data: number[] }> = [];
    sniffer.on('packet', (packet) => {
      packets.push({ direction: packet.direction, data: packet.data });
    });

    sniffer.start();
    radioPort.emitData([0x06]);

    expect(Array.from(computerPort.written[0] ?? [])).toEqual([0x06]);
    expect(packets).toEqual([{ direction: 'RADIO->COMPUTER', data: [0x06] }]);
    expect(sniffer.getStats().bytesRadioToComputer).toBe(1);

    sniffer.stop();
  });

  it('should count received bytes even when the other port is closed', () => {
    const computerPort = new FakeSerialPort('/dev/computer');
    const radioPort = new FakeSerialPort('/dev/radio');
    radioPort.isOpen = false;
    const ports = new Map<string, FakeSerialPort>([
      [computerPort.path, computerPort],
      [radioPort.path, radioPort],
    ]);

    const sniffer = new RadioSniffer({
      computerPort: computerPort.path,
      radioPort: radioPort.path,
      logger: new MockLogLayer(),
      trafficLogger: new FakeTrafficLogger(),
      packetIdleMs: 0,
      serialPortFactory: ({ path }) => ports.get(path) as FakeSerialPort,
    });

    const packets: Array<{ direction: string; data: number[] }> = [];
    sniffer.on('packet', (packet) => {
      packets.push({ direction: packet.direction, data: packet.data });
    });

    sniffer.start();
    computerPort.emitData([0x0d]);

    expect(packets).toEqual([{ direction: 'COMPUTER->RADIO', data: [0x0d] }]);
    expect(sniffer.getStats()).toMatchObject({
      bytesComputerToRadio: 1,
      writeErrors: 1,
      radioPortOpen: false,
    });

    sniffer.stop();
  });
});
