import { SerialPort } from 'serialport';

async function listSerialPorts() {
  const ports = await SerialPort.list();
  console.log(ports.map((port) => port.path));
}

listSerialPorts().catch(console.error);
