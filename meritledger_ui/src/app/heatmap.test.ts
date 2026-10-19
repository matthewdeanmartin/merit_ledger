import { describe, expect, it } from 'vitest';
import { cells, columns } from './heatmap';

describe('cells', () => {
  it('puts Sunday on the top row and starts a new column each week', () => {
    // 2026-10-18 is a Sunday.
    const c = cells('2026-10-16', [1, 2, 3, 4], '2026-10-01');
    expect(c.map((x) => [x.date, x.col, x.row])).toEqual([
      ['2026-10-16', 0, 5],
      ['2026-10-17', 0, 6],
      ['2026-10-18', 1, 0],
      ['2026-10-19', 1, 1],
    ]);
    expect(columns(c)).toBe(2);
  });

  it('marks days before the history as unknown', () => {
    const c = cells('2026-10-16', [0, 0, 1], '2026-10-17');
    expect(c.map((x) => x.unknown)).toEqual([true, false, false]);
  });

  it('lays out 26 weeks in 27 columns at most', () => {
    expect(columns(cells('2026-04-20', new Array(182).fill(0), '2026-01-01'))).toBeLessThanOrEqual(27);
    expect(columns([])).toBe(0);
  });
});
