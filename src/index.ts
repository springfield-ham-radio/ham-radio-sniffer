import { SerialPort } from 'serialport';
import { ByteLengthParser } from '@serialport/parser-byte-length';
import { toHexWords } from '@springfield/ham-radio-utils';

try {
  const computerPort = new SerialPort({ path: process.argv[2], baudRate: 9600 });
  const computerParser = computerPort.pipe(new ByteLengthParser({ length: 1 }));

  const radioPort = new SerialPort({ path: process.argv[3], baudRate: 9600 });
  const radioParser = radioPort.pipe(new ByteLengthParser({ length: 1 }));
  let dataFromComputer: number[] = [];
  let dataFromRadio: number[] = [];

  computerParser.on('data', (data) => {
    radioPort.write(data);
    dataFromComputer.push(data[0]);

    if (dataFromRadio.length > 1) {
      console.log(`fromRadio: ${toHexWords(Uint8Array.from(dataFromRadio))}`);
      dataFromRadio = [];
    }
  });

  radioParser.on('data', (data) => {
    computerPort.write(data);
    dataFromRadio.push(data[0]);

    if (dataFromComputer.length > 1) {
      console.log(`fromComputer: ${toHexWords(Uint8Array.from(dataFromComputer))}`);
      dataFromComputer = [];
    }
  });
} catch (error) {
  console.log(error);
}
