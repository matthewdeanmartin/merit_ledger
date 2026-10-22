import { Component, effect, inject, input, signal } from '@angular/core';
import { EventView, problem } from '../api';
import { ContentApi } from '../content-api';
import { eventWhen } from '../invite';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** `/e/:id?t=<token>`: a guest answers an invitation, no account needed. */
@Component({
  selector: 'ml-guest-rsvp',
  styles: `
    .answers { display: flex; gap: 0.5rem; flex-wrap: wrap; }
    .answers button.on { outline: 3px solid var(--accent); }
  `,
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (event(); as e) {
      <section class="card narrow">
        <p class="muted small">{{ e.host_name }} invites you to</p>
        <h1>{{ e.title }}</h1>
        @if (e.cancelled) { <p class="error">This event is cancelled.</p> }
        <p>{{ when(e.start) }}{{ e.place ? ' · ' + e.place : '' }}</p>
        @if (e.notes) { <p>{{ e.notes }}</p> }
        <p><a [href]="e.ics + '?t=' + t()" download>Add to my calendar</a></p>
      </section>
      @if (!e.cancelled) {
        <section class="card narrow">
          <h2>Can you come?</h2>
          <div class="answers">
            <button type="button" class="secondary" [class.on]="e.my_answer === 'yes'" [disabled]="busy()" (click)="answer('yes')">Yes, I'll come</button>
            <button type="button" class="secondary" [class.on]="e.my_answer === 'maybe'" [disabled]="busy()" (click)="answer('maybe')">Maybe</button>
            <button type="button" class="secondary" [class.on]="e.my_answer === 'no'" [disabled]="busy()" (click)="answer('no')">Sorry, I can't</button>
          </div>
          <label>Bringing anything? (optional) <input maxlength="100" [value]="bringing()" (input)="bringing.set(v($event))" /></label>
          @if (saved()) { <p class="ok" role="status">Thanks! {{ e.host_name }} will see your answer.</p> }
        </section>
      }
    } @else if (!error()) {
      <p class="muted">Opening your invitation…</p>
    }
  `,
})
export class GuestRsvp {
  private readonly content = inject(ContentApi);
  readonly id = input.required<string>();
  /** The token in the invitation link. */
  readonly t = input<string>('');
  readonly event = signal<EventView | null>(null);
  readonly bringing = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly saved = signal(false);
  readonly v = value;

  constructor() {
    effect(() => void this.load(Number(this.id()), this.t()));
  }

  private async load(id: number, token: string): Promise<void> {
    try {
      const e = await this.content.guestEvent(id, token);
      this.event.set(e);
      this.bringing.set(e.my_bringing);
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  when(t: number): string {
    return eventWhen(t);
  }

  async answer(a: string): Promise<void> {
    this.busy.set(true);
    try {
      this.event.set(await this.content.guestRsvp(Number(this.id()), this.t(), a, this.bringing()));
      this.saved.set(true);
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
