import { Component, inject, input, OnDestroy, OnInit, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { Api } from '../api';
import { OAuthStatus, oauthStep } from '../mastodon';

/**
 * `/signin/finishing?t=<ticket>`: where the Mastodon instance sends the
 * browser back. The board finishes the sign-in (or link) on its worker;
 * this page waits for it, then goes on.
 */
@Component({
  selector: 'ml-finishing',
  imports: [RouterLink],
  template: `
    <section class="card narrow">
      @if (error()) {
        <h1>That didn't work</h1>
        <p class="error" role="alert">{{ error() }}</p>
        <p><a routerLink="/signin">Back to sign-in</a> · <a routerLink="/me">Settings</a></p>
      } @else {
        <h1>Finishing…</h1>
        <p class="muted">Checking with your Mastodon instance.</p>
      }
    </section>
  `,
})
export class Finishing implements OnInit, OnDestroy {
  private readonly api = inject(Api);
  private readonly router = inject(Router);
  /** Query parameter: the sign-in's ticket. */
  readonly t = input<string>('');
  readonly error = signal('');
  private timer?: ReturnType<typeof setTimeout>;
  private tries = 0;

  ngOnInit(): void {
    void this.check();
  }

  ngOnDestroy(): void {
    clearTimeout(this.timer);
  }

  private async check(): Promise<void> {
    try {
      const s = await this.api.get<OAuthStatus>(`/api/v1/auth/oauth/${encodeURIComponent(this.t())}`, 'OAuthStatus');
      const step = oauthStep(s);
      if (step.kind === 'done') {
        await this.api.refresh();
        await this.router.navigateByUrl(step.to);
        return;
      }
      if (step.kind === 'fail') return this.error.set(step.message);
      if (step.kind === 'go') return void (location.href = step.url);
    } catch {
      // The board may be busy for a moment: keep trying.
    }
    if (++this.tries > 60) return this.error.set('The board took too long; try again');
    this.timer = setTimeout(() => void this.check(), 500);
  }
}
