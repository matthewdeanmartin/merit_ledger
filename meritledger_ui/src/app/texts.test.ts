import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { blocks, frontMatter, paginate, toHtml } from './texts';

const dir = new URL('../../public/texts/', import.meta.url);

describe('built-in texts', () => {
  const index = JSON.parse(readFileSync(new URL('index.json', dir), 'utf-8')) as { texts: { id: string; title: string }[] };
  const files = readdirSync(dir).filter((f) => f.endsWith('.txt'));

  it('are all in the index, and only those', () => {
    expect(index.texts.map((t) => `${t.id}.txt`).sort()).toEqual(files.sort());
  });

  it.each(files)('%s has provenance, license and source', (file) => {
    const { meta, body } = frontMatter(readFileSync(new URL(file, dir), 'utf-8'));
    expect(meta.id).toBe(file.replace('.txt', ''));
    expect(meta.title).toBe(index.texts.find((t) => t.id === meta.id)?.title);
    expect(['llm-draft', 'llm-draft-reviewed', 'household', 'public-domain']).toContain(meta.provenance);
    expect(meta.license.length).toBeGreaterThan(3);
    expect(meta.source.length).toBeGreaterThan(3);
    if (meta.provenance === 'llm-draft-reviewed') expect(meta.reviewed_by && meta.reviewed_on).toBeTruthy();
    expect(body.length).toBeGreaterThan(20);
  });

  it('page without empty pages', () => {
    for (const file of files) {
      const { body } = frontMatter(readFileSync(new URL(file, dir), 'utf-8'));
      for (const page of paginate(body)) expect(page.join('').trim()).not.toBe('');
    }
  });
});

describe('front matter', () => {
  it('splits meta from body', () => {
    const { meta, body } = frontMatter('---\nid: x\ntitle: A: B\n---\n\n# Hi\n');
    expect(meta.title).toBe('A: B');
    expect(body).toBe('# Hi');
    expect(frontMatter('No front matter').body).toBe('No front matter');
  });
});

describe('markdown', () => {
  it('renders the subset', () => {
    expect(toHtml('# Title\nFirst line\nsecond\n\n- a\n- b\n\n1. one\n2. two')).toBe(
      '<h2>Title</h2><p>First line<br>second</p><ul><li>a</li><li>b</li></ul><ol><li>one</li><li>two</li></ol>',
    );
    expect(toHtml('**Nam** *myoho* [x](https://a.example)')).toBe(
      '<p><strong>Nam</strong> <em>myoho</em> <a href="https://a.example" rel="nofollow noopener noreferrer" target="_blank">x</a></p>',
    );
  });

  it('keeps hostile input as text', () => {
    for (const md of ['<script>alert(1)</script>', '[x](javascript:alert(1))', '<img src=x onerror=alert(1)>', '[a](https://x" onmouseover="y)']) {
      const html = toHtml(md);
      const tags = [...html.matchAll(/<\/?([a-z0-9]+)/g)].map((m) => m[1]);
      expect(tags.every((t) => ['p', 'a', 'em', 'strong', 'br'].includes(t))).toBe(true);
      expect(html).not.toMatch(/href="(?!https?:)/);
    }
  });
});

describe('pager', () => {
  const para = (n: number) => `Paragraph ${n} ${'x'.repeat(200)}`;

  it('splits on paragraph boundaries, never mid-paragraph', () => {
    const body = [1, 2, 3, 4, 5].map(para).join('\n\n');
    const pages = paginate(body, 450);
    expect(pages.map((p) => p.length)).toEqual([2, 2, 1]);
    expect(pages.flat()).toEqual(blocks(body));
  });

  it('starts a page at a heading and never ends one with a heading', () => {
    const pages = paginate(`${para(1)}\n\n# Next\n\n${para(2)}`, 1000);
    expect(pages).toEqual([[para(1)], ['# Next', para(2)]]);
    const tight = paginate(`# Head\n\n${para(1)}\n\n${para(2)}`, 100);
    expect(tight[0]).toEqual(['# Head', para(1)]);
  });

  it('keeps a long paragraph whole and returns nothing for nothing', () => {
    expect(paginate('x'.repeat(5000), 600)).toEqual([['x'.repeat(5000)]]);
    expect(paginate('  \n\n  ')).toEqual([]);
  });
});
