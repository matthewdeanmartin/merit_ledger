import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { VOW_STYLES } from '../vow-styles';
import { ContentApi } from '../content-api';
import { Api, PreceptSet, PreceptsView, ProfileView, VowsPage, problem } from '../api';
import { EntryItem } from '../entry-item';
import { Cell, cells, columns } from '../heatmap';
import { amountLabel, dayHeading, visibilityLabel, VISIBILITIES } from '../practice';
import { fivePart, takenItems, takenSummary } from '../precepts';
import { progress, ReadingView } from '../books';
import { isCurrent, stateLabel, vowDates } from '../vows';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

const CELL = 11;
const GAP = 2;

/** Someone's public profile: streaks, this week, a 26-week heatmap, recent practice. */
@Component({
  selector: 'ml-person',
  styles: VOW_STYLES,
  imports: [EntryItem, RouterLink],
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (profile(); as p) {
      <section class="card">
        <h1>{{ p.display_name }}</h1>
        <div class="stats">
          <div><strong>{{ p.current_streak }}</strong><span>day streak</span></div>
          <div><strong>{{ p.longest_streak }}</strong><span>longest</span></div>
          <div><strong>{{ minutes(p.week_minutes) }}</strong><span>this week</span></div>
        </div>
        @if (p.own && p.demerits_30 > 0) {
          <p class="muted">Only you (and admins) see this: {{ p.demerits_30 }} demerit{{ p.demerits_30 === 1 ? '' : 's' }} in the last 30 days.</p>
        }
      </section>

      @if (vows(); as page) {
        @if (page.precepts.length || page.own) {
          <section class="card">
            <h2>Precepts</h2>
            @for (r of page.precepts; track r.set) {
              <div class="precept-record">
                <p><a [routerLink]="['/precepts', r.set]"><strong>{{ r.title }}</strong></a>
                  @if (r.generic) { <span class="muted"> · has taken these</span> }
                  @else { <span class="muted"> · {{ takenSummary(r) }}{{ r.set === 'five' ? ', ' + fivePart(r.majors.length) : '' }}</span> }
                  @if (r.visibility !== 'public' && page.own) { <span class="muted small"> · {{ r.visibility === 'private' ? '🔒' : '◐' }}</span> }
                </p>
                @if (r.date || r.note) { <p class="muted small">{{ r.date ? 'Taken ' + r.date : '' }}{{ r.date && r.note ? ' · ' : '' }}{{ r.note }}</p> }
                @if (!r.generic && r.majors.length + r.minors.length < r.total_majors + r.total_minors) {
                  @if (setOf(r); as set) {
                    <ul class="taken">
                      @for (p of items(set, r); track p.kind + p.n) { <li>{{ p.title }}</li> }
                    </ul>
                  }
                }
              </div>
            } @empty {
              <p class="muted">No precepts recorded.</p>
            }
            @if (page.own) { <p><a routerLink="/precepts" class="button secondary small">Record precepts</a></p> }
          </section>
        }

        @if (page.vows.length || page.own) {
          <section class="card">
            <h2>Vows</h2>
            <ul class="vows">
              @for (vow of page.vows; track vow.id) {
                <li [class.past]="!isCurrent(vow)">
                  <a [routerLink]="['/v', vow.id]">{{ vow.generic ? 'A vow' : vow.text }}</a>
                  <span class="muted small">{{ stateLabel(vow.state) }} · {{ vowDates(vow.started, vow.ends) }}{{ vow.posts ? ' · ' + vow.posts + (vow.posts === 1 ? ' post' : ' posts') : '' }}</span>
                </li>
              } @empty {
                <li class="muted">No vows yet.</li>
              }
            </ul>
            @if (page.own) {
              @if (!composing()) {
                <button type="button" (click)="composing.set(true)">Take a vow</button>
              } @else {
                <form (submit)="takeVow($event)" class="card inset">
                  <label>I vow… <textarea maxlength="280" rows="2" required [value]="vowText()" (input)="vowText.set(v($event))"></textarea></label>
                  <label>Until (optional) <input type="date" [value]="vowEnds()" (input)="vowEnds.set(v($event))" /></label>
                  <label>Who sees it
                    <select [value]="vowVisibility()" (change)="vowVisibility.set(v($event))">
                      @for (o of visibilities; track o) { <option [value]="o">{{ visibilityLabel(o) }}</option> }
                    </select>
                  </label>
                  @if (vowError()) { <p class="error" role="alert">{{ vowError() }}</p> }
                  <div class="row">
                    <button type="submit" [disabled]="busy() || !vowText().trim()">Take this vow</button>
                    <button type="button" class="secondary" (click)="composing.set(false)">Cancel</button>
                  </div>
                  <p class="muted small">To make a precept you've taken into a vow, use its page under Precepts.</p>
                </form>
              }
            }
          </section>
        }

        @if (reading().length) {
          <section class="card">
            <h2>Reading</h2>
            <ul class="vows">
              @for (b of reading(); track b.id) {
                <li>
                  <span><strong>{{ b.title }}</strong>{{ b.author ? ' · ' + b.author : '' }}</span>
                  <span class="muted small">{{ b.status === 'done' ? 'Finished ' + b.finished : b.status === 'want' ? 'Wants to read' : progress(b.page, b.pages) }}{{ b.minutes ? ' · ' + b.minutes + ' min' : '' }}</span>
                </li>
              }
            </ul>
            @if (page.own) { <p><a routerLink="/books" class="button secondary small">My books</a></p> }
          </section>
        }

        @if (page.posts.length) {
          <section class="card">
            <h2>Posts</h2>
            @for (p of page.posts; track p.id) {
              <article class="post">
                <h3><a [routerLink]="['/v', p.vow]">{{ p.title }}</a></h3>
                <p class="muted small">{{ day(p.date) }} · on “{{ p.vow_text }}”</p>
                <div class="post-body" [innerHTML]="p.html"></div>
              </article>
            }
          </section>
        }
      }

      <section class="card">
        <h2>Last 26 weeks</h2>
        <svg class="heatmap" role="img" [attr.width]="width()" [attr.height]="height"
          [attr.viewBox]="'0 0 ' + width() + ' ' + height"
          [attr.aria-label]="p.display_name + ' practised on ' + practisedDays() + ' of the last 182 days'">
          @for (c of grid(); track c.date) {
            <rect [attr.x]="c.col * step" [attr.y]="c.row * step" [attr.width]="cell" [attr.height]="cell" rx="2"
              [attr.class]="c.unknown ? 'unknown' : 'l' + c.level"><title>{{ day(c.date) }}{{ c.unknown ? ': before the board remembers' : '' }}</title></rect>
          }
        </svg>
        @if (p.history_from > p.heatmap_start) {
          <p class="muted small">The board remembers back to {{ day(p.history_from) }}; outlined days are before that.</p>
        }
      </section>

      @if (p.week.length > 0) {
        <section class="card">
          <h2>This week</h2>
          <ul class="bars">
            @for (c of p.week; track c.category) {
              <li>
                <span class="label">{{ c.category }}</span>
                <span class="bar-track"><span class="bar-fill" [style.width.%]="share(c.minutes, c.count)"></span></span>
                <span class="muted small">{{ c.minutes ? minutes(c.minutes) : c.count + '×' }}</span>
              </li>
            }
          </ul>
        </section>
      }

      <section class="card">
        <h2>Recently</h2>
        <ul class="entries">
          @for (e of p.recent; track e.id) {
            <li><ml-entry [entry]="e" [showName]="false" /></li>
          } @empty {
            <li class="muted">Nothing yet.</li>
          }
        </ul>
      </section>
    } @else if (!error()) {
      <p class="muted">Asking the board…</p>
    }
  `,
})
export class Person {
  private readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly router = inject(Router);
  readonly vows = signal<VowsPage | null>(null);
  readonly sets = signal<PreceptSet[]>([]);
  readonly composing = signal(false);
  readonly vowText = signal('');
  readonly vowEnds = signal('');
  readonly vowVisibility = signal('public');
  readonly vowError = signal('');
  readonly busy = signal(false);
  readonly visibilities = VISIBILITIES;
  readonly v = value;
  readonly visibilityLabel = visibilityLabel;
  readonly takenSummary = takenSummary;
  readonly fivePart = fivePart;
  readonly stateLabel = stateLabel;
  readonly vowDates = vowDates;
  readonly isCurrent = isCurrent;
  readonly reading = signal<ReadingView[]>([]);
  readonly progress = progress;
  /** Route parameter `/p/:id`. */
  readonly id = input.required<string>();
  readonly profile = signal<ProfileView | null>(null);
  readonly error = signal('');
  readonly cell = CELL;
  readonly step = CELL + GAP;
  readonly height = 7 * (CELL + GAP) - GAP;
  readonly grid = computed<Cell[]>(() => {
    const p = this.profile();
    return p ? cells(p.heatmap_start, p.heatmap, p.history_from) : [];
  });
  readonly width = computed(() => Math.max(columns(this.grid()) * this.step - GAP, 0));
  readonly practisedDays = computed(() => this.grid().filter((c) => c.level > 0).length);

  constructor() {
    effect(() => void this.load(Number(this.id())));
  }

  private async load(id: number): Promise<void> {
    this.error.set('');
    try {
      const [profile, vows] = await Promise.all([this.content.profile(id), this.content.vowsPage(id)]);
      this.profile.set(profile);
      this.vows.set(vows);
      if (this.api.me()) {
        this.api
          .get<{ books: ReadingView[] }>(`/api/v1/reading?person=${id}`, 'ReadingList')
          .then((r) => this.reading.set(r.books))
          .catch(() => undefined);
      }
      if (vows.precepts.length) this.sets.set(await this.content.precepts());
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  setOf(r: PreceptsView): PreceptSet | undefined {
    return this.sets().find((s) => s.key === r.set);
  }

  items(set: PreceptSet, r: PreceptsView) {
    return takenItems(set, r);
  }

  async takeVow(event: Event): Promise<void> {
    event.preventDefault();
    this.busy.set(true);
    this.vowError.set('');
    try {
      const d = await this.content.addVow({ text: this.vowText(), ends: this.vowEnds(), visibility: this.vowVisibility() });
      await this.router.navigate(['/v', d.vow.id], { queryParams: { compose: 'took' } });
    } catch (e) {
      this.vowError.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }

  minutes(m: number): string {
    return amountLabel('minutes', m);
  }

  day(date: string): string {
    return dayHeading(date);
  }

  /** Bar length against the week's biggest category. */
  share(minutes: number, count: number): number {
    const week = this.profile()?.week ?? [];
    const byMinutes = week.some((c) => c.minutes > 0);
    const top = Math.max(1, ...week.map((c) => (byMinutes ? c.minutes : c.count)));
    return Math.round((100 * (byMinutes ? minutes : count)) / top);
  }
}
