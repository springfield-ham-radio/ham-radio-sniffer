export const SNIFFER_SERVICE_NAME = 'ham-radio-sniffer';

export interface SnifferHealth {
  ok: boolean;
  service: string;
  version: string;
}

export function snifferHealthPayload(version: string): SnifferHealth {
  return {
    ok: true,
    service: SNIFFER_SERVICE_NAME,
    version,
  };
}

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
  /**
   * RTS line after open. Defaults to true. TH-F6 CAT needs false; TM-D710 clone
   * cables typically need true.
   */
  rts?: boolean;
  /** DTR line after open. Defaults to true. */
  dtr?: boolean;
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
  /** Resolved RTS line applied to both ports after open. */
  rts?: boolean;
  /** Resolved DTR line applied to both ports after open. */
  dtr?: boolean;
}

export interface SnifferLogResponse {
  status: SnifferStatus;
  packets: SnifferPacket[];
}

export type SnifferEvent =
  | { type: 'status'; status: SnifferStatus }
  | { type: 'packet'; packet: SnifferPacket }
  | { type: 'error'; message: string; source?: 'computer' | 'radio' };
