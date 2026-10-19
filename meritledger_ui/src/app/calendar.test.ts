import { describe, expect, it } from 'vitest';
import { addMonths, firstDay, grid, lastDay, monthOf, monthTitle, phaseIcon, systemName } from './calendar';

describe('grid', () => {
  it('starts weeks on Sunday with blanks around the month', () => {
    // October 2026 starts on a Thursday and has 31 days.
    const weeks = grid({ year: 2026, month: 10 });
    expect(weeks[0]).toEqual([null, null, null, null, '2026-10-01', '2026-10-02', '2026-10-03']);
    expect(weeks.every((w) => w.length === 7)).toBe(true);
    expect(weeks.flat().filter(Boolean).length).toBe(31);
    expect(weeks[weeks.length - 1]).toContain('2026-10-31');
  });

  it('handles leap Februaries', () => {
    expect(grid({ year: 2028, month: 2 }).flat().filter(Boolean).length).toBe(29);
    expect(lastDay({ year: 2026, month: 2 })).toBe('2026-02-28');
  });
});

describe('months', () => {
  it('steps across years', () => {
    expect(addMonths({ year: 2026, month: 12 }, 1)).toEqual({ year: 2027, month: 1 });
    expect(addMonths({ year: 2026, month: 1 }, -1)).toEqual({ year: 2025, month: 12 });
    expect(monthOf('2026-10-19')).toEqual({ year: 2026, month: 10 });
    expect(firstDay({ year: 2026, month: 5 })).toBe('2026-05-01');
    expect(monthTitle({ year: 2026, month: 10 })).toBe('October 2026');
  });
});

describe('labels', () => {
  it('names phases and systems', () => {
    expect(phaseIcon('full')).toBe('🌕');
    expect(phaseIcon('')).toBe('');
    expect(systemName('theravada')).toBe('Theravada-inspired');
  });
});
