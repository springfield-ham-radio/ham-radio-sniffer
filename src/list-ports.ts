import { SerialPort } from 'serialport';
import type { SerialPortInfo } from '../shared/types/sniffer.ts';

export type SerialPortListFn = () => Promise<SerialPortInfo[]>;

/**
 * Enumerates serial devices the sniffer can bind to.
 *
 * The list function is injectable so unit tests can cover mapping without
 * opening real hardware.
 */
export async function listSerialPorts(list: SerialPortListFn = () => SerialPort.list()): Promise<SerialPortInfo[]> {
  const ports = await list();

  return ports.map((port) => ({
    path: port.path,
    manufacturer: port.manufacturer,
    serialNumber: port.serialNumber,
    pnpId: port.pnpId,
    locationId: port.locationId,
    productId: port.productId,
    vendorId: port.vendorId,
  }));
}
