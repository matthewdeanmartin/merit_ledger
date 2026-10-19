import { describe, expect, it } from 'vitest';
import type { ImportReport, Person } from './api';
import { importSummary, initials, pinProblem, signInOrder, size } from './people';

describe('pinProblem', () => {
  it('wants 4 to 8 digits', () => {
    expect(pinProblem('123', '123')).toMatch(/4 to 8/);
    expect(pinProblem('123456789', '123456789')).toMatch(/4 to 8/);
    expect(pinProblem('12a4', '12a4')).toMatch(/4 to 8/);
  });

  it('wants both entries to match', () => {
    expect(pinProblem('1234', '1235')).toMatch(/match/);
    expect(pinProblem('1234', '1234')).toBe('');
  });
});

describe('initials', () => {
  it('takes the first and last word', () => {
    expect(initials('Ada Lovelace')).toBe('AL');
    expect(initials('  dad ')).toBe('D');
    expect(initials('Mary Ann Evans')).toBe('ME');
    expect(initials('')).toBe('?');
  });
});

describe('signInOrder', () => {
  const p = (id: number, display_name: string, role: Person['role'], can_sign_in = true): Person => ({
    id,
    display_name,
    role,
    can_sign_in,
  });

  it('skips people without a PIN and puts guests last', () => {
    const order = signInOrder([p(1, 'Zed', 'guest'), p(2, 'Mum', 'member'), p(3, 'Baby', 'member', false), p(4, 'Ada', 'admin')]);
    expect(order.map((x) => x.display_name)).toEqual(['Ada', 'Mum', 'Zed']);
  });
});

describe('size', () => {
  it('formats bytes', () => {
    expect(size(512)).toBe('512 B');
    expect(size(1536)).toBe('1.5 KB');
    expect(size(32 * 1024 ** 3)).toBe('32 GB');
  });
});

describe('importSummary', () => {
  const r = (over: Partial<ImportReport>): ImportReport => ({
    dry_run: true,
    people_matched: 0,
    people_created: 0,
    kinds_created: 0,
    entries_added: 0,
    entries_skipped: 0,
    entries_summarized: 0,
    notes: [],
    ...over,
  });

  it('says what a dry run would do', () => {
    expect(importSummary(r({ entries_added: 91, people_created: 1 }))).toBe('add 91 entries, add 1 person.');
    expect(importSummary(r({ entries_skipped: 3 }))).toBe('skip 3 entries already here.');
    expect(importSummary(r({}))).toBe('nothing new.');
  });

  it('says what an import did', () => {
    expect(importSummary(r({ dry_run: false, entries_added: 1, entries_summarized: 2 }))).toBe(
      'added 1 entry, kept 2 older entries as daily totals.',
    );
  });
});
