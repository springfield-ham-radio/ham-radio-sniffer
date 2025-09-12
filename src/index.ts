import { SerialPort } from 'serialport';
import { ByteLengthParser } from '@serialport/parser-byte-length';
import { SerialLogger } from '@springfield/ham-radio-driver';
import type { ILogLayer } from 'loglayer';
import { ConsoleTransport, LogLayer } from 'loglayer';

/**
 * Enhanced Ham Radio Sniffer with improved logging capabilities
 *
 * This tool sniffs serial communication between a computer and a ham radio,
 * providing detailed JSON logging in a format that can be easily compared with
 * the ham-radio-driver output. The log file contains structured data with
 * timestamps, direction, and raw byte data for each communication event.
 */

interface SnifferOptions {
  computerPort: string;
  radioPort: string;
  baudRate?: number;
  logFile?: string;
  logger?: ILogLayer;
}

class RadioSniffer {
  private computerPort!: SerialPort;
  private radioPort!: SerialPort;
  private computerParser: any;
  private radioParser: any;
  private serialLogger!: SerialLogger;
  private options: SnifferOptions;
  private logger: ILogLayer;

  constructor(options: SnifferOptions) {
    this.options = options;
    this.logger = options.logger || this.createDefaultLogger();
    this.initializeLogging();
    this.initializePorts();
  }

  private createDefaultLogger(): ILogLayer {
    return new LogLayer({
      transport: [
        new ConsoleTransport({
          logger: console,
          level: 'info',
        }),
      ],
    });
  }

  private initializeLogging(): void {
    const logFile = this.options.logFile || this.generateLogFileName();

    this.serialLogger = new SerialLogger(logFile);

    this.logger
      .withMetadata({
        logFile,
        computerPort: this.options.computerPort,
        radioPort: this.options.radioPort,
        baudRate: this.options.baudRate || 9600,
      })
      .info('Sniffer started');
  }

  private generateLogFileName(): string {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    return `radio-sniffer-${timestamp}.json`;
  }

  private logData(data: Uint8Array, direction: string): void {
    if (direction === 'COMPUTER->RADIO') {
      this.serialLogger.logSend(data, 'Computer to Radio');
    } else if (direction === 'RADIO->COMPUTER') {
      this.serialLogger.logReceive(data, 'Radio to Computer');
    }
  }

  private initializePorts(): void {
    const baudRate = this.options.baudRate || 9600;

    this.logger
      .withMetadata({
        port: this.options.computerPort,
        baudRate,
      })
      .debug('Opening computer port');
    this.computerPort = new SerialPort({
      path: this.options.computerPort,
      baudRate: baudRate,
    });
    this.computerParser = this.computerPort.pipe(new ByteLengthParser({ length: 1 }));

    this.logger
      .withMetadata({
        port: this.options.radioPort,
        baudRate,
      })
      .debug('Opening radio port');
    this.radioPort = new SerialPort({
      path: this.options.radioPort,
      baudRate: baudRate,
    });
    this.radioParser = this.radioPort.pipe(new ByteLengthParser({ length: 1 }));

    this.setupEventHandlers();
  }

  private setupEventHandlers(): void {
    // Computer -> Radio data flow
    this.computerParser.on('data', (data: Buffer) => {
      this.radioPort.write(data);
      this.logData(Uint8Array.from(data), 'COMPUTER->RADIO');
    });

    // Radio -> Computer data flow
    this.radioParser.on('data', (data: Buffer) => {
      this.computerPort.write(data);
      this.logData(Uint8Array.from(data), 'RADIO->COMPUTER');
    });

    // Error handling
    this.computerPort.on('error', (error: Error) => {
      this.logger.withError(error).error('Computer port error');
    });

    this.radioPort.on('error', (error: Error) => {
      this.logger.withError(error).error('Radio port error');
    });

    // Port open events
    this.computerPort.on('open', () => {
      this.logger.info('Computer port opened successfully');
    });

    this.radioPort.on('open', () => {
      this.logger.info('Radio port opened successfully');
    });
  }

  public start(): void {
    this.logger.info('Waiting for data transfer - press Ctrl+C to stop');
  }

  public stop(): void {
    this.logger.info('Stopping sniffer...');
    this.serialLogger.close();

    if (this.computerPort && this.computerPort.isOpen) {
      this.computerPort.close();
    }

    if (this.radioPort && this.radioPort.isOpen) {
      this.radioPort.close();
    }
  }
}

// Main execution
try {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.log('Usage: node index.js <computer-port> <radio-port> [baud-rate] [--log-file <filename>]');
    console.log('');
    console.log('Examples:');
    console.log('  node index.js /dev/ttyS0 /dev/ttyUSB0');
    console.log('  node index.js /dev/ttyS0 /dev/ttyUSB0 9600');
    console.log('  node index.js /dev/ttyS0 /dev/ttyUSB0 9600 --log-file my-sniffer.json');
    process.exit(1);
  }

  const computerPort = args[0];
  const radioPort = args[1];
  let baudRate = 9600;
  let logFile: string | undefined;

  // Parse additional arguments
  for (let i = 2; i < args.length; i++) {
    if (args[i] === '--log-file' && i + 1 < args.length) {
      logFile = args[i + 1];
      i++; // Skip the next argument since we consumed it
    } else if (!isNaN(Number(args[i]))) {
      baudRate = parseInt(args[i], 10);
    }
  }

  // Create logger for main execution
  const logger = new LogLayer({
    transport: [
      new ConsoleTransport({
        logger: console,
        level: 'info',
      }),
    ],
  });

  const options: SnifferOptions = {
    computerPort,
    radioPort,
    baudRate,
    logFile,
    logger,
  };

  const sniffer = new RadioSniffer(options);
  sniffer.start();

  process.on('SIGINT', () => {
    sniffer.stop();
    process.exit(0);
  });
} catch (error) {
  console.error('Sniffer error:', error);
  process.exit(1);
}
