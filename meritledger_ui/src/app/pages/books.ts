import { BookCover } from '../book-cover';
import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, problem } from '../api';
import { BookHit, coverUrl, parseSearch, progress, ReadingView, searchUrl, workUrl } from '../books';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

type ListResponse = { books: ReadingView[] };

/** `/books`: your reading list, logging reading, and Open Library search. */
@Component({
  selector: 'ml-books',
  imports: [RouterLink, BookCover],
  styles: `
    .books { list-style: none; padding: 0; margin: 0; }
    .books > li { display: flex; gap: 0.75rem; padding: 0.75rem 0; border-bottom: 1px solid var(--line); }
    .cover { width: 3.5rem; min-width: 3.5rem; height: 5rem; object-fit: cover; border-radius: 0.25rem; background: var(--line); }
    .info { flex: 1; min-width: 0; }
    .info p { margin: 0.15rem 0; }
    .meter { height: 0.4rem; background: var(--line); border-radius: 1rem; overflow: hidden; margin: 0.3rem 0; }
    .meter span { display: block; height: 100%; background: var(--ok); }
    .log { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: end; margin-top: 0.4rem; }
    .log label { flex: 1; min-width: 6rem; margin: 0; }
  `,
  template: `
    <div class="bar">
      <h1>Books</h1>
    </div>
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }

    <section class="card">
      <form class="row" (submit)="search($event)">
        <label class="grow">Find a book (title, author or ISBN)
          <input type="search" [value]="q()" (input)="q.set(v($event))" placeholder="Lotus Sutra Watson" /></label>
        <button type="submit" [disabled]="searching() || !q().trim()">{{ searching() ? 'Searching…' : 'Search' }}</button>
      </form>
      @if (hits().length) {
        <ul class="books">
          @for (h of hits(); track h.work) {
            <li>
              <ml-book-cover [id]="h.cover" />
              <div class="info">
                <p><strong>{{ h.title }}</strong></p>
                <p class="muted small">{{ h.author }}{{ h.year ? ' · ' + h.year : '' }}{{ h.pages ? ' · ' + h.pages + ' pages' : '' }}</p>
                @if (onList(h.work)) {
                  <p class="ok small">On your list</p>
                } @else if (api.me()) {
                  <div class="row">
                    <button type="button" class="secondary small" [disabled]="busy()" (click)="add(h, 'reading')">I'm reading it</button>
                    <button type="button" class="secondary small" [disabled]="busy()" (click)="add(h, 'want')">Want to read</button>
                  </div>
                }
              </div>
            </li>
          }
        </ul>
        <p class="muted small">From <a href="https://openlibrary.org" target="_blank" rel="noopener">Open Library</a>.</p>
      } @else if (searched() && !searching()) {
        <p class="muted">Nothing found.</p>
      }
    </section>

    @for (group of groups(); track group.title) {
      <section class="card">
        <h2>{{ group.title }}</h2>
        <ul class="books">
          @for (b of group.books; track b.id) {
            <li>
              <ml-book-cover [id]="b.cover" />
              <div class="info">
                <p><strong>{{ b.title }}</strong>
                  @if (b.work) { <a class="small" [href]="work(b.work)" target="_blank" rel="noopener">↗</a> }</p>
                <p class="muted small">{{ b.author }}{{ b.minutes ? ' · ' + b.minutes + ' min read' : '' }}{{ b.visibility === 'private' ? ' · 🔒' : '' }}</p>
                @if (b.status === 'reading') {
                  <p class="small">{{ progress(b.page, b.pages) }}</p>
                  @if (b.pages) { <div class="meter"><span [style.width.%]="percent(b)"></span></div> }
                  @if (logging() === b.id) {
                    <form class="log" (submit)="log($event, b)">
                      <label>Minutes <input type="number" min="1" max="1440" required [value]="minutes()" (input)="minutes.set(+v($event))" /></label>
                      <label>Got to page <input type="number" min="0" [max]="b.pages || 100000" [value]="page() || ''" (input)="page.set(+v($event))" /></label>
                      <button type="submit" [disabled]="busy()">Log</button>
                      <button type="button" class="link" (click)="logging.set(0)">Cancel</button>
                    </form>
                  } @else {
                    <div class="row">
                      <button type="button" class="small" (click)="startLog(b)">Log reading</button>
                      <button type="button" class="secondary small" (click)="setStatus(b, 'done')">Finished</button>
                    </div>
                  }
                } @else if (b.status === 'want') {
                  <button type="button" class="secondary small" (click)="setStatus(b, 'reading')">Start reading</button>
                } @else {
                  <p class="muted small">Finished {{ b.finished }}</p>
                }
                <div class="row">
                  <button type="button" class="link small" (click)="togglePrivate(b)">{{ b.visibility === 'private' ? 'Show on my profile' : 'Keep private' }}</button>
                  <button type="button" class="link small" (click)="remove(b)">Remove</button>
                </div>
              </div>
            </li>
          }
        </ul>
      </section>
    }
    @if (!api.me()) {
      <p class="muted card">Search freely. To keep a reading list and log reading, <a routerLink="/signin">sign in or start a guest visit</a>.</p>
    }
    @if (loaded() && !list().length) {
      <p class="muted card">Your list is empty. Find a book above. Reading you log counts as practice.</p>
    }
    <p><a routerLink="/house">What the house is reading →</a></p>
  `,
})
export class Books implements OnInit {
  readonly api = inject(Api);
  readonly list = signal<ReadingView[]>([]);
  readonly hits = signal<BookHit[]>([]);
  readonly q = signal('');
  readonly searching = signal(false);
  readonly searched = signal(false);
  readonly loaded = signal(false);
  readonly busy = signal(false);
  readonly error = signal('');
  readonly logging = signal(0);
  readonly minutes = signal(20);
  readonly page = signal(0);
  readonly v = value;
  readonly progress = progress;
  readonly cover = coverUrl;
  readonly work = workUrl;
  readonly groups = computed(() =>
    [
      { title: 'Reading', books: this.list().filter((b) => b.status === 'reading') },
      { title: 'Want to read', books: this.list().filter((b) => b.status === 'want') },
      { title: 'Finished', books: this.list().filter((b) => b.status === 'done') },
    ].filter((g) => g.books.length),
  );

  async ngOnInit(): Promise<void> {
    if (!this.api.me()) return; // visitors search; lists need an account
    await this.run(async () => {
      this.list.set((await this.api.get<ListResponse>('/api/v1/reading', 'ReadingList')).books);
      this.loaded.set(true);
    });
  }

  onList(work: string): boolean {
    return this.list().some((b) => b.work === work);
  }

  percent(b: ReadingView): number {
    return b.pages ? Math.min(100, Math.round((100 * b.page) / b.pages)) : 0;
  }

  async search(event: Event): Promise<void> {
    event.preventDefault();
    this.searching.set(true);
    this.error.set('');
    try {
      const res = await fetch(searchUrl(this.q()));
      if (!res.ok) throw new Error(`Open Library said HTTP ${res.status}`);
      this.hits.set(parseSearch(await res.json()));
      this.searched.set(true);
    } catch (e) {
      this.error.set(e instanceof TypeError ? 'Couldn’t reach Open Library. Is the internet up?' : String(e instanceof Error ? e.message : e));
    } finally {
      this.searching.set(false);
    }
  }

  async add(h: BookHit, status: 'want' | 'reading'): Promise<void> {
    await this.run(async () => {
      const body = { work: h.work, title: h.title, author: h.author, cover: h.cover, pages: h.pages, year: h.year, status, visibility: '' };
      this.list.set((await this.api.send<ListResponse>('POST', '/api/v1/reading', body, 'ReadingList')).books);
    });
  }

  startLog(b: ReadingView): void {
    this.minutes.set(20);
    this.page.set(b.page);
    this.logging.set(b.id);
  }

  async log(event: Event, b: ReadingView): Promise<void> {
    event.preventDefault();
    await this.run(async () => {
      const body = { minutes: this.minutes(), page: this.page() > b.page ? this.page() : 0, note: '' };
      this.list.set((await this.api.send<ListResponse>('POST', `/api/v1/reading/${b.id}/log`, body, 'ReadingList')).books);
      this.logging.set(0);
    });
  }

  async setStatus(b: ReadingView, status: string): Promise<void> {
    await this.patch(b, { status });
  }

  async togglePrivate(b: ReadingView): Promise<void> {
    await this.patch(b, { visibility: b.visibility === 'private' ? 'public' : 'private' });
  }

  private async patch(b: ReadingView, change: Record<string, unknown>): Promise<void> {
    await this.run(async () => {
      this.list.set((await this.api.send<ListResponse>('PATCH', `/api/v1/reading/${b.id}`, { rev: b.rev, ...change }, 'ReadingList')).books);
    });
  }

  async remove(b: ReadingView): Promise<void> {
    if (!confirm(`Take “${b.title}” off your list? The reading you logged stays.`)) return;
    await this.run(async () => {
      this.list.set((await this.api.send<ListResponse>('DELETE', `/api/v1/reading/${b.id}`, undefined, 'ReadingList')).books);
    });
  }

  private async run(action: () => Promise<void>): Promise<void> {
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
