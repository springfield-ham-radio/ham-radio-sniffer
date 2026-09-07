import { describe, it } from 'node:test';
import { expect } from 'chai';
import { resolveSnifferLogLevel } from '../../src/logger.ts';

describe('resolveSnifferLogLevel', () => {
  it('should default to info when unset', () => {
    expect(resolveSnifferLogLevel(undefined)).to.equal('info');
    expect(resolveSnifferLogLevel('')).to.equal('info');
    expect(resolveSnifferLogLevel('nope')).to.equal('info');
  });

  it('should accept debug aliases and explicit levels', () => {
    expect(resolveSnifferLogLevel('debug')).to.equal('debug');
    expect(resolveSnifferLogLevel('TRACE')).to.equal('debug');
    expect(resolveSnifferLogLevel('warn')).to.equal('warn');
    expect(resolveSnifferLogLevel('warning')).to.equal('warn');
    expect(resolveSnifferLogLevel('error')).to.equal('error');
    expect(resolveSnifferLogLevel('fatal')).to.equal('error');
    expect(resolveSnifferLogLevel('info')).to.equal('info');
  });
});
