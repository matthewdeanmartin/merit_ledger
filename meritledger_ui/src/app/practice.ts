import type { EntryView, KindView } from './api';

/** "20 min", "108×", "done" (for `once`). */
export function amountLabel(unit: string, amount: number): string {
  if (unit === 'minutes') return amount >= 60 && amount % 60 === 0 ? `${amount / 60} h` : `${amount} min`;
  if (unit === 'count') return `${amount.toLocaleString('en-US')}×`;
  return 'done';
}

/** What a tile shows under its name: today's total, or what a tap records. */
export function tileLabel(unit: string, count: number, total: number, defaultAmount: number): string {
  if (count === 0) return unit === 'once' ? 'tap when done' : amountLabel(unit, defaultAmount);
  if (unit === 'once') return count > 1 ? `✓ ${count} times` : '✓ done';
  return `✓ ${amountLabel(unit, total)}`;
}

export const VISIBILITIES = ['public', 'generic', 'private'] as const;
export type Visibility = (typeof VISIBILITIES)[number];

export function visibilityLabel(v: string): string {
  switch (v) {
    case 'private':
      return '🔒 Only me (and admins)';
    case 'generic':
      return '◐ Others see only the kind of practice';
    default:
      return '🌐 Everyone in the house';
  }
}

/** "Sunday, October 18" from "2026-10-18" (no time zone games: it's a date). */
export function dayHeading(iso: string): string {
  const [y, m, d] = iso.split('-').map(Number);
  const date = new Date(Date.UTC(y, m - 1, d));
  return date.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric', timeZone: 'UTC' });
}

/** Moves `key` one place up (-1) or down (+1) in `keys`. */
export function move(keys: string[], key: string, by: -1 | 1): string[] {
  const at = keys.indexOf(key);
  const to = at + by;
  if (at < 0 || to < 0 || to >= keys.length) return keys;
  const out = [...keys];
  [out[at], out[to]] = [out[to], out[at]];
  return out;
}

/** Kinds grouped for the practices page: merit first, then demerit; archived last. */
export function groupKinds(kinds: KindView[]): { title: string; kinds: KindView[] }[] {
  const live = kinds.filter((k) => !k.archived);
  const groups = [
    { title: 'Merit', kinds: live.filter((k) => k.polarity === 'merit') },
    { title: 'Demerit', kinds: live.filter((k) => k.polarity === 'demerit') },
    { title: 'Archived', kinds: kinds.filter((k) => k.archived) },
  ];
  return groups.filter((g) => g.kinds.length > 0);
}

/** One line for an entry in a list. */
export function entryLine(e: EntryView): string {
  if (e.generic) return e.kind_name;
  const amount = e.unit === 'once' ? '' : ` · ${amountLabel(e.unit, e.amount)}`;
  return `${e.kind_name}${amount}`;
}
