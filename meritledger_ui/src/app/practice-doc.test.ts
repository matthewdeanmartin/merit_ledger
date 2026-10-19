import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { newStep, pageIndex, pageUrl, practicePages, STEP_KINDS, stepAmount, stepHeading, TEMPLATES } from './practice-doc';

const builtins = (
  JSON.parse(readFileSync(new URL('../../public/texts/index.json', import.meta.url), 'utf-8')) as { texts: { id: string }[] }
).texts.map((t) => t.id);
const kinds = (
  JSON.parse(readFileSync(new URL('../../../meritledger_rs/catalog/practices.json', import.meta.url), 'utf-8')) as {
    kinds: { key: string }[];
  }
).kinds.map((k) => k.key);
const timers = (
  JSON.parse(readFileSync(new URL('../../../meritledger_rs/catalog/timers.json', import.meta.url), 'utf-8')) as {
    timers: { key: string }[];
  }
).timers.map((t) => t.key);

describe('templates', () => {
  it.each(TEMPLATES)('$key uses real texts, practices and timers', (t) => {
    for (const s of t.steps) {
      expect(STEP_KINDS.map((k) => k.key)).toContain(s.kind);
      if (s.text) expect(builtins).toContain(s.text);
      if (s.logs_as) expect(kinds).toContain(s.logs_as);
      if (s.timer) expect(timers).toContain(s.timer);
    }
  });

  it('include SGI-style gongyo: chapters 2 and 16, then daimoku', () => {
    const g = TEMPLATES.find((t) => t.key === 'gongyo')!;
    expect(g.steps.map((s) => s.text)).toEqual(['daimoku', 'lotus-2', 'lotus-16-verses', 'daimoku', 'silent-prayers', 'daimoku']);
    expect(g.steps.filter((s) => s.logs_as).map((s) => s.logs_as)).toEqual(['recite', 'daimoku', 'dedication']);
  });
});

describe('steps', () => {
  it('have headings and amounts', () => {
    expect(stepHeading(newStep('recite'), 'Heart Sutra')).toBe('Recite: Heart Sutra');
    expect(stepHeading({ ...newStep('chant'), title: 'Three daimoku' })).toBe('Three daimoku');
    expect(stepAmount({ minutes: 20, count: 0 })).toBe('20 min');
    expect(stepAmount({ minutes: 0, count: 1080 })).toBe('1,080×');
  });
});

describe('follow-along pages', () => {
  const steps = [newStep('meditate'), { ...newStep('recite'), text: 'long' }, newStep('note')];
  const long = Array.from({ length: 6 }, (_, i) => `Para ${i} ${'x'.repeat(250)}`).join('\n\n');

  it('give every step a page and run long texts on', () => {
    const pages = practicePages(steps, { long }, 600);
    expect(pages.map((p) => [p.step, p.first])).toEqual([
      [0, true],
      [1, true],
      [1, false],
      [1, false],
      [2, true],
    ]);
    expect(pages.every((p) => p.step !== 1 || p.blocks.length > 0)).toBe(true);
  });

  it('round-trip page numbers through URLs', () => {
    for (let i = 0; i < 5; i++) {
      const url = pageUrl(7, i);
      expect(pageIndex(url.split('/').pop(), 5)).toBe(i);
    }
    expect(pageUrl(7, 0)).toBe('/practice/7/1');
    expect(pageIndex('99', 5)).toBe(4);
    expect(pageIndex('0', 5)).toBe(0);
    expect(pageIndex('abc', 5)).toBe(0);
    expect(pageIndex(undefined, 5)).toBe(0);
  });
});
