import { Component, inject, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { NotificationView, problem } from '../api';
import { ContentApi } from '../content-api';
import { Alerts } from '../alerts';

/** `/notifications`: the bell's list. Opening it marks everything read. */
@Component({
  selector: 'ml-notifications',
  imports: [RouterLink],
  styles: `
    .bell { list-style: none; padding: 0; margin: 0; }
    .bell li { padding: 0.6rem 0; border-bottom: 1px solid var(--line); }
    .bell li.unread { font-weight: 600; }
  `,
  template: `
    <div class="bar">
      <h1>Notifications</h1>
      <a routerLink="/me" class="button secondary small">Settings</a>
    </div>
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    <section class="card">
      <ul class="bell">
        @for (n of list(); track n.id) {
          <li [class.unread]="!n.read">
            @if (n.link) { <a [routerLink]="n.link">{{ n.text }}</a> } @else { {{ n.text }} }
            <span class="muted small"> · {{ ago(n.created) }}</span>
          </li>
        } @empty {
          <li class="muted">{{ loaded() ? 'Nothing yet.' : 'Asking the board…' }}</li>
        }
      </ul>
    </section>
  `,
})
export class Notifications implements OnInit {
  private readonly content = inject(ContentApi);
  private readonly alerts = inject(Alerts);
  readonly list = signal<NotificationView[]>([]);
  readonly loaded = signal(false);
  readonly error = signal('');

  async ngOnInit(): Promise<void> {
    try {
      const first = await this.content.notifications();
      this.list.set(first.notifications);
      this.loaded.set(true);
      if (first.unread > 0) {
        await this.content.markRead(first.notifications[0]?.id ?? 0);
        this.alerts.unread.set(0);
      }
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  ago(t: number): string {
    const s = Math.max(0, Math.floor(Date.now() / 1000) - t);
    if (s < 60) return 'just now';
    if (s < 3600) return `${Math.floor(s / 60)} min ago`;
    if (s < 86_400) return `${Math.floor(s / 3600)} h ago`;
    return new Date(t * 1000).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
  }
}
