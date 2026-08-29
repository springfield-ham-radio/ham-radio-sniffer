import { describe, it } from 'node:test';
import { expect } from 'chai';
import { listSerialPorts } from '../../src/list-ports.ts';

describe('listSerialPorts', () => {
  it('should map serialport list entries to public port info', async () => {
    const ports = await listSerialPorts(async () => [
      {
        path: '/dev/tty.usbserial-A',
        manufacturer: 'FTDI',
        serialNumber: 'A123',
        pnpId: undefined,
        locationId: '1',
        productId: '6001',
        vendorId: '0403',
      },
    ]);

    expect(ports).to.deep.equal([
      {
        path: '/dev/tty.usbserial-A',
        manufacturer: 'FTDI',
        serialNumber: 'A123',
        pnpId: undefined,
        locationId: '1',
        productId: '6001',
        vendorId: '0403',
      },
    ]);
  });
});
