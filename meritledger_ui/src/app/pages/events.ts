import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { Api, EventView, Person, problem } from '../api';
import { ContentApi } from '../content-api';
import { canShare, eventInvite, eventWhen, fromUnix, Invite, share, shareTargets, toUnix } from '../invite';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

const ANSWERS: Record<string, string> = { yes: 'Coming', maybe: 'Maybe', no: "Can't come", '': 'No answer yet' };

/** `/events`: what's coming up. `/events/new` and `/events/:id`: one event. */
@Component({
  selector: 'ml-events',
  imports: [RouterLink],
  styles: `
    .list { list-style: none; padding: 0; margin: 0; }
    .list li { padding: 0.6rem 0; border-bottom: 1px solid var(--line); }
    .answers { display: flex; gap: 0.5rem; flex-wrap: wrap; }
    .answers button.on { outline: 3px solid var(--accent); }
    .who { list-style: none; padding: 0; margin: 0; }
    .who li { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 0.5rem; padding: 0.35rem 0; border-bottom: 1px solid var(--line); }
    .share { display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 0.5rem 0; }
    .cancelled { text-decoration: line-through; }
    textarea { width: 100%; font: inherit; padding: 0.5rem; border: 1px solid var(--line); border-radius: 0.5rem; background: var(--bg); color: var(--text); }
  `,
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (!id()) {
      <div class="bar">
        <h1>Events</h1>
        @if (canPlan()) { <a routerLink="/events/new" class="button small">Plan one</a> }
      </div>
      <section class="card">
        <ul class="list">
          @for (e of events(); track e.id) {
            <li>
              <a [routerLink]="['/events', e.id]" [class.cancelled]="e.cancelled"><strong>{{ e.title }}</strong></a>
              <span class="muted small"> · {{ when(e.start) }}{{ e.place ? ' · ' + e.place : '' }} · {{ e.host_name }}</span>
              @if (e.my_answer) { <span class="small"> · {{ answers[e.my_answer] }}</span> }
            </li>
          } @empty {
            <li class="muted">{{ loaded() ? 'Nothing planned.' : 'Asking the board…' }}</li>
          }
        </ul>
      </section>
    } @else if (editing()) {
      <h1>{{ id() === 'new' ? 'Plan an event' : 'Edit event' }}</h1>
      <form class="card" (submit)="save($event)">
        <label>What <input maxlength="100" required [value]="title()" (input)="title.set(v($event))" placeholder="Uposatha dinner" /></label>
        <div class="row">
          <label>Day <input type="date" required [value]="date()" (input)="date.set(v($event))" /></label>
          <label>Time <input type="time" required [value]="time()" (input)="time.set(v($event))" /></label>
          <label>Until (optional) <input type="time" [value]="endTime()" (input)="endTime.set(v($event))" /></label>
        </div>
        <label>Where <input maxlength="100" [value]="place()" (input)="place.set(v($event))" /></label>
        <label>Notes <textarea rows="3" maxlength="1000" [value]="notes()" (input)="notes.set(v($event))"></textarea></label>
        <fieldset>
          <legend>Invite from the house (they get it in their bell, and by Mastodon DM)</legend>
          @for (p of others(); track p.id) {
            <label class="check"><input type="checkbox" [checked]="invitees().includes(p.id)" (change)="toggle(p.id)" /> {{ p.display_name }}</label>
          }
          <p class="muted small">People outside the house: save, then add them as guests to get a link to send.</p>
        </fieldset>
        @if (formError()) { <p class="error" role="alert">{{ formError() }}</p> }
        <div class="row">
          <button type="submit" [disabled]="busy()">Save</button>
          <button type="button" class="secondary" (click)="back()">Cancel</button>
        </div>
      </form>
    } @else if (event(); as e) {
      <section class="card">
        <h1 [class.cancelled]="e.cancelled">{{ e.title }}</h1>
        @if (e.cancelled) { <p class="error">Cancelled.</p> }
        <p>{{ when(e.start) }}{{ e.end ? ' – ' + endOf(e.end) : '' }}{{ e.place ? ' · ' + e.place : '' }}</p>
        <p class="muted small">Hosted by {{ e.host_name }} · <a [href]="e.ics" download>Add to my calendar</a></p>
        @if (e.notes) { <p>{{ e.notes }}</p> }
      </section>

      @if (!e.cancelled && api.me()?.id !== e.host) {
        <section class="card">
          <h2>Can you come?</h2>
          <div class="answers">
            @for (a of ['yes', 'maybe', 'no']; track a) {
              <button type="button" class="secondary" [class.on]="e.my_answer === a" [disabled]="busy()" (click)="answer(a)">{{ answers[a] }}</button>
            }
          </div>
          <label>Bringing (optional) <input maxlength="100" [value]="bringing()" (input)="bringing.set(v($event))" (change)="e.my_answer && answer(e.my_answer)" placeholder="Dumplings" /></label>
        </section>
      }

      <section class="card">
        <h2>Who's coming</h2>
        <ul class="who">
          @for (p of e.people; track p.person) {
            <li><span>{{ p.name }}{{ p.person === e.host ? ' (host)' : '' }}</span>
              <span class="muted small">{{ answers[p.answer] }}{{ p.bringing ? ' · bringing ' + p.bringing : '' }}</span></li>
          }
          @for (g of e.guests; track g.id) {
            <li><span>{{ g.name }} <span class="muted small">(guest)</span></span>
              <span class="muted small">{{ answers[g.answer] }}{{ g.bringing ? ' · bringing ' + g.bringing : '' }}</span>
              @if (e.can_edit) {
                <span class="share">
                  @for (t of targets(g.link); track t.id) { <a class="button secondary small" [href]="t.href" target="_blank" rel="noopener">{{ t.label }}</a> }
                  @if (shareable) { <button type="button" class="secondary small" (click)="shareSheet(g.link)">Share…</button> }
                  <button type="button" class="secondary small" (click)="copy(g.link)">Copy link</button>
                  <button type="button" class="link" (click)="removeGuest(g.id)">Remove</button>
                </span>
              }
            </li>
          }
        </ul>
        @if (e.can_edit && !e.cancelled) {
          <form class="row" (submit)="addGuest($event)">
            <label class="grow">Invite someone from outside the house
              <input maxlength="60" [value]="guestName()" (input)="guestName.set(v($event))" placeholder="Their name" /></label>
            <button type="submit" [disabled]="busy() || !guestName().trim()">Add guest</button>
          </form>
          <p class="muted small">Each guest gets their own link; send it from your phone with the buttons above. They answer on the board, no account needed (on the home Wi-Fi).</p>
          @if (copied()) { <p class="ok" role="status">Link copied.</p> }
        }
      </section>

      @if (e.can_edit) {
        <div class="row">
          <button type="button" class="secondary" (click)="startEdit(e)">Edit</button>
          @if (!e.cancelled) { <button type="button" class="secondary" (click)="cancel()">Cancel the event</button> }
          <button type="button" class="danger" (click)="remove()">Delete</button>
        </div>
      }
    } @else if (!error()) {
      <p class="muted">Asking the board…</p>
    }
  `,
})
export class Events {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly router = inject(Router);
  /** Route parameter `/events/:id` ('new' for a new one). */
  readonly id = input<string>();
  readonly events = signal<EventView[]>([]);
  readonly event = signal<EventView | null>(null);
  readonly people = signal<Person[]>([]);
  readonly loaded = signal(false);
  readonly editing = signal(false);
  readonly title = signal('');
  readonly date = signal('');
  readonly time = signal('');
  readonly endTime = signal('');
  readonly place = signal('');
  readonly notes = signal('');
  readonly invitees = signal<number[]>([]);
  readonly bringing = signal('');
  readonly guestName = signal('');
  readonly copied = signal(false);
  readonly error = signal('');
  readonly formError = signal('');
  readonly busy = signal(false);
  readonly answers = ANSWERS;
  readonly v = value;
  readonly shareable = canShare();
  readonly canPlan = computed(() => {
    const me = this.api.me();
    return !!me && me.role !== 'guest';
  });
  readonly others = computed(() => this.people().filter((p) => p.id !== this.api.me()?.id));

  constructor() {
    effect(() => void this.load(this.id()));
  }

  private async load(id: string | undefined): Promise<void> {
    this.error.set('');
    this.editing.set(false);
    try {
      if (!id) {
        this.events.set(await this.content.events());
        this.loaded.set(true);
        return;
      }
      this.people.set(await this.api.people());
      if (id === 'new') {
        this.title.set('');
        this.date.set('');
        this.time.set('18:00');
        this.endTime.set('');
        this.place.set('Home');
        this.notes.set('');
        this.invitees.set(this.others().map((p) => p.id));
        this.editing.set(true);
        return;
      }
      const e = await this.content.eventDetail(Number(id));
      this.event.set(e);
      this.bringing.set(e.my_bringing);
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  when(t: number): string {
    return eventWhen(t);
  }

  endOf(t: number): string {
    return fromUnix(t)[1];
  }

  toggle(id: number): void {
    this.invitees.update((l) => (l.includes(id) ? l.filter((x) => x !== id) : [...l, id]));
  }

  startEdit(e: EventView): void {
    const [date, time] = fromUnix(e.start);
    this.title.set(e.title);
    this.date.set(date);
    this.time.set(time);
    this.endTime.set(e.end ? fromUnix(e.end)[1] : '');
    this.place.set(e.place);
    this.notes.set(e.notes);
    this.invitees.set(e.people.map((p) => p.person).filter((p) => p !== e.host));
    this.editing.set(true);
  }

  back(): void {
    if (this.id() === 'new') void this.router.navigateByUrl('/events');
    else this.editing.set(false);
  }

  async save(event: Event): Promise<void> {
    event.preventDefault();
    this.busy.set(true);
    this.formError.set('');
    try {
      const start = toUnix(this.date(), this.time());
      const input = {
        title: this.title(),
        start,
        end: this.endTime() ? toUnix(this.date(), this.endTime()) : 0,
        place: this.place(),
        notes: this.notes(),
        invitees: this.invitees(),
        rev: this.event()?.rev ?? 0,
      };
      const id = this.id();
      const saved = id === 'new' ? await this.content.addEvent(input) : await this.content.updateEvent(Number(id), input);
      this.event.set(saved);
      this.editing.set(false);
      if (id === 'new') await this.router.navigate(['/events', saved.id]);
    } catch (e) {
      this.formError.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }

  async answer(a: string): Promise<void> {
    await this.run(async () => this.event.set(await this.content.rsvp(this.event()!.id, a, this.bringing())));
  }

  async addGuest(event: Event): Promise<void> {
    event.preventDefault();
    await this.run(async () => {
      this.event.set(await this.content.addGuest(this.event()!.id, this.guestName()));
      this.guestName.set('');
    });
  }

  async removeGuest(guest: number): Promise<void> {
    if (!confirm('Remove this guest? Their link stops working.')) return;
    await this.run(async () => this.event.set(await this.content.removeGuest(this.event()!.id, guest)));
  }

  private invite(link: string): Invite {
    const e = this.event()!;
    return eventInvite(e, this.api.me()?.display_name ?? e.host_name, location.origin + link);
  }

  targets(link: string) {
    return shareTargets(this.invite(link));
  }

  async shareSheet(link: string): Promise<void> {
    await share(this.invite(link)).catch(() => undefined);
  }

  async copy(link: string): Promise<void> {
    await navigator.clipboard?.writeText(location.origin + link).catch(() => undefined);
    this.copied.set(true);
    setTimeout(() => this.copied.set(false), 2_000);
  }

  async cancel(): Promise<void> {
    if (!confirm('Cancel this event? Everyone invited is told.')) return;
    await this.run(async () => this.event.set(await this.content.cancelEvent(this.event()!.id)));
  }

  async remove(): Promise<void> {
    if (!confirm('Delete this event? (Cancelling tells people; deleting just removes it.)')) return;
    await this.run(async () => {
      await this.content.deleteEvent(this.event()!.id);
      await this.router.navigateByUrl('/events');
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
