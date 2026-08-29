import { describe, it } from 'node:test';
import { expect } from 'chai';
import { parseStartSnifferRequest, StartRequestError } from '../../src/parse-start-request.ts';

describe('parseStartSnifferRequest', () => {
  it('should accept a valid start request', () => {
    const request = parseStartSnifferRequest({
      computerPort: '/dev/tty.usbserial-A',
      radioPort: '/dev/tty.usbserial-B',
      baudRate: 9600,
      logFile: 'capture.json',
    });

    expect(request).to.deep.equal({
      computerPort: '/dev/tty.usbserial-A',
      radioPort: '/dev/tty.usbserial-B',
      baudRate: 9600,
      logFile: 'capture.json',
    });
  });

  it('should reject a missing computer port', () => {
    expect(() => {
      parseStartSnifferRequest({ radioPort: '/dev/ttyUSB0' });
    })
      .to.throw(StartRequestError)
      .with.property('message', 'computerPort is required');
  });

  it('should reject the same path for both ports', () => {
    expect(() => {
      parseStartSnifferRequest({
        computerPort: '/dev/ttyUSB0',
        radioPort: '/dev/ttyUSB0',
      });
    })
      .to.throw(StartRequestError)
      .with.property('message', 'computerPort and radioPort must be different');
  });

  it('should reject a non-positive baud rate', () => {
    expect(() => {
      parseStartSnifferRequest({
        computerPort: '/dev/ttyUSB0',
        radioPort: '/dev/ttyUSB1',
        baudRate: 0,
      });
    })
      .to.throw(StartRequestError)
      .with.property('message', 'baudRate must be a positive integer');
  });

  it('should reject a non-object body', () => {
    expect(() => {
      parseStartSnifferRequest(null);
    })
      .to.throw(StartRequestError)
      .with.property('message', 'Request body must be a JSON object');
  });
});
