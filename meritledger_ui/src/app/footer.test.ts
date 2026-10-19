import { describe, expect, it } from 'vitest';
import { footerLine } from './footer';

const base = { app: 'meritledger', version: '0.1.0', build: 'abc1234', clock_synced: true, secure: true };

describe('footerLine', () => {
  it('names the transport, version and build', () => {
    expect(footerLine(base)).toBe('HTTPS · v0.1.0 (abc1234)');
  });

  it('says when the clock is not set', () => {
    expect(footerLine({ ...base, secure: false, clock_synced: false })).toBe(
      'HTTP · v0.1.0 (abc1234) · clock not set yet',
    );
  });
});
