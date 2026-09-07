import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createDefaultLogger, resolveSnifferLogLevel } from './logger.ts';
import { listSerialPorts } from './list-ports.ts';
import { RadioSniffer } from './radio-sniffer.ts';

function snifferPackageVersion(): string {
  const packagePath = join(dirname(fileURLToPath(import.meta.url)), '..', 'package.json');
  const parsed = JSON.parse(readFileSync(packagePath, 'utf8')) as { version?: string };
  return parsed.version ?? '0.0.0';
}

function printUsage(): void {
  console.log('Usage: yarn sniff <computer-port> <radio-port> [baud-rate] [--log-file <filename>] [--no-rts] [--no-dtr]');
  console.log('       yarn sniff --list-ports');
  console.log('       yarn sniff --version');
  console.log('');
  console.log('RTS and DTR default on (typical USB clone cable). Use --no-rts for TH-F6 CAT.');
  console.log('');
  console.log('Examples:');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0 9600');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0 9600 --log-file my-sniffer.json');
  console.log('  yarn sniff /dev/ttyS0 /dev/ttyUSB0 9600 --no-rts');
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  if (args.includes('--version') || args.includes('-v')) {
    console.log(snifferPackageVersion());
    return;
  }

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
  let rts = true;
  let dtr = true;

  for (let i = 2; i < args.length; i++) {
    if (args[i] === '--log-file' && i + 1 < args.length) {
      logFile = args[i + 1];
      i += 1;
    } else if (args[i] === '--no-rts') {
      rts = false;
    } else if (args[i] === '--no-dtr') {
      dtr = false;
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
    rts,
    dtr,
    logger,
  });

  sniffer.start();
  logger.withMetadata({ level: resolveSnifferLogLevel() }).info('Waiting for data transfer - press Ctrl+C to stop');

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
