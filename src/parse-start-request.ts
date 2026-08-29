import type { StartSnifferRequest } from '../shared/types/sniffer.ts';

export class StartRequestError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'StartRequestError';
  }
}

function readOptionalString(value: unknown, fieldName: string): string | undefined {
  if (value === undefined || value === null || value === '') {
    return undefined;
  }

  if (typeof value !== 'string') {
    throw new StartRequestError(`${fieldName} must be a string`);
  }

  const trimmed = value.trim();

  if (trimmed.length === 0) {
    return undefined;
  }

  return trimmed;
}

function readRequiredString(value: unknown, fieldName: string): string {
  const parsed = readOptionalString(value, fieldName);

  if (!parsed) {
    throw new StartRequestError(`${fieldName} is required`);
  }

  return parsed;
}

function readOptionalBaudRate(value: unknown): number | undefined {
  if (value === undefined || value === null || value === '') {
    return undefined;
  }

  if (typeof value !== 'number' || !Number.isInteger(value) || value <= 0) {
    throw new StartRequestError('baudRate must be a positive integer');
  }

  return value;
}

/**
 * Validates a JSON body for POST /api/sniffer/start.
 *
 * Ports are required and must be distinct so the sniffer can sit between a
 * programming cable and the radio without opening the same device twice.
 */
export function parseStartSnifferRequest(body: unknown): StartSnifferRequest {
  if (body === null || typeof body !== 'object' || Array.isArray(body)) {
    throw new StartRequestError('Request body must be a JSON object');
  }

  const record = body as Record<string, unknown>;
  const computerPort = readRequiredString(record.computerPort, 'computerPort');
  const radioPort = readRequiredString(record.radioPort, 'radioPort');

  if (computerPort === radioPort) {
    throw new StartRequestError('computerPort and radioPort must be different');
  }

  const request: StartSnifferRequest = {
    computerPort,
    radioPort,
  };

  const baudRate = readOptionalBaudRate(record.baudRate);

  if (baudRate !== undefined) {
    request.baudRate = baudRate;
  }

  const logFile = readOptionalString(record.logFile, 'logFile');

  if (logFile !== undefined) {
    request.logFile = logFile;
  }

  return request;
}
