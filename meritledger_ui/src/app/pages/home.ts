import { Component, computed, inject, OnDestroy, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, EntryView, Person, problem, Tile, TodayView } from '../api';
import { amountLabel, dayHeading, entryLine, tileLabel, VISIBILITIES, visibilityLabel } from '../practice';
import { Sessions } from '../sessions';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** What the last tap did, for the undo bar. */
interface Logged {
  entry: EntryView;
  text: string;
}

/**
 * Today: one tap on a tile records that practice, done for the day
 * (Strava-like). Recording for someone else is a picker at the top.
 */
@Component({
  selector: 'ml-today',
  imports: [RouterLink],
  template: `
    <section class="today">
      <div class="bar">
        <div>
          <h1>{{ view() ? heading(view()!.date) : 'Today' }}</h1>
          @if (people().length > 1 && !guest()) {
            <label class="for">For
              <select [value]="person()" (change)="choose(+v($event))">
                @for (p of people(); track p.id) {
                  <option [value]="p.id">{{ p.id === api.me()?.id ? 'me (' + p.display_name + ')' : p.display_name }}</option>
                }
              </select>
            </label>
          }
        </div>
        <div class="row">
          <a routerLink="/timer" class="button small">🔔 Timer</a>
          <a routerLink="/practices" class="button secondary small">Change tiles</a>
        </div>
      </div>

      @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }

      @if (view(); as t) {
        @for (o of t.observances; track o.name) {
          <p class="observance card" [class.feast]="o.feast">
            {{ o.feast ? '✦' : '🪷' }} Today: <strong>{{ o.name }}</strong>@if (o.feast) { · a feast day }
            @if (o.shifted_from) { <span class="muted"> (observed on Sunday)</span> }
          </p>
        }
        <ul class="tilegrid">
          @for (tile of t.tiles; track tile.key) {
            <li>
              <button type="button" class="ptile" [class.done]="tile.count > 0" [class.demerit]="tile.polarity === 'demerit'"
                [disabled]="busy()" (click)="tap(tile)">
                <span class="icon" aria-hidden="true">{{ tile.icon }}</span>
                <span class="name">{{ tile.name }}</span>
                <span class="sub">{{ label(tile) }}</span>
              </button>
            </li>
          }
        </ul>

        @if (logged(); as l) {
          <div class="toast" role="status">
            <span>{{ l.text }}</span>
            <button type="button" class="link" (click)="undo(l.entry)">Undo</button>
            <button type="button" class="link" (click)="edit(l.entry)">Edit</button>
          </div>
        }

        <section class="card">
          <h2>{{ t.person === api.me()?.id ? 'Your day' : t.person_name + "'s day" }}</h2>
          <ul class="entries">
            @for (e of t.entries; track e.id) {
              <li>
                @if (editing()?.id === e.id) {
                  <form (submit)="save($event, e)">
                    <strong>{{ e.icon }} {{ e.kind_name }}</strong>
                    <div class="row">
                      @if (e.unit !== 'once') {
                        <label>{{ e.unit === 'minutes' ? 'Minutes' : 'Count' }}
                          <input type="number" min="1" inputmode="numeric" [value]="e.amount" (input)="amount.set(+v($event))" /></label>
                      }
                      <label>Day <input type="date" [value]="e.date" [max]="t.date" (input)="date.set(v($event))" /></label>
                    </div>
                    <label>Note <input maxlength="500" [value]="e.note" (input)="note.set(v($event))" /></label>
                    <label>Who sees it
                      <select [value]="e.visibility" (change)="visibility.set(v($event))">
                        @for (vis of visibilities; track vis) { <option [value]="vis">{{ visLabel(vis) }}</option> }
                      </select>
                    </label>
                    <div class="row">
                      <button type="submit" [disabled]="busy()">Save</button>
                      <button type="button" class="secondary" (click)="editing.set(null)">Cancel</button>
                      <button type="button" class="secondary danger" (click)="undo(e)">Delete</button>
                    </div>
                  </form>
                } @else {
                  <span class="icon" aria-hidden="true">{{ e.icon }}</span>
                  <span class="what">
                    {{ line(e) }}
                    @if (e.visibility !== 'public') { <span class="muted" [title]="visLabel(e.visibility)">{{ e.visibility === 'private' ? '🔒' : '◐' }}</span> }
                    @if (e.note) { <span class="muted note">{{ e.note }}</span> }
                    @if (e.recorded_by_name && e.recorded_by_name !== e.person_name) {
                      <span class="muted note">recorded by {{ e.recorded_by_name }}</span>
                    }
                  </span>
                  @if (e.can_edit) {
                    <button type="button" class="link" (click)="edit(e)">Edit</button>
                  }
                }
              </li>
            } @empty {
              <li class="muted">Nothing yet today. Tap a tile when you've done it.</li>
            }
          </ul>
        </section>
      } @else if (!error()) {
        <p class="muted">Asking the board…</p>
      }
    </section>
  `,
})
export class Home implements OnInit, OnDestroy {
  readonly api = inject(Api);
  private readonly sessions = inject(Sessions);
  readonly view = signal<TodayView | null>(null);
  readonly people = signal<Person[]>([]);
  readonly person = signal(0);
  readonly error = signal('');
  readonly busy = signal(false);
  readonly logged = signal<Logged | null>(null);
  readonly editing = signal<EntryView | null>(null);
  readonly amount = signal(0);
  readonly note = signal('');
  readonly date = signal('');
  readonly visibility = signal('');
  readonly guest = computed(() => this.api.me()?.role === 'guest');
  readonly visibilities = VISIBILITIES;
  readonly v = value;
  readonly heading = dayHeading;
  readonly line = entryLine;
  readonly visLabel = visibilityLabel;
  private toastTimer?: ReturnType<typeof setTimeout>;

  async ngOnInit(): Promise<void> {
    this.person.set(this.api.me()?.id ?? 0);
    // Timer sessions finished off the Wi-Fi: send them before showing today.
    await this.sessions.flush().catch(() => undefined);
    await this.attempt(async () => {
      const [view, people] = await Promise.all([this.api.today(), this.api.people()]);
      this.view.set(view);
      this.people.set(people);
    });
  }

  ngOnDestroy(): void {
    clearTimeout(this.toastTimer);
  }

  label(tile: Tile): string {
    return tileLabel(tile.unit, tile.count, tile.total, tile.default_amount);
  }

  async choose(person: number): Promise<void> {
    this.person.set(person);
    this.logged.set(null);
    this.editing.set(null);
    await this.attempt(async () => this.view.set(await this.api.today(person)));
  }

  async tap(tile: Tile): Promise<void> {
    await this.attempt(async () => {
      const view = await this.api.record(this.person(), tile.key);
      this.view.set(view);
      // The newest entry of this kind is the one just made.
      const entry = view.entries.filter((e) => e.kind === tile.key).sort((a, b) => b.id - a.id)[0];
      if (entry) {
        const amount = entry.unit === 'once' ? '' : ` · ${amountLabel(entry.unit, entry.amount)}`;
        const who = view.person === this.api.me()?.id ? '' : ` for ${view.person_name}`;
        this.logged.set({ entry, text: `Logged ${entry.kind_name}${amount}${who}` });
        clearTimeout(this.toastTimer);
        this.toastTimer = setTimeout(() => this.logged.set(null), 6000);
      }
    });
  }

  async undo(entry: EntryView): Promise<void> {
    this.logged.set(null);
    this.editing.set(null);
    await this.attempt(async () => this.view.set(await this.api.deleteEntry(entry.id)));
  }

  edit(entry: EntryView): void {
    this.logged.set(null);
    this.editing.set(entry);
    this.amount.set(entry.amount);
    this.note.set(entry.note);
    this.date.set(entry.date);
    this.visibility.set(entry.visibility);
  }

  async save(event: Event, e: EntryView): Promise<void> {
    event.preventDefault();
    await this.attempt(async () => {
      this.view.set(
        await this.api.updateEntry(e.id, {
          rev: e.rev,
          amount: e.unit === 'once' ? undefined : this.amount(),
          note: this.note(),
          date: this.date(),
          visibility: this.visibility(),
        }),
      );
      this.editing.set(null);
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
