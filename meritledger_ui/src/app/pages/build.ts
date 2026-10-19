import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { ContentApi } from '../content-api';
import { Api, KindView, PracticeStep, PracticeView, problem } from '../api';
import { newStep, STEP_KINDS, stepAmount, stepHeading, stepKind, TEMPLATES, Template } from '../practice-doc';
import { move } from '../practice';
import { LibraryEntry, TextLibrary } from '../text-library';
import type { TimerPreset } from '../timer';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** `/build`: practices to follow, and templates. `/build/:id` (or `new`): the builder. */
@Component({
  selector: 'ml-build',
  imports: [RouterLink],
  styles: `
    .steps { list-style: none; padding: 0; margin: 0 0 1rem; counter-reset: step; }
    .steps > li { border: 1px solid var(--line); border-radius: 0.75rem; padding: 0.75rem; margin-bottom: 0.75rem; }
    .steps .top { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: center; justify-content: space-between; }
    .steps .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr)); gap: 0 0.75rem; }
    .add { display: flex; flex-wrap: wrap; gap: 0.5rem; }
    .doclist { list-style: none; padding: 0; margin: 0; }
    .doclist li { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: center; justify-content: space-between; padding: 0.6rem 0; border-bottom: 1px solid var(--line); }
    textarea { width: 100%; font: inherit; padding: 0.5rem; border: 1px solid var(--line); border-radius: 0.5rem; background: var(--bg); color: var(--text); }
  `,
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (!id()) {
      <div class="bar">
        <h1>Practices to follow</h1>
        @if (api.me()) { <a routerLink="/build/new" class="button small">Build one</a> }
      </div>
      <section class="card">
        <ul class="doclist">
          @for (p of list(); track p.id) {
            <li>
              <span><a [routerLink]="['/practice', p.id, 1]"><strong>{{ p.title }}</strong></a>
                <span class="muted small"> · {{ p.elements.length }} steps · {{ p.owner_name }}{{ p.visibility === 'private' ? ' · 🔒' : '' }}</span></span>
              <span class="row">
                <a class="button secondary small" [routerLink]="['/practice', p.id, 'print']">Print</a>
                @if (p.can_edit) { <a class="button secondary small" [routerLink]="['/build', p.id]">Edit</a> }
              </span>
            </li>
          } @empty {
            <li class="muted">{{ loaded() ? 'No practices yet.' : 'Asking the board…' }}</li>
          }
        </ul>
      </section>
      @if (api.me()) {
        <h2>Start from a template</h2>
        @for (t of templates; track t.key) {
          <section class="card">
            <h3>{{ t.title }}</h3>
            <p class="muted small">{{ t.about }}</p>
            <a class="button secondary small" [routerLink]="['/build', 'new']" [queryParams]="{ template: t.key }">Use this</a>
          </section>
        }
      }
      <p><a routerLink="/texts">The text library →</a></p>
    } @else {
      <div class="bar">
        <h1>{{ id() === 'new' ? 'Build a practice' : 'Edit practice' }}</h1>
        <a routerLink="/build" class="button secondary small">All practices</a>
      </div>
      <form (submit)="save($event)">
        <section class="card">
          <label>Name <input maxlength="100" required [value]="title()" (input)="title.set(v($event))" /></label>
          <label>Who can follow it
            <select [value]="visibility()" (change)="visibility.set(v($event))">
              <option value="public">🌐 Anyone on the home Wi-Fi</option>
              <option value="private">🔒 Only me (and admins)</option>
            </select>
          </label>
        </section>

        <ol class="steps">
          @for (s of steps(); track $index; let i = $index) {
            <li>
              <div class="top">
                <strong>{{ i + 1 }}. {{ kindOf(s.kind).icon }} {{ heading(s) }}</strong>
                <span class="row">
                  <button type="button" class="secondary small" aria-label="Move up" [disabled]="i === 0" (click)="shift(i, -1)">↑</button>
                  <button type="button" class="secondary small" aria-label="Move down" [disabled]="i === steps().length - 1" (click)="shift(i, 1)">↓</button>
                  <button type="button" class="secondary small" (click)="remove(i)">Remove</button>
                </span>
              </div>
              <div class="grid">
                <label>Heading (optional) <input maxlength="100" [value]="s.title" (input)="set(i, 'title', v($event))" /></label>
                @if (kindOf(s.kind).usesText) {
                  <label>Text
                    <select [value]="s.text" (change)="set(i, 'text', v($event))">
                      <option value="">(none)</option>
                      @for (t of texts(); track t.id) { <option [value]="t.id">{{ t.title }}{{ t.builtin ? '' : ' (household)' }}</option> }
                    </select>
                  </label>
                }
                @if (kindOf(s.kind).measures.includes('minutes')) {
                  <label>Minutes <input type="number" min="0" max="600" [value]="s.minutes || ''" (input)="setNum(i, 'minutes', v($event))" /></label>
                }
                @if (kindOf(s.kind).measures.includes('count')) {
                  <label>Count <input type="number" min="0" max="100000" [value]="s.count || ''" (input)="setNum(i, 'count', v($event))" /></label>
                }
                @if (s.kind === 'meditate' || s.kind === 'chant' || s.kind === 'prostrations') {
                  <label>Timer
                    <select [value]="s.timer" (change)="set(i, 'timer', v($event))">
                      <option value="">(none)</option>
                      @for (t of timers(); track t.key) { <option [value]="t.key">{{ t.name }}</option> }
                    </select>
                  </label>
                }
                <label>“I did this” logs
                  <select [value]="s.logs_as" (change)="set(i, 'logs_as', v($event))">
                    <option value="">(nothing)</option>
                    @for (k of kinds(); track k.key) { <option [value]="k.key">{{ k.icon }} {{ k.name }}</option> }
                  </select>
                </label>
              </div>
              @if (s.kind === 'note' || s.kind === 'art' || s.kind === 'read') {
                <label>{{ s.kind === 'art' ? 'Prompt' : s.kind === 'read' ? 'What to read (book, chapter, pages)' : 'Text' }}
                  <textarea rows="2" maxlength="1000" [value]="s.note" (input)="set(i, 'note', v($event))"></textarea></label>
              }
            </li>
          } @empty {
            <li class="muted">No steps yet. Add one below.</li>
          }
        </ol>

        <section class="card">
          <h2>Add a step</h2>
          <div class="add">
            @for (k of stepKinds; track k.key) {
              <button type="button" class="secondary small" (click)="add(k.key)">{{ k.icon }} {{ k.label }}</button>
            }
          </div>
        </section>

        @if (formError()) { <p class="error" role="alert">{{ formError() }}</p> }
        <div class="row">
          <button type="submit" [disabled]="busy() || !title().trim() || steps().length === 0">Save</button>
          @if (rev()) {
            <button type="button" class="danger" [disabled]="busy()" (click)="deleteDoc()">Delete</button>
          }
        </div>
      </form>
    }
  `,
})
export class Build {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly library = inject(TextLibrary);
  private readonly router = inject(Router);
  /** Route parameter `/build/:id` ('new' for a new one). */
  readonly id = input<string>();
  /** Query parameter: a template key to start from. */
  readonly template = input<string>();
  readonly list = signal<PracticeView[]>([]);
  readonly loaded = signal(false);
  readonly texts = signal<LibraryEntry[]>([]);
  readonly kinds = signal<KindView[]>([]);
  readonly timers = signal<TimerPreset[]>([]);
  readonly title = signal('');
  readonly visibility = signal('public');
  readonly steps = signal<PracticeStep[]>([]);
  readonly rev = signal(0);
  readonly error = signal('');
  readonly formError = signal('');
  readonly busy = signal(false);
  readonly templates: Template[] = TEMPLATES;
  readonly stepKinds = STEP_KINDS;
  readonly kindOf = stepKind;
  readonly v = value;
  private readonly titles = computed(() => Object.fromEntries(this.texts().map((t) => [t.id, t.title])));

  constructor() {
    effect(() => void this.load(this.id(), this.template()));
  }

  private async load(id: string | undefined, template: string | undefined): Promise<void> {
    this.error.set('');
    try {
      if (!id) {
        this.list.set(await this.content.practices());
        this.loaded.set(true);
        return;
      }
      const [texts, kinds, timers] = await Promise.all([this.library.entries(), this.api.kinds(), this.api.timers()]);
      this.texts.set(texts);
      this.kinds.set(kinds.filter((k) => !k.archived && k.polarity === 'merit'));
      this.timers.set(timers.filter((t) => !t.archived));
      if (id === 'new') {
        const t = TEMPLATES.find((x) => x.key === template);
        this.title.set(t?.title ?? '');
        this.steps.set(t ? t.steps.map((s) => ({ ...s })) : []);
        this.visibility.set('public');
        this.rev.set(0);
      } else {
        const doc = await this.content.practiceDoc(Number(id));
        this.title.set(doc.title);
        this.visibility.set(doc.visibility);
        this.steps.set(doc.elements.map((s) => ({ ...s })));
        this.rev.set(doc.rev);
      }
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  heading(s: PracticeStep): string {
    return stepHeading(s, this.titles()[s.text] ?? '');
  }

  add(kind: string): void {
    this.steps.update((list) => [...list, newStep(kind)]);
  }

  remove(i: number): void {
    this.steps.update((list) => list.filter((_, n) => n !== i));
  }

  shift(i: number, by: -1 | 1): void {
    this.steps.update((list) => {
      const keys = list.map((_, n) => String(n));
      return move(keys, String(i), by).map((k) => list[Number(k)]);
    });
  }

  set(i: number, field: 'title' | 'text' | 'timer' | 'logs_as' | 'note', value: string): void {
    this.steps.update((list) => list.map((s, n) => (n === i ? { ...s, [field]: value } : s)));
  }

  setNum(i: number, field: 'minutes' | 'count', value: string): void {
    const n = Math.max(0, Math.floor(Number(value) || 0));
    this.steps.update((list) => list.map((s, k) => (k === i ? { ...s, [field]: n } : s)));
  }

  amount(s: PracticeStep): string {
    return stepAmount(s);
  }

  async save(event: Event): Promise<void> {
    event.preventDefault();
    this.busy.set(true);
    this.formError.set('');
    try {
      const input = { title: this.title(), visibility: this.visibility(), elements: this.steps(), rev: this.rev() };
      const id = this.id();
      const doc = id === 'new' ? await this.content.addPractice(input) : await this.content.replacePractice(Number(id), input);
      await this.router.navigate(['/practice', doc.id, 1]);
    } catch (e) {
      this.formError.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }

  async deleteDoc(): Promise<void> {
    if (!confirm(`Delete “${this.title()}”?`)) return;
    try {
      await this.content.deletePractice(Number(this.id()));
      await this.router.navigateByUrl('/build');
    } catch (e) {
      this.formError.set(problem(e));
    }
  }
}
