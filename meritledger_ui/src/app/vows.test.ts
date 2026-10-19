import { describe, expect, it } from 'vitest';
import { actionInfo, isCurrent, stateLabel, suggestedPost, vowDates } from './vows';

describe('vows', () => {
  it('names states and actions plainly', () => {
    expect(stateLabel('breached')).toBe('Repairing');
    expect(actionInfo('breach').defaultVisibility).toBe('generic');
    expect(actionInfo('repair').prompt).toContain('repentance');
    expect(actionInfo('mystery').label).toBe('mystery');
  });

  it('keeps current vows first', () => {
    expect(isCurrent({ state: 'paused' })).toBe(true);
    expect(isCurrent({ state: 'retired' })).toBe(false);
  });

  it('suggests a post without posting it', () => {
    expect(suggestedPost('Do not kill.', 'took')).toEqual({ title: 'A new vow', body: 'Today I vowed: *Do not kill*.\n\n' });
    expect(suggestedPost('Sit daily', 'complete').title).toBe('I kept my vow');
  });

  it('says when', () => {
    expect(vowDates('2025-05-12', '')).toBe('since 12 May 2025');
    expect(vowDates('2026-05-12', '2026-06-30')).toBe('12 May 2026 – 30 June 2026');
  });
});
