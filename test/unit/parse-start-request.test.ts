import { describe, expect, it } from 'vitest';
import { parseStartSnifferRequest, StartRequestError } from '../../src/parse-start-request.ts';

describe('parseStartSnifferRequest', () => {
  it('should accept a valid start request', () => {
    const request = parseStartSnifferRequest({
      computerPort: '/dev/tty.usbserial-A',
      radioPort: '/dev/tty.usbserial-B',
      baudRate: 9600,
      logFile: 'capture.json',
    });

    expect(request).toEqual({
      computerPort: '/dev/tty.usbserial-A',
      radioPort: '/dev/tty.usbserial-B',
      baudRate: 9600,
      logFile: 'capture.json',
    });
  });

  it('should reject a missing computer port', () => {
    let thrown: unknown;

    try {
      parseStartSnifferRequest({ radioPort: '/dev/ttyUSB0' });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(StartRequestError);
    expect((thrown as Error).message).toBe('computerPort is required');
  });

  it('should reject the same path for both ports', () => {
    let thrown: unknown;

    try {
      parseStartSnifferRequest({
        computerPort: '/dev/ttyUSB0',
        radioPort: '/dev/ttyUSB0',
      });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(StartRequestError);
    expect((thrown as Error).message).toBe('computerPort and radioPort must be different');
  });

  it('should reject a non-positive baud rate', () => {
    let thrown: unknown;

    try {
      parseStartSnifferRequest({
        computerPort: '/dev/ttyUSB0',
        radioPort: '/dev/ttyUSB1',
        baudRate: 0,
      });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(StartRequestError);
    expect((thrown as Error).message).toBe('baudRate must be a positive integer');
  });

  it('should reject a non-object body', () => {
    let thrown: unknown;

    try {
      parseStartSnifferRequest(null);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(StartRequestError);
    expect((thrown as Error).message).toBe('Request body must be a JSON object');
  });
});
