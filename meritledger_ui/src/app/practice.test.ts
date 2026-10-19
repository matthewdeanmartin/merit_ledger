import { describe, expect, it } from 'vitest';
import type { EntryView, KindView } from './api';
import { amountLabel, dayHeading, entryLine, groupKinds, move, tileLabel } from './practice';

describe('amountLabel', () => {
  it('formats minutes, counts and once', () => {
    expect(amountLabel('minutes', 20)).toBe('20 min');
    expect(amountLabel('minutes', 120)).toBe('2 h');
    expect(amountLabel('minutes', 90)).toBe('90 min');
    expect(amountLabel('count', 1080)).toBe('1,080×');
    expect(amountLabel('once', 0)).toBe('done');
  });
});

describe('tileLabel', () => {
  it('shows what a tap records until done, then the total', () => {
    expect(tileLabel('minutes', 0, 0, 20)).toBe('20 min');
    expect(tileLabel('minutes', 2, 30, 20)).toBe('✓ 30 min');
    expect(tileLabel('once', 0, 0, 0)).toBe('tap when done');
    expect(tileLabel('once', 1, 0, 0)).toBe('✓ done');
    expect(tileLabel('once', 3, 0, 0)).toBe('✓ 3 times');
  });
});

describe('dayHeading', () => {
  it('names the day without shifting it', () => {
    expect(dayHeading('2026-10-18')).toBe('Sunday, October 18');
    expect(dayHeading('2027-01-01')).toBe('Friday, January 1');
  });
});

describe('move', () => {
  it('swaps neighbours and ignores the edges', () => {
    expect(move(['a', 'b', 'c'], 'b', -1)).toEqual(['b', 'a', 'c']);
    expect(move(['a', 'b', 'c'], 'b', 1)).toEqual(['a', 'c', 'b']);
    expect(move(['a', 'b'], 'a', -1)).toEqual(['a', 'b']);
    expect(move(['a', 'b'], 'z', 1)).toEqual(['a', 'b']);
  });
});

describe('groupKinds', () => {
  const k = (key: string, polarity: string, archived = false) => ({ key, polarity, archived }) as KindView;
  it('puts merit, demerit, then archived, skipping empty groups', () => {
    const groups = groupKinds([k('a', 'demerit'), k('b', 'merit'), k('c', 'merit', true)]);
    expect(groups.map((g) => [g.title, g.kinds.map((x) => x.key)])).toEqual([
      ['Merit', ['b']],
      ['Demerit', ['a']],
      ['Archived', ['c']],
    ]);
  });
});

describe('entryLine', () => {
  const e = (over: Partial<EntryView>) => ({ kind_name: 'Meditation', unit: 'minutes', amount: 20, generic: false, ...over }) as EntryView;
  it('shows the amount unless once or generic', () => {
    expect(entryLine(e({}))).toBe('Meditation · 20 min');
    expect(entryLine(e({ unit: 'once', kind_name: 'Vegetarian today' }))).toBe('Vegetarian today');
    expect(entryLine(e({ generic: true, kind_name: 'A repentance practice' }))).toBe('A repentance practice');
  });
});
