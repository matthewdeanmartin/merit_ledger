import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { Api, CalendarDay, CalendarView, Observance, problem } from '../api';
import { addMonths, firstDay, grid, lastDay, Month, monthOf, monthTitle, phaseIcon, SYSTEMS, systemName } from '../calendar';
import { dayHeading } from '../practice';

/** The meritledger lunar calendar: a month at a time (Sprint 7). */
@Component({
  selector: 'ml-calendar',
  template: `
    <div class="bar">
      <h1>{{ title() }}</h1>
      <div class="row">
        <button type="button" class="secondary small" aria-label="Previous month" (click)="go(-1)">‹</button>
        <button type="button" class="secondary small" (click)="thisMonth()">Today</button>
        <button type="button" class="secondary small" aria-label="Next month" (click)="go(1)">›</button>
      </div>
    </div>
    <p class="muted small">
      <label class="inline">Calendar
        <select [value]="system()" (change)="pick($any($event.target).value)">
          @for (s of systems; track s.key) {
            @if (s.key !== 'none') { <option [value]="s.key">{{ s.name }}</option> }
          }
        </select>
      </label>
      @if (view()?.sunday_shift) { · observances move to the closest Sunday }
    </p>
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }

    <table class="month" aria-label="Month">
      <thead><tr>@for (d of weekdays; track d) { <th scope="col">{{ d }}</th> }</tr></thead>
      <tbody>
        @for (week of weeks(); track $index) {
          <tr>
            @for (date of week; track $index) {
              @if (date) {
                <td [class.today]="date === today()" [class.feast]="feastOn(date)">
                  <div class="num">
                    <span>{{ +date.slice(8) }}</span>
                    @if (day(date); as d) {
                      <span class="lunar" [title]="lunarTitle(d)">{{ phase(d.phase) }}{{ d.lunar_month ? d.lunar_month + '/1' : d.lunar_day }}</span>
                    }
                  </div>
                  @for (o of on(date); track o.name) {
                    <div [class]="'obs ' + o.kind" [title]="o.name">{{ o.feast ? '✦ ' : '' }}{{ short(o) }}</div>
                  }
                </td>
              } @else {
                <td class="blank"></td>
              }
            }
          </tr>
        }
      </tbody>
    </table>

    <section class="card">
      <h2>This month</h2>
      <ul class="obslist">
        @for (o of view()?.observances ?? []; track o.date + o.name) {
          <li [class.feast]="o.feast">
            <span class="when">{{ heading(o.date) }}</span>
            <span>{{ o.feast ? '✦ ' : '' }}{{ o.name }}@if (o.feast) { <span class="muted"> · feast day</span> }
              @if (o.shifted_from) { <span class="muted"> (moved from {{ heading(o.shifted_from) }})</span> }</span>
          </li>
        } @empty {
          <li class="muted">No observances this month.</li>
        }
      </ul>
    </section>

    <section class="card">
      <h2>In your phone's calendar</h2>
      <p class="muted small">Subscribe to the {{ name(system()) }} calendar. It updates itself; phones on the home
        Wi-Fi can reach it (cloud calendars like Google's can't).</p>
      <p><a class="button secondary" [href]="webcal()">Subscribe</a>
        <span class="muted small"> or copy <code>{{ feed() }}</code></span></p>
    </section>
  `,
})
export class CalendarPage implements OnInit {
  private readonly api = inject(Api);
  readonly systems = SYSTEMS;
  readonly weekdays = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  readonly month = signal<Month>(monthOf(new Date().toISOString().slice(0, 10)));
  readonly system = signal('chinese');
  readonly view = signal<CalendarView | null>(null);
  readonly error = signal('');
  readonly today = signal('');
  readonly title = computed(() => monthTitle(this.month()));
  readonly weeks = computed(() => grid(this.month()));
  private readonly days = computed(() => new Map((this.view()?.days ?? []).map((d) => [d.date, d])));
  private readonly byDate = computed(() => {
    const m = new Map<string, Observance[]>();
    for (const o of this.view()?.observances ?? []) m.set(o.date, [...(m.get(o.date) ?? []), o]);
    return m;
  });
  readonly heading = dayHeading;
  readonly phase = phaseIcon;
  readonly name = systemName;

  async ngOnInit(): Promise<void> {
    const me = this.api.me();
    // The person's own calendar, else the household's (the board decides).
    const first = await this.api.calendar(firstDay(this.month()), lastDay(this.month())).catch(() => null);
    if (first) this.system.set(me?.calendar || first.system);
    this.today.set(new Date().toLocaleDateString('en-CA'));
    await this.load();
  }

  go(n: number): void {
    this.month.set(addMonths(this.month(), n));
    void this.load();
  }

  thisMonth(): void {
    this.month.set(monthOf(this.today() || new Date().toISOString().slice(0, 10)));
    void this.load();
  }

  pick(system: string): void {
    this.system.set(system);
    void this.load();
  }

  private async load(): Promise<void> {
    this.error.set('');
    try {
      this.view.set(await this.api.calendar(firstDay(this.month()), lastDay(this.month()), this.system()));
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  day(date: string): CalendarDay | undefined {
    return this.days().get(date);
  }

  on(date: string): Observance[] {
    return this.byDate().get(date) ?? [];
  }

  feastOn(date: string): boolean {
    return this.on(date).some((o) => o.feast);
  }

  lunarTitle(d: CalendarDay): string {
    const month = d.lunar_month ? `Lunar month ${d.lunar_month} begins. ` : '';
    return `${month}Day ${d.lunar_day} of the lunar month`;
  }

  /** Short labels for small cells. */
  short(o: Observance): string {
    if (o.kind === 'fasting') return 'Fast';
    if (o.kind === 'uposatha') return 'Uposatha';
    return o.name.split(' (')[0].split(':')[0];
  }

  feed(): string {
    return `${location.origin}/cal/${this.system()}.ics`;
  }

  webcal(): string {
    return this.feed().replace(/^https?:/, 'webcal:');
  }
}
