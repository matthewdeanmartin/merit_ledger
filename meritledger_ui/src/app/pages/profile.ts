import { Component, inject, OnInit, signal, WritableSignal } from '@angular/core';
import { Router } from '@angular/router';
import { Api, Reminder, ReminderInput, problem } from '../api';
import { beginOAuth, ProviderView } from '../mastodon';
import { ContentApi } from '../content-api';
import { SYSTEMS } from '../calendar';
import { pinProblem } from '../people';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** Your name, email and PIN, and signing out. */
@Component({
  selector: 'ml-profile',
  styles: `
    .reminders { list-style: none; padding: 0; margin: 0 0 0.75rem; }
    .reminders li { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 0.5rem; padding: 0.4rem 0; border-bottom: 1px solid var(--line); }
    fieldset { border: none; padding: 0; display: flex; flex-wrap: wrap; gap: 0 0.75rem; }
  `,
  template: `
    @if (api.me(); as me) {
      <section class="card narrow">
        <h1>{{ me.display_name }}</h1>
        <p class="muted">{{ me.role }} · signed in on {{ me.sessions }} {{ me.sessions === 1 ? 'device' : 'devices' }}</p>
        <form (submit)="save($event)">
          <label>Name <input maxlength="32" required [value]="name()" (input)="name.set(v($event))" /></label>
          <label>Email (for invitations later; optional)
            <input type="email" [value]="email()" (input)="email.set(v($event))" /></label>
          @if (saved()) { <p class="ok" role="status">Saved.</p> }
          @if (error()) { <p class="error" role="alert">{{ error() }}</p> }
          <button type="submit" [disabled]="busy()">Save</button>
        </form>
      </section>

      <section class="card narrow">
        <h2>{{ me.has_pin ? 'Change PIN' : 'Set a PIN' }}</h2>
        <form (submit)="changePin($event)">
          @if (me.has_pin) {
            <label>Current PIN
              <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="current-password" maxlength="8" required (input)="current.set(v($event))" /></label>
          }
          <label>New PIN
            <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="new-password" maxlength="8" required (input)="pin.set(v($event))" /></label>
          <label>New PIN again
            <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="new-password" maxlength="8" required (input)="pin2.set(v($event))" /></label>
          @if (pinSaved()) { <p class="ok" role="status">PIN changed.</p> }
          @if (pinError()) { <p class="error" role="alert">{{ pinError() }}</p> }
          <button type="submit" [disabled]="busy()">Change PIN</button>
        </form>
      </section>

      <section class="card narrow">
        <h2>Messages</h2>
        <form (submit)="saveDelivery($event)">
          <label>Your Mastodon handle (the household bot DMs you notifications)
            <input [value]="mastodon()" (input)="mastodon.set(v($event))" placeholder="@you@mastomini.local" /></label>
          <label class="check"><input type="checkbox" [checked]="!me.dm_off" (change)="dmOff.set(!checked($event))" /> Mastodon DMs for invitations, calls and news</label>
          <label class="check"><input type="checkbox" [checked]="!me.email_off" (change)="emailOff.set(!checked($event))" /> Reminder emails (to your email address above)</label>
          @if (deliverySaved()) { <p class="ok" role="status">Saved.</p> }
          @if (deliveryError()) { <p class="error" role="alert">{{ deliveryError() }}</p> }
          <button type="submit" [disabled]="busy()">Save</button>
        </form>
      </section>

      <section class="card narrow">
        <h2>Mastodon</h2>
        @if (me.linked) {
          <p>Linked: <strong>{{ me.linked }}</strong>. You can sign in with it, and share your practice as yourself.</p>
          <label class="check"><input type="checkbox" [checked]="me.auto_publish" (change)="publishing({ auto_publish: checked($event) })" />
            Share my public practice and vow posts automatically</label>
          <label>Who sees what I share
            <select [value]="me.publish_visibility" (change)="publishing({ publish_visibility: v($event) })">
              <option value="public">Public</option>
              <option value="unlisted">Unlisted (not in public timelines)</option>
              <option value="private">Followers only</option>
            </select>
          </label>
          <button type="button" class="link" (click)="unlink()">Unlink</button>
        } @else {
          <p class="muted small">Link your account to sign in with it and to share what you practise. Only what everyone can
            see on the board can go there, never private entries, demerits or repentance.</p>
          <div class="row">
            @for (p of providers(); track p.id) {
              <button type="button" class="secondary" [disabled]="busy()" (click)="link(p.id)">Link {{ p.name }}</button>
            } @empty {
              <span class="muted small">This board has no Mastodon sign-in.</span>
            }
          </div>
        }
        @if (mastodonError()) { <p class="error" role="alert">{{ mastodonError() }}</p> }
      </section>

      <section class="card narrow">
        <h2>Reminders</h2>
        <p class="muted small">{{ delivery() }}</p>
        <ul class="reminders">
          @for (r of reminders(); track r.id) {
            <li>
              <span>{{ describe(r) }}{{ r.off ? ' (off)' : '' }}</span>
              <span class="row">
                <button type="button" class="secondary small" (click)="toggleReminder(r)">{{ r.off ? 'Turn on' : 'Turn off' }}</button>
                <button type="button" class="link" (click)="deleteReminder(r)">Delete</button>
              </span>
            </li>
          } @empty {
            <li class="muted">None yet.</li>
          }
        </ul>
        <form (submit)="addReminder($event)">
          <div class="segmented small" role="radiogroup" aria-label="Kind of reminder">
            <button type="button" [class.on]="rKind() === 'daily'" (click)="rKind.set('daily')">At a time</button>
            <button type="button" [class.on]="rKind() === 'observance'" (click)="rKind.set('observance')">For observances</button>
          </div>
          <label>Time <input type="time" required [value]="rTime()" (input)="rTime.set(v($event))" /></label>
          @if (rKind() === 'daily') {
            <fieldset>
              <legend>Days (none ticked = every day)</legend>
              @for (d of weekdays; track d; let i = $index) {
                <label class="check"><input type="checkbox" [checked]="dayOn(i)" (change)="toggleDay(i)" /> {{ d }}</label>
              }
            </fieldset>
          } @else {
            <label>Which
              <select [value]="rWhich()" (change)="rWhich.set(v($event))">
                <option value="any">Any observance</option>
                <option value="uposatha">Uposatha days</option>
                <option value="fasting">Fasting days</option>
                <option value="festival">Festivals</option>
                <option value="feast">Feast days</option>
              </select>
            </label>
            <label>When
              <select [value]="'' + rBefore()" (change)="rBefore.set(+v($event))">
                <option value="0">On the day</option>
                <option value="1">The day before</option>
                <option value="2">Two days before</option>
              </select>
            </label>
          }
          <label>Saying (optional) <input maxlength="120" [value]="rText()" (input)="rText.set(v($event))" placeholder="Sit for twenty minutes" /></label>
          @if (reminderError()) { <p class="error" role="alert">{{ reminderError() }}</p> }
          <button type="submit" [disabled]="busy()">Add reminder</button>
        </form>
      </section>

      <section class="card narrow">
        <h2>Your calendar</h2>
        <label>Calendar
          <select [value]="me.calendar" (change)="setCalendar(v($event))">
            <option value="">The household's</option>
            @for (s of systems; track s.key) { <option [value]="s.key">{{ s.name }}</option> }
          </select>
        </label>
      </section>

      <section class="card narrow">
        <h2>Your data</h2>
        <p><a class="button secondary" [href]="'/api/v1/people/' + me.id + '/entries.csv'" download>Download my entries (CSV)</a></p>
        <p class="muted small">Every entry the board has for you, private ones included, for a spreadsheet.</p>
      </section>

      <section class="card narrow">
        <h2>Sign out</h2>
        <div class="row">
          <button type="button" class="secondary" (click)="signOut(false)">Sign out here</button>
          @if (me.sessions > 1) {
            <button type="button" class="secondary" (click)="signOut(true)">Sign out everywhere</button>
          }
        </div>
      </section>
    }
  `,
})
export class Profile implements OnInit {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  readonly mastodon = signal('');
  readonly dmOff = signal(false);
  readonly emailOff = signal(false);
  readonly deliverySaved = signal(false);
  readonly deliveryError = signal('');
  readonly reminders = signal<Reminder[]>([]);
  readonly providers = signal<ProviderView[]>([]);
  readonly mastodonError = signal('');
  readonly delivery = signal('');
  readonly reminderError = signal('');
  readonly rKind = signal<'daily' | 'observance'>('daily');
  readonly rTime = signal('07:00');
  readonly rDays = signal(0);
  readonly rWhich = signal('uposatha');
  readonly rBefore = signal(1);
  readonly rText = signal('');
  readonly weekdays = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  readonly checked = (event: Event) => (event.target as HTMLInputElement).checked;
  private readonly router = inject(Router);
  readonly name = signal('');
  readonly email = signal('');
  readonly current = signal('');
  readonly pin = signal('');
  readonly pin2 = signal('');
  readonly saved = signal(false);
  readonly pinSaved = signal(false);
  readonly error = signal('');
  readonly pinError = signal('');
  readonly busy = signal(false);
  readonly v = value;
  readonly systems = SYSTEMS;

  ngOnInit(): void {
    const me = this.api.me();
    this.name.set(me?.display_name ?? '');
    this.email.set(me?.email ?? '');
    this.mastodon.set(me?.mastodon ?? '');
    this.dmOff.set(me?.dm_off ?? false);
    this.emailOff.set(me?.email_off ?? false);
    void this.api
      .get<{ providers: ProviderView[] }>('/api/v1/auth/providers', 'ProviderList')
      .then((r) => this.providers.set(r.providers))
      .catch(() => undefined);
    void this.content
      .reminders()
      .then((r) => this.showReminders(r))
      .catch(() => undefined);
  }

  private showReminders(r: { reminders: Reminder[]; delivery: string }): void {
    this.reminders.set(r.reminders);
    this.delivery.set(r.delivery);
  }

  async saveDelivery(event: Event): Promise<void> {
    event.preventDefault();
    await this.run(this.deliveryError, this.deliverySaved, () =>
      this.api.updateMe({ mastodon: this.mastodon(), dm_off: this.dmOff(), email_off: this.emailOff() }),
    );
    this.mastodon.set(this.api.me()?.mastodon ?? this.mastodon());
  }

  async link(provider: string): Promise<void> {
    this.busy.set(true);
    this.mastodonError.set('');
    try {
      await beginOAuth(this.api, provider, true);
    } catch (e) {
      this.mastodonError.set(e instanceof Error ? e.message : problem(e));
      this.busy.set(false);
    }
  }

  async unlink(): Promise<void> {
    if (!confirm('Unlink your Mastodon account? Nothing already shared is deleted.')) return;
    try {
      await this.api.send('DELETE', '/api/v1/me/mastodon', undefined, 'Me');
      await this.api.refresh();
    } catch (e) {
      this.mastodonError.set(problem(e));
    }
  }

  async publishing(change: { auto_publish?: boolean; publish_visibility?: string }): Promise<void> {
    this.mastodonError.set('');
    try {
      await this.api.updateMe(change);
    } catch (e) {
      this.mastodonError.set(problem(e));
    }
  }

  dayOn(i: number): boolean {
    return (this.rDays() & (1 << i)) !== 0;
  }

  toggleDay(i: number): void {
    this.rDays.update((d) => d ^ (1 << i));
  }

  describe(r: Reminder): string {
    const text = r.text ? `: ${r.text}` : '';
    if (r.kind === 'daily') {
      const days = r.days === 0 ? 'Every day' : this.weekdays.filter((_, i) => r.days & (1 << i)).join(', ');
      return `${days} at ${r.time}${text}`;
    }
    const which = { any: 'observances', uposatha: 'uposatha', fasting: 'fasting days', festival: 'festivals', feast: 'feast days' }[r.observances] ?? r.observances;
    const when = r.before === 0 ? 'on the day of' : r.before === 1 ? 'the day before' : `${r.before} days before`;
    return `${r.time} ${when} ${which}${text}`;
  }

  private input(r?: Reminder): ReminderInput {
    return r
      ? { kind: r.kind, time: r.time, days: r.days, before: r.before, observances: r.observances, text: r.text, off: r.off }
      : {
          kind: this.rKind(),
          time: this.rTime(),
          days: this.rDays(),
          before: this.rBefore(),
          observances: this.rWhich(),
          text: this.rText(),
          off: false,
        };
  }

  async addReminder(event: Event): Promise<void> {
    event.preventDefault();
    this.reminderError.set('');
    try {
      this.showReminders(await this.content.addReminder(this.input()));
      this.rText.set('');
    } catch (e) {
      this.reminderError.set(problem(e));
    }
  }

  async toggleReminder(r: Reminder): Promise<void> {
    try {
      this.showReminders(await this.content.updateReminder(r.id, { ...this.input(r), off: !r.off }));
    } catch (e) {
      this.reminderError.set(problem(e));
    }
  }

  async deleteReminder(r: Reminder): Promise<void> {
    try {
      this.showReminders(await this.content.deleteReminder(r.id));
    } catch (e) {
      this.reminderError.set(problem(e));
    }
  }

  async save(event: Event): Promise<void> {
    event.preventDefault();
    await this.run(this.error, this.saved, () => this.api.updateMe({ display_name: this.name(), email: this.email() }));
  }

  async changePin(event: Event): Promise<void> {
    event.preventDefault();
    const bad = pinProblem(this.pin(), this.pin2());
    if (bad) return this.pinError.set(bad);
    await this.run(this.pinError, this.pinSaved, () => this.api.changePin(this.current(), this.pin()));
  }

  async setCalendar(calendar: string): Promise<void> {
    await this.run(this.error, this.saved, () => this.api.updateMe({ calendar }));
  }

  async signOut(everywhere: boolean): Promise<void> {
    await this.api.signOut(everywhere).catch(() => undefined);
    await this.router.navigateByUrl('/signin');
  }

  private async run(error: WritableSignal<string>, done: WritableSignal<boolean>, action: () => Promise<void>): Promise<void> {
    this.busy.set(true);
    error.set('');
    done.set(false);
    try {
      await action();
      done.set(true);
    } catch (e) {
      error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
