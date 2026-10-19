/** The calendar page's pure helpers (Sprint 7). Dates are `YYYY-MM-DD`. */

export interface Month {
  year: number;
  /** 1–12 */
  month: number;
}

function iso(y: number, m: number, d: number): string {
  return `${y}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
}

function daysIn({ year, month }: Month): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

export function monthOf(isoDate: string): Month {
  const [year, month] = isoDate.split('-').map(Number);
  return { year, month };
}

export function addMonths(m: Month, n: number): Month {
  const i = m.year * 12 + (m.month - 1) + n;
  return { year: Math.floor(i / 12), month: (i % 12) + 1 };
}

export function firstDay(m: Month): string {
  return iso(m.year, m.month, 1);
}

export function lastDay(m: Month): string {
  return iso(m.year, m.month, daysIn(m));
}

/** "October 2026" */
export function monthTitle(m: Month): string {
  return new Date(Date.UTC(m.year, m.month - 1, 1)).toLocaleDateString('en-US', {
    month: 'long',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

/**
 * The month as weeks, Sunday first; days outside the month are `null`
 * (blank cells).
 */
export function grid(m: Month): (string | null)[][] {
  const lead = new Date(Date.UTC(m.year, m.month - 1, 1)).getUTCDay();
  const cells: (string | null)[] = Array.from({ length: lead }, () => null);
  for (let d = 1; d <= daysIn(m); d++) cells.push(iso(m.year, m.month, d));
  while (cells.length % 7 !== 0) cells.push(null);
  const weeks: (string | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  return weeks;
}

/** 🌑 🌓 🌕 🌗 for a phase day, '' otherwise. */
export function phaseIcon(phase: string): string {
  return { new: '🌑', first: '🌓', full: '🌕', last: '🌗' }[phase] ?? '';
}

export const SYSTEMS = [
  { key: 'chinese', name: 'Chinese-inspired (lunisolar months, fasting days)' },
  { key: 'theravada', name: 'Theravada-inspired (uposatha, Vesak)' },
  { key: 'japanese', name: 'Japanese-inspired (fixed dates, Higan)' },
  { key: 'none', name: 'None' },
] as const;

export function systemName(key: string): string {
  return SYSTEMS.find((s) => s.key === key)?.name.split(' (')[0] ?? key;
}
