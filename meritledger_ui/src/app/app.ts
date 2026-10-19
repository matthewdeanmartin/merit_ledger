import { Component, inject } from '@angular/core';
import { Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { Api } from './api';
import { footerLine } from './footer';
import { UpdateRecovery } from './update-recovery';

@Component({
  selector: 'ml-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  template: `
    <header>
      <a class="brand" routerLink="/">☸️ Merit Ledger</a>
      @if (api.me(); as me) {
        <nav aria-label="Main">
          <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{ exact: true }">Today</a>
          <a routerLink="/timer" routerLinkActive="active">Timer</a>
          <a routerLink="/build" routerLinkActive="active">Practice</a>
          <a routerLink="/calendar" routerLinkActive="active">Calendar</a>
          <a routerLink="/house" routerLinkActive="active">House</a>
          <a routerLink="/practices" routerLinkActive="active">Practices</a>
          <a [routerLink]="['/p', me.id]" routerLinkActive="active">{{ me.display_name }}</a>
          <a routerLink="/me" routerLinkActive="active" aria-label="Settings">⚙︎</a>
          @if (me.role === 'admin') {
            <a routerLink="/admin" routerLinkActive="active">Household</a>
          }
          <button type="button" class="link" (click)="signOut()">Sign out</button>
        </nav>
      } @else if (api.status() && !api.status()!.setup_needed) {
        <nav aria-label="Main">
          <a routerLink="/house" routerLinkActive="active">House</a>
          <a routerLink="/calendar" routerLinkActive="active">Calendar</a>
          <a routerLink="/build" routerLinkActive="active">Practice</a>
          <a routerLink="/signin" routerLinkActive="active">Sign in</a>
        </nav>
      }
    </header>
    @if (recovery.failed()) {
      <div class="overlay" role="alertdialog" aria-labelledby="update-title">
        <h1 id="update-title">Couldn't finish updating</h1>
        <p>Merit Ledger was updated, but this page couldn't load the new version. The board may still be
          restarting. Try again in a moment.</p>
        <button type="button" (click)="recovery.retry()">Try again</button>
      </div>
    } @else if (recovery.updating()) {
      <div class="overlay" role="status" aria-live="polite">
        <div class="spinner" aria-hidden="true"></div>
        <h1>Merit Ledger was updated</h1>
        <p>Loading the new version…</p>
      </div>
    }
    <main>
      @if (api.ready() && !api.status()) {
        <p class="error card" role="alert">The board is not responding. Is it on, and are you on the home Wi-Fi?</p>
      }
      <router-outlet />
    </main>
    @if (api.status(); as s) {
      <footer>{{ footer(s) }}</footer>
    }
  `,
})
export class App {
  protected readonly api = inject(Api);
  protected readonly recovery = inject(UpdateRecovery);
  private readonly router = inject(Router);
  protected readonly footer = footerLine;

  async signOut(): Promise<void> {
    await this.api.signOut().catch(() => undefined);
    await this.router.navigateByUrl('/signin');
  }
}
