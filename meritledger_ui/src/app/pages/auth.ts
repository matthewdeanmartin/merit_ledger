import { Component, inject, OnInit, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { Api, Person, problem } from '../api';
import { initials, pinProblem, signInOrder } from '../people';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** First run: whoever submits this becomes admin. */
@Component({
  selector: 'ml-setup',
  template: `
    <section class="card narrow">
      <h1>Set up Merit Ledger</h1>
      <p>You're the first one here, so you'll be the <strong>admin</strong>. Afterwards you can add the rest of
        the household (kids included) from the admin page.</p>
      <form (submit)="submit($event)">
        <label>Your name <input autocomplete="nickname" maxlength="32" required (input)="name.set(v($event))" /></label>
        <label>PIN (4–8 digits)
          <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="new-password" maxlength="8" required (input)="pin.set(v($event))" /></label>
        <label>PIN again
          <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="new-password" maxlength="8" required (input)="pin2.set(v($event))" /></label>
        @if (api.status()?.setup_code_required) {
          <label>Setup code <input autocomplete="off" required (input)="code.set(v($event))" /></label>
        }
        @if (error()) { <p class="error" role="alert">{{ error() }}</p> }
        <button type="submit" [disabled]="busy()">Set up</button>
      </form>
    </section>
  `,
})
export class Setup {
  readonly api = inject(Api);
  private readonly router = inject(Router);
  readonly name = signal('');
  readonly pin = signal('');
  readonly pin2 = signal('');
  readonly code = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly v = value;

  async submit(event: Event): Promise<void> {
    event.preventDefault();
    const bad = pinProblem(this.pin(), this.pin2());
    if (bad) return this.error.set(bad);
    this.busy.set(true);
    this.error.set('');
    try {
      await this.api.setup(this.name(), this.pin(), this.code());
      await this.router.navigateByUrl('/');
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}

/** Tap your name, enter your PIN. */
@Component({
  selector: 'ml-signin',
  imports: [RouterLink],
  template: `
    <section class="card narrow">
      @if (!picked()) {
        <h1>Who's here?</h1>
        @if (loading()) {
          <p class="muted">Asking the board…</p>
        } @else {
          <ul class="tiles">
            @for (p of people(); track p.id) {
              <li>
                <button type="button" class="tile" (click)="pick(p)">
                  <span class="avatar" aria-hidden="true">{{ initials(p.display_name) }}</span>
                  <span>{{ p.display_name }}</span>
                </button>
              </li>
            } @empty {
              <li class="muted">Nobody here can sign in yet. Ask an admin to give you a PIN.</li>
            }
          </ul>
        }
        @if (api.status()?.signup_open) {
          <p class="aside">Visiting? <a routerLink="/signup">Make a guest account</a></p>
        }
      } @else {
        <h1>Hello, {{ picked()!.display_name }}</h1>
        <form (submit)="submit($event)">
          <label>Your PIN
            <input #pinBox type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="current-password" maxlength="8"
              required autofocus (input)="pin.set(v($event))" /></label>
          @if (error()) { <p class="error" role="alert">{{ error() }}</p> }
          <div class="row">
            <button type="submit" [disabled]="busy()">Sign in</button>
            <button type="button" class="secondary" (click)="back()">Not me</button>
          </div>
        </form>
      }
    </section>
  `,
})
export class SignIn implements OnInit {
  readonly api = inject(Api);
  private readonly router = inject(Router);
  readonly people = signal<Person[]>([]);
  readonly loading = signal(true);
  readonly picked = signal<Person | null>(null);
  readonly pin = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly v = value;
  readonly initials = initials;

  async ngOnInit(): Promise<void> {
    try {
      this.people.set(signInOrder(await this.api.people()));
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.loading.set(false);
    }
  }

  pick(p: Person): void {
    this.picked.set(p);
    this.pin.set('');
    this.error.set('');
  }

  back(): void {
    this.picked.set(null);
    this.error.set('');
  }

  async submit(event: Event): Promise<void> {
    event.preventDefault();
    const who = this.picked();
    if (!who) return;
    this.busy.set(true);
    this.error.set('');
    try {
      await this.api.signIn(who.id, this.pin());
      await this.router.navigateByUrl('/');
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}

/** A guest makes their own account while sign-up is open. */
@Component({
  selector: 'ml-signup',
  imports: [RouterLink],
  template: `
    <section class="card narrow">
      <h1>Guest account</h1>
      <p>Pick a name the household will recognize and a PIN. An admin can make you a full member later.</p>
      <form (submit)="submit($event)">
        <label>Your name <input autocomplete="nickname" maxlength="32" required (input)="name.set(v($event))" /></label>
        <label>PIN (4–8 digits)
          <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="new-password" maxlength="8" required (input)="pin.set(v($event))" /></label>
        <label>PIN again
          <input type="password" inputmode="numeric" pattern="[0-9]*" autocomplete="new-password" maxlength="8" required (input)="pin2.set(v($event))" /></label>
        @if (error()) { <p class="error" role="alert">{{ error() }}</p> }
        <button type="submit" [disabled]="busy()">Make my account</button>
      </form>
      <p class="aside"><a routerLink="/signin">Back to sign in</a></p>
    </section>
  `,
})
export class SignUp {
  readonly api = inject(Api);
  private readonly router = inject(Router);
  readonly name = signal('');
  readonly pin = signal('');
  readonly pin2 = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly v = value;

  async submit(event: Event): Promise<void> {
    event.preventDefault();
    const bad = pinProblem(this.pin(), this.pin2());
    if (bad) return this.error.set(bad);
    this.busy.set(true);
    this.error.set('');
    try {
      await this.api.signUp(this.name(), this.pin());
      await this.router.navigateByUrl('/');
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
