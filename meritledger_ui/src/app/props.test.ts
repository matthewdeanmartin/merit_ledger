/**
 * Property tests (fast-check, like Python's Hypothesis): each states what
 * must hold for every input and lets the generator hunt for, and shrink, a
 * counterexample. Aimed at what takes outside input (uploaded texts, Open
 * Library's replies, what people type) and at the date arithmetic.
 *
 * `FC_RUNS=10000 npx vitest run props` searches harder.
 */
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { isIsbn, parseSearch, progress, searchUrl } from './books';
import { addMonths, firstDay, grid, lastDay, monthOf } from './calendar';
import { fromUnix, mailto, shareTargets, toUnix } from './invite';
import { pageIndex } from './practice-doc';
import { move } from './practice';
import { initials } from './people';
import { blocks, paginate, toHtml } from './texts';

const runs = Number(globalThis.process?.env?.['FC_RUNS'] ?? 300);
const opts = { numRuns: runs };

/** Text that looks like markdown, with the characters that cause trouble. */
const markdownish = fc
  .array(
    fc.oneof(
      fc.string(),
      fc.constantFrom('# ', '## ', '- ', '1. ', '**', '*', '_', '[', '](', ')', '\n', '\n\n', '<', '>', '"', "'", '&'),
      fc.constantFrom(
        '<script>alert(1)</script>',
        '[x](javascript:alert(1))',
        '[x](https://e.com" onmouseover="alert(1))',
        '<img src=x onerror=alert(1)>',
        '[a](https://ok.example/path)',
        '](https://x.y/"><b>',
      ),
    ),
    { maxLength: 40 },
  )
  .map((parts) => parts.join(''));

/** Tags `toHtml` may produce, and the only attributes it may write. */
const ALLOWED = /^<\/?(h2|h3|h4|p|br|ul|ol|li|strong|em|a)( href="https?:\/\/[^"<>\s]+" rel="nofollow noopener noreferrer" target="_blank")?>$/;

describe('uploaded texts render safely', () => {
  it('only ever emits allowed tags and http(s) links', () => {
    fc.assert(
      fc.property(markdownish, (md) => {
        const html = toHtml(md);
        for (const tag of html.match(/<[^>]*>?/g) ?? []) {
          expect(tag, `in ${JSON.stringify(html)}`).toMatch(ALLOWED);
          if (tag.startsWith('<a ')) expect(tag).not.toMatch(/javascript:/i);
        }
      }),
      opts,
    );
  });

  it('pages keep every block, in order, and are never empty', () => {
    fc.assert(
      fc.property(markdownish, fc.integer({ min: 1, max: 2000 }), (md, size) => {
        const pages = paginate(md, size);
        for (const p of pages) expect(p.length).toBeGreaterThan(0);
        expect(pages.flat()).toEqual(blocks(md));
      }),
      opts,
    );
  });
});

describe('Open Library replies', () => {
  const doc = fc.record(
    {
      key: fc.oneof(fc.string(), fc.string().map((s) => `/works/${s}`), fc.integer(), fc.constant(null)),
      title: fc.oneof(fc.string(), fc.integer(), fc.constant(null)),
      author_name: fc.oneof(fc.array(fc.string()), fc.string(), fc.constant(null), fc.array(fc.integer())),
      first_publish_year: fc.oneof(fc.integer(), fc.string(), fc.constant(null)),
      cover_i: fc.oneof(fc.integer(), fc.string(), fc.constant(null)),
      number_of_pages_median: fc.oneof(fc.integer(), fc.string(), fc.constant(null)),
    },
    { requiredKeys: [] },
  );
  const reply = fc.oneof(
    fc.record({ docs: fc.array(fc.oneof(doc, fc.constant(null), fc.integer(), fc.string()), { maxLength: 8 }) }),
    fc.record({ docs: fc.oneof(fc.constant(null), fc.string(), fc.integer(), fc.object()) }),
    fc.anything(),
  );

  it('never throw, and every hit is well formed', () => {
    fc.assert(
      fc.property(reply, (json) => {
        for (const hit of parseSearch(json)) {
          expect(typeof hit.work).toBe('string');
          expect(hit.work.length).toBeGreaterThan(0);
          expect(typeof hit.title).toBe('string');
          expect(hit.title.length).toBeGreaterThan(0);
          expect(typeof hit.author).toBe('string');
          for (const n of [hit.year, hit.cover, hit.pages]) expect(Number.isInteger(n) && n >= 0).toBe(true);
        }
      }),
      opts,
    );
  });

  it('searches stay on openlibrary.org, whatever is typed', () => {
    fc.assert(
      fc.property(fc.string(), (q) => {
        const url = new URL(searchUrl(q));
        expect(url.origin).toBe('https://openlibrary.org');
        expect(url.searchParams.get(isIsbn(q.trim()) ? 'isbn' : 'q')).not.toBeNull();
      }),
      opts,
    );
  });

  it('progress reads sensibly', () => {
    fc.assert(
      fc.property(fc.nat(100_000), fc.nat(100_000), (page, pages) => {
        expect(progress(page, pages)).not.toMatch(/NaN|Infinity|undefined/);
      }),
      opts,
    );
  });
});

describe('dates', () => {
  const month = fc.record({ year: fc.integer({ min: 1900, max: 2200 }), month: fc.integer({ min: 1, max: 12 }) });

  it('adding months composes and stays 1-12', () => {
    fc.assert(
      fc.property(month, fc.integer({ min: -2400, max: 2400 }), fc.integer({ min: -2400, max: 2400 }), (m, a, b) => {
        const once = addMonths(m, a + b);
        expect(addMonths(addMonths(m, a), b)).toEqual(once);
        expect(once.month).toBeGreaterThanOrEqual(1);
        expect(once.month).toBeLessThanOrEqual(12);
        expect(monthOf(firstDay(once))).toEqual(once);
      }),
      opts,
    );
  });

  it('a month grid holds each day once, in week rows', () => {
    fc.assert(
      fc.property(month, (m) => {
        const weeks = grid(m);
        for (const w of weeks) expect(w.length).toBe(7);
        const days = weeks.flat().filter((d): d is string => d !== null);
        expect(days[0]).toBe(firstDay(m));
        expect(days[days.length - 1]).toBe(lastDay(m));
        expect(new Set(days).size).toBe(days.length);
        expect(days.length).toBe(Number(lastDay(m).slice(8)));
      }),
      opts,
    );
  });

  it('event times round-trip through the date and time inputs', () => {
    // Whole minutes, 2000-2100 (what the inputs can say).
    fc.assert(
      fc.property(fc.integer({ min: 946_684_800 / 60, max: 4_102_444_800 / 60 }), (minutes) => {
        const t = minutes * 60;
        const [date, time] = fromUnix(t);
        const back = toUnix(date, time);
        // A daylight-saving fold can make one local time mean two instants.
        expect(Math.abs(back - t) === 0 || Math.abs(back - t) === 3600).toBe(true);
      }),
      opts,
    );
  });
});

describe('invitations', () => {
  const invite = fc.record({ subject: fc.string(), body: fc.string() });

  it('mailto links carry the text exactly', () => {
    fc.assert(
      fc.property(invite, (inv) => {
        const url = new URL(mailto(inv));
        expect(url.protocol).toBe('mailto:');
        const params = new URLSearchParams(url.search);
        expect(params.get('subject') ?? '').toBe(inv.subject);
        expect(params.get('body') ?? '').toBe(inv.body);
      }),
      opts,
    );
  });

  it('share targets only use http(s), mailto or sms', () => {
    fc.assert(
      fc.property(invite, (inv) => {
        for (const t of shareTargets(inv)) expect(new URL(t.href).protocol).toMatch(/^(https?|mailto|sms):$/);
      }),
      opts,
    );
  });
});

describe('small helpers', () => {
  it('moving a practice keeps the same practices', () => {
    fc.assert(
      fc.property(fc.uniqueArray(fc.string(), { maxLength: 10 }), fc.string(), fc.constantFrom(-1 as const, 1 as const), (keys, key, by) => {
        expect([...move(keys, key, by)].sort()).toEqual([...keys].sort());
      }),
      opts,
    );
  });

  it('page numbers from the URL land on a page', () => {
    fc.assert(
      fc.property(fc.option(fc.string(), { nil: undefined }), fc.integer({ min: 1, max: 500 }), (param, total) => {
        const i = pageIndex(param, total);
        expect(Number.isInteger(i) && i >= 0 && i < total).toBe(true);
      }),
      opts,
    );
  });

  it('initials are short and never throw', () => {
    fc.assert(
      fc.property(fc.string(), (name) => {
        expect([...initials(name)].length).toBeLessThanOrEqual(2);
      }),
      opts,
    );
  });
});
