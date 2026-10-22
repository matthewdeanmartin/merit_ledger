import { Component, inject, OnDestroy, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { DeliveryView, problem, ScreenSettings } from '../api';
import { ContentApi } from '../content-api';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** `/admin/messages`: the household Mastodon bot, Brevo email, and the outbox. */
@Component({
  selector: 'ml-messages-admin',
  imports: [RouterLink],
  styles: `
    .jobs { list-style: none; padding: 0; margin: 0; font-size: 0.9rem; }
    .jobs li { padding: 0.4rem 0; border-bottom: 1px solid var(--line); overflow-wrap: anywhere; }
    .failed { color: var(--error); }
    .frame { max-width: 100%; height: auto; border-radius: 0.5rem; image-rendering: pixelated; }
  `,
  template: `
    <div class="bar">
      <h1>Messages</h1>
      <a routerLink="/admin" class="button secondary small">Household</a>
    </div>
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (view(); as d) {
      <section class="card">
        <h2>Mastodon bot</h2>
        @if (d.bot_handle) {
          <p>DMs come from <strong>{{ d.bot_handle }}</strong>.</p>
          @if (d.bot_error) { <p class="error">Last error: {{ d.bot_error }}</p> }
          <div class="row">
            <button type="button" class="secondary" [disabled]="busy()" (click)="test('dm')">Send me a test DM</button>
            <button type="button" class="link" (click)="removeBot()">Remove the bot</button>
          </div>
        } @else {
          <p class="muted small">Create a bot account on mastomini (its admin login is used once, never stored), or paste an access
            token from any Mastodon account's Preferences → Development (scopes: read:accounts, write:statuses).</p>
          <form (submit)="setupBot($event)">
            <label>Instance <input [value]="instance()" (input)="instance.set(v($event))" /></label>
            <label>Bot username <input [value]="username()" (input)="username.set(v($event))" /></label>
            <label>mastomini admin username <input autocomplete="off" [value]="admin()" (input)="admin.set(v($event))" /></label>
            <label>mastomini admin password <input type="password" autocomplete="off" [value]="password()" (input)="password.set(v($event))" /></label>
            <p class="muted small">…or an access token instead:</p>
            <label>Access token <input type="password" autocomplete="off" [value]="token()" (input)="token.set(v($event))" /></label>
            <button type="submit" [disabled]="busy()">Set up the bot</button>
          </form>
        }
        @if (d.bot_setup) { <p class="muted small">Setup: {{ d.bot_setup }}</p> }
      </section>

      <section class="card">
        <h2>Email (Brevo)</h2>
        @if (d.mail_sender) {
          <p>Reminders come from <strong>{{ d.mail_sender }}</strong>{{ d.mail_built_in ? ' (the key built into the firmware)' : '' }}.
            {{ d.sent_today }} of {{ d.daily_limit }} sent today.</p>
          @if (d.mail_error) { <p class="error">Last error: {{ d.mail_error }}</p> }
          <button type="button" class="secondary" [disabled]="busy()" (click)="test('email')">Send me a test email</button>
        } @else {
          <form (submit)="setupMail($event)">
            <label>Brevo API key (xkeysib-…) <input type="password" autocomplete="off" [value]="apiKey()" (input)="apiKey.set(v($event))" /></label>
            <label>Sender (optional; a verified Brevo sender) <input type="email" [value]="sender()" (input)="sender.set(v($event))" /></label>
            <button type="submit" [disabled]="busy()">Set up email</button>
          </form>
        }
        @if (d.mail_setup) { <p class="muted small">Setup: {{ d.mail_setup }}</p> }
      </section>

      <section class="card">
        <h2>The board's screen</h2>
        <p class="muted small">It shows the time, today's observances, calls to practice and the next event, a card every ten seconds.</p>
        <div class="row">
          @for (p of [0, 1, 2]; track p) {
            <img class="frame" [src]="'/api/v1/display/frame.bmp?page=' + p + '&v=' + stamp()" width="320" height="172" alt="The screen" />
          }
        </div>
        @if (screen(); as s) {
          <label class="check"><input type="checkbox" [checked]="s.flip" (change)="setScreen({ flip: checked($event) })" /> Upside down? Turn it 180°</label>
          <label class="check"><input type="checkbox" [checked]="s.native" (change)="setScreen({ native: checked($event) })" /> Colours look wrong? Swap the colour order</label>
          <label>Brightness {{ s.brightness }}%
            <input type="range" min="5" max="60" step="5" [value]="s.brightness" (change)="setScreen({ brightness: +v($event) })" /></label>
          <p class="muted">Hold the screen for a second to open its menu. If taps land in the wrong place, touch a corner, then:</p>
          <label class="check"><input type="checkbox" [checked]="s.touch_flip_x" (change)="setScreen({ touch_flip_x: checked($event) })" /> Taps land mirrored left to right</label>
          <label class="check"><input type="checkbox" [checked]="s.touch_flip_y" (change)="setScreen({ touch_flip_y: checked($event) })" /> Taps land mirrored top to bottom</label>
          <p class="muted">Last touch: {{ s.last_touch || 'none yet' }} <button type="button" class="secondary small" (click)="reloadScreen()">Refresh</button></p>
        }
      </section>

      <section class="card">
        <h2>Outbox</h2>
        <ul class="jobs">
          @for (j of d.jobs; track j.id) {
            <li [class.failed]="j.state === 'failed'">
              {{ j.kind === 'dm' ? '💬' : '✉️' }} {{ j.text }} · <strong>{{ j.state }}</strong>
              @if (j.attempts > 1) { · {{ j.attempts }} tries }
              @if (j.error) { · {{ j.error }} }
              @if (j.state === 'failed') { <button type="button" class="link" (click)="retry(j.id)">Retry</button> }
            </li>
          } @empty {
            <li class="muted">Nothing sent yet.</li>
          }
        </ul>
      </section>
    }
  `,
})
export class MessagesAdmin implements OnInit, OnDestroy {
  private readonly content = inject(ContentApi);
  readonly view = signal<DeliveryView | null>(null);
  readonly instance = signal('https://mastomini.local');
  readonly username = signal('meritledger');
  readonly admin = signal('');
  readonly password = signal('');
  readonly token = signal('');
  readonly apiKey = signal('');
  readonly sender = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly v = value;
  readonly checked = (event: Event) => (event.target as HTMLInputElement).checked;
  readonly screen = signal<ScreenSettings | null>(null);
  readonly stamp = signal(Date.now());
  private timer?: ReturnType<typeof setInterval>;

  async ngOnInit(): Promise<void> {
    await this.run(async () => this.view.set(await this.content.delivery()));
    this.reloadScreen();
    // Setups and sends run on the board's worker: watch them finish.
    this.timer = setInterval(() => {
      if (document.visibilityState === 'visible') void this.content.delivery().then((d) => this.view.set(d)).catch(() => undefined);
    }, 5_000);
  }

  ngOnDestroy(): void {
    clearInterval(this.timer);
  }

  async setupBot(event: Event): Promise<void> {
    event.preventDefault();
    await this.run(async () => {
      this.view.set(
        await this.content.setupBot({
          instance: this.instance(),
          token: this.token(),
          admin: this.admin(),
          password: this.password(),
          username: this.username(),
          display_name: 'Merit Ledger',
        }),
      );
      this.password.set('');
      this.token.set('');
    });
  }

  async removeBot(): Promise<void> {
    if (!confirm('Remove the bot? DMs stop until it is set up again.')) return;
    await this.run(async () => this.view.set(await this.content.removeBot()));
  }

  async setupMail(event: Event): Promise<void> {
    event.preventDefault();
    await this.run(async () => {
      this.view.set(await this.content.setupMail(this.apiKey(), this.sender()));
      this.apiKey.set('');
    });
  }

  async test(kind: 'dm' | 'email'): Promise<void> {
    await this.run(async () => this.view.set(await this.content.testDelivery(kind)));
  }

  reloadScreen(): void {
    void this.content.screen().then((s) => this.screen.set(s)).catch(() => undefined);
  }

  async setScreen(change: Partial<ScreenSettings>): Promise<void> {
    const s = this.screen();
    if (!s) return;
    await this.run(async () => {
      this.screen.set(await this.content.setScreen({ ...s, ...change }));
      this.stamp.set(Date.now());
    });
  }

  async retry(id: number): Promise<void> {
    await this.run(async () => this.view.set(await this.content.retryDelivery(id)));
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
