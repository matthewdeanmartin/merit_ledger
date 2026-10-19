import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { VOW_STYLES } from '../vow-styles';
import { ContentApi } from '../content-api';
import { Api, PreceptSet, PreceptsView, VowView, problem } from '../api';
import { visibilityLabel, VISIBILITIES } from '../practice';
import { all, cbetaUrl, fivePart, itemLabel, majors, minors, takenSummary, toggle } from '../precepts';
import { isCurrent } from '../vows';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/**
 * `/precepts`: the sets. `/precepts/:set`: one set with its sources, and
 * (signed in) which of them you've taken, and "Make this a vow".
 */
@Component({
  selector: 'ml-precepts-page',
  styles: VOW_STYLES,
  imports: [RouterLink],
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (!set()) {
      <h1>Precepts</h1>
      <p class="muted">The precept sets of Chinese Buddhism, each traced line by line to the classical text on CBETA.
        Record the ones you've taken: all of a set, or only some.</p>
      @for (s of sets(); track s.key) {
        <a class="card tile" [routerLink]="['/precepts', s.key]">
          <h2>{{ s.title }} <span class="muted">{{ s.zh }}</span></h2>
          <p class="muted small">{{ count(s) }} · {{ s.source.title_zh }}
            @if (mine()[s.key]; as v) { · <strong>you hold {{ takenSummary(v).toLowerCase() }}</strong> }</p>
        </a>
      } @empty {
        @if (!error()) { <p class="muted">Asking the board…</p> }
      }
    } @else if (current(); as s) {
      <div class="bar">
        <h1>{{ s.title }} <span class="muted">{{ s.zh }}</span></h1>
        <a routerLink="/precepts" class="button secondary small">All sets</a>
      </div>
      <section class="card">
        <p class="small">From {{ s.source.title_zh }} ({{ s.source.title }}), {{ s.source.canon }}.
          The Chinese is from <a href="https://cbetaonline.dila.edu.tw/" target="_blank" rel="noopener">CBETA</a>
          (Taipei); each precept links to its line.</p>
        @if (s.provenance !== 'llm-draft-reviewed') {
          <p class="muted small">✎ These English renderings are drafts, awaiting the household's review.</p>
        }
      </section>

      @if (api.me()) {
        <section class="card">
          <h2>Your record</h2>
          @if (!editing()) {
            @if (mine()[s.key]; as v) {
              <p><strong>{{ takenSummary(v) }}</strong>
                @if (s.key === 'five') { · {{ fivePart(v.majors.length) }} }</p>
              <p class="muted small">{{ v.date ? 'Taken ' + v.date : '' }}{{ v.date && v.note ? ' · ' : '' }}{{ v.note }}</p>
              <p class="muted small">{{ visibilityLabel(v.visibility) }}</p>
              <button type="button" class="secondary" (click)="edit()">Change</button>
            } @else {
              <p class="muted">You haven't recorded taking these.</p>
              <button type="button" (click)="edit()">I've taken these</button>
            }
          } @else {
            <form (submit)="save($event)">
              <p class="muted small">Tick the precepts you took below{{ s.key === 'five' ? ' (taking only some is the old custom: one, two, three, four or all five)' : '' }}.</p>
              <div class="row">
                <button type="button" class="secondary small" (click)="pickAll()">All of them</button>
                <button type="button" class="secondary small" (click)="pickNone()">None</button>
              </div>
              <p><strong>{{ pickedCount() }}</strong> chosen
                @if (s.key === 'five' && pickedMajors().length) { · {{ fivePart(pickedMajors().length) }} }</p>
              <label>When (optional) <input type="date" [value]="date()" (input)="date.set(v($event))" /></label>
              <label>Where, with whom (optional) <input maxlength="200" [value]="note()" (input)="note.set(v($event))" /></label>
              <label>Who sees it
                <select [value]="visibility()" (change)="visibility.set(v($event))">
                  @for (o of visibilities; track o) { <option [value]="o">{{ visibilityLabel(o) }}</option> }
                </select>
              </label>
              @if (formError()) { <p class="error" role="alert">{{ formError() }}</p> }
              <div class="row">
                <button type="submit" [disabled]="busy() || pickedCount() === 0">Save</button>
                <button type="button" class="secondary" (click)="editing.set(false)">Cancel</button>
                @if (mine()[s.key]) {
                  <button type="button" class="danger" [disabled]="busy()" (click)="remove()">No longer holding these</button>
                }
              </div>
            </form>
          }
        </section>
      }

      @for (group of groups(); track group.title) {
        <section class="card">
          @if (group.title) { <h2>{{ group.title }}</h2> }
          <ol class="precepts">
            @for (p of group.items; track p.n) {
              <li [id]="p.kind + '-' + p.n" [class.taken]="taken(p.kind, p.n)">
                <div class="head">
                  @if (editing()) {
                    <label class="check">
                      <input type="checkbox" [checked]="picked(p.kind, p.n)" (change)="pick(p.kind, p.n)" />
                      <span><strong>{{ p.n }}. {{ p.title }}</strong></span>
                    </label>
                  } @else {
                    <strong>{{ p.n }}. {{ p.title }}</strong>
                    @if (taken(p.kind, p.n)) { <span class="muted small">✓ taken</span> }
                  }
                  <span class="muted">{{ p.label_zh }}</span>
                </div>
                <p>{{ p.text }}</p>
                <p class="zh">{{ p.quote }}</p>
                <p class="muted small">
                  @for (c of p.cite; track c) { <a [href]="cbetaUrl(c)" target="_blank" rel="noopener">{{ c }}</a> }
                  @if (p.label_cite) { · title: <a [href]="cbetaUrl(p.label_cite)" target="_blank" rel="noopener">{{ p.label_cite }}</a> }
                </p>
                @if (!editing() && taken(p.kind, p.n)) {
                  @if (vowFor(p.kind, p.n); as vow) {
                    <a class="small" [routerLink]="['/v', vow.id]">One of your vows →</a>
                  } @else {
                    <button type="button" class="secondary small" [disabled]="busy()" (click)="makeVow(p.kind, p.n)">Make this a vow</button>
                  }
                }
              </li>
            }
          </ol>
        </section>
      }
    } @else if (!error()) {
      <p class="muted">Asking the board…</p>
    }
  `,
})
export class PreceptsPage {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly router = inject(Router);
  /** Route parameter `/precepts/:set` (absent on `/precepts`). */
  readonly set = input<string>();
  readonly sets = signal<PreceptSet[]>([]);
  readonly records = signal<PreceptsView[]>([]);
  readonly vows = signal<VowView[]>([]);
  readonly error = signal('');
  readonly formError = signal('');
  readonly busy = signal(false);
  readonly editing = signal(false);
  readonly pickedMajors = signal<number[]>([]);
  readonly pickedMinors = signal<number[]>([]);
  readonly date = signal('');
  readonly note = signal('');
  readonly visibility = signal('public');
  readonly visibilities = VISIBILITIES;
  readonly v = value;
  readonly cbetaUrl = cbetaUrl;
  readonly fivePart = fivePart;
  readonly takenSummary = takenSummary;
  readonly visibilityLabel = visibilityLabel;

  readonly current = computed(() => this.sets().find((s) => s.key === this.set()) ?? null);
  readonly mine = computed<Record<string, PreceptsView>>(() =>
    Object.fromEntries(this.records().map((r) => [r.set, r])),
  );
  readonly groups = computed(() => {
    const s = this.current();
    if (!s) return [];
    const minor = minors(s);
    if (minor.length === 0) return [{ title: '', items: majors(s) }];
    return [
      { title: 'Major precepts', items: majors(s) },
      { title: 'Minor precepts', items: minor },
    ];
  });
  readonly pickedCount = computed(() => this.pickedMajors().length + this.pickedMinors().length);

  constructor() {
    effect(() => {
      this.set();
      this.editing.set(false);
      void this.load();
    });
  }

  private async load(): Promise<void> {
    this.error.set('');
    try {
      this.sets.set(await this.content.precepts());
      const me = this.api.me();
      if (me) {
        const page = await this.content.vowsPage(me.id);
        this.records.set(page.precepts);
        this.vows.set(page.vows);
      }
      if (this.set() && !this.current()) this.error.set('No such precept set.');
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  count(s: PreceptSet): string {
    const minor = minors(s).length;
    return minor ? `${majors(s).length} major, ${minor} minor` : `${s.items.length} precepts`;
  }

  taken(kind: string, n: number): boolean {
    const r = this.mine()[this.set() ?? ''];
    return !!r && (kind === 'major' ? r.majors : r.minors).includes(n);
  }

  vowFor(kind: string, n: number): VowView | undefined {
    return this.vows().find((v) => v.precept_set === this.set() && v.precept_kind === kind && v.precept_n === n && isCurrent(v));
  }

  edit(): void {
    const r = this.mine()[this.set() ?? ''];
    this.pickedMajors.set(r?.majors ?? []);
    this.pickedMinors.set(r?.minors ?? []);
    this.date.set(r?.date ?? '');
    this.note.set(r?.note ?? '');
    this.visibility.set(r?.visibility ?? 'public');
    this.formError.set('');
    this.editing.set(true);
  }

  picked(kind: string, n: number): boolean {
    return (kind === 'major' ? this.pickedMajors() : this.pickedMinors()).includes(n);
  }

  pick(kind: string, n: number): void {
    const list = kind === 'major' ? this.pickedMajors : this.pickedMinors;
    list.update((l) => toggle(l, n));
  }

  pickAll(): void {
    const s = this.current();
    if (!s) return;
    this.pickedMajors.set(all(majors(s).length));
    this.pickedMinors.set(all(minors(s).length));
  }

  pickNone(): void {
    this.pickedMajors.set([]);
    this.pickedMinors.set([]);
  }

  async save(event: Event): Promise<void> {
    event.preventDefault();
    await this.run(async () => {
      const page = await this.content.setPrecepts(this.set()!, {
        date: this.date(),
        note: this.note(),
        majors: this.pickedMajors(),
        minors: this.pickedMinors(),
        visibility: this.visibility(),
      });
      this.records.set(page.precepts);
      this.editing.set(false);
    });
  }

  async remove(): Promise<void> {
    if (!confirm('Remove this set from your profile? Vows you made from it stay.')) return;
    await this.run(async () => {
      this.records.set((await this.content.removePrecepts(this.set()!)).precepts);
      this.editing.set(false);
    });
  }

  async makeVow(kind: 'major' | 'minor', n: number): Promise<void> {
    const s = this.current();
    if (!s) return;
    await this.run(async () => {
      const detail = await this.content.addVow({ text: '', precept_set: s.key, precept_kind: kind, precept_n: n });
      await this.router.navigate(['/v', detail.vow.id], { queryParams: { compose: 'took' } });
    }, `${itemLabel(s, { kind, n })}: `);
  }

  private async run(action: () => Promise<void>, prefix = ''): Promise<void> {
    this.busy.set(true);
    this.formError.set('');
    try {
      await action();
    } catch (e) {
      const message = prefix + problem(e);
      if (this.editing()) this.formError.set(message);
      else this.error.set(message);
    } finally {
      this.busy.set(false);
    }
  }
}
