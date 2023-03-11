import { SerialPort } from 'serialport';

try {
  const port = new SerialPort({ path: process.argv[2], baudRate: 9600 });
  port.write([0x06]);
  port.drain(() => port.close());
} catch (error) {
  console.log(error);
}
