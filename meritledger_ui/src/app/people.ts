import type { ImportReport, Person } from './api';

/** Why a new PIN can't be used, or '' when it's fine. */
export function pinProblem(pin: string, again: string): string {
  if (!/^[0-9]{4,8}$/.test(pin)) return 'A PIN is 4 to 8 digits.';
  if (pin !== again) return "The two PINs don't match.";
  return '';
}

/** One or two letters for a name tile ("Ada Lovelace" -> "AL"). */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return '?';
  const first = [...words[0]][0] ?? '';
  const last = words.length > 1 ? ([...words[words.length - 1]][0] ?? '') : '';
  return (first + last).toUpperCase();
}

/** The sign-in picker: people who can sign in, household before guests, by name. */
export function signInOrder(people: Person[]): Person[] {
  const rank = (p: Person) => (p.role === 'guest' ? 1 : 0);
  return people
    .filter((p) => p.can_sign_in)
    .sort((a, b) => rank(a) - rank(b) || a.display_name.localeCompare(b.display_name));
}

/** One sentence for an import report. */
export function importSummary(r: ImportReport): string {
  const parts: string[] = [];
  const n = (count: number, one: string, many: string) => `${count.toLocaleString('en-US')} ${count === 1 ? one : many}`;
  if (r.entries_added) parts.push(`add ${n(r.entries_added, 'entry', 'entries')}`);
  if (r.entries_summarized) parts.push(`keep ${n(r.entries_summarized, 'older entry', 'older entries')} as daily totals`);
  if (r.people_created) parts.push(`add ${n(r.people_created, 'person', 'people')}`);
  if (r.kinds_created) parts.push(`add ${n(r.kinds_created, 'practice', 'practices')}`);
  if (r.entries_skipped) parts.push(`skip ${n(r.entries_skipped, 'entry', 'entries')} already here`);
  if (parts.length === 0) return 'nothing new.';
  const verbs = r.dry_run ? parts : parts.map((p) => p.replace(/^(add|keep|skip) /, (_, v: string) => ({ add: 'added ', keep: 'kept ', skip: 'skipped ' })[v] ?? ''));
  return `${verbs.join(', ')}.`;
}

/** Bytes as "1.2 GB" (powers of 1024, one decimal under 10). */
export function size(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let n = bytes;
  let u = 0;
  while (n >= 1024 && u < units.length - 1) {
    n /= 1024;
    u++;
  }
  return `${n < 10 && u > 0 ? n.toFixed(1) : Math.round(n)} ${units[u]}`;
}
