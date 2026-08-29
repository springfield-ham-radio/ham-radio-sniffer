import { createDefaultLogger } from './logger.ts';
import { listSerialPorts } from './list-ports.ts';
import { RadioSniffer } from './radio-sniffer.ts';

function printUsage(): void {
  console.log('Usage: yarn sniff <computer-port> <radio-port> [baud-rate] [--log-file <filename>]');
  console.log('       yarn sniff --list-ports');
  console.log('');
  console.log('Examples:');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0 9600');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0 9600 --log-file my-sniffer.json');
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  if (args.includes('--list-ports')) {
    const ports = await listSerialPorts();
    console.log(ports.map((port) => port.path).join('\n'));
    return;
  }

  if (args.length < 2) {
    printUsage();
    process.exit(1);
  }

  const computerPort = args[0];
  const radioPort = args[1];

  if (!computerPort || !radioPort) {
    printUsage();
    process.exit(1);
  }

  let baudRate = 9600;
  let logFile: string | undefined;

  for (let i = 2; i < args.length; i++) {
    if (args[i] === '--log-file' && i + 1 < args.length) {
      logFile = args[i + 1];
      i += 1;
    } else if (!Number.isNaN(Number(args[i]))) {
      baudRate = Number.parseInt(args[i] as string, 10);
    }
  }

  const logger = createDefaultLogger();
  const sniffer = new RadioSniffer({
    computerPort,
    radioPort,
    baudRate,
    logFile,
    logger,
  });

  sniffer.start();
  logger.info('Waiting for data transfer - press Ctrl+C to stop');

  process.on('SIGINT', () => {
    sniffer.stop();
    process.exit(0);
  });
}

try {
  await main();
} catch (error) {
  console.error('Sniffer error:', error);
  process.exit(1);
}
