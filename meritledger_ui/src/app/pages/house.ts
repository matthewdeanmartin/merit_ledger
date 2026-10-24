import { Component, inject, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, CallView, EntryView, Person, problem } from '../api';
import { ContentApi } from '../content-api';
import { coverUrl, progress, ReadingView } from '../books';
import { EntryItem } from '../entry-item';
import { initials } from '../people';

/** The household: everyone's profile, and what the house has been practising. */
@Component({
  selector: 'ml-house',
  imports: [RouterLink, EntryItem],
  styles: `
    .shelf { list-style: none; padding: 0; margin: 0; display: grid; gap: 0.5rem; }
    .shelf li { display: flex; gap: 0.6rem; align-items: center; }
    .shelf img { width: 2.5rem; height: 3.6rem; object-fit: cover; border-radius: 0.2rem; }
  `,
  template: `
    <h1>The house</h1>
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    <ul class="tiles people-row">
      @for (p of people(); track p.id) {
        <li>
          <a class="tile" [routerLink]="['/p', p.id]">
            <span class="avatar" aria-hidden="true">{{ initials(p.display_name) }}</span>
            <span>{{ p.display_name }}</span>
          </a>
        </li>
      }
    </ul>
    @if (api.me(); as me) {
      <section class="card">
        <h2>Call to practice</h2>
        @for (note of deliveryNotes(); track note) { <p class="muted small" role="status">{{ note }}</p> }
        @for (c of calls(); track c.id) {
          <p>📣 <strong>{{ c.by_name }}</strong>{{ c.message ? ': ' + c.message : ' calls everyone to practice' }}{{ c.when ? ' (' + c.when + ')' : '' }}
            <span class="muted small"> · {{ ago(c.created) }}</span></p>
        }
        @if (me.role !== 'guest') {
          @if (!calling()) {
            <button type="button" (click)="calling.set(true)">📣 Call everyone</button>
          } @else {
            <form (submit)="call($event)">
              <label>Message (optional) <input maxlength="200" [value]="message()" (input)="message.set(v($event))" placeholder="Dinner's ready, then chanting" /></label>
              <label>When (optional) <input maxlength="40" [value]="when()" (input)="when.set(v($event))" placeholder="now, at 7…" /></label>
              <p class="muted small">Everyone in the house gets it in their bell and, if they've added their Mastodon handle, as a DM. It shows on the board's screen too.</p>
              <div class="row">
                <button type="submit" [disabled]="busy()">Call</button>
                <button type="button" class="secondary" (click)="calling.set(false)">Cancel</button>
              </div>
            </form>
          }
        }
      </section>
    }
    @if (shelf().length) {
      <section class="card">
        <h2>The house is reading</h2>
        <ul class="shelf">
          @for (b of shelf(); track b.id) {
            <li>
              @if (b.cover) { <img [src]="cover(b.cover)" alt="" loading="lazy" /> }
              <span><strong>{{ b.title }}</strong><br /><span class="muted small">{{ b.person_name }} · {{ progress(b.page, b.pages) }}</span></span>
            </li>
          }
        </ul>
      </section>
    }
    <section class="card">
      <h2>Recently</h2>
      <ul class="entries">
        @for (e of entries(); track e.id) {
          <li><ml-entry [entry]="e" /></li>
        } @empty {
          <li class="muted">Nothing yet.</li>
        }
      </ul>
      @if (more()) {
        <button type="button" class="secondary" [disabled]="busy()" (click)="loadMore()">Older</button>
      }
    </section>
  `,
})
export class House implements OnInit {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  readonly deliveryNotes = this.content.callDeliveryNotes;
  readonly calls = signal<CallView[]>([]);
  readonly shelf = signal<ReadingView[]>([]);
  readonly cover = coverUrl;
  readonly progress = progress;
  readonly calling = signal(false);
  readonly message = signal('');
  readonly when = signal('');
  readonly v = (event: Event) => (event.target as HTMLInputElement).value;
  readonly people = signal<Person[]>([]);
  readonly entries = signal<EntryView[]>([]);
  readonly more = signal(false);
  readonly busy = signal(false);
  readonly error = signal('');
  readonly initials = initials;

  async ngOnInit(): Promise<void> {
    await this.attempt(async () => {
      const [people, feed] = await Promise.all([this.api.people(), this.content.feed()]);
      this.people.set(people);
      this.entries.set(feed.entries);
      this.more.set(feed.more);
      if (this.api.me()) {
        this.calls.set(await this.content.calls());
        this.shelf.set((await this.api.get<{ books: ReadingView[] }>('/api/v1/shelf', 'ReadingList')).books);
      }
    });
  }

  async call(event: Event): Promise<void> {
    event.preventDefault();
    await this.attempt(async () => {
      this.calls.set(await this.content.callToService(this.message(), this.when()));
      this.message.set('');
      this.when.set('');
      this.calling.set(false);
    });
  }

  ago(t: number): string {
    const m = Math.max(0, Math.floor((Date.now() / 1000 - t) / 60));
    return m < 1 ? 'just now' : m < 60 ? `${m} min ago` : `${Math.floor(m / 60)} h ago`;
  }

  async loadMore(): Promise<void> {
    await this.attempt(async () => {
      const feed = await this.content.feed(this.entries().length);
      this.entries.update((old) => [...old, ...feed.entries]);
      this.more.set(feed.more);
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
