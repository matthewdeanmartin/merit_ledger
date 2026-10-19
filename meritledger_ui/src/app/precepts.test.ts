import { describe, expect, it } from 'vitest';
import type { PreceptSet } from './api';
import { all, cbetaUrl, fivePart, itemLabel, takenItems, takenSummary, toggle } from './precepts';

const item = (kind: 'major' | 'minor', n: number) => ({
  n,
  kind,
  label_zh: '',
  title: `${kind} ${n}`,
  text: '',
  quote: '',
  cite: ['T24n1488_p1049a28'],
});

const upasaka: PreceptSet = {
  key: 'upasaka',
  title: 'The Upāsaka Precepts',
  zh: '優婆塞戒',
  source: { work: 'T1488', juan: 3, title_zh: '', title: '', canon: '' },
  provenance: 'llm-draft',
  items: [item('major', 1), item('major', 2), item('minor', 1), item('minor', 2), item('minor', 3)],
};

describe('precepts', () => {
  it('links to CBETA by line', () => {
    expect(cbetaUrl('T24n1484_p1004b16')).toBe('https://cbetaonline.dila.edu.tw/zh/T24n1484_p1004b16');
  });

  it('names partial taking of the Five as T1488 does', () => {
    expect([0, 1, 2, 3, 4, 5].map(fivePart)).toEqual([
      '',
      'one part (一分)',
      'a small part (少分)',
      'a large part (多分)',
      'a large part (多分)',
      'the full part (滿分)',
    ]);
  });

  it('summarizes what was taken', () => {
    expect(takenSummary({ majors: [1, 2, 3, 4, 5], minors: [], total_majors: 5, total_minors: 0 })).toBe('All 5');
    expect(takenSummary({ majors: [1, 4], minors: [], total_majors: 5, total_minors: 0 })).toBe('2 of 5');
    expect(takenSummary({ majors: all(6), minors: [21, 22], total_majors: 6, total_minors: 28 })).toBe(
      'all 6 major · 2 of 28 minor',
    );
  });

  it('lists exactly the items taken, in order', () => {
    expect(takenItems(upasaka, { majors: [2], minors: [3, 1] }).map((p) => p.title)).toEqual([
      'major 2',
      'minor 1',
      'minor 3',
    ]);
    expect(itemLabel(upasaka, { kind: 'minor', n: 21 })).toBe('Minor 21');
  });

  it('toggles items', () => {
    expect(toggle([1, 4], 2)).toEqual([1, 2, 4]);
    expect(toggle([1, 2, 4], 2)).toEqual([1, 4]);
  });
});
