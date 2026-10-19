import type { Precept, PreceptSet, PreceptsView } from './api';

/** CBETA Online, line by line (`T24n1484_p1004b16`). */
export function cbetaUrl(lineId: string): string {
  return `https://cbetaonline.dila.edu.tw/zh/${lineId}`;
}

export function majors(set: PreceptSet): Precept[] {
  return set.items.filter((i) => i.kind === 'major');
}

export function minors(set: PreceptSet): Precept[] {
  return set.items.filter((i) => i.kind === 'minor');
}

/**
 * The Upāsakaśīla Sūtra's names for how much of the Five someone holds
 * (T1488, p1049a18–27): one, two, three or four, or all five.
 */
export function fivePart(count: number): string {
  if (count <= 0) return '';
  if (count === 1) return 'one part (一分)';
  if (count === 2) return 'a small part (少分)';
  if (count < 5) return 'a large part (多分)';
  return 'the full part (滿分)';
}

/** "All 5", "2 of 5", "6 major · 2 of 28 minor". */
export function takenSummary(v: Pick<PreceptsView, 'majors' | 'minors' | 'total_majors' | 'total_minors'>): string {
  const part = (n: number, total: number) => (n === total ? `all ${total}` : `${n} of ${total}`);
  if (v.total_minors === 0) {
    const s = part(v.majors.length, v.total_majors);
    return s[0].toUpperCase() + s.slice(1);
  }
  return `${part(v.majors.length, v.total_majors)} major · ${part(v.minors.length, v.total_minors)} minor`;
}

/** The items someone took, in the set's order. */
export function takenItems(set: PreceptSet, v: Pick<PreceptsView, 'majors' | 'minors'>): Precept[] {
  return set.items.filter((i) => (i.kind === 'major' ? v.majors : v.minors).includes(i.n));
}

/** Adds or removes `n`, keeping the list sorted. */
export function toggle(list: number[], n: number): number[] {
  return list.includes(n) ? list.filter((x) => x !== n) : [...list, n].sort((a, b) => a - b);
}

/** 1..count. */
export function all(count: number): number[] {
  return Array.from({ length: count }, (_, i) => i + 1);
}

/** "Major 3" / "Minor 21" / "3" (sets with only majors). */
export function itemLabel(set: PreceptSet, p: Pick<Precept, 'kind' | 'n'>): string {
  if (minors(set).length === 0) return `${p.n}`;
  return `${p.kind === 'major' ? 'Major' : 'Minor'} ${p.n}`;
}
