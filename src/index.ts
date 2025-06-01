import { SerialPort } from 'serialport';
import { ByteLengthParser } from '@serialport/parser-byte-length';
import { toHexWords } from '@springfield/ham-radio-utils';

try {
  console.log(`Opening computer port ${process.argv[2]}`);
  const computerPort = new SerialPort({ path: process.argv[2], baudRate: 9600 });
  const computerParser = computerPort.pipe(new ByteLengthParser({ length: 1 }));

  console.log(`Opening radio port ${process.argv[3]}`);
  const radioPort = new SerialPort({ path: process.argv[3], baudRate: 9600 });
  const radioParser = radioPort.pipe(new ByteLengthParser({ length: 1 }));

  console.log(`Starting data transfer`);

  let currentComputerLine: number[] = [];
  let currentRadioLine: number[] = [];
  let computerLines: number[][] = [];
  let radioLines: number[][] = [];

  computerParser.on('data', (data) => {
    radioPort.write(data);
    currentComputerLine.push(data[0]);

    // End the current radio line and start a new one
    if (currentRadioLine.length > 0) {
      radioLines.push([...currentRadioLine]);
      currentRadioLine = [];
    }
  });

  radioParser.on('data', (data) => {
    computerPort.write(data);
    currentRadioLine.push(data[0]);

    // End the current computer line and start a new one
    if (currentComputerLine.length > 0) {
      computerLines.push([...currentComputerLine]);
      currentComputerLine = [];
    }
  });

  process.on('SIGINT', () => {
    console.log('\nReceived Ctrl+C. Writing stored data to console:');

    // Add any remaining data in the current lines
    if (currentComputerLine.length > 0) {
      computerLines.push([...currentComputerLine]);
    }
    if (currentRadioLine.length > 0) {
      radioLines.push([...currentRadioLine]);
    }

    console.log('Data from computer:');
    computerLines.forEach((line, index) => {
      console.log(`Line ${index + 1}: ${toHexWords(Uint8Array.from(line))}`);
    });

    console.log('\nData from radio:');
    radioLines.forEach((line, index) => {
      console.log(`Line ${index + 1}: ${toHexWords(Uint8Array.from(line))}`);
    });

    process.exit(0);
  });
} catch (error) {
  console.log(error);
}
