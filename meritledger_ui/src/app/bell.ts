/**
 * Bells, synthesized with the Web Audio API (Sprint 6): no sound files, no
 * licences, nothing for the board to serve. A strike is a few sine partials
 * at inharmonic ratios, each fading at its own rate, which is what makes a
 * bowl sound like a bowl and not a beep.
 *
 * Bells are scheduled ahead on the `AudioContext` clock, so they ring on
 * time even while the tab's timers are throttled.
 */

export type Timbre = 'bowl' | 'bell' | 'block';

export interface Partial {
  /** Frequency as a multiple of the voice's base. */
  ratio: number;
  gain: number;
  /** Seconds for the partial to fade by 60 dB. */
  decay: number;
}

export interface Voice {
  base: number;
  partials: Partial[];
  /** Seconds between strikes of one bell. */
  spacing: number;
}

/** Ratios from measured singing bowls and handbells, rounded. */
export const VOICES: Record<Timbre, Voice> = {
  bowl: {
    base: 196,
    partials: [
      { ratio: 1, gain: 0.55, decay: 9 },
      { ratio: 2.76, gain: 0.3, decay: 6 },
      { ratio: 5.4, gain: 0.12, decay: 3.5 },
      { ratio: 8.93, gain: 0.05, decay: 2 },
    ],
    spacing: 5,
  },
  bell: {
    base: 523,
    partials: [
      { ratio: 1, gain: 0.5, decay: 5 },
      { ratio: 2.0, gain: 0.2, decay: 3 },
      { ratio: 2.4, gain: 0.25, decay: 3.5 },
      { ratio: 3.0, gain: 0.08, decay: 2 },
      { ratio: 4.5, gain: 0.06, decay: 1.2 },
    ],
    spacing: 3.5,
  },
  block: {
    base: 820,
    partials: [
      { ratio: 1, gain: 0.6, decay: 0.12 },
      { ratio: 2.62, gain: 0.25, decay: 0.07 },
    ],
    spacing: 0.9,
  },
};

/** When each strike of a bell sounds, from its first at `at` (seconds). */
export function strikeTimes(at: number, strikes: number, timbre: Timbre): number[] {
  return Array.from({ length: strikes }, (_, i) => at + i * VOICES[timbre].spacing);
}

/** How long a strike rings, for scheduling and for stopping it. */
export function ringSeconds(timbre: Timbre): number {
  return Math.max(...VOICES[timbre].partials.map((p) => p.decay));
}

/**
 * One strike at `when` (AudioContext seconds). Returns a function that
 * silences it (for "End" before the strike has finished).
 */
export function strike(ctx: AudioContext, when: number, timbre: Timbre, volume = 0.5): () => void {
  const voice = VOICES[timbre];
  const out = ctx.createGain();
  out.gain.value = volume;
  out.connect(ctx.destination);
  const oscillators: OscillatorNode[] = [];
  for (const p of voice.partials) {
    const osc = ctx.createOscillator();
    const env = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.value = voice.base * p.ratio;
    // A fast attack (a mallet), then an exponential fade: -60 dB at `decay`.
    env.gain.setValueAtTime(0.0001, when);
    env.gain.exponentialRampToValueAtTime(p.gain, when + 0.008);
    env.gain.exponentialRampToValueAtTime(p.gain / 1000, when + p.decay);
    osc.connect(env).connect(out);
    osc.start(when);
    osc.stop(when + p.decay + 0.05);
    oscillators.push(osc);
  }
  return () => {
    for (const osc of oscillators) {
      try {
        osc.stop();
      } catch {
        // Already stopped.
      }
    }
    out.disconnect();
  };
}

/** Bells scheduled ahead, which can all be silenced at once (pause, end). */
export class BellSchedule {
  private stops: (() => void)[] = [];

  constructor(private readonly ctx: AudioContext) {}

  /**
   * Schedules `bells` (session seconds) for a session at `elapsed` seconds
   * now, replacing whatever was scheduled.
   */
  schedule(bells: { at: number; strikes: number }[], elapsed: number, timbre: Timbre): void {
    this.cancel();
    const now = this.ctx.currentTime;
    for (const b of bells) {
      for (const t of strikeTimes(b.at, b.strikes, timbre)) {
        const when = now + Math.max(0, t - elapsed);
        this.stops.push(strike(this.ctx, when, timbre));
      }
    }
  }

  /** One bell now (a preview, or a count's closing bell). */
  ringNow(strikes: number, timbre: Timbre): void {
    for (const t of strikeTimes(this.ctx.currentTime + 0.05, strikes, timbre)) {
      this.stops.push(strike(this.ctx, t, timbre));
    }
  }

  cancel(): void {
    for (const stop of this.stops) stop();
    this.stops = [];
  }
}
