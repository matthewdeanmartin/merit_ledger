/**
 * Open Library, from the phone (Sprint 12). Open Library allows any origin
 * and needs no key, so the board never calls it: the phone searches, and
 * the board stores what someone saves. Browsers can't set a User-Agent, so
 * the polite part is asking only for the fields we use, 20 at a time.
 */

export interface BookHit {
  /** `OL45804W` */
  work: string;
  title: string;
  author: string;
  year: number;
  /** Cover id (0 = none). */
  cover: number;
  /** Pages, when Open Library knows (0 = not). */
  pages: number;
}

export interface ReadingView {
  id: number;
  person: number;
  person_name: string;
  work: string;
  title: string;
  author: string;
  cover: number;
  status: 'want' | 'reading' | 'done';
  page: number;
  pages: number;
  started: string;
  finished: string;
  visibility: string;
  rev: number;
  year: number;
  minutes: number;
  can_edit: boolean;
}

const FIELDS = 'key,title,author_name,first_publish_year,cover_i,number_of_pages_median';

/** An ISBN-10 or ISBN-13, with or without dashes and spaces. */
export function isIsbn(q: string): boolean {
  const digits = q.replace(/[\s-]/g, '');
  return /^(\d{9}[\dXx]|\d{13})$/.test(digits);
}

/** The search URL for what someone typed: an ISBN, or title and author words. */
export function searchUrl(q: string): string {
  const query = q.trim();
  const params = new URLSearchParams({ fields: FIELDS, limit: '20' });
  if (isIsbn(query)) params.set('isbn', query.replace(/[\s-]/g, ''));
  else params.set('q', query);
  return `https://openlibrary.org/search.json?${params.toString()}`;
}

interface SearchDoc {
  key?: string;
  title?: string;
  author_name?: string[];
  first_publish_year?: number;
  cover_i?: number;
  number_of_pages_median?: number;
}

/** A count from the reply: a whole number >= 0, or 0. */
const whole = (n: unknown) => (typeof n === 'number' && Number.isInteger(n) && n > 0 ? n : 0);

/**
 * Hits from a `search.json` reply, skipping anything without a work key or
 * title. Trusts nothing about the reply's shape (tests/props: `{"docs":0}`
 * threw).
 */
export function parseSearch(json: unknown): BookHit[] {
  const docs: unknown = (json as { docs?: unknown } | null)?.docs;
  if (!Array.isArray(docs)) return [];
  return docs
    .filter((d): d is SearchDoc => typeof d === 'object' && d !== null)
    .filter((d) => typeof d.key === 'string' && d.key.length > '/works/'.length && d.key.startsWith('/works/'))
    .filter((d) => typeof d.title === 'string' && d.title.length > 0)
    .map((d) => ({
      work: d.key!.slice('/works/'.length),
      title: d.title!,
      author: (Array.isArray(d.author_name) ? d.author_name : [])
        .filter((a): a is string => typeof a === 'string')
        .slice(0, 2)
        .join(', '),
      year: whole(d.first_publish_year),
      cover: whole(d.cover_i),
      pages: whole(d.number_of_pages_median),
    }));
}

/** A cover image, fetched by the phone from Open Library (`S`, `M` or `L`). */
export function coverUrl(id: number, size: 'S' | 'M' | 'L' = 'M'): string {
  return id ? `https://covers.openlibrary.org/b/id/${id}-${size}.jpg` : '';
}

/** "p. 40 of 54 (74%)", "p. 40", "not started". */
export function progress(page: number, pages: number): string {
  if (!page) return 'not started';
  if (!pages) return `p. ${page}`;
  return `p. ${page} of ${pages} (${Math.round((100 * page) / pages)}%)`;
}

/** The work's page on Open Library. */
export function workUrl(work: string): string {
  return work ? `https://openlibrary.org/works/${work}` : '';
}
