import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { ContentApi } from '../content-api';
import { Api, problem } from '../api';
import { LibraryEntry, LoadedText, TextLibrary } from '../text-library';
import { toHtml } from '../texts';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

const TRADITIONS: Record<string, string> = {
  nichiren: 'Nichiren',
  mahayana: 'Mahāyāna',
  theravada: 'Theravāda',
  zen: 'Zen',
  '': 'Other',
};

/** `/texts`: the library. `/texts/:id`: one text, with where it comes from. */
@Component({
  selector: 'ml-texts-page',
  imports: [RouterLink],
  styles: `
    .lib { list-style: none; padding: 0; margin: 0; }
    .lib li { padding: 0.5rem 0; border-bottom: 1px solid var(--line); }
    .reading { font-size: 1.15rem; line-height: 1.6; }
    .about { font-size: 0.9rem; }
    textarea { width: 100%; font: inherit; padding: 0.5rem; border: 1px solid var(--line); border-radius: 0.5rem; background: var(--bg); color: var(--text); }
  `,
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (!id()) {
      <div class="bar">
        <h1>Texts</h1>
        <a routerLink="/build" class="button secondary small">Practices</a>
      </div>
      @for (g of groups(); track g.name) {
        <section class="card">
          <h2>{{ g.name }}</h2>
          <ul class="lib">
            @for (t of g.texts; track t.id) {
              <li><a [routerLink]="['/texts', t.id]">{{ t.title }}</a>{{ t.builtin ? '' : ' · household' }}</li>
            }
          </ul>
        </section>
      }
      @if (canAdd()) {
        <section class="card">
          <h2>Add a household text</h2>
          <form (submit)="save($event)">
            <label>Title <input maxlength="100" required [value]="title()" (input)="title.set(v($event))" /></label>
            <label>Text (markdown: blank lines for paragraphs, # for headings)
              <textarea rows="10" [value]="body()" (input)="body.set(v($event))"></textarea></label>
            <label>Tradition
              <select [value]="tradition()" (change)="tradition.set(v($event))">
                @for (t of traditionKeys; track t) { <option [value]="t">{{ traditions[t] }}</option> }
              </select>
            </label>
            <label>Where it comes from, and its license <input maxlength="100" [value]="license()" (input)="license.set(v($event))" /></label>
            @if (formError()) { <p class="error" role="alert">{{ formError() }}</p> }
            <button type="submit" [disabled]="busy() || !title().trim() || !body().trim()">Add</button>
          </form>
        </section>
      }
    } @else if (text(); as t) {
      <div class="bar">
        <h1>{{ t.meta.title }}</h1>
        <a routerLink="/texts" class="button secondary small">All texts</a>
      </div>
      <section class="card about muted">
        <p>{{ t.meta.source }}</p>
        <p>{{ t.meta.license }}</p>
        @if (t.meta.provenance === 'llm-draft') { <p>✎ A draft rendering, awaiting the household's review.</p> }
        @if (t.meta.note) { <p>{{ t.meta.note }}</p> }
      </section>
      @if (!editing()) {
        <article class="card reading" [innerHTML]="html()"></article>
        @if (canEdit()) {
          <div class="row">
            <button type="button" class="secondary" (click)="startEdit()">Edit</button>
            <button type="button" class="danger" (click)="remove()">Delete</button>
          </div>
        }
      } @else {
        <form class="card" (submit)="save($event)">
          <label>Title <input maxlength="100" required [value]="title()" (input)="title.set(v($event))" /></label>
          <label>Text <textarea rows="16" [value]="body()" (input)="body.set(v($event))"></textarea></label>
          <label>Where it comes from, and its license <input maxlength="100" [value]="license()" (input)="license.set(v($event))" /></label>
          @if (formError()) { <p class="error" role="alert">{{ formError() }}</p> }
          <div class="row">
            <button type="submit" [disabled]="busy()">Save</button>
            <button type="button" class="secondary" (click)="editing.set(false)">Cancel</button>
          </div>
        </form>
      }
    } @else if (!error()) {
      <p class="muted">Loading…</p>
    }
  `,
})
export class TextsPage {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly library = inject(TextLibrary);
  private readonly router = inject(Router);
  /** Route parameter `/texts/:id`. */
  readonly id = input<string>();
  readonly entries = signal<LibraryEntry[]>([]);
  readonly text = signal<LoadedText | null>(null);
  readonly editRev = signal(0);
  readonly canEdit = signal(false);
  readonly editing = signal(false);
  readonly title = signal('');
  readonly body = signal('');
  readonly tradition = signal('');
  readonly license = signal('');
  readonly error = signal('');
  readonly formError = signal('');
  readonly busy = signal(false);
  readonly traditions = TRADITIONS;
  readonly traditionKeys = Object.keys(TRADITIONS);
  readonly v = value;
  readonly canAdd = computed(() => {
    const me = this.api.me();
    return !!me && me.role !== 'guest';
  });
  readonly groups = computed(() => {
    const out: { name: string; texts: LibraryEntry[] }[] = [];
    for (const key of this.traditionKeys) {
      const texts = this.entries().filter((t) => (TRADITIONS[t.tradition] ? t.tradition : '') === key);
      if (texts.length) out.push({ name: TRADITIONS[key], texts });
    }
    return out;
  });
  readonly html = computed(() => toHtml(this.text()?.body ?? ''));

  constructor() {
    effect(() => void this.load(this.id()));
  }

  private async load(id: string | undefined): Promise<void> {
    this.error.set('');
    this.editing.set(false);
    try {
      if (!id) {
        this.entries.set(await this.library.entries());
        return;
      }
      this.library.forget(id);
      this.text.set(await this.library.text(id));
      if (/^h\d+$/.test(id)) {
        const t = await this.content.householdText(id);
        this.editRev.set(t.rev);
        this.canEdit.set(t.can_edit);
        this.tradition.set(t.tradition);
      } else {
        this.canEdit.set(false);
      }
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  startEdit(): void {
    const t = this.text()!;
    this.title.set(t.meta.title);
    this.body.set(t.body);
    this.license.set(t.meta.license);
    this.formError.set('');
    this.editing.set(true);
  }

  async save(event: Event): Promise<void> {
    event.preventDefault();
    this.busy.set(true);
    this.formError.set('');
    try {
      const input = { title: this.title(), body: this.body(), tradition: this.tradition(), license: this.license(), rev: this.editRev() };
      const id = this.id();
      if (id) {
        await this.content.updateText(id, input);
        await this.load(id);
      } else {
        const list = await this.content.addText(input);
        const added = list.find((t) => t.title === this.title().trim());
        this.title.set('');
        this.body.set('');
        this.license.set('');
        if (added) await this.router.navigate(['/texts', added.id]);
        else this.entries.set(await this.library.entries());
      }
    } catch (e) {
      this.formError.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }

  async remove(): Promise<void> {
    const id = this.id()!;
    if (!confirm('Delete this text? Practices that use it will show it as missing.')) return;
    try {
      await this.content.deleteText(id);
      this.library.forget(id);
      await this.router.navigateByUrl('/texts');
    } catch (e) {
      this.error.set(problem(e));
    }
  }
}
