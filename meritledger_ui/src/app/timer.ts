/**
 * The meditation timer's engine (Sprint 6): pure functions over a preset,
 * plus a pausable clock. No DOM, no audio: `timer-page` drives it, `bell.ts`
 * makes the sound, and tests use a fake clock.
 *
 * Time comes from wall-clock deltas (`Date.now()` by default), never from
 * counting `setInterval` ticks: a throttled or locked tab then still knows
 * exactly how long it has run. (Wall-clock rather than `performance.now()`
 * because some phones stop the monotonic clock while asleep, and a locked
 * phone must still end the sit on time.)
 */

export type SegmentKind = 'sit' | 'walk';

export interface Segment {
  kind: SegmentKind;
  minutes: number;
}

export interface TimerPreset {
  key: string;
  name: string;
  segments: Segment[];
  open_strikes: number;
  interval_minutes: number;
  close_strikes: number;
  /** A count preset's target (a mala); 0 for a timed one. */
  count_target: number;
  /** Practice kind that sitting (or the count) logs as. */
  logs_as: string;
  timbre: 'bowl' | 'bell' | 'block';
  owner: number;
  archived: boolean;
  builtin: boolean;
}

/** One part of a timed session, in seconds from its start. */
export interface Phase {
  kind: SegmentKind;
  start: number;
  end: number;
}

/** A bell: `strikes` strikes starting at `at` seconds. */
export interface Bell {
  at: number;
  strikes: number;
}

export interface Plan {
  phases: Phase[];
  bells: Bell[];
  /** Seconds. */
  total: number;
}

/** No interval bell this close (seconds) to the end of a sitting part. */
const INTERVAL_MARGIN = 30;

/**
 * Phases and bells for a timed preset: the opening bell, one strike at
 * every change of part, interval bells inside sitting parts, and the
 * closing bell at the end.
 */
export function plan(p: Pick<TimerPreset, 'segments' | 'open_strikes' | 'interval_minutes' | 'close_strikes'>): Plan {
  const phases: Phase[] = [];
  const bells: Bell[] = [];
  let t = 0;
  for (const seg of p.segments) {
    phases.push({ kind: seg.kind, start: t, end: t + seg.minutes * 60 });
    t += seg.minutes * 60;
  }
  if (p.open_strikes > 0) bells.push({ at: 0, strikes: p.open_strikes });
  phases.forEach((ph, i) => {
    if (i > 0) bells.push({ at: ph.start, strikes: 1 });
    if (ph.kind === 'sit' && p.interval_minutes > 0) {
      for (let at = ph.start + p.interval_minutes * 60; at < ph.end - INTERVAL_MARGIN; at += p.interval_minutes * 60) {
        bells.push({ at, strikes: 1 });
      }
    }
  });
  if (p.close_strikes > 0 && t > 0) bells.push({ at: t, strikes: p.close_strikes });
  bells.sort((a, b) => a.at - b.at);
  return { phases, bells, total: t };
}

export interface Position {
  /** Index of the current phase (the last one once done). */
  index: number;
  phase: Phase;
  /** Seconds left in this phase, and in the whole session. */
  remaining: number;
  remainingTotal: number;
  done: boolean;
}

export function position(pl: Plan, elapsed: number): Position {
  const e = Math.max(0, elapsed);
  let index = pl.phases.findIndex((ph) => e < ph.end);
  const done = index < 0;
  if (done) index = pl.phases.length - 1;
  const phase = pl.phases[index];
  return {
    index,
    phase,
    remaining: done ? 0 : phase.end - e,
    remainingTotal: Math.max(0, pl.total - e),
    done,
  };
}

/** Bells still to ring after `elapsed` seconds (for scheduling on resume). */
export function bellsFrom(pl: Plan, elapsed: number): Bell[] {
  return pl.bells.filter((b) => b.at >= elapsed - 0.25);
}

/** One entry the finished session logs. */
export interface Credit {
  kind: string;
  amount: number;
}

/**
 * What a session that ran `elapsed` seconds logs: whole minutes of
 * sitting as `logsAs` and of walking as `walking`, each only if at least
 * a minute (an interrupted sit logs what was sat).
 */
export function credit(pl: Plan, elapsed: number, logsAs: string): Credit[] {
  const seconds: Record<string, number> = {};
  for (const ph of pl.phases) {
    const sat = Math.max(0, Math.min(elapsed, ph.end) - ph.start);
    const kind = ph.kind === 'walk' ? 'walking' : logsAs;
    seconds[kind] = (seconds[kind] ?? 0) + sat;
  }
  return Object.entries(seconds)
    .map(([kind, s]) => ({ kind, amount: Math.floor(s / 60 + 1e-9) }))
    .filter((c) => c.amount >= 1);
}

/** What a count session logs: the beads counted, if any. */
export function countCredit(count: number, logsAs: string): Credit[] {
  return count > 0 ? [{ kind: logsAs, amount: count }] : [];
}

/** A stopwatch that can pause, reading time from `now` (ms). */
export class Clock {
  private startedAt: number | null = null;
  /** Milliseconds accumulated before the current run. */
  private banked = 0;

  constructor(private readonly now: () => number = () => Date.now()) {}

  get running(): boolean {
    return this.startedAt !== null;
  }

  start(): void {
    this.banked = 0;
    this.startedAt = this.now();
  }

  pause(): void {
    if (this.startedAt === null) return;
    this.banked += this.now() - this.startedAt;
    this.startedAt = null;
  }

  resume(): void {
    if (this.startedAt === null) this.startedAt = this.now();
  }

  /** Seconds run so far, pauses excluded. */
  elapsed(): number {
    const live = this.startedAt === null ? 0 : this.now() - this.startedAt;
    return (this.banked + live) / 1000;
  }
}

/** "4:05", or "1:02:03" past an hour. */
export function clockText(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = String(s % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${sec}` : `${m}:${sec}`;
}

/** The phone's local date, `YYYY-MM-DD` (the household's, in practice). */
export function localDate(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
