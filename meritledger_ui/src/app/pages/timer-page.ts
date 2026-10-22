import { Component, computed, inject, input, OnDestroy, OnInit, signal } from '@angular/core';
import { ContentApi } from '../content-api';
import { Api, KindView, problem, TimerInput } from '../api';
import { BellSchedule, Timbre } from '../bell';
import { amountLabel } from '../practice';
import { Sessions } from '../sessions';
import { bellsFrom, Clock, clockText, countCredit, credit, Credit, plan, Plan, position, Segment, TimerPreset } from '../timer';

type Mode = 'pick' | 'run' | 'count' | 'done';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/**
 * The meditation timer (Sprint 6): bells on this phone, the screen kept on
 * (HTTPS), and one entry logged when the session ends.
 */
@Component({
  selector: 'ml-timer',
  template: `
    @switch (mode()) {
      @case ('pick') {
        <h1>Timer</h1>
        @if (!secure) {
          <p class="card muted">This page isn't on HTTPS, so it can't keep your screen on. Keep the screen awake
            yourself, or use <a href="https://meritledger.local/timer">https://meritledger.local</a>
            (once this phone trusts the household certificate: <a href="/trust">how</a>).</p>
        }
        @if (sessions.waiting() > 0) {
          <p class="card muted">{{ sessions.waiting() }} finished session{{ sessions.waiting() === 1 ? ' is' : 's are' }}
            waiting to reach the board. <button type="button" class="link" (click)="retry()">Try now</button></p>
        }
        @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
        @if (suggested(); as t) {
          <section class="card">
            <p>For your practice: <strong>{{ t.name }}</strong> <span class="muted small">{{ describe(t) }}</span></p>
            <button type="button" (click)="start(t)">Start</button>
          </section>
        }
        <ul class="presets">
          @for (t of live(); track t.key) {
            <li class="card">
              <div>
                <strong>{{ t.name }}</strong>
                <span class="muted small">{{ describe(t) }}</span>
              </div>
              <div class="row">
                <button type="button" (click)="start(t)">Start</button>
                @if (!t.builtin) {
                  <button type="button" class="secondary small" (click)="edit(t)">Edit</button>
                }
              </div>
            </li>
          }
        </ul>
        <section class="card">
          <h2>{{ editing() ? 'Edit ' + editing()!.name : 'Your own timer' }}</h2>
          <form (submit)="save($event)">
            <label>Name <input maxlength="32" required [value]="name()" (input)="name.set(v($event))" /></label>
            <div class="segmented small" role="radiogroup" aria-label="Kind of timer">
              <button type="button" [class.on]="!counting()" (click)="counting.set(false)">Timed</button>
              <button type="button" [class.on]="counting()" (click)="counting.set(true)">Counted (mala)</button>
            </div>
            @if (counting()) {
              <label>Count to <input type="number" min="1" max="100000" inputmode="numeric" [value]="target()" (input)="target.set(+v($event))" /></label>
            } @else {
              @for (s of segments(); track $index) {
                <div class="row">
                  <label>Part {{ $index + 1 }}
                    <select [value]="s.kind" (change)="setSegment($index, { kind: $any(v($event)) })">
                      <option value="sit">Sit</option>
                      <option value="walk">Walk</option>
                    </select>
                  </label>
                  <label>Minutes <input type="number" min="1" max="240" inputmode="numeric" [value]="s.minutes" (input)="setSegment($index, { minutes: +v($event) })" /></label>
                  @if (segments().length > 1) {
                    <button type="button" class="secondary small" (click)="removeSegment($index)">Remove</button>
                  }
                </div>
              }
              <div class="row">
                <button type="button" class="secondary small" (click)="addSegment()">Add a part</button>
                <label>Bell every <input type="number" min="0" max="120" inputmode="numeric" [value]="interval()" (input)="interval.set(+v($event))" /> min (0 = none)</label>
              </div>
            }
            <div class="row">
              <label>Log it as
                <select [value]="logsAs()" (change)="logsAs.set(v($event))">
                  @for (k of loggable(); track k.key) { <option [value]="k.key">{{ k.icon }} {{ k.name }}</option> }
                </select>
              </label>
              <label>Sound
                <select [value]="timbre()" (change)="timbre.set($any(v($event)))">
                  <option value="bowl">Singing bowl</option>
                  <option value="bell">Small bell</option>
                  <option value="block">Wooden block</option>
                </select>
              </label>
              <button type="button" class="secondary small" (click)="preview()">Hear it</button>
            </div>
            <div class="row">
              <button type="submit" [disabled]="busy()">{{ editing() ? 'Save' : 'Add' }}</button>
              @if (editing()) {
                <button type="button" class="secondary" (click)="archive()">Remove this timer</button>
                <button type="button" class="secondary" (click)="resetForm()">Cancel</button>
              }
            </div>
          </form>
        </section>
      }

      @case ('run') {
        <section class="session" [class.walking]="pos()?.phase?.kind === 'walk'">
          <p class="muted">{{ preset()?.name }}</p>
          <p class="phase">{{ pos()?.phase?.kind === 'walk' ? 'Walking' : 'Sitting' }}
            @if ((plan()?.phases?.length ?? 0) > 1) { <span class="muted">· part {{ (pos()?.index ?? 0) + 1 }} of {{ plan()?.phases?.length }}</span> }</p>
          <p class="big" aria-live="off">{{ clock(pos()?.remaining ?? 0) }}</p>
          <div class="bar-track" aria-hidden="true"><span class="bar-fill" [style.width.%]="progress()"></span></div>
          <p class="muted">{{ clock(pos()?.remainingTotal ?? 0) }} left in all
            @if (paused()) { · paused }
            @if (!awake()) { · <span title="The screen may lock: bells may not sound until you unlock it">screen may sleep</span> }</p>
          <div class="row center">
            <button type="button" class="secondary" (click)="togglePause()">{{ paused() ? 'Resume' : 'Pause' }}</button>
            <button type="button" class="secondary" (click)="end()">End</button>
          </div>
        </section>
      }

      @case ('count') {
        <section class="session count">
          <p class="muted">{{ preset()?.name }}</p>
          <button type="button" class="beads" (click)="bead()" [attr.aria-label]="'Count, ' + count() + ' of ' + preset()?.count_target">
            <span class="big">{{ count() }}</span>
            <span class="muted">of {{ preset()?.count_target }}</span>
          </button>
          <div class="row center">
            <button type="button" class="secondary" [disabled]="count() === 0" (click)="count.set(count() - 1)">−1</button>
            <button type="button" class="secondary" (click)="end()">Finish</button>
          </div>
        </section>
      }

      @case ('done') {
        <section class="card session-done">
          <h1>🙏</h1>
          @if (logged().length === 0) {
            <p>Nothing to log: less than a minute.</p>
          } @else {
            <ul class="entries">
              @for (c of logged(); track c.kind) {
                <li><span>{{ kindName(c.kind) }} · {{ amount(c) }}</span></li>
              }
            </ul>
            <p class="muted">{{ outcome() }}</p>
          }
          <button type="button" (click)="mode.set('pick')">Done</button>
        </section>
      }
    }
  `,
})
export class TimerPage implements OnInit, OnDestroy {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  readonly sessions = inject(Sessions);
  readonly mode = signal<Mode>('pick');
  readonly presets = signal<TimerPreset[]>([]);
  readonly kinds = signal<KindView[]>([]);
  readonly live = computed(() => this.presets().filter((t) => !t.archived));
  /** Query parameter `?from=<preset>`: a practice step's timer, offered first. */
  readonly from = input<string>();
  readonly suggested = computed(() => this.live().find((t) => t.key === this.from()) ?? null);
  readonly preset = signal<TimerPreset | null>(null);
  readonly plan = signal<Plan | null>(null);
  readonly elapsed = signal(0);
  readonly paused = signal(false);
  readonly awake = signal(false);
  readonly count = signal(0);
  readonly logged = signal<Credit[]>([]);
  readonly outcome = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly pos = computed(() => {
    const p = this.plan();
    return p ? position(p, this.elapsed()) : null;
  });
  readonly progress = computed(() => {
    const p = this.plan();
    return p && p.total > 0 ? Math.min(100, (100 * this.elapsed()) / p.total) : 0;
  });
  // The editor.
  readonly editing = signal<TimerPreset | null>(null);
  readonly name = signal('');
  readonly counting = signal(false);
  readonly target = signal(108);
  readonly segments = signal<Segment[]>([{ kind: 'sit', minutes: 20 }]);
  readonly interval = signal(0);
  readonly logsAs = signal('meditation');
  readonly timbre = signal<Timbre>('bowl');
  readonly loggable = computed(() =>
    this.kinds().filter((k) => !k.archived && k.polarity === 'merit' && k.unit === (this.counting() ? 'count' : 'minutes')),
  );
  readonly secure = globalThis.isSecureContext ?? false;
  readonly v = value;
  readonly clock = clockText;

  private readonly stopwatch = new Clock();
  private ctx: AudioContext | null = null;
  private bells: BellSchedule | null = null;
  private wakeLock: WakeLockSentinel | null = null;
  private tick?: ReturnType<typeof setInterval>;
  private readonly onVisible = () => void this.visible();

  async ngOnInit(): Promise<void> {
    document.addEventListener('visibilitychange', this.onVisible);
    await this.attempt(async () => {
      const [presets, kinds] = await Promise.all([this.content.timers(), this.content.kinds()]);
      this.presets.set(presets);
      this.kinds.set(kinds);
    });
    void this.sessions.flush().catch(() => undefined);
  }

  ngOnDestroy(): void {
    document.removeEventListener('visibilitychange', this.onVisible);
    this.stop();
  }

  describe(t: TimerPreset): string {
    if (t.count_target > 0) return `count to ${t.count_target} · logs ${this.kindName(t.logs_as)}`;
    const parts = t.segments.map((s) => `${s.minutes} ${s.kind}`).join(' + ');
    const every = t.interval_minutes ? ` · bell every ${t.interval_minutes} min` : '';
    return `${parts}${every}`;
  }

  kindName(key: string): string {
    return this.kinds().find((k) => k.key === key)?.name ?? key;
  }

  amount(c: Credit): string {
    const unit = this.kinds().find((k) => k.key === c.kind)?.unit ?? 'minutes';
    return amountLabel(unit, c.amount);
  }

  /** Audio must start from a tap (autoplay rules): make or wake it here. */
  private audio(): BellSchedule {
    if (!this.ctx) {
      this.ctx = new AudioContext();
      this.bells = new BellSchedule(this.ctx);
    }
    void this.ctx.resume();
    return this.bells!;
  }

  async start(t: TimerPreset): Promise<void> {
    this.preset.set(t);
    this.paused.set(false);
    this.logged.set([]);
    const bells = this.audio();
    await this.keepAwake();
    this.stopwatch.start();
    this.elapsed.set(0);
    if (t.count_target > 0) {
      this.count.set(0);
      this.plan.set(null);
      bells.ringNow(t.open_strikes, t.timbre);
      this.mode.set('count');
      return;
    }
    const p = plan(t);
    this.plan.set(p);
    bells.schedule(p.bells, 0, t.timbre);
    this.mode.set('run');
    this.tick = setInterval(() => this.update(), 250);
  }

  private update(): void {
    this.elapsed.set(this.stopwatch.elapsed());
    if (this.pos()?.done) void this.finish();
  }

  togglePause(): void {
    const t = this.preset();
    const p = this.plan();
    if (!t || !p) return;
    if (this.paused()) {
      this.stopwatch.resume();
      this.audio().schedule(bellsFrom(p, this.stopwatch.elapsed()), this.stopwatch.elapsed(), t.timbre);
    } else {
      this.stopwatch.pause();
      this.bells?.cancel();
    }
    this.paused.set(!this.paused());
    this.update();
  }

  bead(): void {
    const t = this.preset();
    if (!t) return;
    const n = this.count() + 1;
    this.count.set(n);
    // A tick on every bead, a longer one at each quarter of a 108 mala.
    navigator.vibrate?.(n % 27 === 0 ? 120 : 15);
    if (n >= t.count_target) void this.finish();
  }

  /** "End" or "Finish" before the natural end: log what was done. */
  async end(): Promise<void> {
    if (this.mode() === 'count' || this.paused() || !this.pos()?.done) {
      this.bells?.cancel();
    }
    await this.finish();
  }

  private async finish(): Promise<void> {
    const t = this.preset();
    if (!t || this.mode() === 'done') return;
    clearInterval(this.tick);
    const elapsed = this.stopwatch.elapsed();
    this.stopwatch.pause();
    const p = this.plan();
    const credits = t.count_target > 0 ? countCredit(this.count(), t.logs_as) : credit(p!, elapsed, t.logs_as);
    if (t.count_target > 0 && this.count() >= t.count_target) this.bells?.ringNow(t.close_strikes, t.timbre);
    this.mode.set('done');
    this.logged.set(credits);
    await this.release();
    if (credits.length === 0) return;
    if (!this.api.me()) {
      this.outcome.set('Not logged: you\'re looking around without an account. Sign in or start a guest visit to log sits.');
      return;
    }
    this.outcome.set('Saving…');
    try {
      const r = await this.sessions.finish(credits);
      this.outcome.set(r.left > 0 ? 'Saved on this phone; it goes to the board when it can reach it.' : 'Logged.');
    } catch {
      this.outcome.set('Saved on this phone; it goes to the board when it can reach it.');
    }
  }

  async retry(): Promise<void> {
    await this.attempt(async () => void (await this.sessions.flush()));
  }

  preview(): void {
    this.audio().ringNow(1, this.timbre());
  }

  private stop(): void {
    clearInterval(this.tick);
    this.bells?.cancel();
    void this.release();
    void this.ctx?.close();
    this.ctx = null;
    this.bells = null;
  }

  /** Keeps the screen on while a session runs (HTTPS only). */
  private async keepAwake(): Promise<void> {
    try {
      this.wakeLock = (await navigator.wakeLock?.request('screen')) ?? null;
      this.awake.set(this.wakeLock !== null);
      this.wakeLock?.addEventListener('release', () => this.awake.set(false));
    } catch {
      this.awake.set(false);
    }
  }

  private async release(): Promise<void> {
    await this.wakeLock?.release().catch(() => undefined);
    this.wakeLock = null;
    this.awake.set(false);
  }

  /** Back from a lock or another tab: catch up, re-lock the screen, maybe finish. */
  private async visible(): Promise<void> {
    if (document.visibilityState !== 'visible') return;
    if (this.mode() === 'run' || this.mode() === 'count') {
      await this.keepAwake();
      void this.ctx?.resume();
      if (this.mode() === 'run') this.update();
    }
  }

  // ---- the editor -----------------------------------------------------------

  setSegment(i: number, change: Partial<Segment>): void {
    this.segments.update((list) => list.map((s, j) => (j === i ? { ...s, ...change } : s)));
  }

  addSegment(): void {
    this.segments.update((list) => [...list, { kind: list.length % 2 === 1 ? 'sit' : 'walk', minutes: 5 }]);
  }

  removeSegment(i: number): void {
    this.segments.update((list) => list.filter((_, j) => j !== i));
  }

  edit(t: TimerPreset): void {
    this.editing.set(t);
    this.name.set(t.name);
    this.counting.set(t.count_target > 0);
    this.target.set(t.count_target || 108);
    this.segments.set(t.segments.length ? t.segments.map((s) => ({ ...s })) : [{ kind: 'sit', minutes: 20 }]);
    this.interval.set(t.interval_minutes);
    this.logsAs.set(t.logs_as);
    this.timbre.set(t.timbre);
  }

  resetForm(): void {
    this.editing.set(null);
    this.name.set('');
    this.counting.set(false);
    this.segments.set([{ kind: 'sit', minutes: 20 }]);
    this.interval.set(0);
    this.logsAs.set('meditation');
  }

  private input(archived = false): TimerInput {
    const counting = this.counting();
    const kinds = this.loggable();
    const logsAs = kinds.some((k) => k.key === this.logsAs()) ? this.logsAs() : (kinds[0]?.key ?? '');
    return {
      name: this.name(),
      segments: counting ? [] : this.segments(),
      open_strikes: counting ? 1 : 3,
      interval_minutes: counting ? 0 : this.interval(),
      close_strikes: counting ? 3 : this.segments().length > 1 ? 3 : 1,
      count_target: counting ? this.target() : 0,
      logs_as: logsAs,
      timbre: this.timbre(),
      household: false,
      archived,
    };
  }

  async save(event: Event): Promise<void> {
    event.preventDefault();
    await this.attempt(async () => {
      const e = this.editing();
      this.presets.set(e ? await this.content.replaceTimer(e.key, this.input()) : await this.content.addTimer(this.input()));
      this.resetForm();
    });
  }

  async archive(): Promise<void> {
    const e = this.editing();
    if (!e) return;
    await this.attempt(async () => {
      this.presets.set(await this.content.replaceTimer(e.key, this.input(true)));
      this.resetForm();
    });
  }

  private async attempt(action: () => Promise<void>): Promise<void> {
    this.busy.set(true);
    this.error.set('');
    try {
      await action();
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
