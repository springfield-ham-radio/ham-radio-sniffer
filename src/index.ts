import { SerialPort } from 'serialport';
import { ByteLengthParser } from '@serialport/parser-byte-length';
import { toHexWords } from '@springfield/ham-radio-driver-utils';

try {
  const computerPort = new SerialPort({ path: process.argv[2], baudRate: 9600 });
  const computerParser = computerPort.pipe(new ByteLengthParser({ length: 1 }));

  const radioPort = new SerialPort({ path: process.argv[3], baudRate: 9600 });
  const radioParser = radioPort.pipe(new ByteLengthParser({ length: 1 }));
  let radioData: number[] = [];

  computerParser.on('data', (data) => {
    radioPort.write(data);

    if (radioData.length > 1) {
      console.log(toHexWords(Uint8Array.from(radioData)));
    }

    radioData = [];
  });

  radioParser.on('data', (data) => {
    computerPort.write(data);
    radioData.push(data[0]);
  });
} catch (error) {
  console.log(error);
}
