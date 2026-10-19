import { describe, expect, it } from 'vitest';
import { bellsFrom, Clock, clockText, countCredit, credit, plan, position } from './timer';

const sesshin = {
  segments: [
    { kind: 'sit' as const, minutes: 25 },
    { kind: 'walk' as const, minutes: 5 },
    { kind: 'sit' as const, minutes: 25 },
  ],
  open_strikes: 3,
  interval_minutes: 0,
  close_strikes: 3,
};

describe('plan', () => {
  it('lays out parts and bells for sit, walk, sit', () => {
    const p = plan(sesshin);
    expect(p.total).toBe(55 * 60);
    expect(p.phases.map((ph) => [ph.kind, ph.start, ph.end])).toEqual([
      ['sit', 0, 1500],
      ['walk', 1500, 1800],
      ['sit', 1800, 3300],
    ]);
    expect(p.bells).toEqual([
      { at: 0, strikes: 3 },
      { at: 1500, strikes: 1 },
      { at: 1800, strikes: 1 },
      { at: 3300, strikes: 3 },
    ]);
  });

  it('rings interval bells inside sitting parts, not right before the end', () => {
    const p = plan({ segments: [{ kind: 'sit', minutes: 30 }], open_strikes: 1, interval_minutes: 10, close_strikes: 1 });
    expect(p.bells.map((b) => b.at)).toEqual([0, 600, 1200, 1800]);
    const tight = plan({ segments: [{ kind: 'sit', minutes: 20 }], open_strikes: 0, interval_minutes: 10, close_strikes: 0 });
    // 1200 s would collide with the end of the part: no bell there.
    expect(tight.bells.map((b) => b.at)).toEqual([600]);
  });
});

describe('position', () => {
  const p = plan(sesshin);
  it('knows the part and the time left', () => {
    expect(position(p, 0)).toMatchObject({ index: 0, remaining: 1500, remainingTotal: 3300, done: false });
    expect(position(p, 1600)).toMatchObject({ index: 1, remaining: 200, done: false });
    expect(position(p, 1600).phase.kind).toBe('walk');
  });

  it('is done at the end, however late the tab looks', () => {
    expect(position(p, 3300)).toMatchObject({ done: true, remaining: 0, remainingTotal: 0, index: 2 });
    expect(position(p, 99_999).done).toBe(true);
  });
});

describe('bellsFrom', () => {
  it('keeps only the bells still to come after a resume', () => {
    const p = plan(sesshin);
    expect(bellsFrom(p, 1600).map((b) => b.at)).toEqual([1800, 3300]);
  });
});

describe('credit', () => {
  const p = plan(sesshin);
  it('logs sitting and walking separately for a full session', () => {
    expect(credit(p, 3300, 'meditation')).toEqual([
      { kind: 'meditation', amount: 50 },
      { kind: 'walking', amount: 5 },
    ]);
  });

  it('logs what was sat when interrupted, and nothing under a minute', () => {
    expect(credit(p, 12 * 60 + 59, 'meditation')).toEqual([{ kind: 'meditation', amount: 12 }]);
    expect(credit(p, 1620, 'meditation')).toEqual([
      { kind: 'meditation', amount: 25 },
      { kind: 'walking', amount: 2 },
    ]);
    expect(credit(p, 59, 'meditation')).toEqual([]);
  });

  it('logs a count as counted', () => {
    expect(countCredit(108, 'daimoku')).toEqual([{ kind: 'daimoku', amount: 108 }]);
    expect(countCredit(0, 'daimoku')).toEqual([]);
  });
});

describe('Clock', () => {
  it('excludes pauses and needs no ticks', () => {
    let now = 1_000;
    const c = new Clock(() => now);
    c.start();
    now += 60_000;
    expect(c.elapsed()).toBe(60);
    c.pause();
    now += 600_000; // ten paused minutes
    expect(c.elapsed()).toBe(60);
    c.resume();
    now += 30_000;
    expect(c.elapsed()).toBe(90);
    expect(c.running).toBe(true);
  });

  it('keeps counting through a locked phone (no ticks at all)', () => {
    let now = 0;
    const c = new Clock(() => now);
    c.start();
    now += 21 * 60_000; // locked for 21 minutes
    const p = plan({ segments: [{ kind: 'sit', minutes: 20 }], open_strikes: 3, interval_minutes: 0, close_strikes: 1 });
    expect(position(p, c.elapsed()).done).toBe(true);
    expect(credit(p, c.elapsed(), 'meditation')).toEqual([{ kind: 'meditation', amount: 20 }]);
  });
});

describe('clockText', () => {
  it('rounds up and shows hours past an hour', () => {
    expect(clockText(245.2)).toBe('4:06');
    expect(clockText(59)).toBe('0:59');
    expect(clockText(3723)).toBe('1:02:03');
    expect(clockText(-3)).toBe('0:00');
  });
});
