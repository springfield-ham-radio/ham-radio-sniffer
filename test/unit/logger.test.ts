import { describe, expect, it } from 'vitest';
import { resolveSnifferLogLevel } from '../../src/logger.ts';

describe('resolveSnifferLogLevel', () => {
  it('should default to info when unset', () => {
    expect(resolveSnifferLogLevel(undefined)).toBe('info');
    expect(resolveSnifferLogLevel('')).toBe('info');
    expect(resolveSnifferLogLevel('nope')).toBe('info');
  });

  it('should accept debug aliases and explicit levels', () => {
    expect(resolveSnifferLogLevel('debug')).toBe('debug');
    expect(resolveSnifferLogLevel('TRACE')).toBe('debug');
    expect(resolveSnifferLogLevel('warn')).toBe('warn');
    expect(resolveSnifferLogLevel('warning')).toBe('warn');
    expect(resolveSnifferLogLevel('error')).toBe('error');
    expect(resolveSnifferLogLevel('fatal')).toBe('error');
    expect(resolveSnifferLogLevel('info')).toBe('info');
  });
});
