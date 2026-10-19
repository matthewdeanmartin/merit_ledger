import { Component, inject, OnInit, signal, WritableSignal } from '@angular/core';
import { Router } from '@angular/router';
import { Api, problem } from '../api';
import { SYSTEMS } from '../calendar';
import { pinProblem } from '../people';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** Your name, email and PIN, and signing out. */
@Component({
  selector: 'ml-profile',
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
