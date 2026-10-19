import { describe, expect, it } from 'vitest';
import { ApiError } from 'miniframework-ng';
import { strikeTimes, ringSeconds, VOICES } from './bell';
import { KeyValue, newClientId, PendingEntry, SessionQueue } from './session-queue';

function store(): KeyValue & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}

const entry = (id: string, amount = 20): PendingEntry => ({
  client_id: id,
  person: 0,
  kind: 'meditation',
  amount,
  date: '2026-10-19',
  note: '',
});

describe('SessionQueue', () => {
  it('keeps sessions until the board takes them, across page loads', async () => {
    const s = store();
    new SessionQueue(s).add([entry('a'), entry('b')]);
    const later = new SessionQueue(s); // a reload
    expect(later.list().map((e) => e.client_id)).toEqual(['a', 'b']);
    const sent: string[] = [];
    const r = await later.flush(async (e) => void sent.push(e.client_id));
    expect(r).toEqual({ sent: 2, dropped: 0, left: 0 });
    expect(sent).toEqual(['a', 'b']);
  });

  it('stops at a network failure and keeps the rest', async () => {
    const q = new SessionQueue(store());
    q.add([entry('a'), entry('b'), entry('c')]);
    let calls = 0;
    const r = await q.flush(async () => {
      if (++calls === 2) throw new TypeError('Failed to fetch');
    });
    expect(r).toEqual({ sent: 1, dropped: 0, left: 2 });
    expect(q.list().map((e) => e.client_id)).toEqual(['b', 'c']);
  });

  it('drops an entry the board refuses for good, and retries a 5xx', async () => {
    const q = new SessionQueue(store());
    q.add([entry('bad'), entry('ok')]);
    const r = await q.flush(async (e) => {
      if (e.client_id === 'bad') throw new ApiError(400, 'bad_request', 'nope');
    });
    expect(r).toEqual({ sent: 1, dropped: 1, left: 0 });
    q.add([entry('later')]);
    const busy = await q.flush(async () => {
      throw new ApiError(500, 'storage_failed', 'card');
    });
    expect(busy.left).toBe(1);
  });

  it('never queues the same session twice, and survives no storage', () => {
    const q = new SessionQueue(store());
    q.add([entry('a')]);
    q.add([entry('a'), entry('b')]);
    expect(q.list().length).toBe(2);
    expect(new SessionQueue(null).list()).toEqual([]);
    expect(newClientId()).toMatch(/^[0-9a-f]{24}$/);
  });
});

describe('bells', () => {
  it('spaces the strikes of one bell by the voice', () => {
    expect(strikeTimes(10, 3, 'bowl')).toEqual([10, 15, 20]);
    expect(strikeTimes(0, 1, 'block')).toEqual([0]);
  });

  it('has inharmonic partials that fade, longest first for a bowl', () => {
    for (const v of Object.values(VOICES)) {
      expect(v.partials[0].ratio).toBe(1);
      expect(v.partials.every((p) => p.gain > 0 && p.decay > 0)).toBe(true);
    }
    expect(ringSeconds('bowl')).toBe(9);
    expect(ringSeconds('block')).toBeLessThan(0.5);
  });
});
