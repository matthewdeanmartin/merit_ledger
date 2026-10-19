import { Component, inject, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, EntryView, Person, problem } from '../api';
import { EntryItem } from '../entry-item';
import { initials } from '../people';

/** The household: everyone's profile, and what the house has been practising. */
@Component({
  selector: 'ml-house',
  imports: [RouterLink, EntryItem],
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
  private readonly api = inject(Api);
  readonly people = signal<Person[]>([]);
  readonly entries = signal<EntryView[]>([]);
  readonly more = signal(false);
  readonly busy = signal(false);
  readonly error = signal('');
  readonly initials = initials;

  async ngOnInit(): Promise<void> {
    await this.attempt(async () => {
      const [people, feed] = await Promise.all([this.api.people(), this.api.feed()]);
      this.people.set(people);
      this.entries.set(feed.entries);
      this.more.set(feed.more);
    });
  }

  async loadMore(): Promise<void> {
    await this.attempt(async () => {
      const feed = await this.api.feed(this.entries().length);
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
