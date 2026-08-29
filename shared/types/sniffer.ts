export type SnifferDirection = 'COMPUTER->RADIO' | 'RADIO->COMPUTER';

export interface SnifferPacket {
  id: number;
  timestamp: string;
  elapsedMs: number;
  direction: SnifferDirection;
  data: number[];
  description?: string;
}

export interface SerialPortInfo {
  path: string;
  manufacturer?: string;
  serialNumber?: string;
  pnpId?: string;
  locationId?: string;
  productId?: string;
  vendorId?: string;
}

export interface StartSnifferRequest {
  computerPort: string;
  radioPort: string;
  baudRate?: number;
  logFile?: string;
}

export interface SnifferStatus {
  running: boolean;
  computerPort?: string;
  radioPort?: string;
  baudRate?: number;
  logFile?: string;
  startedAt?: string;
  packetCount: number;
  /** Bytes forwarded computer → radio since start. */
  bytesComputerToRadio?: number;
  /** Bytes forwarded radio → computer since start. */
  bytesRadioToComputer?: number;
  /** Failed or dropped writes (port closed / write error). */
  writeErrors?: number;
  computerPortOpen?: boolean;
  radioPortOpen?: boolean;
}

export interface SnifferLogResponse {
  status: SnifferStatus;
  packets: SnifferPacket[];
}

export type SnifferEvent =
  | { type: 'status'; status: SnifferStatus }
  | { type: 'packet'; packet: SnifferPacket }
  | { type: 'error'; message: string; source?: 'computer' | 'radio' };
