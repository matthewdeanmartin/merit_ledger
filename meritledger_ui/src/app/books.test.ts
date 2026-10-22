import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { coverUrl, isIsbn, parseSearch, progress, searchUrl } from './books';

// A real reply, recorded once (no live calls in tests).
const fixture = JSON.parse(readFileSync(new URL('./fixtures/ol-search-lotus-sutra.json', import.meta.url), 'utf-8'));

describe('Open Library search', () => {
  it('parses a recorded reply', () => {
    const hits = parseSearch(fixture);
    expect(hits).toHaveLength(3);
    expect(hits[0]).toEqual({
      work: 'OL19812913W',
      title: 'The Lotus Sutra',
      author: 'Tsugunari Kubo, Akira Yuyama',
      year: 2007,
      cover: 5020364,
      pages: 362,
    });
    expect(hits[2].author).toBe('Burton Watson');
  });

  it('skips docs without a work or a title, and survives junk', () => {
    expect(parseSearch({ docs: [{ title: 'No key' }, { key: '/books/OL1M', title: 'An edition' }, { key: '/works/OL2W' }] })).toEqual([]);
    expect(parseSearch(null)).toEqual([]);
    expect(parseSearch({ docs: [{ key: '/works/OL3W', title: 'Bare' }] })).toEqual([
      { work: 'OL3W', title: 'Bare', author: '', year: 0, cover: 0, pages: 0 },
    ]);
  });

  it('searches by ISBN or by words', () => {
    expect(isIsbn('978-1-888375-27-5')).toBe(true);
    expect(isIsbn('093807716X')).toBe(true);
    expect(isIsbn('lotus sutra')).toBe(false);
    expect(searchUrl('978-1-888375-27-5')).toContain('isbn=9781888375275');
    const words = searchUrl('  lotus sutra watson ');
    expect(words).toContain('q=lotus+sutra+watson');
    expect(words).toContain('limit=20');
    expect(words).toContain('fields=key%2Ctitle');
  });

  it('builds covers and progress', () => {
    expect(coverUrl(5020364)).toBe('https://covers.openlibrary.org/b/id/5020364-M.jpg');
    expect(coverUrl(0)).toBe('');
    expect(progress(0, 362)).toBe('not started');
    expect(progress(40, 0)).toBe('p. 40');
    expect(progress(181, 362)).toBe('p. 181 of 362 (50%)');
  });
});
