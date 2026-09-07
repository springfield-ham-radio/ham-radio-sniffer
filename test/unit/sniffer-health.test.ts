import { describe, it } from 'node:test';
import { expect } from 'chai';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SNIFFER_SERVICE_NAME, snifferHealthPayload } from '../../shared/types/sniffer.ts';

const packageJson = JSON.parse(
  readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../../package.json'), 'utf8'),
) as { version: string };

describe('sniffer health payload', () => {
  it('should include the package version and service name', () => {
    expect(snifferHealthPayload(packageJson.version)).to.deep.equal({
      ok: true,
      service: SNIFFER_SERVICE_NAME,
      version: packageJson.version,
    });
    expect(packageJson.version).to.match(/^\d+\.\d+\.\d+/);
  });
});
